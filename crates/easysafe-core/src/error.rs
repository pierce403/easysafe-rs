use std::io;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum EasySafeError {
    #[error("workspace metadata not found at {0}")]
    MissingWorkspace(String),
    #[error("workspace already exists at {0}")]
    WorkspaceExists(String),
    #[error("invalid passphrase")]
    InvalidPassphrase,
    #[error("write access requires the write passphrase")]
    MissingWriteAccess,
    #[error("object {0} was not found")]
    MissingObject(String),
    #[error("remote server did not publish a head for this workspace")]
    MissingRemoteHead,
    #[error("remote server did not publish a config for this workspace")]
    MissingRemoteConfig,
    #[error("document {0} was not found")]
    MissingDocument(String),
    #[error("object {expected} failed integrity check; actual id was {actual}")]
    ObjectIdMismatch { expected: String, actual: String },
    #[error("stored object kind mismatch: expected {expected}, got {actual}")]
    ObjectKindMismatch {
        expected: &'static str,
        actual: &'static str,
    },
    #[error("unsupported crypto suite {0}")]
    UnsupportedSuite(String),
    #[error("crypto error: {0}")]
    Crypto(String),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("io error: {0}")]
    Io(#[from] io::Error),
}

pub type Result<T> = std::result::Result<T, EasySafeError>;
