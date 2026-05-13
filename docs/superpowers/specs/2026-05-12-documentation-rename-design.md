# Design: Professional Documentation + Spanish-to-English Rename

**Date:** 2026-05-12  
**Status:** Approved

## Summary

Add professional Rust doc comments to every public item across the codebase, and simultaneously rename all Spanish identifiers to English. Both changes are done in a single pass per file to avoid intermediate states where doc comments reference Spanish names that no longer exist.

## Scope

### Files to modify (in order)

| File | Key renames | Doc additions |
|------|-------------|---------------|
| `src/transactions.rs` | `firma` → `signature`, `firmar` → `sign` | `//!` module, `///` struct + fields + methods |
| `src/merkle.rs` | local vars only | `//!` module, `///` function |
| `src/block.rs` | `transacciones` → `transactions`, `hash_previo` → `prev_hash`, `firma` → `signature`, `autor` → `author`, `calcular_hash` → `compute_hash`, `minar` → `mine`, `firmar` → `sign`, `corromper` → `corrupt` | `//!` module, `///` struct + all methods |
| `src/blockchain.rs` | `cadena` → `chain`, `minar` → `mine`, `validar` → `validate`, `firmar_bloque` → `sign_block`, `guardar` → `save`, `cargar` → `load`, `cadena()` → `chain()`, `corromper_bloque` → `corrupt_block`, `new_blockchain` → `new`, `new_blockchain_with_difficulty` → `with_difficulty` | `//!` module, `///` struct + all methods |
| `src/wallet.rs` | `guardar_cifrado` → `save_encrypted`, `cargar_cifrado` → `load_encrypted`, `guardar` → `save`, `cargar` → `load` | `//!` module, `///` structs + methods |
| `src/cli.rs` | no renames needed | `//!` module, `///` struct + enum + variants |
| `src/lib.rs` | — | `//!` crate-level doc |
| `src/main.rs` | local vars + user-facing messages | `///` `wallet_password` |
| `src/bin/api.rs` | local vars + messages | `///` handlers + payload struct |
| `src/bin/node.rs` | `cadena_recibida` → `received_chain`, struct field `cadena` → `chain` | `///` network structs |

### Out of scope

- `blockchain.json` — will be reset to a new empty chain (JSON field names will change due to struct field renames)
- No new functionality
- No refactoring beyond rename + docs

## Documentation Style

### What gets documented

- `//!` crate/module-level doc on every file — one sentence describing the module's role, plus key concepts if non-obvious
- `///` on every `pub struct`, `pub enum`, `pub fn`, and public fields
- `// ` inline comments only where the logic is non-obvious (e.g., the two-phase lock release in `api.rs::mine`)

### What does NOT get documented

- Private helper functions whose name is self-explanatory
- Test functions
- `# Examples` sections (not a published crate)
- Comments that restate what the identifier name already communicates

### Content focus

- **Structs/Enums:** what it represents, invariants it maintains
- **Methods:** contract (what it does, params, return), plus `# Errors` or `# Panics` sections where applicable
- No "what" explanations — only "why" and "contract"

## Rename Mapping (complete)

### `src/transactions.rs`
| Before | After |
|--------|-------|
| `Transaction.firma` | `Transaction.signature` |
| `Transaction::firmar` | `Transaction::sign` |

### `src/block.rs`
| Before | After |
|--------|-------|
| `Block.transacciones` | `Block.transactions` |
| `Block.hash_previo` | `Block.prev_hash` |
| `Block.firma` | `Block.signature` |
| `Block.autor` | `Block.author` |
| `Block::calcular_hash` | `Block::compute_hash` |
| `Block::firmar` | `Block::sign` |
| `Block::minar` | `Block::mine` |
| `Block::corromper` (test) | `Block::corrupt` (test) |

### `src/blockchain.rs`
| Before | After |
|--------|-------|
| `Blockchain.cadena` | `Blockchain.chain` |
| `Blockchain::new_blockchain` | `Blockchain::new` |
| `Blockchain::new_blockchain_with_difficulty` | `Blockchain::with_difficulty` |
| `Blockchain::minar` | `Blockchain::mine` |
| `Blockchain::validar` | `Blockchain::validate` |
| `Blockchain::firmar_bloque` | `Blockchain::sign_block` |
| `Blockchain::guardar` | `Blockchain::save` |
| `Blockchain::cargar` | `Blockchain::load` |
| `Blockchain::cadena()` | `Blockchain::chain()` |
| `Blockchain::corromper_bloque` (test) | `Blockchain::corrupt_block` (test) |

### `src/wallet.rs`
| Before | After |
|--------|-------|
| `Wallet::guardar_cifrado` | `Wallet::save_encrypted` |
| `Wallet::cargar_cifrado` | `Wallet::load_encrypted` |
| `Wallet::guardar` | `Wallet::save` |
| `Wallet::cargar` | `Wallet::load` |

## Implementation Order

Process files in dependency order to keep the compiler happy throughout:

1. `src/transactions.rs` — no deps on other project files
2. `src/merkle.rs` — depends on `transactions`
3. `src/block.rs` — depends on `transactions` + `merkle`
4. `src/blockchain.rs` — depends on `block` + `transactions` + `merkle`
5. `src/wallet.rs` — no deps on other project files
6. `src/cli.rs` — no deps on other project files
7. `src/lib.rs` — re-exports all modules
8. `src/main.rs` — depends on all of the above
9. `src/bin/api.rs` — depends on all of the above
10. `src/bin/node.rs` — depends on all of the above

After all files are done: reset `blockchain.json` with `Blockchain::new().save("blockchain.json")` equivalent (empty genesis chain).

## Acceptance Criteria

- `cargo build` passes with no errors or warnings
- `cargo test` passes (all existing tests continue to work)
- All public items have doc comments
- No Spanish identifiers remain in the codebase (field names, method names, local variables in public APIs)
- `blockchain.json` reflects the new field names
