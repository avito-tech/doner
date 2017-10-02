use serde_json;
use std::fmt;
use jobs::reply_to::ReplyTo;

// todo: Remove pubs
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Done {
    pub id: String,
    pub url: String,
    pub reply_to: ReplyTo,
    pub local_path: String,
    pub static_url: String,
    pub error: Option<String>,
}

impl fmt::Display for Done {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,
               "({}: I downloaded from \"{}\" and put result to endpoint \"{}\" with target \
                \"{}\")",
               self.id,
               self.url,
               self.reply_to.endpoint,
               self.reply_to.target)
    }
}

impl Into<Vec<u8>> for Done {
    fn into(self) -> Vec<u8> {
        serde_json::to_vec(&self).unwrap()
    }
}
