# Attestation Snapshots

## Overview

The attestation snapshot contract stores periodic checkpoints of key attestation-derived metrics for efficient historical queries. It is optimized for read-heavy analytics patterns (e.g. lenders querying trailing revenue and anomaly counts for underwriting).

## Snapshot lifecycle and query APIs

### Lifecycle

1. **Initialize**  
   One-time: admin sets the contract and optionally binds an attestation contract. If bound, `record_snapshot` will require a non-revoked attestation for the (business, period) before storing.

2. **Record**  
   Authorized writers (admin or addresses with writer role) call `record_snapshot(business, period, trailing_revenue, anomaly_count, attestation_count)`.  
   - One snapshot per (business, period); re-recording overwrites (idempotent for the same period).  
   - Snapshot frequency is determined by the writer (off-chain or on-chain trigger); the contract does not enforce a schedule.

3. **Query**  
   - `get_snapshot(business, period)` – returns the snapshot for that (business, period), if any.  
   - `get_snapshots_for_business(business)` – returns all snapshot records for that business (all known periods).

### Snapshot fields (NatSpec-style)

| Field               | Type  | Description |
|---------------------|-------|-------------|
| `period`            | String | Period identifier (e.g. `"2026-02"`). |
| `trailing_revenue`  | i128  | Trailing revenue over the window used by the writer (smallest unit). |
| `anomaly_count`     | u32   | Number of anomalies detected in the period/window. |
| `attestation_count` | u64   | Attestation count for the business at snapshot time (from attestation contract). |
| `recorded_at`       | u64   | Ledger timestamp when this snapshot was recorded. |

### Update rules

- One snapshot record per (business, period). Re-recording for the same (business, period) overwrites the previous record.
- If an attestation contract is configured, the contract verifies that a non-revoked attestation exists for (business, period) before allowing a record.

## Integration with attestation and triggers

- The contract optionally stores an attestation contract address. When set, `record_snapshot` uses cross-contract calls to verify that an attestation exists and is not revoked for the given (business, period).
- Snapshots are written by off-chain or on-chain triggers (e.g. indexers or cron jobs) that compute derived metrics from attestations and call `record_snapshot`. The contract does not pull attestation data on its own except for this validation.

## Build (WASM)

When building the snapshot contract for `wasm32-unknown-unknown`, the attestation contract WASM must exist first (the snapshot uses `contractimport!`; the path is relative to the workspace root). From the workspace root, run:

```bash
cargo build --release -p veritasor-attestation --target wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown
```

CI builds the attestation WASM before Check, Test, and Build WASM so the snapshot compiles.

## Snapshot frequency

Snapshot frequency is not enforced on-chain. Design notes: typical choices are daily, weekly, or per-attestation (each new attestation triggers a snapshot for that business/period). The writer role can be granted to an automated address that writes at the desired cadence.

## Epoch finalization idempotency

### Invariants

The snapshot contract guarantees the following idempotency invariants for epoch finalization:

1. **Last-write-wins overwrite**: Re-recording a snapshot for the same `(business, period)` key overwrites the previous record completely. There is no blending, merging, or accumulation of old and new values. The snapshot always reflects the most recent `record_snapshot` call for that key.

2. **Period index deduplication**: The `BusinessPeriods` index (used by `get_snapshots_for_business`) never contains duplicate entries. Recording a snapshot for an existing period appends to the index only on the first write; subsequent re-recordings for the same period are no-ops on the index.

3. **Cross-business isolation**: Snapshots for different businesses sharing the same period string are completely independent. Overwriting business A's snapshot for period P has no effect on business B's snapshot for the same period P.

4. **Cross-epoch isolation**: Each `(business, period)` key is independent. Recording or overwriting one epoch never affects another epoch's data for the same business.

5. **Role-agnostic overwrite**: Overwrite semantics are identical regardless of whether the caller is admin or a writer. The last write wins regardless of caller role.

### Assumptions

- **No monotonicity enforcement**: The contract does not enforce monotonically increasing values for `trailing_revenue`, `anomaly_count`, or `attestation_count`. Overwriting with lower values is permitted (this supports correction workflows).
- **No period format validation**: Period strings are free-form (including empty strings). The contract does not parse or validate period identifiers.
- **No ordering constraint**: Epochs can be finalized in any order (not necessarily chronological). The `BusinessPeriods` index reflects insertion order, not chronological order.
- **Boundary values**: All integer fields accept their full range: `trailing_revenue` supports `i128::MIN` to `i128::MAX`, `anomaly_count` supports `0` to `u32::MAX`, and `attestation_count` supports `0` to `u64::MAX`.

### Expected behavior

| Scenario | Expected result |
|----------|----------------|
| Record same `(business, period)` with identical data | State unchanged (pure idempotency) |
| Record same `(business, period)` with different data | All fields overwritten to new values |
| Record same period 10 times | `get_snapshots_for_business` returns exactly 1 record |
| Two businesses, same period, one overwrites | Other business's snapshot unaffected |
| Writer records, then admin overwrites | Admin's data wins (last-write-wins) |
| Record with `trailing_revenue = i128::MIN` | Stored and retrievable as `i128::MIN` |
| Unauthorized address attempts overwrite | Panic: `caller must be admin or writer` |
| Record for period without attestation (when attestation contract set) | Panic: `attestation must exist` |

### Security notes

- **Authorization check on every write**: Every `record_snapshot` call (including re-recordings) verifies that the caller is admin or has writer role. An unauthorized address cannot overwrite an existing snapshot even if the epoch was previously finalized.
- **Attestation validation on every write**: When an attestation contract is configured, every `record_snapshot` call (including re-recordings) validates that a non-revoked attestation exists for `(business, period)`. A previously valid epoch cannot be re-finalized after the underlying attestation is revoked.
- **Removed writers cannot re-finalize**: Revoking writer role immediately prevents that address from recording or overwriting any snapshot.
- **No gas amplification**: Re-recording the same period does not increase the `BusinessPeriods` index size, so gas cost does not grow with re-recording count for the same epoch.
