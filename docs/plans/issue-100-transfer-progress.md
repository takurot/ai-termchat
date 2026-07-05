# Plan: Issue #100 — Progress bar for active file transfers (Option 1)

Selected approach: **bytes-only indicator** (no wire-protocol change). Matches the Issue's recommended starting point and avoids `NetMessage` backward-compatibility surface area.

## Scope

Track bytes received per active transfer and render a non-blocking progress line in the ops-ai status panel. No protocol change; no percentage bar; no cancellation UI (explicitly out of scope).

## Affected files

- `src/state.rs` — replace `active_transfers: HashMap<(String, String), PathBuf>` with a struct value that carries `temp_path` + `bytes_received`; add accessor/mutator methods; existing public methods (`start_transfer`, `get_transfer_temp_path`, `remove_transfer`) keep their signatures so call sites in `application/mod.rs` are unaffected.
- `src/application/mod.rs` — in `Chunk::Data` branch of `process_user_data`, call `state.record_transfer_bytes(...)` after a successful append.
- `src/ui/status_panel.rs` — render up to N active-transfer lines in the right column with a human-readable byte counter; add `format_bytes` helper + unit tests.
- `docs/SPEC.md` — document the receiving progress indicator behavior.
- `tests/send_file.rs` — integration test asserting the byte counter accumulates across `Chunk::Data` and clears on `Chunk::End`/`Chunk::Error`.

## Data model

```rust
// src/state.rs
#[derive(Clone, Debug)]
pub struct ActiveTransfer {
    pub temp_path: PathBuf,
    pub bytes_received: u64,
}

/// Read-only view used by the UI renderer. Clones the (short) user/filename
/// strings so the renderer does not borrow from `&State`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActiveTransferView {
    pub user: String,
    pub filename: String,
    pub bytes_received: u64,
}

// field change
active_transfers: HashMap<(String, String), ActiveTransfer>,
```

### Key sanitization (revised after plan review)

`process_user_data` already computes `sanitized_user` and `sanitized_file` (via `sanitize_filename`, which only allows `[A-Za-z0-9._-]` and converts everything else — including control chars — to `_`). The pre-existing code keyed `active_transfers` on `(sanitized_user, raw_file_name)`, which is inconsistent and surfaces raw network bytes to any UI that renders the key. **This plan unifies the key to `(sanitized_user, sanitized_file)`** so filesystem ops, lookups, and display all agree and the status-panel rendering is safe by construction.

Concretely, in `src/application/mod.rs` `process_user_data`:
- `Chunk::Data` lookup/create: `get_transfer_temp_path(&sanitized_user, &sanitized_file)` and `start_transfer(sanitized_user.clone(), sanitized_file.clone(), ...)`.
- `Chunk::End`: `remove_transfer(&sanitized_user, &sanitized_file)`.
- `Chunk::Error`: `remove_transfer(&sanitized_user, &sanitized_file)`.
- After successful `write_all`: `record_transfer_bytes(&sanitized_user, &sanitized_file, len)`.

This is more consistent but technically expands the surface beyond the literal Issue scope; justified because the new UI panel directly renders the key.

Public API (signatures preserved, behavior extended):

- `start_transfer(user, filename, temp_path)` — inserts with `bytes_received: 0`.
- `get_transfer_temp_path(user, filename) -> Option<&PathBuf>` — unchanged signature (reads through to `ActiveTransfer.temp_path`).
- `remove_transfer(user, filename) -> Option<PathBuf>` — unchanged signature (drops the whole entry including the counter).
- **NEW** `record_transfer_bytes(user, filename, bytes)` — saturating-add on `bytes_received`; no-op if no entry exists.
- **NEW** `active_transfers_view() -> Vec<ActiveTransferView>` — snapshot for UI rendering, sorted by `(user, filename)` for deterministic output.
- **NEW (test-only)** `#[cfg(test)] fn transfer_bytes_received(user, filename) -> Option<u64>` — kept out of the public API surface.

`disconnected_user` already removes entries for the disconnected user — the counter is dropped along with the entry. A new unit test will assert this explicitly because no existing test covers `disconnected_user`.

## Application wiring

```rust
// src/application/mod.rs  process_user_data, Chunk::Data branch, after write_all:
std::fs::OpenOptions::new().append(true).open(temp_path)?.write_all(&data)?;
self.state
    .record_transfer_bytes(&sanitized_user, &sanitized_file, data.len() as u64);
```

Note: only count bytes on successful append (after `write_all` succeeds), so failed writes don't inflate the counter. Failure-non-inflation is verified by code position only — `record_transfer_bytes` is called inside the `try_write` closure after `write_all` returns `Ok`. Portable injection of a `write_all` failure (e.g. read-only parent directory) would require platform-specific setup; documented as a known coverage gap rather than a test.

## UI rendering

In `draw_status_panel` right column, render a `Receiving` section. Placement and budget:

