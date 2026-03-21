use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum KdfAlgorithm {
    Argon2i,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum HashAlgorithm {
    Blake2b512,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum AeadAlgorithm {
    ChaCha20Poly1305,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SignatureAlgorithm {
    Ed25519,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ObjectKind {
    WorkspaceManifest,
    DocumentSnapshot,
    WorkspaceHead,
}

impl ObjectKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::WorkspaceManifest => "workspace-manifest",
            Self::DocumentSnapshot => "document-snapshot",
            Self::WorkspaceHead => "workspace-head",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum KdfProfile {
    Draft,
    Balanced,
    DevFast,
}

impl KdfProfile {
    pub fn parameters(self) -> KdfParameters {
        match self {
            Self::Draft => KdfParameters {
                algorithm: KdfAlgorithm::Argon2i,
                memory_kib: 1_048_576,
                iterations: 40,
                parallelism: 16,
                output_len: 32,
                profile: self,
            },
            Self::Balanced => KdfParameters {
                algorithm: KdfAlgorithm::Argon2i,
                memory_kib: 131_072,
                iterations: 3,
                parallelism: 4,
                output_len: 32,
                profile: self,
            },
            Self::DevFast => KdfParameters {
                algorithm: KdfAlgorithm::Argon2i,
                memory_kib: 16_384,
                iterations: 2,
                parallelism: 1,
                output_len: 32,
                profile: self,
            },
        }
    }
}

impl std::str::FromStr for KdfProfile {
    type Err = String;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        match value {
            "draft" => Ok(Self::Draft),
            "balanced" => Ok(Self::Balanced),
            "dev-fast" => Ok(Self::DevFast),
            other => Err(format!("unsupported kdf profile {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KdfParameters {
    pub algorithm: KdfAlgorithm,
    pub memory_kib: u32,
    pub iterations: u32,
    pub parallelism: u32,
    pub output_len: u32,
    pub profile: KdfProfile,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CryptoSuiteConfig {
    pub suite_id: String,
    pub kdf: KdfParameters,
    pub hash: HashAlgorithm,
    pub aead: AeadAlgorithm,
    pub signature: SignatureAlgorithm,
}

impl CryptoSuiteConfig {
    pub fn new(kdf: KdfParameters) -> Self {
        Self {
            suite_id: "draft-v1".to_string(),
            kdf,
            hash: HashAlgorithm::Blake2b512,
            aead: AeadAlgorithm::ChaCha20Poly1305,
            signature: SignatureAlgorithm::Ed25519,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceConfig {
    pub version: u32,
    pub workspace_id: String,
    pub server_url: String,
    pub created_at: DateTime<Utc>,
    pub page_size: u32,
    pub crypto: CryptoSuiteConfig,
    pub read_key_check_b64: String,
    pub write_public_key_b64: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceState {
    pub current_head_id: Option<String>,
    pub last_pulled_at: Option<DateTime<Utc>>,
    pub last_pushed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoredObject {
    pub version: u32,
    pub kind: ObjectKind,
    pub suite_id: String,
    pub salt_b64: String,
    pub nonce_b64: String,
    pub ciphertext_b64: String,
    pub signer_b64: String,
    pub signature_b64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DocumentSummary {
    pub document_id: String,
    pub title: String,
    pub latest_revision_id: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceManifest {
    pub version: u32,
    pub workspace_id: String,
    pub sequence: u64,
    pub previous_manifest_id: Option<String>,
    pub updated_at: DateTime<Utc>,
    pub documents: Vec<DocumentSummary>,
}

impl WorkspaceManifest {
    pub fn empty(workspace_id: impl Into<String>) -> Self {
        Self {
            version: 1,
            workspace_id: workspace_id.into(),
            sequence: 0,
            previous_manifest_id: None,
            updated_at: Utc::now(),
            documents: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DocumentSnapshot {
    pub version: u32,
    pub workspace_id: String,
    pub document_id: String,
    pub title: String,
    pub body: String,
    pub previous_revision_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceHead {
    pub version: u32,
    pub workspace_id: String,
    pub sequence: u64,
    pub manifest_object_id: String,
    pub previous_head_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub signer_b64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HeadRef {
    pub workspace_id: String,
    pub head_id: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoteConfig {
    pub workspace_id: String,
    pub config: WorkspaceConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HealthResponse {
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommitResult {
    pub document_id: String,
    pub revision_id: String,
    pub manifest_id: String,
    pub head_id: String,
}
