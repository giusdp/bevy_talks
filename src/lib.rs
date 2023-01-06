use serde::Deserialize;

pub mod local;
pub mod repo;

#[derive(Debug, Deserialize)]
pub struct Talker {
    pub name: String,
    pub asset: String,
}

#[derive(Debug, Deserialize)]
pub struct Dialogue {
    pub id: i32,
    pub text: String,
    pub talker: Talker,
}