- Only render the section when there is at least one active transfer (no visual noise otherwise).
- The right column already budget proposals (up to 2) + TODOs (up to 3) against `right_chunk.height`. The new section must respect that budget: compute remaining line budget after proposals, then cap the receiving lines at `min(2, remaining)` (or 1 + overflow if budget is tight). Skip TODOs further if necessary — the existing TODO budgeting pattern (`status_panel.rs:95-99`) is the model.
- Helper `format_bytes(u64) -> String`: binary units (KB = 1024), one decimal place when the value is < 10 of the unit, integer otherwise. e.g. `0 B`, `512 B`, `2.0 KB`, `1.4 MB`, `15 MB`.
- Show at most 2 entries; overflow line matches existing `overflow_line` style.
- Use `Color::Cyan` for the section header and `Color::LightCyan` for entries.

```
Receiving
↓ 1.4 MB  user: file.txt
↓ 512 KB  alice: pic.png  … +1 more
```

## Acceptance criteria mapping (from Issue)

- [x] `State` tracks `bytes_received` per active transfer and exposes it through a method (unit-tested in `src/state.rs`).
- [x] `Application::process_user_data` updates the byte counter on every `Chunk::Data` append.
- [x] TUI renders an active-transfer indicator that updates as chunks arrive (unit-tested via `TestBackend`; manual E2E for live rendering).
- [x] Counter cleaned up on `Chunk::End`, `Chunk::Error`, and peer disconnect (`State::disconnected_user`).
- [x] `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test --tests`, `cargo build --release` pass.
- [x] SPEC.md updated.

## Test cases

Unit (`src/state.rs`):
1. `start_transfer_initializes_zero_bytes` — `assert_eq!(transfer_bytes_received(u, f), Some(0))` (exact value, not `is_some()`).
2. `record_transfer_bytes_accumulates` — two calls sum correctly.
3. `record_transfer_bytes_noop_when_no_active_transfer` — does not insert a new entry.
4. `remove_transfer_clears_counter` — `transfer_bytes_received` returns `None` after removal.
5. `disconnected_user_clears_active_transfers` — **new test** (no existing `disconnected_user` test in repo). Construct an `Endpoint`, seed `lan_users`, call `start_transfer` + `record_transfer_bytes`, then `disconnected_user(endpoint)`, assert `transfer_bytes_received == None`.
6. `active_transfers_view_is_sorted_by_user_then_filename` — insert 3 entries out of order, assert sorted output.

Unit (`src/ui/status_panel.rs`):
7. `format_bytes_zero` — `format_bytes(0) == "0 B"`.
8. `format_bytes_kib` — `format_bytes(2048) == "2.0 KB"`.
9. `format_bytes_mib` — `format_bytes(1_500_000) == "1.4 MB"`.
10. `rendered_panel_shows_active_transfer` — state with one transfer renders the filename + formatted byte count.
11. `rendered_panel_omits_receiving_section_when_idle` — `State::default()` rendering does not contain "Receiving".
12. `rendered_panel_budgets_receiving_against_proposals_and_todos` — all three sections populated at small height does not panic and stays within `right_chunk.height`.

Integration (`tests/send_file.rs`):
13. `chunk_data_accumulates_byte_counter` — inject two `Chunk::Data(vec![u8; 1024])`, assert `state.transfer_bytes_received("sender", "test.txt") == Some(2048)`. No `End`.
14. `chunk_end_clears_byte_counter` — inject `Data(1024)` then `End`, assert `transfer_bytes_received == None`.
15. `chunk_error_clears_byte_counter` — inject `Data(1024)` then `Error`, assert `transfer_bytes_received == None`.

Note: `sanitize_filename("sender") == "sender"` and `sanitize_filename("test.txt") == "test.txt"`, so test assertions query with the same strings they inject — no test-side sanitization surprise.

## Risks

- **Public API change for `active_transfers` field type.** Field is private; only `State` methods touch it. Verified via `grep` (7 references, all inside `src/state.rs`). No risk to external callers.
- **Key-scope expansion.** Switching `active_transfers` keys from raw `file_name` to `sanitized_file` touches three branches in `process_user_data`. Pre-existing chat error/success messages still render the raw `file_name` — those are out of scope (the Issue only concerns the new status-panel indicator). The key change is consistent with the existing `sanitized_user` convention.
- **UI overflow at small heights.** Mitigation: only render section when transfers exist; budget against `right_chunk.height` after proposals; cap at 2 entries + overflow marker; unit test covers the all-three-populated case.
- **Determinism in `active_transfers_view`.** HashMap iteration is unordered. Sort by `(user, filename)` for stable UI and tests.
- **Failed-write non-inflation** is verified by code position only (see "Application wiring").
- **No protocol change**, so no backward-compatibility risk. Verified: `Chunk` enum is unchanged.

## Verification steps

1. `cargo test --lib state::tests` — state unit tests pass.
2. `cargo test --lib ui::status_panel::tests` — UI unit tests pass.
3. `cargo test --test send_file` — integration tests pass.
4. `cargo fmt --all -- --check`
5. `cargo clippy --all-targets -- -D warnings`
6. `cargo test --tests`
7. `cargo build --release`

## Out of scope (per Issue)

- Percentage bar via protocol change (follow-up issue).
- Cancellation UI.
