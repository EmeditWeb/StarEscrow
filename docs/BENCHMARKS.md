# StarEscrow Contract Benchmarks

Resource consumption measurements for each contract function using the Soroban test environment budget API.

## How to Run

```bash
cargo test -p escrow bench -- --nocapture
```

## Results

> Last measured: _run `cargo test -p escrow bench -- --nocapture` to update_

| Function      | CPU Instructions | Memory (bytes) |
| ------------- | ---------------- | -------------- |
| `create`      | TBD              | TBD            |
| `submit_work` | TBD              | TBD            |
| `approve`     | TBD              | TBD            |
| `cancel`      | TBD              | TBD            |
| `expire`      | TBD              | TBD            |
| `get_status`  | TBD              | TBD            |

## Regression Threshold

CI will fail if any function exceeds the following limits:

| Function      | Max CPU Instructions | Max Memory (bytes) |
| ------------- | -------------------- | ------------------ |
| `create`      | 150,000,000          | 5,000,000          |
| `submit_work` | 100,000,000          | 3,000,000          |
| `approve`     | 150,000,000          | 5,000,000          |
| `cancel`      | 150,000,000          | 5,000,000          |
| `expire`      | 150,000,000          | 5,000,000          |
| `get_status`  | 50,000,000           | 1,000,000          |

These thresholds are conservative starting points. Tighten them after establishing a baseline.

## Notes

- Measurements use `soroban_sdk::testutils::budget::Budget` in the test environment.
- CPU instructions and memory are reset before each function call so only that function's cost is captured.
- Yield protocol interactions are not included in the base benchmarks above.

## WASM Size

`stellar contract optimize` is applied in CI and via `make optimize`. The optimized artifact is uploaded as `escrow-optimized-wasm`.

| Stage           | Size                                       |
| --------------- | ------------------------------------------ |
| Before optimize | Reported in CI `optimize-wasm` job summary |
| After optimize  | Reported in CI `optimize-wasm` job summary |

The `optimize-wasm` job writes a before/after byte table to the GitHub Actions step summary and uploads `escrow.optimized.wasm` as an artifact.

## WASM Bloat Analysis

Run `cargo bloat` to identify which functions contribute most to binary size:

```bash
# Install once
cargo install cargo-bloat

# Analyse the escrow contract (release profile)
cargo bloat -p escrow --release --crates
cargo bloat -p escrow --release -n 10
```

### Top 10 Size Contributors

> Last measured: _run the commands above to update_

| # | Function / Section | Size (bytes) | % of total |
|---|---|---|---|
| 1 | TBD | TBD | TBD |
| 2 | TBD | TBD | TBD |
| 3 | TBD | TBD | TBD |
| 4 | TBD | TBD | TBD |
| 5 | TBD | TBD | TBD |
| 6 | TBD | TBD | TBD |
| 7 | TBD | TBD | TBD |
| 8 | TBD | TBD | TBD |
| 9 | TBD | TBD | TBD |
| 10 | TBD | TBD | TBD |

### Optimization Opportunities

Common contributors in Soroban contracts and how to address them:

- `soroban_sdk` serialization/deserialization — unavoidable for on-chain types; minimize the number of distinct `#[contracttype]` structs.
- `core::fmt` / panic infrastructure — use `#![no_std]` (already applied) and avoid `unwrap`/`expect` in hot paths; prefer `?` with typed errors.
- Large match arms in `contractimpl` — split rarely-used entry points into a separate contract if WASM size becomes a concern.
- Unused generic monomorphisations — audit `Vec<T>` and `Map<K,V>` usage; each unique type combination generates separate code.

Run `cargo bloat -p escrow --release -n 10` after any significant feature addition and update the table above.
