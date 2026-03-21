use std::fs;
use std::path::{Path, PathBuf};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use chrono::Utc;
use rand::RngCore;

use crate::crypto::{derive_workspace_id, derive_workspace_keys, open_object, seal_object};
use crate::error::{EasySafeError, Result};
use crate::model::{
    CommitResult, CryptoSuiteConfig, DocumentSnapshot, DocumentSummary, KdfParameters, ObjectKind,
    WorkspaceConfig, WorkspaceHead, WorkspaceManifest, WorkspaceState,
};
use crate::store::ObjectStore;

const META_DIR: &str = ".easysafe";
const CONFIG_FILE: &str = "workspace.json";
const STATE_FILE: &str = "state.json";
const OBJECTS_DIR: &str = "objects";

#[derive(Debug, Clone)]
pub struct Workspace {
    root: PathBuf,
    config: WorkspaceConfig,
    state: WorkspaceState,
}

#[derive(Debug)]
pub struct UnlockedWorkspace {
    workspace: Workspace,
    keys: crate::crypto::DerivedWorkspaceKeys,
}

impl Workspace {
    pub fn init(
        root: impl AsRef<Path>,
        server_url: &str,
        read_passphrase: &str,
        write_passphrase: Option<&str>,
        kdf: KdfParameters,
    ) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        let meta_dir = root.join(META_DIR);
        let config_path = meta_dir.join(CONFIG_FILE);
        if config_path.exists() {
            return Err(EasySafeError::WorkspaceExists(
                meta_dir.display().to_string(),
            ));
        }

        fs::create_dir_all(meta_dir.join(OBJECTS_DIR))?;
        let mut bootstrap = WorkspaceConfig {
            version: 1,
            workspace_id: String::new(),
            server_url: server_url.trim_end_matches('/').to_string(),
            created_at: Utc::now(),
            page_size: 4096,
            crypto: CryptoSuiteConfig::new(kdf),
            read_key_check_b64: String::new(),
            write_public_key_b64: String::new(),
        };
        let keys = derive_workspace_keys(read_passphrase, write_passphrase, &bootstrap)?;

        bootstrap.read_key_check_b64 = STANDARD.encode(keys.read_key_check);
        bootstrap.write_public_key_b64 = STANDARD.encode(keys.write_public_key.to_bytes());
        bootstrap.workspace_id = derive_workspace_id(&keys.seed_key, &keys.write_public_key, &bootstrap);

        let state = WorkspaceState::default();
        write_json(&config_path, &bootstrap)?;
        write_json(&meta_dir.join(STATE_FILE), &state)?;

        Ok(Self {
            root,
            config: bootstrap,
            state,
        })
    }

    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        let meta_dir = root.join(META_DIR);
        let config_path = meta_dir.join(CONFIG_FILE);
        let state_path = meta_dir.join(STATE_FILE);
        if !config_path.exists() {
            return Err(EasySafeError::MissingWorkspace(
                meta_dir.display().to_string(),
            ));
        }
        let config: WorkspaceConfig = read_json(&config_path)?;
        let state = if state_path.exists() {
            read_json(&state_path)?
        } else {
            WorkspaceState::default()
        };
        Ok(Self {
            root,
            config,
            state,
        })
    }

    pub fn unlock(
        self,
        read_passphrase: &str,
        write_passphrase: Option<&str>,
    ) -> Result<UnlockedWorkspace> {
        let keys = derive_workspace_keys(read_passphrase, write_passphrase, &self.config)?;
        Ok(UnlockedWorkspace {
            workspace: self,
            keys,
        })
    }

    pub fn config(&self) -> &WorkspaceConfig {
        &self.config
    }

    pub fn state(&self) -> &WorkspaceState {
        &self.state
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn object_store(&self) -> ObjectStore {
        ObjectStore::new(self.meta_dir().join(OBJECTS_DIR))
    }

    pub fn ingest_object(&self, object_id: &str, bytes: &[u8]) -> Result<()> {
        let store = self.object_store();
        store.init()?;
        store.put(object_id, bytes)
    }

    pub fn object_bytes(&self, object_id: &str) -> Result<Vec<u8>> {
        self.object_store().get(object_id)
    }

    pub fn object_ids(&self) -> Result<Vec<String>> {
        self.object_store().ids()
    }

    pub fn update_current_head(&mut self, head_id: Option<String>) -> Result<()> {
        self.state.current_head_id = head_id;
        self.write_state()
    }

    pub fn touch_pull(&mut self) -> Result<()> {
        self.state.last_pulled_at = Some(Utc::now());
        self.write_state()
    }

    pub fn touch_push(&mut self) -> Result<()> {
        self.state.last_pushed_at = Some(Utc::now());
        self.write_state()
    }

    pub fn current_head_id(&self) -> Option<&str> {
        self.state.current_head_id.as_deref()
    }

    pub fn write_config(&self) -> Result<()> {
        write_json(&self.meta_dir().join(CONFIG_FILE), &self.config)
    }

    fn write_state(&self) -> Result<()> {
        write_json(&self.meta_dir().join(STATE_FILE), &self.state)
    }

    fn meta_dir(&self) -> PathBuf {
        self.root.join(META_DIR)
    }
}

impl UnlockedWorkspace {
    pub fn config(&self) -> &WorkspaceConfig {
        &self.workspace.config
    }

    pub fn state(&self) -> &WorkspaceState {
        &self.workspace.state
    }

    pub fn current_head_id(&self) -> Option<&str> {
        self.workspace.current_head_id()
    }

    pub fn workspace(&self) -> &Workspace {
        &self.workspace
    }

