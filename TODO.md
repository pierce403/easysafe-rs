# EasySafe POC TODO

## Goal

Build the smallest credible EasySafe-style system that proves the core idea:

- passphrase-derived access
- encrypted object storage
- signed revisions
- multi-client sync

Do not start by implementing the full 2019 draft. The draft is strongest on encrypted storage, key separation, and revisioned data. It is weakest where major protocol details are left unspecified.

Current implementation stance:

- build the Rust `CLI` and central `node` as currently described
- stay close to the draft before introducing deliberate crypto changes
- keep module boundaries loose enough that crypto and storage components can be swapped later

## POC Shape

Start with three Rust components:

1. `easysafe-core`
2. `easysafe-node`
3. `easysafe-cli`

The first POC should use a central sync server, not a DHT or full peer-to-peer swarm.

## Phase 0: Spec Triage

- Decide whether the goal is draft compatibility or a modernized reimplementation inspired by the draft.
- Until further feedback arrives, prefer draft fidelity over modernization where the draft is specific enough to implement.
- Identify which parts of the 2019 document are normative for a POC and which are design notes only.
- Freeze a narrow v0 spec for:
  - key derivation
  - object format
  - revision format
  - server API
  - CLI behavior
- Document every place where the draft is incomplete, ambiguous, or internally inconsistent.

## Phase 1: Rust Workspace

- Create a Cargo workspace.
- Add crates:
  - `easysafe-core`
  - `easysafe-node`
  - `easysafe-cli`
- Isolate algorithm and storage concerns behind explicit modules or traits so later swaps do not require a large rewrite.
- Add shared dependencies for:
  - serialization
  - crypto
  - async runtime
  - error handling
  - secrets handling and zeroization
- Add a basic integration test harness covering client/server round trips.

## Phase 2: Core Crypto and Formats

- Implement passphrase-based key derivation.
- Implement read/write key separation.
- Implement config file encoding and decoding.
- Implement FSID derivation.
- Keep the first implementation close to the draft where the draft is concrete enough to do so.
- Avoid speculative crypto substitutions until draft deviations are explicitly approved.
- Implement encrypted object format for:
  - small inline values
  - single-page blobs
  - multi-page blobs
- Implement signed revision objects that point to a root manifest.
- Add test vectors for all deterministic derivations and encodings.
- Add property tests for serialize/deserialize stability.
- Version formats and keep crypto parameters explicit in encoded metadata wherever practical.

## Phase 3: Encrypted Object Store

- Define a content-addressed or tag-addressed local object store API.
- Support storing:
  - config
  - pages
  - tree chunks
  - revision objects
  - manifests
- Add garbage-collection strategy later; do not block the POC on it.
- Ensure the store can be backed by the local filesystem first.

## Phase 4: Central Sync Node

- Build a server that stores opaque encrypted blobs and signed revision heads.
- Keep the server plaintext-blind.
- Minimal API surface:
  - create or discover workspace by public identifier
  - upload object
  - download object
  - publish head
  - fetch latest head
  - list recent heads
- Add authentication only if needed for abuse control; do not confuse transport auth with workspace crypto.
- Use ordinary TLS for the POC.

## Phase 5: CLI

- `easysafe init`
- `easysafe info`
- `easysafe put <path>`
- `easysafe get <id>`
- `easysafe ls`
- `easysafe cat <id>`
- `easysafe commit`
- `easysafe history`
- `easysafe push`
- `easysafe pull`
- `easysafe share-read`
- `easysafe share-write`

The CLI should make the system demonstrable before any GUI exists.

## Phase 6: Data Model for Documents

- Treat the first user-facing workload as encrypted documents, not a generic mounted filesystem.
- Define a manifest format:
  - document id
  - title
  - creation/update timestamps
  - latest revision id
  - optional metadata safe to expose only to authorized clients
- Support binary attachments later; start with UTF-8 text documents.
- Keep revision history explicit and inspectable.

## Phase 7: Frontend

- Build the frontend only after the Rust core and sync path work.
- Start with a thin web app that can:
  - unlock a workspace
  - list documents
  - open a document
  - edit locally
  - save a new encrypted revision
- A Google Docs-like appearance is reasonable.
- Google Docs-like real-time collaboration is a separate milestone.

## Phase 8: Realtime Collaboration

- If collaborative editing matters, use a CRDT instead of inventing merge logic.
- Evaluate:
  - `yrs` / Yjs-compatible model
  - ProseMirror or TipTap integration
- Encrypt CRDT updates client-side before persistence where practical.
- Be explicit about what the server learns:
  - object sizes
  - timing
  - access patterns
  - collaboration topology

## Explicit Non-Goals for v0

- no DHT
- no Kademlia implementation
- no custom Noise handshake variant
- no packet-length obfuscation
- no seed-only peers
- no FUSE mount
- no metadata-hiding guarantees beyond encrypted-at-rest objects and opaque server storage

## Risks and Design Questions

- The draft does not fully specify several important plaintext structures and protocol details.
- The current design may be better framed as an encrypted revisioned object store than a filesystem.
- “Looks like Google Docs” is mostly a product and UX task.
- “Behaves like Google Docs” is a distributed systems problem and should not drive v0 scope.
- A compatibility target that is too strict may force poor crypto and protocol decisions.
- Premature crypto modernization may make it harder to compare the implementation against the original draft.

## Suggested Demo Milestone

Reach this before expanding scope:

1. Initialize a workspace from a passphrase.
2. Create a document locally.
3. Commit an encrypted revision.
4. Push to a central node.
5. Pull and decrypt from a second machine.
6. Show revision history and signature verification.

If that works cleanly, the project is real enough to justify a frontend.
