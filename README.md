# EasySafe RS

EasySafe RS is a first-pass Rust and TypeScript implementation inspired by the original EasySafe draft: an encrypted, passphrase-driven document and sync system with a central node, CLI, and browser client.

This repository currently focuses on the practical core:

- a Rust core library for workspace config, key derivation, encrypted objects, manifests, and signed heads
- a central HTTP sync node that stores opaque blobs and published head references
- a Rust CLI for initializing workspaces, committing document revisions, and syncing
- a React/TypeScript frontend that opens an existing workspace from the node, decrypts locally, and publishes new encrypted revisions

This is not a full implementation of the original DHT or peer-to-peer swarm design.

## Status

The project is in an early but working state.

Implemented:

- workspace initialization from read/write passphrases
- encrypted object storage
- signed revisions and published workspace heads
- push/pull sync through a central node
- document listing and reading through the CLI
- browser-based workspace unlock, document editing, and revision publishing

Not implemented:

- DHT peer discovery
- custom Noise transport
- seed-only peers
- FUSE or mounted filesystem support
- real-time collaborative editing

## Repository Layout

- `crates/easysafe-core`
  Shared Rust implementation of config, crypto helpers, object formats, local object store, and workspace operations.
- `crates/easysafe-node`
  Axum-based HTTP node for object storage and published workspace metadata.
- `crates/easysafe-cli`
  CLI for workspace init, commit, listing, reading, history, and sync.
- `frontend`
  React/TypeScript browser client.
- `TODO.md`
  Practical roadmap for the POC.
- `CRYPTO_FEEDBACK.md`
  Notes on algorithm choices, compatibility, and possible future changes.
- `FEATURES.md`
  Feature inventory and current status.
- `AGENTS.md`
  Project-specific instructions and accumulated learnings for coding agents.

Note:

- The original `easysafe-doc/` draft checkout is intentionally not tracked in this repo.
- The root `.gitignore` excludes `easysafe-doc/`, `target/`, `frontend/node_modules/`, and `frontend/dist/`.

## Quick Start

Prerequisites:

- Rust toolchain
- Node.js and npm

### 1. Start the node

```bash
cargo run -p easysafe-node -- --listen 127.0.0.1:8090 --data-dir /tmp/easysafe-node
```

### 2. Initialize a workspace

```bash
mkdir -p /tmp/easysafe-ws1

cargo run -p easysafe-cli -- \
  --dir /tmp/easysafe-ws1 \
  init \
  --server http://127.0.0.1:8090 \
  --kdf-profile dev-fast \
  --read-passphrase read-secret \
  --write-passphrase write-secret
```

### 3. Create a document revision

```bash
cargo run -p easysafe-cli -- \
  --dir /tmp/easysafe-ws1 \
  commit \
  --title "Hello" \
  --content "Hello from EasySafe" \
  --read-passphrase read-secret \
  --write-passphrase write-secret
```

### 4. Push to the node

```bash
cargo run -p easysafe-cli -- --dir /tmp/easysafe-ws1 push
```

### 5. Pull from a second workspace

```bash
mkdir -p /tmp/easysafe-ws2

cargo run -p easysafe-cli -- \
  --dir /tmp/easysafe-ws2 \
  init \
  --server http://127.0.0.1:8090 \
  --kdf-profile dev-fast \
  --read-passphrase read-secret \
  --write-passphrase write-secret

cargo run -p easysafe-cli -- \
  --dir /tmp/easysafe-ws2 \
  pull \
  --read-passphrase read-secret
```

### 6. Inspect documents

```bash
cargo run -p easysafe-cli -- --dir /tmp/easysafe-ws2 ls --read-passphrase read-secret
cargo run -p easysafe-cli -- --dir /tmp/easysafe-ws2 cat <document-id> --read-passphrase read-secret
```

## Frontend

The browser client currently assumes the workspace already exists and has been published through the CLI.

Run it with:

```bash
cd frontend
npm install
npm run dev
```

Then open the Vite app, enter:

- node URL, for example `http://127.0.0.1:8090`
- workspace ID from the initialized workspace
- read passphrase
- optional write passphrase

The frontend will fetch the workspace config and published head from the node, decrypt locally in the browser, and let you publish new encrypted revisions.

## CLI Commands

Current CLI commands:

- `init`
- `info`
- `put`
- `commit`
- `ls`
- `cat`
- `history`
- `push`
- `pull`
- `share-read`
- `share-write`

## Build And Test

Verified commands:

```bash
cargo check
cargo test
cd frontend && npm install
cd frontend && npm run build
```

An end-to-end smoke test has also been run with:

- node on `127.0.0.1:8090`
- one workspace creating and pushing a document
- a second workspace pulling and reading that document successfully

## Current Design Notes

- The implementation stays close to the draft where practical, but it is not a wire-compatible reimplementation of the whole 2019 design.
- The sync path is intentionally centralized for now.
- Crypto and storage code are kept modular enough to change later.
- One current implementation detail is intentionally pragmatic: subkey derivation uses a deterministic Blake2b-based helper rather than a direct `hkdf` composition over the chosen Rust Blake2 types.

## Caveats

- This is early-stage code, not a reviewed production system.
- The browser client and Rust core must stay aligned on object/config formats for interoperability.
- The current frontend is an editor for existing workspaces, not a full browser-first bootstrap flow.
- Real-time collaboration is not implemented yet.

## Related Docs

- [FEATURES.md](FEATURES.md)
- [TODO.md](TODO.md)
- [CRYPTO_FEEDBACK.md](CRYPTO_FEEDBACK.md)
- [AGENTS.md](AGENTS.md)
