use serde_json;
use jobs::reply_to::ReplyTo;

use std::fmt::{self, Write};
use sha2::{Sha256, Digest};
use errors::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Order {
    #[serde(default = "default_id")]
    _id: String,
    url: String,
    reply_to: ReplyTo,
}

fn default_id() -> String {
    String::new()
}

impl Order {
    pub fn get_id(&self) -> String {
        self._id.clone()
    }

    pub fn get_reply_to(&self) -> ReplyTo {
        self.reply_to.clone()
    }

    pub fn get_url(&self) -> String {
        self.url.clone()
    }

    pub fn from_json(data: &str) -> Result<Order> {
        let mut order: Order = serde_json::from_str(data)?;
        let json = serde_json::to_vec(&order)?;
        let mut hasher = Sha256::new();
        hasher.input(json.as_slice());

        hasher
            .result()
            .as_slice()
            .iter()
            .map(|byte| write!(&mut order._id, "{:x}", byte))
            .last();

        Ok(order)
    }
}

impl fmt::Display for Order {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,
               "(Order #{}. From \"{}\" to \"{}\": \"{}\")",
               self._id,
               self.url,
               self.reply_to.endpoint,
               self.reply_to.target)
    }
}
