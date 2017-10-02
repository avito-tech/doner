use self::Endpoint::*;
use std::fmt;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Endpoint {
    #[serde(rename = "url")]
    Url,
    #[serde(rename = "queue")]
    Queue,
}

impl fmt::Display for Endpoint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Url => write!(f, "endpoint: url"),
            Queue => write!(f, "endpoint: queue"),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ReplyTo {
    pub endpoint: Endpoint,
    pub target: String,
}
