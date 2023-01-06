use crate::Dialogue;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RepoError {
    #[error("unable to read {name}: {source}")]
    ReadResource {
        source: std::io::Error,
        name: String,
    },
    #[error("unable to parse json from resource: {0}")]
    ParseResource(String),
    #[error("unable to connect to source: {0}")]
    AccessFailure(String),
}

pub trait Repo {
    fn find(&self, name: &str) -> Result<Vec<Dialogue>, RepoError>;
}
