export type ObjectKind =
  | 'workspace-manifest'
  | 'document-snapshot'
  | 'workspace-head'

export interface KdfParameters {
  algorithm: 'argon2i'
  memory_kib: number
  iterations: number
  parallelism: number
  output_len: number
  profile: 'draft' | 'balanced' | 'dev-fast'
}

export interface CryptoSuiteConfig {
  suite_id: string
  kdf: KdfParameters
  hash: string
  aead: string
  signature: string
}

export interface WorkspaceConfig {
  version: number
  workspace_id: string
  server_url: string
  created_at: string
  page_size: number
  crypto: CryptoSuiteConfig
  read_key_check_b64: string
  write_public_key_b64: string
}

export interface HeadRef {
  workspace_id: string
  head_id: string
  updated_at: string
}

export interface StoredObject {
  version: number
  kind: ObjectKind
  suite_id: string
  salt_b64: string
  nonce_b64: string
  ciphertext_b64: string
  signer_b64: string
  signature_b64: string
}

export interface DocumentSummary {
  document_id: string
  title: string
  latest_revision_id: string
  updated_at: string
}

export interface WorkspaceManifest {
  version: number
  workspace_id: string
  sequence: number
  previous_manifest_id: string | null
  updated_at: string
  documents: DocumentSummary[]
}

export interface DocumentSnapshot {
  version: number
  workspace_id: string
  document_id: string
  title: string
  body: string
  previous_revision_id: string | null
  created_at: string
}

export interface WorkspaceHead {
  version: number
  workspace_id: string
  sequence: number
  manifest_object_id: string
  previous_head_id: string | null
  created_at: string
  signer_b64: string
}

export interface HealthResponse {
  status: string
}

export interface DerivedWorkspaceKeys {
  rootKey: Uint8Array
  seedKey: Uint8Array
  fsKey: Uint8Array
  readKeyCheck: Uint8Array
  writeSecretKey?: Uint8Array
  writePublicKey: Uint8Array
}

export interface LoadedWorkspace {
  config: WorkspaceConfig
  currentHeadId: string | null
  currentHead: WorkspaceHead | null
  currentManifestId: string | null
  manifest: WorkspaceManifest
  documents: DocumentSnapshot[]
  health: HealthResponse
  keys: DerivedWorkspaceKeys
}
