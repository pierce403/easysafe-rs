use std::fs;
use std::path::{Path, PathBuf};

use crate::crypto::derive_object_id;
use crate::error::{EasySafeError, Result};

#[derive(Debug, Clone)]
pub struct ObjectStore {
    root: PathBuf,
}

impl ObjectStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn init(&self) -> Result<()> {
        fs::create_dir_all(&self.root)?;
        Ok(())
    }

    pub fn exists(&self, object_id: &str) -> bool {
        self.path_for(object_id).exists()
    }

    pub fn put(&self, object_id: &str, bytes: &[u8]) -> Result<()> {
        let actual = derive_object_id(bytes);
        if actual != object_id {
            return Err(EasySafeError::ObjectIdMismatch {
                expected: object_id.to_string(),
                actual,
            });
        }
        fs::create_dir_all(&self.root)?;
        fs::write(self.path_for(object_id), bytes)?;
        Ok(())
    }

    pub fn get(&self, object_id: &str) -> Result<Vec<u8>> {
        let path = self.path_for(object_id);
        if !path.exists() {
            return Err(EasySafeError::MissingObject(object_id.to_string()));
        }
        Ok(fs::read(path)?)
    }

    pub fn ids(&self) -> Result<Vec<String>> {
        if !self.root.exists() {
            return Ok(Vec::new());
        }
        let mut ids = Vec::new();
        for entry in fs::read_dir(&self.root)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if let Some(stem) = path.file_stem().and_then(|value| value.to_str()) {
                ids.push(stem.to_string());
            }
        }
        ids.sort();
        Ok(ids)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn path_for(&self, object_id: &str) -> PathBuf {
        self.root.join(format!("{object_id}.json"))
    }
}
