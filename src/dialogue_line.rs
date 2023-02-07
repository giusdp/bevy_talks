use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct DialogueLine {
    pub id: i32,
    pub text: String,
    pub talker: Option<String>,
    pub choices: Option<Vec<Choice>>,
    pub next: Option<i32>,
    pub start: Option<bool>,
    pub end: Option<bool>,
}

impl DialogueLine {
    pub fn is_end(&self) -> bool {
        self.choices.is_none() && self.next == Some(-1)
    }

    pub fn has_next(&self) -> bool {
        self.next.is_some()
    }

    pub fn has_choices(&self) -> bool {
        self.choices.is_some()
    }
}
#[derive(Debug, Deserialize, Clone)]
pub struct Choice {
    pub text: String,
    pub next: i32,
}
