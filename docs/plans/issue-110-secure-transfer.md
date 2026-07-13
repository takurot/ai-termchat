# Plan: Issue #110 — File transfer data plane bypasses ChaCha20Poly1305 secure session

Link: https://github.com/takurot/triadchat/issues/110
Severity: High / Security / P1
Branch: `issue/110-secure-transfer-data-plane`

## Problem

`SendFile::process` (the only `Action` implementor) sends both `TransferOffer`
and `NetMessage::UserData` via raw `crate::util::send_all` + plaintext `Encoder`,
bypassing `send_secure_to_peer`. Up to 100 MB (`MAX_TRANSFER_SIZE`) of transfer
content is observable/mutable on the wire even after a secure session exists.

On the receiver side, `NetMessage::UserData` is dispatched with
`accepts_authenticated_peer_message_inner(..., /* require_secure = */ false)`,
explicitly waiving the plaintext-downgrade guard added in #101.

## Root cause

- `src/commands/send_file.rs:82-93` — `TransferOffer` sent plaintext.
- `src/commands/send_file.rs:127-136` — `UserData` chunks sent plaintext.
- `src/application/mod.rs:374-385` — receiver waives `require_secure` for `UserData`.
- Architectural: the `Action` trait only receives `&mut State` + `&NetworkController`,
  with no access to `SecureState`, so it cannot encrypt even if it wanted to.

## Acceptance criteria (from Issue)

1. `SendFile::process` routes through secure sending (`send_secure_to_peer` or
   application wraps `UserData` in `Secure`).
2. Receiver passes `require_secure = true` for `UserData` when a session exists,
   mirroring `UserMessage`.
3. Regression test: no `UserData` frame is transmitted unencrypted once
   `has_session(endpoint)` is true.

## Changes

### 1. `src/secure.rs` — shared encode helper

Add a unit-testable core function + a multi-endpoint sender:

- `encode_for_endpoint(secure_state, endpoint, message) -> Result<Vec<u8>, &'static str>`
  - If `has_session(endpoint)`: serialize inner (bincode legacy) → `session.encrypt` →
    wrap in `NetMessage::Secure` → serialize → return bytes.
  - Else: serialize `message` plaintext (backward compat with session-less peers).
  - Mirrors the exact encoding steps of `Application::send_secure_to_peer`.
- `send_secure_to_endpoints(network, secure_state, endpoints, message)
   -> Result<(), Vec<(Endpoint, io::Error)>>`
  - Per-endpoint: `encode_for_endpoint` then `network.send`. Same error shape as
    `util::send_all` so `SendFile`'s failed-endpoint tracking still works.

### 2. `src/action.rs` — give Action access to secure transport

Extend the trait so actions can send encrypted frames. `SendFile` is the only
implementor; `process_action` is the only caller.

```rust
pub trait Action: Send {
    fn process(
        &mut self,
        state: &mut State,
        network: &NetworkController,
        secure: &mut SecureState,
    ) -> Processing;
}
```

Borrow check: in `process_action`, `self.state`, `self.secure_state`, and
`self.node` are disjoint fields → simultaneous `&mut`/`&` borrows are legal.

### 3. `src/commands/send_file.rs` — route transfer through secure sender

- Remove the now-unused `encoder: Encoder` field (both send sites move to the
  helper, which does its own bincode encoding).
- `TransferOffer`: replace `self.encoder.encode(offer)` + `util::send_all` with
  `secure::send_secure_to_endpoints(network, secure, &endpoints, &offer)`.
- `UserData`: same replacement.
- Preserve `failed_endpoints` tracking (helper returns the same error type).

### 4. `src/application/mod.rs`

- `process_action` (line ~1478): pass `&mut self.secure_state` into
  `action.process(...)`.
- `NetMessage::UserData` receiver (line ~374): replace
  `accepts_authenticated_peer_message_inner(endpoint, "file transfer", false)`
  with `accepts_authenticated_peer_message(endpoint, "file transfer")`
  (require_secure = true). Decrypted `UserData` arriving via the `Secure`
  envelope still passes because `processing_from_secure == true` at that point.
- `send_secure_to_peer` (line ~759): delegate to `secure::encode_for_endpoint`
  to remove duplicated security-critical encrypt-or-plaintext logic and prevent
  drift between the two code paths. Behavior preserved (on failure: log + return).

## Tests

### Unit test (`src/secure.rs`)

Directly verifies acceptance criterion #3 without needing a live socket:

- Build a `SecureState` with a session keyed by a constructed `Endpoint`
  (use `complete_key_exchange_as_initiator` + insert into `sessions`).
- `encode_for_endpoint(...)` with that endpoint + a `NetMessage::UserData`
  must decode as `NetMessage::Secure(_)` (NOT `UserData`).
- `encode_for_endpoint(...)` with an endpoint that has NO session must decode
  as the original `NetMessage::UserData(_)`.

### Integration test (`tests/network_encryption.rs`)

Two new tests mirroring the existing `plaintext_rejected_after_secure_session_established`
and `encrypted_chat_messages_exchanged_after_key_exchange` patterns:

- `plaintext_transfer_chunk_rejected_after_secure_session`: establish session,
  inject plaintext `NetMessage::UserData`, assert rejection
  ("rejected plaintext file transfer ... secure session exists") and that no
  file is written.
- `encrypted_file_transfer_completes_after_secure_session`: establish session,
  perform a real `/send` + `/accept` flow, assert the received file bytes equal
  the source (proves the encrypted data plane works end-to-end). Reuse the
  pump/send helpers already in `tests/send_file.rs` pattern.

## Backward compatibility

- Peers without a secure session (older clients, or pre-key-exchange window)
  still receive plaintext `UserData`/`TransferOffer` via the helper's else-branch.
  Matches `send_secure_to_peer` and the #101 backward-compat test.
- `processing_from_secure` flag already routes decrypted frames correctly.

## Risks

- **Action trait signature change**: blast radius is 1 impl + 1 caller (verified
  via grep). Low risk.
- **Existing `send_file` integration test**: connects peers but does NOT wait for
  the secure session, so it exercises the plaintext fallback — should still pass.
  Will confirm.
- **Per-chunk allocation**: helper allocates a `Vec` per send instead of reusing
  `Encoder`'s buffer. Acceptable: chunk payload is already a `Vec`, and transfer
  throughput is rate-limited (100µs delay between chunks).

## Verification

- `cargo test --lib secure` (new unit test)
- `cargo test --test network_encryption` (new integration tests)
- `cargo test --test send_file` (existing transfer tests still pass)
- `cargo clippy --all-targets -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo test --tests` + `cargo build --release` (pre-publish gate)

## Out of scope

- #125 (transfer accept/reject authentication), #133 (byte counter), #117 (chunk
  file reopen), #118 (accept_transfer error swallowing) — separate issues, file
  boundaries not touched by this fix.
