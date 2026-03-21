# EasySafe - Features

## Features

### Draft Protocol Documentation
- **Stability**: stable
- **Description**: The repository contains the original EasySafe draft documentation describing the filesystem, discovery, peer-to-peer, and threat-model design.
- **Properties**:
  - `easysafe-doc/README.md` indexes the document set and chapter structure.
  - The documentation covers filesystem, DHT, peer-to-peer, PBKDF, threat model, and rationale sections.
  - The draft identifies itself as pre-release and unstable.
- **Test Criteria**:
  - [x] `easysafe-doc/README.md` exists at the repository root subtree
  - [x] Documentation files exist under `easysafe-doc/03-Filesystem`, `easysafe-doc/04-DHT`, and `easysafe-doc/05-P2P`
  - [x] The draft status is stated in `easysafe-doc/README.md` and `easysafe-doc/01-Document/01-01-about.md`

### Project Planning Docs
- **Stability**: stable
- **Description**: The repository contains root-level planning and review documents for a modern EasySafe proof of concept.
- **Properties**:
  - `TODO.md` defines a phased implementation plan.
  - `CRYPTO_FEEDBACK.md` records algorithm and protocol modernization guidance.
  - `AGENTS.md` records agent instructions and project-specific learnings.
- **Test Criteria**:
  - [x] `TODO.md` exists
  - [x] `CRYPTO_FEEDBACK.md` exists
  - [x] `AGENTS.md` exists

### Rust Core Library
- **Stability**: in-progress
- **Description**: A Rust crate for key derivation, encrypted object formats, signed revisions, and deterministic test vectors.
- **Properties**:
  - Derives read, write, and filesystem keys from passphrase material
  - Encodes and decodes config, object, and revision formats
  - Verifies signatures on writer-authenticated objects
  - Keeps algorithm and storage boundaries modular enough for later replacement
- **Test Criteria**:
  - [x] Cargo workspace contains `easysafe-core`
  - [x] At least one round-trip unit test passes for encrypted object encoding and decoding
  - [x] `cargo test` passes for the Rust workspace

### Central Sync Node
- **Stability**: in-progress
- **Description**: A server that stores opaque encrypted blobs and revision heads without needing plaintext access.
- **Properties**:
  - Accepts encrypted object uploads
  - Returns stored encrypted objects by identifier
  - Publishes and retrieves latest revision heads for a workspace
  - Implements the central sync approach chosen for the initial POC
- **Test Criteria**:
  - [x] Cargo workspace contains `easysafe-node`
  - [x] A client can upload and download an encrypted object
  - [x] A client can publish and fetch a workspace head

### CLI Workspace Tool
- **Stability**: in-progress
- **Description**: A Rust CLI for initializing workspaces, creating revisions, and synchronizing with a central node.
- **Properties**:
  - Initializes a workspace from a passphrase
  - Creates and inspects encrypted revisions
  - Pushes to and pulls from the sync node
  - Serves as the first implementation surface before any frontend exists
- **Test Criteria**:
  - [x] Cargo workspace contains `easysafe-cli`
  - [x] `easysafe init` creates local workspace metadata
  - [x] `easysafe push` and `easysafe pull` complete against a local node

### Encrypted Document Model
- **Stability**: in-progress
- **Description**: A document-focused data model for the first user-facing workload, with revision history and encrypted storage.
- **Properties**:
  - Stores UTF-8 text documents as encrypted objects
  - Maintains a manifest mapping document metadata to latest revision ids
  - Preserves readable local revision history for authorized clients
- **Test Criteria**:
  - [x] A client can create a new document
  - [ ] A client can save multiple revisions of the same document
  - [x] A second authorized client can pull and decrypt the latest revision

### Web Frontend
- **Stability**: in-progress
- **Description**: A browser-based interface for unlocking a workspace, listing documents, editing them, and saving encrypted revisions.
- **Properties**:
  - Lists available documents for an unlocked workspace
  - Opens and edits a document in the browser
  - Saves encrypted revisions through the sync node
- **Test Criteria**:
  - [x] The React/TypeScript app builds successfully
  - [x] The frontend includes workspace unlock, document list, editor, and save flows
  - [ ] Saved revisions are readable from the CLI after sync

### Realtime Collaboration
- **Stability**: planned
- **Description**: Multi-user document collaboration using a CRDT-based model rather than ad hoc merge logic.
- **Properties**:
  - Concurrent edits converge without manual merge resolution for ordinary text changes
  - The implementation uses a CRDT-compatible stack such as `yrs` or Yjs-compatible tooling
  - Collaboration state can be synchronized through the server without requiring plaintext access to stored documents
- **Test Criteria**:
  - [ ] Two clients can edit the same document concurrently
  - [ ] Both clients converge to the same final state after sync
  - [ ] Conflict-free updates persist and reload correctly
