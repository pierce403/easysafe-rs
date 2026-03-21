use std::convert::TryInto;

use argon2::{Algorithm, Argon2, Params, Version};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use blake2::Blake2b512;
use blake2::Digest;
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Nonce};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::RngCore;
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::error::{EasySafeError, Result};
use crate::model::{ObjectKind, StoredObject, WorkspaceConfig};

pub const ARGON2_SALT: &[u8] = b"easysafe-argon2-salt";
const ZERO_NONCE: [u8; 12] = [0u8; 12];

#[derive(Debug)]
pub struct DerivedWorkspaceKeys {
    pub root_key: [u8; 32],
    pub seed_key: [u8; 32],
    pub fs_key: [u8; 32],
    pub read_key_check: [u8; 32],
    pub write_signing_key: Option<SigningKey>,
    pub write_public_key: VerifyingKey,
}

pub fn derive_workspace_keys(
    read_passphrase: &str,
    write_passphrase: Option<&str>,
    config: &WorkspaceConfig,
) -> Result<DerivedWorkspaceKeys> {
    let read_raw = pbkdf(read_passphrase, config)?;
    let root_key = derive_subkey(&read_raw, b"easysafe-read-key", &[])?;
    let seed_key = derive_subkey(&read_raw, b"easysafe-seed-key", &[])?;
    let fs_key = root_key;
    let read_key_check = derive_subkey(&root_key, b"easysafe-read-check", &[])?;
    let encoded_check = STANDARD.encode(read_key_check);

    if !config.read_key_check_b64.is_empty() && encoded_check != config.read_key_check_b64 {
        return Err(EasySafeError::InvalidPassphrase);
    }

    let write_secret = write_passphrase.unwrap_or(read_passphrase);
    let write_raw = pbkdf(write_secret, config)?;
    let write_seed = derive_subkey(&write_raw, b"easysafe-write-key", &[])?;
    let signing_key = SigningKey::from_bytes(&write_seed);
    let write_public_key = signing_key.verifying_key();
    let expected_public_key = STANDARD.encode(write_public_key.to_bytes());

    let write_signing_key = if config.write_public_key_b64.is_empty() {
        Some(signing_key)
    } else if expected_public_key == config.write_public_key_b64 {
        Some(signing_key)
    } else if write_passphrase.is_some() {
        return Err(EasySafeError::InvalidPassphrase);
    } else {
        None
    };

    Ok(DerivedWorkspaceKeys {
        root_key,
        seed_key,
        fs_key,
        read_key_check,
        write_signing_key,
        write_public_key,
    })
}

pub fn derive_workspace_id(
    seed_key: &[u8; 32],
    write_public_key: &VerifyingKey,
    config: &WorkspaceConfig,
) -> String {
    let mut hasher = Blake2b512::new();
    hasher.update(b"easysafe-workspace-id");
    hasher.update(seed_key);
    hasher.update(write_public_key.to_bytes());
    hasher.update(config.crypto.suite_id.as_bytes());
    hasher.update(config.page_size.to_le_bytes());
    hex::encode(hasher.finalize())
}

pub fn derive_object_id(bytes: &[u8]) -> String {
    let mut hasher = Blake2b512::new();
    hasher.update(b"easysafe-object-id");
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

pub fn seal_object<T: Serialize>(
    kind: ObjectKind,
    value: &T,
    config: &WorkspaceConfig,
    keys: &DerivedWorkspaceKeys,
) -> Result<(String, Vec<u8>)> {
    let signing_key = keys
        .write_signing_key
        .as_ref()
        .ok_or(EasySafeError::MissingWriteAccess)?;
    let plaintext = serde_json::to_vec(value)?;
    let mut salt = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut salt);
    let nonce = ZERO_NONCE;
    let object_key = derive_subkey(&keys.fs_key, kind.as_str().as_bytes(), &salt)?;
    let cipher = ChaCha20Poly1305::new((&object_key).into());
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce), plaintext.as_ref())
        .map_err(|err| EasySafeError::Crypto(err.to_string()))?;

    let message = signature_message(1, kind, &salt, &nonce, &ciphertext);
    let signature = signing_key.sign(&message);

    let stored = StoredObject {
        version: 1,
        kind,
        suite_id: config.crypto.suite_id.clone(),
        salt_b64: STANDARD.encode(salt),
        nonce_b64: STANDARD.encode(nonce),
        ciphertext_b64: STANDARD.encode(ciphertext),
        signer_b64: STANDARD.encode(keys.write_public_key.to_bytes()),
        signature_b64: STANDARD.encode(signature.to_bytes()),
    };

    let bytes = serde_json::to_vec_pretty(&stored)?;
    let object_id = derive_object_id(&bytes);
    Ok((object_id, bytes))
}

