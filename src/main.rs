extern crate futures;
extern crate futures_cpupool;
extern crate tokio_core;

extern crate tokio_rustls;
extern crate rustls;

extern crate tokio_service;
extern crate tokio_proto;
extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate serde_derive;
extern crate error_chain;

#[macro_use]
extern crate derive_error_chain;

extern crate amqp;
extern crate docopt;
extern crate rustc_serialize;
#[macro_use]
extern crate slog;
extern crate slog_term;
extern crate hyper;
extern crate quire;

extern crate sha2;

mod errors;
mod config;
mod amqp_queue;
mod jobs;
mod downloader;
mod distributor;
mod https;
mod http_static;

use config::*;
use downloader::{Downloader, ResponseToFile};
use distributor::Distributor;

use std::thread;
use std::sync::Arc;

use slog::{DrainExt, Logger};

use futures::sync::{mpsc, oneshot};
use futures::sink::Sink;
use futures::stream::Stream;
use futures::{Future, IntoFuture};

use tokio_core::reactor::{Core, Remote};

use amqp_queue::{RabbitClient, OrderConsumer, PendingConsumer};

use rustls::ClientConfig;

pub struct Global {
    config: Config,
    root_log: Logger,
    main_core: Remote,
    tls_config: Arc<ClientConfig>,
}

