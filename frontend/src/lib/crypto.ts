import { chacha20poly1305 } from '@noble/ciphers/chacha.js'
import { ed25519 } from '@noble/curves/ed25519.js'
import { blake2b } from '@noble/hashes/blake2.js'
import { bytesToHex } from '@noble/hashes/utils.js'
import { argon2i } from 'hash-wasm'

import type {
  DerivedWorkspaceKeys,
  DocumentSnapshot,
  ObjectKind,
  StoredObject,
  WorkspaceConfig,
  WorkspaceHead,
  WorkspaceManifest,
} from './types'
import {
  base64ToBytes,
  bytesToBase64,
  concatBytes,
  decodeUtf8,
  encoder,
  isoNow,
  le32,
  randomDocumentId,
  utf8,
} from './utils'

export const ARGON2_SALT = utf8('easysafe-argon2-salt')

const ZERO_NONCE = new Uint8Array(12)

export async function deriveWorkspaceKeys(
  readPassphrase: string,
  writePassphrase: string | undefined,
  config: WorkspaceConfig,
): Promise<DerivedWorkspaceKeys> {
  const readRaw = await pbkdf(readPassphrase, config)
  const rootKey = deriveSubkey(readRaw, utf8('easysafe-read-key'), new Uint8Array())
  const seedKey = deriveSubkey(readRaw, utf8('easysafe-seed-key'), new Uint8Array())
  const readKeyCheck = deriveSubkey(
    rootKey,
    utf8('easysafe-read-check'),
    new Uint8Array(),
  )

  if (bytesToBase64(readKeyCheck) !== config.read_key_check_b64) {
    throw new Error('Invalid passphrase')
  }

  const writeSecret = writePassphrase || readPassphrase
  const writeRaw = await pbkdf(writeSecret, config)
  const writeSeed = deriveSubkey(
    writeRaw,
    utf8('easysafe-write-key'),
    new Uint8Array(),
  )
  const writePublicKey = ed25519.getPublicKey(writeSeed)
  const encodedPublicKey = bytesToBase64(writePublicKey)

  let writeSecretKey: Uint8Array | undefined
  if (encodedPublicKey === config.write_public_key_b64) {
    writeSecretKey = writeSeed
  } else if (writePassphrase) {
    throw new Error('Invalid write passphrase')
  }

  return {
    rootKey,
    seedKey,
    fsKey: rootKey,
    readKeyCheck,
    writeSecretKey,
    writePublicKey,
  }
}

export function deriveObjectId(bytes: Uint8Array): string {
  return bytesToHex(blake2b(concatBytes(utf8('easysafe-object-id'), bytes), { dkLen: 64 }))
}

export function emptyManifest(workspaceId: string): WorkspaceManifest {
  return {
    version: 1,
    workspace_id: workspaceId,
    sequence: 0,
    previous_manifest_id: null,
    updated_at: isoNow(),
    documents: [],
  }
}

export async function openObject<T>(
  expectedKind: ObjectKind,
  bytes: Uint8Array,
  config: WorkspaceConfig,
  keys: DerivedWorkspaceKeys,
): Promise<T> {
  const stored = JSON.parse(decodeUtf8(bytes)) as StoredObject
  if (stored.kind !== expectedKind) {
    throw new Error(`Expected ${expectedKind}, got ${stored.kind}`)
  }
  if (stored.suite_id !== config.crypto.suite_id) {
    throw new Error(`Unsupported suite ${stored.suite_id}`)
  }

  const salt = base64ToBytes(stored.salt_b64)
  const nonce = base64ToBytes(stored.nonce_b64)
  const ciphertext = base64ToBytes(stored.ciphertext_b64)
  const signature = base64ToBytes(stored.signature_b64)
  const signer = base64ToBytes(stored.signer_b64)
  const expectedSigner = base64ToBytes(config.write_public_key_b64)

  if (bytesToBase64(signer) !== config.write_public_key_b64) {
    throw new Error('Signer mismatch')
  }
  const message = signatureMessage(stored.version, stored.kind, salt, nonce, ciphertext)
  const verified = ed25519.verify(signature, message, signer)
  if (!verified) {
    throw new Error('Signature verification failed')
  }
  if (bytesToBase64(expectedSigner) !== stored.signer_b64) {
    throw new Error('Workspace signer mismatch')
  }

  const objectKey = deriveSubkey(keys.fsKey, utf8(stored.kind), salt)
  const plaintext = chacha20poly1305(objectKey, nonce).decrypt(ciphertext)
  return JSON.parse(decodeUtf8(plaintext)) as T
}