pub fn open_object<T: DeserializeOwned>(
    expected_kind: ObjectKind,
    bytes: &[u8],
    config: &WorkspaceConfig,
    keys: &DerivedWorkspaceKeys,
) -> Result<T> {
    let stored: StoredObject = serde_json::from_slice(bytes)?;
    if stored.kind != expected_kind {
        return Err(EasySafeError::ObjectKindMismatch {
            expected: expected_kind.as_str(),
            actual: stored.kind.as_str(),
        });
    }
    if stored.suite_id != config.crypto.suite_id {
        return Err(EasySafeError::UnsupportedSuite(stored.suite_id));
    }

    let salt = decode_fixed::<16>(&stored.salt_b64)?;
    let nonce = decode_fixed::<12>(&stored.nonce_b64)?;
    let ciphertext = STANDARD
        .decode(stored.ciphertext_b64.as_bytes())
        .map_err(|err| EasySafeError::Crypto(err.to_string()))?;
    let signature_bytes = decode_fixed::<64>(&stored.signature_b64)?;
    let signer_bytes = decode_fixed::<32>(&stored.signer_b64)?;
    let signature = Signature::from_bytes(&signature_bytes);
    let signer =
        VerifyingKey::from_bytes(&signer_bytes).map_err(|err| EasySafeError::Crypto(err.to_string()))?;
    let message = signature_message(stored.version, stored.kind, &salt, &nonce, &ciphertext);
    signer
        .verify(&message, &signature)
        .map_err(|err| EasySafeError::Crypto(err.to_string()))?;

    let expected_public_key = STANDARD
        .decode(config.write_public_key_b64.as_bytes())
        .map_err(|err| EasySafeError::Crypto(err.to_string()))?;
    if expected_public_key != signer_bytes {
        return Err(EasySafeError::Crypto(
            "stored object signer did not match workspace config".to_string(),
        ));
    }

    let object_key = derive_subkey(&keys.fs_key, stored.kind.as_str().as_bytes(), &salt)?;
    let cipher = ChaCha20Poly1305::new((&object_key).into());
    let plaintext = cipher
        .decrypt(Nonce::from_slice(&nonce), ciphertext.as_ref())
        .map_err(|err| EasySafeError::Crypto(err.to_string()))?;
    Ok(serde_json::from_slice(&plaintext)?)
}

pub fn derive_subkey(parent_key: &[u8; 32], info: &[u8], salt: &[u8]) -> Result<[u8; 32]> {
    let mut hasher = Blake2b512::new();
    hasher.update(b"easysafe-derive-subkey");
    hasher.update(parent_key);
    hasher.update((info.len() as u32).to_le_bytes());
    hasher.update(info);
    hasher.update((salt.len() as u32).to_le_bytes());
    hasher.update(salt);
    let digest = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest[..32]);
    Ok(out)
}

fn pbkdf(passphrase: &str, config: &WorkspaceConfig) -> Result<[u8; 32]> {
    let params = Params::new(
        config.crypto.kdf.memory_kib,
        config.crypto.kdf.iterations,
        config.crypto.kdf.parallelism,
        Some(config.crypto.kdf.output_len as usize),
    )
    .map_err(|err| EasySafeError::Crypto(err.to_string()))?;
    let argon = Argon2::new(Algorithm::Argon2i, Version::V0x13, params);
    let mut out = [0u8; 32];
    argon
        .hash_password_into(passphrase.as_bytes(), ARGON2_SALT, &mut out)
        .map_err(|err| EasySafeError::Crypto(err.to_string()))?;
    Ok(out)
}

