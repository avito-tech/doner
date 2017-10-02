use std::io::Write;
use std::path::Path;
use std::fs::File;
use std::str::FromStr;
use std::sync::Arc;

use hyper::{Uri, Error};
use hyper::client::{HttpConnector, Response, Config as HConfig};
use futures::{Future, IntoFuture};
use futures::stream::Stream;
use futures::sink::Sink;

use futures::sync::mpsc::UnboundedSender;
use tokio_core::reactor::Handle;

use errors::*;
use https::HttpsConnector;
use Global;
use jobs::order::Order;
use jobs::done::Done;

pub struct Downloader {
    g: Arc<Global>,
    url: Uri,
    run_on: Handle,
}

impl Downloader {
    pub fn new(g: Arc<Global>, url: String, run_on: Handle) -> Result<Self> {
        let url: Uri = Uri::from_str(&url)
            .map_err(|e| ErrorKind::HyperUriError(e))?;
        match url.scheme() {
            Some("http") => Ok(()),
            Some("https") => Ok(()),
            _ => Err(ErrorKind::SchemeUnsupported),
        }?;

        Ok(Downloader {
               g: g,
               url: url,
               run_on: run_on,
           })
    }
}

pub type DownloadFuture = Box<Future<Item = Response, Error = Error>>;

impl IntoFuture for Downloader {
    type Item = Response;
    type Error = Error;
    type Future = DownloadFuture;

    fn into_future(self) -> Self::Future {
        let client_config = HConfig::default();
        self.g.root_log.new(o!("url"=>format!("{}", self.url)));
        let dns_threads = self.g.config.downloader.dns_threads;
        // TODO: other client configuration options
        match self.url.scheme() {
            Some("http") => {
                let handle = self.run_on;
                let connector = HttpConnector::new(dns_threads, &handle);
                let client_config = client_config.connector(connector);
                let client = client_config.build(&handle);
                Box::new(client.get(self.url))
            }
            Some("https") => {
                let handle = self.run_on;
                let tls_config = self.g.tls_config.clone();
                let connector = HttpsConnector::new(dns_threads, &handle, tls_config);
                let client_config = client_config.connector(connector);

                let client = client_config.build(&handle);
                Box::new(client.get(self.url))
            }
            _ => unreachable!(),
        }
    }
}

pub struct ResponseToFile {
    g: Arc<Global>,
    response: Response,
    order: Order,
    done: UnboundedSender<Done>,
}

impl ResponseToFile {
    pub fn new(g: Arc<Global>,
               response: Response,
               order: Order,
               done: UnboundedSender<Done>)
               -> Self {
        ResponseToFile {
            g: g,
            order: order,
            response: response,
            done: done,
        }
    }
}

impl IntoFuture for ResponseToFile {
    type Item = ();
    type Error = Error;
    type Future = Box<Future<Item = Self::Item, Error = Self::Error>>;

    fn into_future(self) -> Self::Future {
        let save_path = Path::new(&self.g.config.downloader.save_path);
        let path = save_path.join(self.order.get_id());

        let mut f = File::create(&path).unwrap();
        let mut d_ch = self.done;
        let mut done = Done {
            id: self.order.get_id(),
            url: self.order.get_url(),
            reply_to: self.order.get_reply_to(),
            local_path: path.to_str().unwrap().to_string(),
            static_url: format!("{}{}", self.g.config.http.prefix, self.order.get_id()),
            error: None,
        };

        Box::new(self.response
                     .body()
                     .for_each(move |chunk| {
                                   f.write_all(&chunk).unwrap();
                                   f.sync_all().map_err(From::from)
                               })
                     .then(move |result| {
            done.error = match result {
                Err(err) => Some(format!("{}", err)),
                _ => None,
            };

            d_ch.start_send(done).unwrap();
            d_ch.poll_complete().unwrap();
            //             d_ch.send(done)
            Ok(())
        }))
    }
}
