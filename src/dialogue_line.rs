use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct DialogueLine {
    pub(crate) id: i32,
    pub(crate) text: String,
    pub(crate) talker: Option<String>,
    pub(crate) choices: Option<Vec<Choice>>,
    pub(crate) next: Option<i32>,
    pub(crate) start: Option<bool>,
    pub(crate) end: Option<bool>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Choice {
    pub text: String,
    pub next: i32,
}
