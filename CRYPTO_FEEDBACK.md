# EasySafe Crypto Feedback

## Scope

This file is feedback on the 2019 draft as a basis for a modern proof of concept. The goal is not to dismiss the design. The goal is to separate the durable ideas from the risky or outdated details.

## What Still Looks Reasonable

- Key separation between read, write, seed, and filesystem roles is a good design direction.
- Using modern elliptic-curve primitives instead of older RSA-era constructions was the right instinct.
- A passphrase-derived root plus derived subkeys is still a sensible model.
- Signed revisions and signed encrypted objects are useful if untrusted relays or storage nodes are part of the model.
- Keeping the server blind to plaintext in a first POC is realistic and worth preserving.

## Main Recommendation

Treat the draft as a security model and storage model, not as a wire-compatible crypto spec. For a fresh implementation, prefer current audited Rust crates and standard protocols over custom constructions unless compatibility is a hard requirement.

Current project stance:

- do not change draft crypto choices prematurely
- wait for feedback from the original author before diverging beyond the spec
- still design code boundaries so algorithms and formats can be replaced later with limited churn

That means this file should be read as implementation feedback and future migration guidance, not as an instruction to immediately rewrite the draft.

## Specific Feedback

### 1. Argon2i should probably become Argon2id

The draft specifies Argon2i with fixed global parameters. For a new implementation, Argon2id is the usual default choice unless there is a specific side-channel-driven reason not to use it.

Concerns:

- `1 GiB` memory and `40` iterations may be impractical for many real clients.
- Fixed parameters age poorly.
- A server-blind encrypted app still needs acceptable unlock latency on laptops and mobile devices.

Recommendation:

- Use Argon2id.
- Calibrate parameters to target unlock latency on supported devices.
- Store KDF parameters in the config format so they can evolve.

### 2. Repeated zero nonces are acceptable only if key uniqueness is airtight

The draft often uses AEAD or stream encryption with a zero nonce while deriving a fresh key per object or message. This can be safe if the per-use key is guaranteed unique. In practice this is easy to get wrong in implementations and later protocol changes.

Recommendation:

- Prefer constructions with random or extended nonces such as XChaCha20-Poly1305 where possible.
- If zero nonces are kept, write down and test the uniqueness invariants aggressively.

### 3. Raw ChaCha20 plus custom MAC and signature logic is more fragile than standard AEAD

The `taggedEncrypt` construction uses custom composition:

- padded plaintext
- derived per-plaintext salt
- ChaCha20 encryption
- Ed25519 signature on ciphertext
- HMAC tag for filesystem authentication

That may be defensible in the original seed-peer model, but it is harder to reason about and easier to implement incorrectly than a simpler standard AEAD-based object format.

Recommendation:

- For a POC, simplify object encryption to a standard AEAD-based design.
- Add a signature only where writer authentication is actually needed.
- If seed-only verification is a hard requirement, specify that requirement first, then justify the extra authenticated structure from that requirement.

### 4. The signing and DH key story needs clarification

The draft mixes Ed25519 signing concepts with X25519 key exchange concepts and is not always explicit about whether keys are separate, converted, or just similarly sized. That is an implementation footgun.

Recommendation:

- Use explicit separate key types:
  - `Ed25519` for signatures
  - `X25519` for Diffie-Hellman
- Do not rely on implicit conversions.
- Name the keys clearly in code and in the modern spec.

### 5. The custom Noise variant is high risk for a POC

The handshake draft uses a custom `XKpsk4` variant and additional obfuscation logic. Even if the reasoning is understandable, custom transport crypto is not where a new project should spend its safety budget.

Recommendation:

- For the POC, use ordinary TLS or a standard Noise pattern via a well-audited library.
- Revisit the custom transport only if there is a strong product reason.

### 6. DHT message crypto has no forward secrecy

The DHT design uses static keys for stateless encrypted messages and explicitly accepts retroactive decryption after key compromise. That may be acceptable for routing metadata, but it is still a real tradeoff.

Recommendation:

- Avoid building the DHT first.
- Use a central server for v0.
- If decentralized discovery returns later, restate the threat model in current terms before reusing the draft design.

### 7. Traffic and length obfuscation are unlikely to pay for themselves early

The draft spends real complexity on making packets look random and obscuring ciphertext lengths, while also acknowledging that a determined observer can often infer those lengths anyway.

Recommendation:

- Remove traffic camouflage from the POC.
- Focus on object confidentiality, authenticity, and sound key handling.

### 8. Blake2b is fine; there is no urgent need to replace it

Blake2b remains a respectable choice. A move to BLAKE3 could make sense for performance-oriented hashing, but it is not necessary just because the draft is old.

Recommendation:

- Keep Blake2b unless there is a strong implementation or interoperability reason to change.
- Do not churn hashes without a concrete benefit.

### 9. SipHash for “FastHash” is not important unless the obfuscation scheme survives

If length obfuscation and related custom masking logic are removed from the POC, the `FastHash` function is probably unnecessary.

Recommendation:

- Drop it unless a retained protocol feature strictly needs it.

### 10. Crypto agility should be limited and intentional

The project should not over-engineer algorithm agility from day one. That usually creates complexity without real benefit.

Recommendation:

- Version the object and config formats.
- Make upgrades explicit.
- Support migration instead of unlimited algorithm negotiation.

Implementation note:

- In this project, “crypto agility” should mean modular code structure and explicit versioning, not runtime negotiation of arbitrary algorithms.
- Prefer stable internal interfaces around KDFs, hashes, AEADs, signatures, and transport handshakes.
- Keep serialized formats clear about which algorithms and parameters they use.

## Recommended Modern Rust Stack

If this becomes a fresh Rust implementation, start from well-known crates:

- `argon2`
- `hkdf`
- `blake2`
- `chacha20poly1305`
- `x25519-dalek`
- `ed25519-dalek`
- `snow` if Noise is used later
- `zeroize`
- `secrecy`
- `rand_core`

## Proposed POC Crypto Posture

- client-side encryption only
- central sync server over TLS
- Argon2id for passphrases
- AEAD for stored objects
- signatures only for revision or writer-authenticated objects
- explicit versioned formats
- no custom traffic obfuscation
- no DHT until the storage and sync model is proven

## Questions To Resolve Before Coding

- Is draft compatibility required, or only conceptual fidelity?
- Are seed-only peers a real requirement, or just an interesting idea from the original design?
- Is metadata hiding a core goal for v0, or just ciphertext confidentiality plus opaque object sync?
- Do multiple writers need independent keys, or is the first model single-writer with shared read access?
- Is the end product a filesystem, or an encrypted collaborative document store?
- Which crypto choices are locked for v0 pending feedback from the original author, and which are safe to modularize without semantic change?