export async function sealObject<T>(
  kind: ObjectKind,
  value: T,
  config: WorkspaceConfig,
  keys: DerivedWorkspaceKeys,
): Promise<{ objectId: string; bytes: Uint8Array }> {
  if (!keys.writeSecretKey) {
    throw new Error('Write access requires the write passphrase')
  }

  const plaintext = encoder.encode(JSON.stringify(value))
  const salt = crypto.getRandomValues(new Uint8Array(16))
  const objectKey = deriveSubkey(keys.fsKey, utf8(kind), salt)
  const ciphertext = chacha20poly1305(objectKey, ZERO_NONCE).encrypt(plaintext)
  const message = signatureMessage(1, kind, salt, ZERO_NONCE, ciphertext)
  const signature = ed25519.sign(message, keys.writeSecretKey)
  const stored: StoredObject = {
    version: 1,
    kind,
    suite_id: config.crypto.suite_id,
    salt_b64: bytesToBase64(salt),
    nonce_b64: bytesToBase64(ZERO_NONCE),
    ciphertext_b64: bytesToBase64(ciphertext),
    signer_b64: bytesToBase64(keys.writePublicKey),
    signature_b64: bytesToBase64(signature),
  }
  const bytes = encoder.encode(JSON.stringify(stored, null, 2))
  const objectId = deriveObjectId(bytes)
  return { objectId, bytes }
}

export async function createDocumentCommit(
  config: WorkspaceConfig,
  keys: DerivedWorkspaceKeys,
  manifest: WorkspaceManifest,
  currentManifestId: string | null,
  currentHeadId: string | null,
  currentHead: WorkspaceHead | null,
  documentId: string | null,
  title: string,
  body: string,
): Promise<{
  revisionId: string
  revisionBytes: Uint8Array
  manifestId: string
  manifestBytes: Uint8Array
  headId: string
  headBytes: Uint8Array
  snapshot: DocumentSnapshot
  nextManifest: WorkspaceManifest
  nextHead: WorkspaceHead
}> {
  const existing = documentId
    ? manifest.documents.find((doc) => doc.document_id === documentId) ?? null
    : null
  const resolvedDocumentId = documentId ?? randomDocumentId()
  const createdAt = isoNow()
  const snapshot: DocumentSnapshot = {
    version: 1,
    workspace_id: config.workspace_id,
    document_id: resolvedDocumentId,
    title,
    body,
    previous_revision_id: existing?.latest_revision_id ?? null,
    created_at: createdAt,
  }
  const { objectId: revisionId, bytes: revisionBytes } = await sealObject(
    'document-snapshot',
    snapshot,
    config,
    keys,
  )

  const documents = manifest.documents
    .filter((doc) => doc.document_id !== resolvedDocumentId)
    .concat([
      {
        document_id: resolvedDocumentId,
        title,
        latest_revision_id: revisionId,
        updated_at: createdAt,
      },
    ])
    .sort((left, right) => left.title.localeCompare(right.title))

  const nextManifest: WorkspaceManifest = {
    version: 1,
    workspace_id: config.workspace_id,
    sequence: manifest.sequence + 1,
    previous_manifest_id: currentManifestId,
    updated_at: createdAt,
    documents,
  }
  const { objectId: manifestId, bytes: manifestBytes } = await sealObject(
    'workspace-manifest',
    nextManifest,
    config,
    keys,
  )

  const nextHead: WorkspaceHead = {
    version: 1,
    workspace_id: config.workspace_id,
    sequence: currentHead ? currentHead.sequence + 1 : 1,
    manifest_object_id: manifestId,
    previous_head_id: currentHeadId,
    created_at: createdAt,
    signer_b64: config.write_public_key_b64,
  }

  const { objectId: headId, bytes: headBytes } = await sealObject(
    'workspace-head',
    nextHead,
    config,
    keys,
  )

  return {
    revisionId,
    revisionBytes,
    manifestId,
    manifestBytes,
    headId,
    headBytes,
    snapshot,
    nextManifest,
    nextHead,
  }
}

async function pbkdf(
  passphrase: string,
  config: WorkspaceConfig,
): Promise<Uint8Array> {
  return argon2i({
    password: passphrase,
    salt: ARGON2_SALT,
    iterations: config.crypto.kdf.iterations,
    parallelism: config.crypto.kdf.parallelism,
    memorySize: config.crypto.kdf.memory_kib,
    hashLength: config.crypto.kdf.output_len,
    outputType: 'binary',
  })
}

function deriveSubkey(
  parentKey: Uint8Array,
  info: Uint8Array,
  salt: Uint8Array,
): Uint8Array {
  return blake2b(
    concatBytes(
      utf8('easysafe-derive-subkey'),
      parentKey,
      le32(info.length),
      info,
      le32(salt.length),
      salt,
    ),
    { dkLen: 32 },
  )
}

function signatureMessage(
  version: number,
  kind: ObjectKind,
  salt: Uint8Array,
  nonce: Uint8Array,
  ciphertext: Uint8Array,
): Uint8Array {
  return concatBytes(
    utf8('easysafe-object-signature'),
    le32(version),
    utf8(kind),
    salt,
    nonce,
    ciphertext,
  )
}
