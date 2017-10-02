use amqp::{Basic, Channel, Consumer, Options, Session, Table};
use amqp::protocol::basic;

use errors::*;
use std::error::Error as StdError;

use std::sync::Arc;
use jobs::order::Order;
use std::str;

use Global;
use futures::sink::Sink;
use futures::stream::Stream;

use futures::sync::mpsc::UnboundedSender;

use slog::Logger;

//pub type JobSender<T> = MPMCFutSender<T>;
//pub type JobReceiver<T> = MPMCFutReceiver<T>;

pub struct RabbitClient {
    g: Arc<Global>,
    session: Session,
    channel: Channel,
}

impl RabbitClient {
    pub fn connect(g: Arc<Global>, options: Options) -> Result<Self> {
        //    let mut session = Session::new(Options{ vhost: "/".to_string(), .. Default::default()}).ok().expect("Can't create session");
        println!("Start connection with config {:?}", options);
        let mut session = Session::new(options).ok().expect("Can't create session");

        // TODO; what 1 means?
        let channel = session
            .open_channel(1)
            .ok()
            .expect("Error opening channel 1");
        println!("Opened channel: {:?}", channel.id);

        Ok(RabbitClient {
               g: g,
               session: session,
               channel: channel,
           })
    }

    pub fn consume<C: Consumer + 'static>(&mut self, consumer: C, queue_name: String) {
        let log = self.g
            .root_log
            .new(o!("consumed_queue" => queue_name.clone()));
        //queue: &str, passive: bool, durable: bool, exclusive: bool, auto_delete: bool, nowait: bool, arguments: Table

        let queue_declare = self.channel
            .queue_declare(queue_name.clone(),
                           false,
                           true,
                           false,
                           false,
                           false,
                           Table::new());

        info!(log, "Queue declare: {:?}", queue_declare);
        self.channel
            .basic_prefetch(1)
            .ok()
            .expect("Failed to prefetch");
        //consumer, queue: &str, consumer_tag: &str, no_local: bool, no_ack: bool, exclusive: bool, nowait: bool, arguments: Table

        info!(log, "Declaring consumers...");
        let consumer_name = self.channel
            .basic_consume(consumer,
                           queue_name,
                           "".to_string(),
                           false,
                           false,
                           false,
                           false,
                           Table::new());
        info!(log, "Starting consumer {:?}", consumer_name);
        self.channel.start_consuming();

        self.channel.close(200, "Bye").unwrap();
        self.session.close(200, "Good Bye")
    }

    pub fn publish_all<T: Into<Vec<u8>>, S, E>(&mut self, receiver: S, queue_name: String)
        where S: Stream<Item = T, Error = E>,
              E: ::std::fmt::Debug
    {
        let log = self.g
            .root_log
            .new(o!("publishing_to_queue" => queue_name.clone()));
        let properties = basic::BasicProperties {
            content_type: Some("text".to_owned()),
            ..Default::default()
        };

        let queue_declare = self.channel
            .queue_declare(queue_name.clone(),
                           false,
                           true,
                           false,
                           false,
                           false,
                           Table::new());
        info!(log, "Queue declare: {:?}", queue_declare);

        for pending in receiver.wait() {
            match pending {
                Ok(pending) => {
                    match self.channel
                              .basic_publish("",
                                             &queue_name,
                                             true,
                                             false,
                                             properties.clone(),
                                             pending.into()) {
                        Err(e) => error!(log, "publishing error: {:?}", e),
                        _ => (),
                    }
                }
                Err(e) => error!(log, "could not read channel for publishing: {:?}", e),
            }
        }
    }

    pub fn publish<T: Into<Vec<u8>>>(&mut self, message: T, queue_name: String) -> Result<()> {
        let log = self.g
            .root_log
            .new(o!("publishing_to_queue" => queue_name.clone()));
        info!(log, "PUBLISHING!");
        let properties = basic::BasicProperties {
            content_type: Some("text".to_owned()),
            ..Default::default()
        };

        let queue_declare = self.channel
            .queue_declare(queue_name.clone(),
                           false,
                           true,
                           false,
                           false,
                           false,
                           Table::new());
        info!(log, "Queue declare: {:?}", queue_declare);

        self.channel
            .basic_publish("",
                           &queue_name,
                           true,
                           false,
                           properties.clone(),
                           message.into())
            .map_err(|e| ErrorKind::Mpsc(e.description().into()).into())
    }
}

pub trait FromBody {
    fn from_body(body: &Vec<u8>) -> Result<Self> where Self: Sized;
}

pub struct ChannelingConsumer<T: FromBody, S, E>
    where S: Sink<SinkItem = T, SinkError = E> + Send,
          E: StdError
{
    downstream: S,
    log: Logger,
}

impl<T: FromBody, S, E> ChannelingConsumer<T, S, E>
    where S: Sink<SinkItem = T, SinkError = E> + Send,
          E: StdError
{
    pub fn new(g: Arc<Global>, downstream: S) -> Self {
        ChannelingConsumer {
            downstream: downstream,
            log: g.root_log.new(o!("module"=>"pending consumer")),
        }
    }
}

impl<T: FromBody, S, E> Consumer for ChannelingConsumer<T, S, E>
    where S: Sink<SinkItem = T, SinkError = E> + Send,
          E: StdError
{
    fn handle_delivery(&mut self,
                       channel: &mut Channel,
                       deliver: basic::Deliver,
                       _headers: basic::BasicProperties,
                       body: Vec<u8>) {
        let data = T::from_body(&body);
        match data {
            Ok(data) => {
                self.downstream.start_send(data).unwrap();
                self.downstream.poll_complete().unwrap();
                //println!("SEND: {:?}", self.downstream.try_send(data));
            }
            Err(_) => {
                error!(self.log, "couldn't convert data {:?}", body);
            }
        }
        channel.basic_ack(deliver.delivery_tag, false).unwrap();
    }
}

impl FromBody for Vec<u8> {
    fn from_body(body: &Vec<u8>) -> Result<Self> {
        Ok(body.clone())
    }
}

impl FromBody for Order {
    fn from_body(body: &Vec<u8>) -> Result<Self> {
        let order = str::from_utf8(body)
            .map_err(|e| e.into())
            .and_then(|utf| Order::from_json(utf))
            .unwrap();
        Ok(order)
    }
}

pub type UnboundedConsumer<T> = ChannelingConsumer<T,
                                                   UnboundedSender<T>,
                                                   ::futures::sync::mpsc::SendError<T>>;
pub type PendingConsumer = UnboundedConsumer<Order>;
pub type OrderConsumer = UnboundedConsumer<Vec<u8>>;
