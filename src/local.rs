use std::{fs::File, io::Read, path::PathBuf};

use bevy::prelude::Resource;

use crate::{
    repo::{Repo, RepoError},
    Dialogue,
};

#[derive(Resource, Debug)]
pub struct LocalRepo {
    base_path: PathBuf,
}

impl LocalRepo {
    pub fn new(base_folder: &str) -> LocalRepo {
        LocalRepo {
            base_path: PathBuf::from(base_folder),
        }
    }

    fn build_json_path(&self, name: &str) -> PathBuf {
        self.base_path.join(name).with_extension("json")
    }
}

impl Repo for LocalRepo {
    fn load(&self, name: &str) -> Result<Vec<Dialogue>, RepoError> {
        let path = self.build_json_path(name);

        let mut data = String::new();
        let res = File::open(path)
            .and_then(|mut file| file.read_to_string(&mut data))
            .map_err(|e| RepoError::ReadResource {
                source: e,
                name: name.to_string(),
            });

        match res {
            Ok(_) => {
                serde_json::from_str(&data).map_err(|e| RepoError::ParseResource(e.to_string()))
            }
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    // LocalFileSystem Tests
    use super::*;

    use assert_fs::prelude::*;
    use predicates::prelude::*;

    #[test]
    fn build_json_path_returns_path_with_json() {
        let lfs = LocalRepo::new("test-dir");
        let path = lfs.build_json_path("test");
        assert_eq!(path, PathBuf::from("test-dir/test.json"));
    }

    #[test]
    fn build_json_path_already_json() {
        let lfs = LocalRepo::new("test-dir");
        let path = lfs.build_json_path("test.json");
        assert_eq!(path, PathBuf::from("test-dir/test.json"));
    }

    #[test]
    fn build_json_path_multiple_times() {
        let lfs = LocalRepo::new("test-dir");
        let path = lfs.build_json_path("a");
        assert_eq!(path, PathBuf::from("test-dir/a.json"));

        let another_path = lfs.build_json_path("b.json");
        assert_eq!(another_path, PathBuf::from("test-dir/b.json"));
    }
    #[test]
    fn json_resource_not_found() {
        let temp = assert_fs::TempDir::new().unwrap();

        temp.child("text.json").assert(predicate::path::missing());

        let lfs = LocalRepo::new(temp.to_str().unwrap());
        let res = lfs.load("test").unwrap_err();

        assert!(matches!(res, RepoError::ReadResource { .. }));

        temp.close().unwrap();
    }

    #[test]
    fn invalid_json_resource() {
        let temp = assert_fs::TempDir::new().unwrap();

        temp.child("test.json")
            .write_str("not a dialogue json")
            .unwrap();

        let lfs = LocalRepo::new(temp.to_str().unwrap());
        let res = lfs.load("test").unwrap_err();

        assert!(matches!(res, RepoError::ParseResource { .. }));

        temp.close().unwrap();
    }

    #[test]
    fn valid_resource() {
        let temp = assert_fs::TempDir::new().unwrap();

        temp.child("test.json")
            .write_str(
                r#"
                [
                    {
                        "id": 1,
                        "talker": {
                            "name": "John",
                            "asset": "john.png"
                        },
                        "text": "Hello there!",
                        "choices": [
                            {
                                "text": "Yes",
                                "next": 2
                            },
                            {
                                "text": "No",
                                "next": 3
                            }
                        ]
                    }
                ]
                "#,
            )
            .unwrap();

        let lfs = LocalRepo::new(temp.to_str().unwrap());
        let res = lfs.load("test").unwrap();

        assert_eq!(res.len(), 1);

        assert_eq!(res[0].id, 1);
        assert_eq!(res[0].talker.name, "John");
        assert_eq!(res[0].talker.asset, "john.png");
        assert_eq!(res[0].text, "Hello there!");
        assert!(res[0].choices.is_some());
        assert!(res[0].next.is_none());

        temp.close().unwrap();
    }
}
