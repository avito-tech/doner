use quire::ErrorList;

#[derive(Debug, error_chain)]
pub enum ErrorKind {
    Msg(String),

    #[cfg(unix)]
    #[error_chain(foreign)]
    Io(::std::io::Error),

    #[error_chain(foreign)]
    HyperError(::hyper::error::Error),

    #[error_chain(foreign)]
    Utf8(::std::str::Utf8Error),

    #[error_chain(foreign)]
    HyperUriError(::hyper::error::UriError),

    #[error_chain(foreign)]
    JsonError(::serde_json::Error),

    #[error_chain(foreign)]
    AmqpError(::amqp::AMQPError),

    #[error_chain(foreign)]
    #[cfg(unix)]
    #[error_chain(custom)]
    YamlError(ErrorList),

    #[cfg(unix)]
    #[error_chain(custom)]
    #[error_chain(description = r#"|_| "configuration parsing error""#)]
    #[error_chain(display = r#"|t| write!(f, "invalid toolchain name: '{}'", t)"#)]
    ConfigError(String),

    #[cfg(unix)]
    #[error_chain(custom)]
    #[error_chain(description = r#"|_| "configuration parsing error""#)]
    #[error_chain(display = r#"|t| write!(f, "invalid toolchain name: '{}'", t)"#)]
    Mpsc(String),

    #[cfg(unix)]
    #[error_chain(custom)]
    SchemeUnsupported,
}
