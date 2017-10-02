use amqp_queue::RabbitClient;
use tokio_core::reactor::Remote;
use jobs::done::Done;
use jobs::reply_to::Endpoint;
use futures::Future;
use Global;
use Downloader;
use std::sync::Arc;
use futures::IntoFuture;

pub struct Distributor {
    g: Arc<Global>,
    remote: Remote,
    rabbit_remote: RabbitClient,
}

impl Distributor {
    pub fn new(rr: RabbitClient, g: Arc<Global>) -> Distributor {
        Distributor {
            g: g.clone(),
            remote: g.main_core.clone(),
            rabbit_remote: rr,
        }
    }

    pub fn distribute(&mut self, job: Done) {
        match job.reply_to.endpoint {
            Endpoint::Url => {
                println!("Handle url: {}", job.reply_to.target);
                self.handle_http(job);
            }
            Endpoint::Queue => {
                println!("Handle queue: {}", job.reply_to.target);
                self.handle_queue(job);
            }
        }
    }

    fn handle_http(&mut self, job: Done) {
        let g = self.g.clone();
        self.remote
            .spawn(move |handle| {
                       let download = Downloader::new(g, job.reply_to.target, handle.clone())
                           .unwrap();
                       download.into_future().then(|_| Ok(()))
                   });
    }

    fn handle_queue(&mut self, job: Done) {
        let target = job.reply_to.target.clone();
        let res = self.rabbit_remote.publish(job, target);
        match res {
            Ok(_) => println!("OK!!!"),
            Err(e) => println!("ERR: {}!!!", e),
        }
    }
}
