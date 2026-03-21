# AGENTS.md - Instructions for Coding Agents

## Self-Improvement Directive

**IMPORTANT:** When working on this project, update this file whenever you learn something important about the codebase. Capture both successes to repeat and failures to avoid. Include collaborator observations that help future agents work effectively.

This includes:

- verified build, test, lint, or inspection commands
- project conventions discovered from code or docs
- ambiguous or incomplete parts of the original EasySafe draft
- implementation decisions that intentionally diverge from the draft
- common errors and their solutions
- useful file paths and navigation tips
- collaborator preferences about scope, tone, or formatting

Your learnings should make the next agent faster and less error-prone.

## Project Overview

This repository now contains both the original EasySafe draft documentation and a working first-pass implementation.

Current state:

- `easysafe-doc/` contains the 2019 draft specification for an encrypted distributed filesystem, including filesystem, DHT, peer-to-peer, PBKDF, and threat-model chapters.
- `TODO.md` captures the current recommended proof-of-concept direction: Rust core library, central sync node, CLI first, frontend later.
- `CRYPTO_FEEDBACK.md` captures modernization guidance for algorithm choices and protocol scope.
- `crates/easysafe-core` contains the shared Rust core for workspace config, crypto, encrypted objects, manifests, heads, and the local object store.
- `crates/easysafe-node` contains the central sync node HTTP API.
- `crates/easysafe-cli` contains the CLI for init, commit, list, cat, history, push, pull, and sharing.
- `frontend/` contains the React/TypeScript browser client for opening an existing workspace from the node and publishing encrypted revisions.

Working assumption for new implementation work:

- treat the draft as a source of security and storage ideas
- do not assume full wire compatibility is desirable
- prefer a central sync server for v0 over DHT or custom P2P transport
- build the Rust `CLI` and central `node` as the immediate implementation target
- avoid changing crypto beyond the draft until feedback arrives from the original author
- keep architecture modular enough to swap crypto and storage components later

## Project Structure

- `easysafe-doc/`
  - original EasySafe draft documentation
- `crates/easysafe-core/`
  - shared Rust implementation of config, object encryption/signing, local storage, and workspace operations
- `crates/easysafe-node/`
  - Axum-based HTTP node storing opaque objects plus published workspace config/head refs
- `crates/easysafe-cli/`
  - CLI for local workspace management and sync
- `frontend/`
  - React/TypeScript browser client using the same object/config model as the Rust implementation
- `README.md`
  - root project overview, architecture summary, quick start, and current caveats
- `TODO.md`
  - phased roadmap for a practical POC
- `CRYPTO_FEEDBACK.md`
  - crypto review and modernization notes
- `AGENTS.md`
  - instructions and accumulated learnings for future agents
- `FEATURES.md`
  - project feature inventory using the `features.md` format

## Build & Test Commands

Verified commands:

```bash
ls -la
rg --files
sed -n '1,200p' easysafe-doc/README.md
cargo check
cargo test
cd frontend && npm install
cd frontend && npm run build
cargo run -p easysafe-node -- --listen 127.0.0.1:8090 --data-dir /tmp/easysafe-node-smoke
cargo run -p easysafe-cli -- --dir /tmp/easysafe-ws1-XXXXXX init --server http://127.0.0.1:8090 --kdf-profile dev-fast --read-passphrase read-secret --write-passphrase write-secret
cargo run -p easysafe-cli -- --dir /tmp/easysafe-ws1-XXXXXX commit --title "Smoke Doc" --content "Hello from EasySafe smoke test" --read-passphrase read-secret --write-passphrase write-secret
cargo run -p easysafe-cli -- --dir /tmp/easysafe-ws1-XXXXXX push
cargo run -p easysafe-cli -- --dir /tmp/easysafe-ws2-XXXXXX init --server http://127.0.0.1:8090 --kdf-profile dev-fast --read-passphrase read-secret --write-passphrase write-secret
cargo run -p easysafe-cli -- --dir /tmp/easysafe-ws2-XXXXXX pull --read-passphrase read-secret
cargo run -p easysafe-cli -- --dir /tmp/easysafe-ws2-XXXXXX ls --read-passphrase read-secret
cargo run -p easysafe-cli -- --dir /tmp/easysafe-ws2-XXXXXX cat <document-id> --read-passphrase read-secret
```