fn main() {
    let term = slog_term::streamer().stderr().build();

    let drain = term.fuse();
    let root = slog::Logger::root(drain, o!("version"=>"0.1.0", "program"=>"doner"));

    info!(root, "Welcome to Doner");

    // TODO error handling
    let config = Config::parse("config.yaml").unwrap();
    info!(root, "Parsed config: {:?}", config);

    // Start threads
    let mut handlers = Vec::new();

    // Create main event loop for all async tasks
    let (rx, tx) = oneshot::channel();
    let h = thread::Builder::new()
        .name("main_event_loop".into())
        .spawn(move || {
                   let mut core = Core::new().unwrap();
                   let remote = core.remote();
                   rx.send(remote).unwrap();
                   core.run(futures::empty::<(), ()>()).unwrap();
               })
        .unwrap();
    handlers.push(h);
    let remote = tx.wait().unwrap();

    let dthreads = config.downloader.threads;
    let tls_config = config.get_tls_config().unwrap();
    let global = Arc::new(Global {
                              config: config,
                              root_log: root,
                              main_core: remote,
                              tls_config: Arc::new(tls_config),
                          });

    // jobs for order
    let (or_sender, or_receiver) = mpsc::unbounded();

    //let distributor = Distributor::new(rabbit_remote, rabbit_local);

    // Create queue reader from remote rabbit that get tasks from common queue ant put them to channel
    let g = global.clone();
    let h = thread::Builder::new()
        .name("amqp_remote_to_channel".into())
        .spawn(move || {
            let options = g.config.amqp_remote.clone().into();
            let queue_name = g.config.queues.job.clone();

            let mut rabbit = RabbitClient::connect(g.clone(), options).unwrap();
            let consumer = OrderConsumer::new(g.clone(), or_sender);//.map_err(|e|ErrorKind::MpscSend(e).into());

            rabbit.consume(consumer, queue_name);
        }).unwrap();
    handlers.push(h);

    // Create another client receiving common remote jobs from channel and publishing
    // them "locally"
    let g = global.clone();
    let h = thread::Builder::new()
        .name("amqp_channel_to_local".into())
        .spawn(move || {
                   let options = g.config.amqp_local.clone().into();
                   let queue_name = g.config.queues.pending.clone();

                   let mut rabbit = RabbitClient::connect(g.clone(), options).unwrap();
                   rabbit.publish_all(or_receiver, queue_name);
               })
        .unwrap();
    handlers.push(h);

    // Create channels for downloader threads
    let mut dl_receivers = Vec::new();
    let mut dl_senders = Vec::new();
    for _ in 0..dthreads {
        // jobs for downloaded
        let (tx, rx) = mpsc::unbounded();
        dl_receivers.push(rx);
        dl_senders.push(tx);
    }

    // Create queue reader that get pending tasks and pushes jobs to downloader
    let g = global.clone();

    let (rr_sender, rr_receiver) = mpsc::unbounded();

    g.main_core
        .spawn(move |_| {
            let mut cycle = dl_senders.into_iter().cycle();
            rr_receiver
                .for_each(move |msg| {
                              let sender = cycle.next().unwrap();
                              sender.send(msg).then(|_| Ok(()))
                          })
                .then(|_| Ok(()))
        });

    let g = global.clone();
    let h = thread::Builder::new()
        .name("amqp_local_to_download".into())
        .spawn(move || {

            let options = g.config.amqp_local.clone().into();
            let queue_name = g.config.queues.pending.clone();

            let mut rabbit = RabbitClient::connect(g.clone(), options).unwrap();
            let consumer = PendingConsumer::new(g.clone(), rr_sender);

            rabbit.consume(consumer, queue_name);
        })
        .unwrap();
    handlers.push(h);

    // Create static server
    let g = global.clone();
    let h = thread::Builder::new()
        .name("http_static_server".into())
        .spawn(move || {
                   // todo: log start
                   let config = g.config.get_http_config().unwrap().clone();
                   http_static::start(config);
               })
        .unwrap();
    handlers.push(h);

    // Start downloading threads
    for i in 0..dthreads {
        let g = global.clone();
        let downloader_id = format!("downloader {}", i).into();
        let (rx, tx) = oneshot::channel();

        let (cj_sender, cj_receiver) = mpsc::unbounded();
        let dl_receiver = dl_receivers.pop().unwrap();
        let h = thread::Builder::new()
            .name(downloader_id)
            .spawn(move || {
                // completed jobs

                let log = g.root_log
                    .new(o!("thread" => format!("downloader {}", i)));
                info!(log, "DOWNLOADER THREAD");
                // Create the event loop
                let mut core = Core::new().unwrap();

                let elog = log.clone();
                let remote = core.remote().clone();
                let ext_remote = remote.clone();
                let downloader = dl_receiver
                    .fuse()
                    .for_each(move |order| {
                        info!(log, "ORDER");
                        let g = g.clone();
                        let o: ::jobs::order::Order = order;
                        let cj_sender = cj_sender.clone();
                        remote.spawn(move |handle| {
                            let download = Downloader::new(g.clone(), o.get_url(), handle.clone())
                                .unwrap();
                            download
                                .into_future()
                                .and_then(move |response| {
                                              ResponseToFile::new(g.clone(),
                                                                  response,
                                                                  o.clone(),
                                                                  cj_sender)
                                                      .into_future()
                                          })
                                .map_err(|_| ())
                        });
                        Ok(())
                    });
                rx.send((ext_remote.clone(), downloader))
                    .unwrap_or_else(|_| {
                                        warn!(elog.clone(),
                                              "Failed to send created downloader to main thread")
                                    });
                core.run(futures::empty::<(), ()>()).unwrap();
            })
            .unwrap();

        handlers.push(h);

        let (remote, downloader) = tx.wait().unwrap();
        remote.spawn(|handle| Ok(handle.spawn(downloader)));

        // distributor
        let g = global.clone();
        let options = g.config.amqp_remote.clone().into();
        let rabbit_remote = RabbitClient::connect(global.clone(), options).unwrap();

        let mut distributor = Distributor::new(rabbit_remote, global.clone());

        let h = thread::spawn(move || {
            println!("JOB RECEIVER");
            for done in cj_receiver.wait() {
                println!("{:?}", done);

                let done = done.unwrap();
                distributor.distribute(done);
            }
        });
        handlers.push(h);
    }

    handlers.into_iter().map(|h| h.join()).last();
}
