use futures::Future;
use hyper::*;
use hyper::client::*;
use tokio_core::reactor::Handle;
use tokio_core::net::TcpStream;

use tokio_rustls::TlsStream;
use rustls::{ClientConfig, ClientSession};
use tokio_rustls::ClientConfigExt;

use std::sync::Arc;
use std::io;

pub struct HttpsConnector {
    inner: HttpConnector,
    tls_config: Arc<ClientConfig>,
}

impl HttpsConnector {
    pub fn new(threads: usize, handle: &Handle, tls_config: Arc<ClientConfig>) -> Self {
        HttpsConnector {
            inner: HttpConnector::new(threads, &handle.clone()),
            tls_config: tls_config,
        }
    }
}

impl Service for HttpsConnector {
    type Request = Uri;
    type Response = TlsStream<TcpStream, ClientSession>;
    type Error = io::Error;
    type Future = Box<Future<Item = Self::Response, Error = Self::Error> + 'static>;

    fn call(&self, url: Uri) -> Self::Future {
        let tls_config = self.tls_config.clone();
        let host = url.host().unwrap_or("").to_string();
        let future = self.inner
            .call(url)
            .and_then(move |stream| {
                          ClientSession::new(&tls_config, &host);
                          tls_config.connect_async(&host, stream)
                      });
        Box::new(future) as Box<Future<Item = Self::Response, Error = Self::Error> + 'static>
    }
}