fn signature_message(
    version: u32,
    kind: ObjectKind,
    salt: &[u8; 16],
    nonce: &[u8; 12],
    ciphertext: &[u8],
) -> Vec<u8> {
    let mut out = Vec::with_capacity(64 + ciphertext.len());
    out.extend_from_slice(b"easysafe-object-signature");
    out.extend_from_slice(&version.to_le_bytes());
    out.extend_from_slice(kind.as_str().as_bytes());
    out.extend_from_slice(salt);
    out.extend_from_slice(nonce);
    out.extend_from_slice(ciphertext);
    out
}

fn decode_fixed<const N: usize>(value: &str) -> Result<[u8; N]> {
    let bytes = STANDARD
        .decode(value.as_bytes())
        .map_err(|err| EasySafeError::Crypto(err.to_string()))?;
    bytes
        .as_slice()
        .try_into()
        .map_err(|_| EasySafeError::Crypto(format!("expected {N} decoded bytes")))
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::model::{CryptoSuiteConfig, KdfProfile, WorkspaceConfig, WorkspaceHead};

    fn test_config() -> WorkspaceConfig {
        let kdf = KdfProfile::DevFast.parameters();
        let read_raw = {
            let params = Params::new(
                kdf.memory_kib,
                kdf.iterations,
                kdf.parallelism,
                Some(kdf.output_len as usize),
            )
            .unwrap();
            let argon = Argon2::new(Algorithm::Argon2i, Version::V0x13, params);
            let mut out = [0u8; 32];
            argon
                .hash_password_into(b"read-pass", ARGON2_SALT, &mut out)
                .unwrap();
            out
        };
        let root_key = derive_subkey(&read_raw, b"easysafe-read-key", &[]).unwrap();
        let read_key_check = derive_subkey(&root_key, b"easysafe-read-check", &[]).unwrap();
        let write_raw = {
            let params = Params::new(
                kdf.memory_kib,
                kdf.iterations,
                kdf.parallelism,
                Some(kdf.output_len as usize),
            )
            .unwrap();
            let argon = Argon2::new(Algorithm::Argon2i, Version::V0x13, params);
            let mut out = [0u8; 32];
            argon
                .hash_password_into(b"write-pass", ARGON2_SALT, &mut out)
                .unwrap();
            out
        };
        let write_seed = derive_subkey(&write_raw, b"easysafe-write-key", &[]).unwrap();
        let signing_key = SigningKey::from_bytes(&write_seed);
        WorkspaceConfig {
            version: 1,
            workspace_id: "workspace".to_string(),
            server_url: "http://localhost:8080".to_string(),
            created_at: Utc::now(),
            page_size: 4096,
            crypto: CryptoSuiteConfig::new(kdf),
            read_key_check_b64: STANDARD.encode(read_key_check),
            write_public_key_b64: STANDARD.encode(signing_key.verifying_key().to_bytes()),
        }
    }

    #[test]
    fn round_trip_head() {
        let config = test_config();
        let keys = derive_workspace_keys("read-pass", Some("write-pass"), &config).unwrap();
        let head = WorkspaceHead {
            version: 1,
            workspace_id: config.workspace_id.clone(),
            sequence: 1,
            manifest_object_id: "manifest".to_string(),
            previous_head_id: None,
            created_at: Utc::now(),
            signer_b64: config.write_public_key_b64.clone(),
        };
        let (_id, bytes) = seal_object(ObjectKind::WorkspaceHead, &head, &config, &keys).unwrap();
        let parsed: WorkspaceHead =
            open_object(ObjectKind::WorkspaceHead, &bytes, &config, &keys).unwrap();
        assert_eq!(parsed.workspace_id, head.workspace_id);
        assert_eq!(parsed.sequence, head.sequence);
    }
}