Verified environment notes:

- the repository root is `/home/pierce/projects/easysafe-rs`
- the repository root is now a Git repository on branch `main`
- the `origin` remote is `git@github.com:pierce403/easysafe-rs.git`
- `easysafe-doc/` contains its own nested `.git/` directory

## Coding Conventions

- Keep root documentation concise, direct, and implementation-oriented.
- Preserve the distinction between:
  - what exists in the 2019 draft
  - what is recommended for a modern POC
- For now, when forced to choose, preserve draft behavior over speculative modernization unless the user explicitly asks otherwise.
- Do not invent unspecified protocol details and present them as part of the original design.
- Prefer standard, audited protocols and crates over custom cryptographic mechanisms unless compatibility is explicitly required.
- Keep crypto and transport code loosely coupled so later swaps are possible without changing all call sites.
- The current implementation uses deterministic Blake2b-based subkey derivation helpers rather than `hkdf` because `hkdf` did not compose cleanly with the chosen `blake2` Rust types.
- For implementation planning, bias toward:
  - Rust workspace
  - `easysafe-core`
  - `easysafe-node`
  - `easysafe-cli`
  - frontend only after the sync path works

## Known Issues & Solutions

- The draft explicitly labels itself unstable and incomplete.
  - Solution: document assumptions and scope cuts before implementing.
- Several important plaintext structures and protocol details are out of scope in the draft.
  - Solution: freeze a narrow modern v0 spec instead of trying to fill in every gap ad hoc.
- `easysafe-doc/` is a nested repository that should not be included in the root repo.
  - Solution: keep `easysafe-doc/` ignored in the root `.gitignore`.
- Browser crypto must match Rust object/config formats exactly enough for interoperability.
  - Solution: keep the stored-object JSON layout simple and deterministic, and verify with real builds and runtime smoke tests.
- The frontend currently assumes an existing workspace already exists on the node.
  - Solution: initialize/publish workspaces with the CLI first, then open them in the browser.

## Dependencies & Tools

Current implementation dependencies of note:

- `argon2`
- `blake2`
- `chacha20poly1305`
- `ed25519-dalek`
- `axum`
- `reqwest`
- `clap`
- `@noble/ciphers`
- `@noble/curves`
- `@noble/hashes`
- `hash-wasm`

## Agent Tips

- Read `TODO.md` before proposing implementation scope.
- Read `CRYPTO_FEEDBACK.md` before preserving 2019 crypto choices.
- Read `easysafe-doc/README.md` before citing the draft structure.
- If asked for a POC, recommend the central-server-plus-CLI path first.
- If making implementation choices before further feedback, stay close to the draft where practical and isolate those choices behind clean interfaces.
- Treat “Google Docs-like frontend” and “Google Docs-like collaboration” as separate milestones.
- When adding new docs, keep them in the repository root unless there is a strong structural reason not to.
- The current CLI workflow is meaningful enough for smoke testing: `init`, `commit`, `push`, `pull`, `ls`, and `cat` work together against the node.
- The frontend is a browser client for existing workspaces, not a full bootstrap flow.

## Rapport & Reflection

- The collaborator is evaluating whether an old friend’s draft can be brought to life with modern tools.
- They prefer concrete, practical artifacts over abstract discussion.
- They have asked for repo-root planning documents as decisions emerge.
- They want the Rust `CLI` and central `node` built as described.
- They want crypto changes beyond the draft deferred until the original author provides feedback.
- They still want enough modularity to swap components later.
- Keep answers concise and direct.
- Update this file whenever a new command, constraint, implementation decision, or collaborator preference becomes clear.

If this file starts growing quickly, consolidate repeated guidance into shorter durable rules and move deeper design detail into dedicated docs.
