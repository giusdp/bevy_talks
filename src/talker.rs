use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Talker {
    pub name: String,
    pub asset: String,
}