    pub fn workspace_mut(&mut self) -> &mut Workspace {
        &mut self.workspace
    }

    pub fn list_documents(&self) -> Result<Vec<DocumentSummary>> {
        let (_, manifest) = self.current_manifest()?;
        Ok(manifest.documents)
    }

    pub fn read_document(&self, document_id: &str) -> Result<DocumentSnapshot> {
        let (_, manifest) = self.current_manifest()?;
        let summary = manifest
            .documents
            .into_iter()
            .find(|doc| doc.document_id == document_id)
            .ok_or_else(|| EasySafeError::MissingDocument(document_id.to_string()))?;
        self.load_document_revision(&summary.latest_revision_id)
    }

    pub fn head_history(&self) -> Result<Vec<(String, WorkspaceHead)>> {
        let mut history = Vec::new();
        let mut current = self.workspace.state.current_head_id.clone();
        while let Some(head_id) = current {
            let head = self.load_head(&head_id)?;
            current = head.previous_head_id.clone();
            history.push((head_id, head));
        }
        Ok(history)
    }

    pub fn upsert_document(
        &mut self,
        title: String,
        body: String,
        document_id: Option<String>,
    ) -> Result<CommitResult> {
        if self.keys.write_signing_key.is_none() {
            return Err(EasySafeError::MissingWriteAccess);
        }

        let (previous_manifest_id, mut manifest) = self.current_manifest()?;
        let existing = document_id.as_deref().and_then(|target| {
            manifest
                .documents
                .iter()
                .find(|doc| doc.document_id == target)
                .cloned()
        });
        let document_id = document_id.unwrap_or_else(random_document_id);
        let previous_revision_id = existing.as_ref().map(|doc| doc.latest_revision_id.clone());
        let snapshot = DocumentSnapshot {
            version: 1,
            workspace_id: self.workspace.config.workspace_id.clone(),
            document_id: document_id.clone(),
            title: title.clone(),
            body,
            previous_revision_id,
            created_at: Utc::now(),
        };

        let (revision_id, revision_bytes) =
            seal_object(ObjectKind::DocumentSnapshot, &snapshot, &self.workspace.config, &self.keys)?;
        self.workspace.ingest_object(&revision_id, &revision_bytes)?;

        let updated_at = snapshot.created_at;
        if let Some(summary) = manifest
            .documents
            .iter_mut()
            .find(|doc| doc.document_id == document_id)
        {
            summary.title = title.clone();
            summary.latest_revision_id = revision_id.clone();
            summary.updated_at = updated_at;
        } else {
            manifest.documents.push(DocumentSummary {
                document_id: document_id.clone(),
                title,
                latest_revision_id: revision_id.clone(),
                updated_at,
            });
        }
        manifest.documents.sort_by(|left, right| left.title.cmp(&right.title));
        manifest.version = 1;
        manifest.sequence += 1;
        manifest.previous_manifest_id = previous_manifest_id;
        manifest.updated_at = updated_at;

        let (manifest_id, manifest_bytes) =
            seal_object(ObjectKind::WorkspaceManifest, &manifest, &self.workspace.config, &self.keys)?;
        self.workspace.ingest_object(&manifest_id, &manifest_bytes)?;

        let previous_head_id = self.workspace.state.current_head_id.clone();
        let sequence = if let Some(ref head_id) = previous_head_id {
            self.load_head(head_id)?.sequence + 1
        } else {
            1
        };
        let head = WorkspaceHead {
            version: 1,
            workspace_id: self.workspace.config.workspace_id.clone(),
            sequence,
            manifest_object_id: manifest_id.clone(),
            previous_head_id,
            created_at: updated_at,
            signer_b64: self.workspace.config.write_public_key_b64.clone(),
        };
        let (head_id, head_bytes) =
            seal_object(ObjectKind::WorkspaceHead, &head, &self.workspace.config, &self.keys)?;
        self.workspace.ingest_object(&head_id, &head_bytes)?;
        self.workspace.update_current_head(Some(head_id.clone()))?;

        Ok(CommitResult {
            document_id,
            revision_id,
            manifest_id,
            head_id,
        })
    }

    pub fn load_head(&self, object_id: &str) -> Result<WorkspaceHead> {
        let bytes = self.workspace.object_bytes(object_id)?;
        open_object(
            ObjectKind::WorkspaceHead,
            &bytes,
            &self.workspace.config,
            &self.keys,
        )
    }

    pub fn load_manifest(&self, object_id: &str) -> Result<WorkspaceManifest> {
        let bytes = self.workspace.object_bytes(object_id)?;
        open_object(
            ObjectKind::WorkspaceManifest,
            &bytes,
            &self.workspace.config,
            &self.keys,
        )
    }

    pub fn load_document_revision(&self, object_id: &str) -> Result<DocumentSnapshot> {
        let bytes = self.workspace.object_bytes(object_id)?;
        open_object(
            ObjectKind::DocumentSnapshot,
            &bytes,
            &self.workspace.config,
            &self.keys,
        )
    }

    pub fn current_manifest(&self) -> Result<(Option<String>, WorkspaceManifest)> {
        if let Some(head_id) = self.workspace.state.current_head_id.as_deref() {
            let head = self.load_head(head_id)?;
            let manifest = self.load_manifest(&head.manifest_object_id)?;
            Ok((Some(head.manifest_object_id), manifest))
        } else {
            Ok((
                None,
                WorkspaceManifest::empty(self.workspace.config.workspace_id.clone()),
            ))
        }
    }
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    let bytes = fs::read(path)?;
    Ok(serde_json::from_slice(&bytes)?)
}

fn write_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
}

fn random_document_id() -> String {
    let mut bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}
