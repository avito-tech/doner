use amqp;
use errors::*;
use quire::{parse_config, Options};
use quire::validate::Anything;
use std::path::Path;
use rustls::ClientConfig;
use std::io::BufReader;
use std::fs::File;

// TODO: config validation

#[derive(RustcDecodable, Debug, Clone)]
pub struct Config {
    pub downloader: DownloaderConfig,
    pub amqp_remote: AmqpConfig,
    pub amqp_local: AmqpConfig,
    pub queues: Queues,
    pub tls: TlsConfig,
    pub http: HttpConfig,
}

#[derive(RustcDecodable, Debug, Clone)]
pub struct DownloaderConfig {
    pub threads: usize,
    pub dns_threads: usize,
    pub save_path: String,
}

#[derive(RustcDecodable, Debug, Clone)]
pub struct Queues {
    pub job: String,
    pub pending: String,
}

#[derive(RustcDecodable, Debug, Clone)]
pub struct AmqpConfig {
    pub host: String,
    pub port: u16,
    pub login: String,
    pub password: String,
}

#[derive(RustcDecodable, Debug, Clone)]
pub struct TlsConfig {
    ca_path: String,
}

#[derive(RustcDecodable, Debug, Clone)]
pub struct HttpConfig {
    pub prefix: String,
    host: String,
    port: u16,
    threads: usize,
}

#[derive(RustcDecodable, Debug, Clone)]
pub struct HttpStaticConfig {
    pub host: String,
    pub port: u16,
    pub threads: usize,
    pub root: String,
}

impl Config {
    pub fn parse<P: AsRef<Path>>(path: P) -> Result<Config> {
        parse_config(path, &Anything, &Options::default()).map_err(|e| {
                                                                       ErrorKind::YamlError(e)
                                                                           .into()
                                                                   })
    }

    pub fn get_tls_config(&self) -> Result<ClientConfig> {
        let mut tls_config = ClientConfig::new();
        let pem_file = File::open(&self.tls.ca_path)?;
        let mut pem_file = BufReader::new(pem_file);
        tls_config.root_store.add_pem_file(&mut pem_file).map_err(|()|ErrorKind::ConfigError("reading pem file".to_string()))?;
        Ok(tls_config)
    }

    pub fn get_http_config(&self) -> Result<HttpStaticConfig> {
        Ok(HttpStaticConfig {
               host: self.http.host.clone(),
               port: self.http.port.clone(),
               threads: self.http.threads.clone(),
               root: self.downloader.save_path.clone(),
           })
    }
}

impl Into<amqp::Options> for AmqpConfig {
    fn into(self) -> amqp::Options {
        let mut opts = amqp::Options::default();
        opts.host = self.host;
        opts.port = self.port;
        opts.login = self.login;
        opts.password = self.password;
        opts.vhost = "/".to_string();
        opts
    }
}
