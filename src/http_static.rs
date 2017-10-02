use futures::future;
use tokio_service::Service;
use hyper::server::Http;
use hyper::{Request, Response, StatusCode};
use hyper::header::{ContentType, ContentDisposition, DispositionType, DispositionParam};
use config::HttpStaticConfig;
use std::io::prelude::*;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::io::Error;

struct StaticServer {
    root: PathBuf,
}

impl StaticServer {
    pub fn new(path: PathBuf) -> Result<Self, Error> {
        Ok(StaticServer { root: path })
    }
}

pub fn start(config: HttpStaticConfig) {
    let address = format!("{}:{}", config.host, config.port.to_string())
        .parse()
        .unwrap();

    let root_path = Path::new(&config.root).to_path_buf();
    //let server = StaticServer { root: root_path };
    //let server = Http::new().bind(&address, move || Ok(server)).unwrap();
    let server = Http::new()
        .bind(&address, move || StaticServer::new(root_path.clone()))
        .unwrap();
    server.run().unwrap();
}

fn read_file<P: AsRef<Path>>(path: P) -> Result<Vec<u8>, Error> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(err) => return Err(err),
    };

    let mut content = Vec::new();
    if let Err(err) = file.read_to_end(&mut content) {
        return Err(err);
    }
    Ok(content)
}

impl Service for StaticServer {
    type Request = Request;
    type Response = Response;
    type Error = ::hyper::error::Error;
    type Future = future::Ok<Response, ::hyper::error::Error>;

    fn call(&self, req: Request) -> Self::Future {
        let resp = Response::new();

        // canonicalization allows to avoid path traversal attacks
        let req_path = Path::new(&req.path()).canonicalize().unwrap();
        let static_path = self.root.join(req_path);

        let path = Path::new(&static_path);
        let resp = if !path.exists() {
            resp.with_status(StatusCode::NotFound)
        } else {
            let file_name = path.file_name().unwrap().to_str().unwrap();
            match read_file(&static_path) {
                Ok(content) => {
                    resp.with_header(ContentType::octet_stream())
                        .with_header(ContentDisposition {
                                         disposition: DispositionType::Attachment,
                                         parameters:
                                             vec![DispositionParam::Ext("filename".to_string(),
                                                                        file_name.to_string())],
                                     })
                        .with_body(content)
                }
                Err(_) => resp.with_status(StatusCode::InternalServerError),
            }
        };

        future::ok(resp)
    }
}
