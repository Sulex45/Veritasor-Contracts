# Security Invariants

> **Document status:** Living reference. Update whenever a new invariant is
> identified, strengthened, relaxed, or removed.
>
> **Primary test file:** `contracts/attestation/src/access_control_test.rs`  
> **Common invariant tests:** `contracts/common/src/security_invariant_test.rs`
>
> Every invariant listed here has a corresponding `#[test]` annotated with its
> SI-XXX id. If you add a new invariant, add a matching test. If you remove one,
> document the rationale under [Retired Invariants](#retired-invariants).

---

## Table of Contents

1. [Scope and Threat Model](#scope-and-threat-model)
2. [Role Definitions](#role-definitions)
3. [Enforced Invariants](#enforced-invariants)
   - [Attestation Contract](#attestation-contract)
   - [Integration Registry](#integration-registry)
   - [Attestation Snapshot Contract](#attestation-snapshot-contract)
   - [Aggregated Attestations Contract](#aggregated-attestations-contract)
4. [Detailed Invariant Reference](#detailed-invariant-reference)
5. [Access Control Matrix](#access-control-matrix)
6. [Adversarial Scenarios Considered](#adversarial-scenarios-considered)
7. [Regression Tests and Stress Cases](#regression-tests-and-stress-cases)
8. [How to Add New Invariants](#how-to-add-new-invariants)
9. [Gas and Performance Notes](#gas-and-performance-notes)
10. [Retired Invariants](#retired-invariants)
11. [Changelog](#changelog)

---

## Scope and Threat Model

### In scope

| Asset | Risk |
|---|---|
| Admin address stored in contract state | Unauthorized write access |
| Fee configuration (token, collector, base fee, enabled flag) | Manipulation to drain collector or skip fee payment |
| Role assignments (ADMIN, ATTESTOR, BUSINESS, OPERATOR) | Privilege escalation — unauthorized role grant |
| Attestation records (Merkle root, timestamp, version, fee_paid, proof_hash, expiry) | Forgery, duplication, or unauthorized revocation/migration |
| Multi-period attestation ranges | Overlapping range injection |
| Dispute records | Challenger impersonation; id collision |
| Integration registry providers | Unauthorized registration or status change |
| Snapshot records | Unauthorized writes |
| Portfolio definitions | Unauthorized registration or update |

### Out of scope

- Off-chain storage of revenue datasets (integrity protected by the proof hash
  but not enforced on-chain — see `docs/offchain-proof-hash.md`).
- Key management of admin Stellar keypairs (operational concern).
- Stellar network-level DoS.

### Attacker capabilities assumed

1. Any Stellar account can call any public entry point.
2. An attacker may craft every argument, including `caller` fields.
3. All on-chain state is observable by anyone.
4. A compromised role holder may attempt lateral escalation.
5. A removed/revoked address may attempt to reuse previously granted access.

---

## Role Definitions

| Role | Constant | How obtained | Mutable |
|---|---|---|---|
| **Admin** | `ROLE_ADMIN` | Set at `initialize` | No — immutable post-init |
| **Attestor** | `ROLE_ATTESTOR` | Granted by admin via `grant_role` | Yes |
| **Business** | `ROLE_BUSINESS` | Granted by admin via `grant_role` | Yes |
| **Operator** | `ROLE_OPERATOR` | Granted by admin via `grant_role` | Yes |
| **Governance** | _(registry)_ | Set at registry `initialize` | Contract-specific |
| **Writer** | _(snapshot)_ | Set at snapshot `initialize` | Contract-specific |

---

## Enforced Invariants

### Attestation Contract

- **Single initialization** (SI-001)  
  The contract can be initialized only once. A second call to `initialize`
  panics with `"already initialized"`.

- **No unauthorized role grants** (SI-003)  
  Only an address with `ROLE_ADMIN` can call `grant_role`. Any other caller
  causes a panic (auth failure or `"caller is not admin"`).

- **No unauthorized writes to attestation data** (SI-004, SI-005, SI-006)  
  Attestation submission requires the business address to authorize (`require_auth`).
  Revocation requires admin (`caller` validated against stored admin).
  Migration requires admin and a strictly increasing version number.

- **No duplicate attestations** (SI-004)  
  A second submission for the same `(business, period)` panics with
  `"attestation exists"`.

- **No overlapping multi-period ranges** (SI-007)  
  `submit_multi_period_attestation` panics with `"overlap"` if the new range
  intersects any existing non-revoked range for the same business.

- **Expiry semantics** (SI-008)  
  `is_expired` returns `true` when `expiry_timestamp ≤ ledger().timestamp()`.
  Attestations without an expiry are never expired.

- **Dispute challenger auth** (SI-009)  
  `open_dispute` requires the challenger address to authorize. Impersonation
  panics.

- **Admin immutability** (SI-010)  
  No `set_admin` entry point exists. `get_admin` is read-only and always
  returns the address supplied at `initialize`.

- **Fee admin isolation** (SI-011)  
  Configuring fees requires admin. Role holders (ATTESTOR, OPERATOR, etc.)
  cannot call `configure_fees`. Fee configuration does not grant any role.

- **Caller-field spoofing prevention** (SI-012)  
  Methods accepting an explicit `caller: Address` argument (`revoke_attestation`,
  `migrate_attestation`, `grant_role`) validate `caller` against the stored admin
  using `require_admin`. Passing a non-admin address as `caller` panics even
  when `mock_all_auths` is active.

- **Uninitialized state guard** (SI-002, SI-003, SI-004)  
  Admin-gated methods (`configure_fees`, `grant_role`) and business methods
  (`submit_attestation`) panic if called before `initialize`.

- **Read-only methods are side-effect-free** (SI-013)  
  `get_attestation`, `get_admin`, `has_role`, `is_expired`, `get_dispute` do
  not mutate storage.

### Integration Registry

- **Single initialization**  
  The registry can be initialized only once. A second `initialize` panics with
  `"already initialized"`.

- **No unauthorized provider registration**  
  Only addresses with the governance role can register, enable, disable, or
  update providers. A non-governance address calling `register_provider` (or
  similar) panics (e.g. `"caller does not have governance role"`).

### Attestation Snapshot Contract

- **Admin or writer for recording**  
  Only the contract admin or an address with the writer role can call
  `record_snapshot`. Unauthorized callers panic with
  `"caller must be admin or writer"`.

### Aggregated Attestations Contract

- **Admin-only portfolio registration**  
  Only the contract admin can register or update portfolios. Unauthorized
  callers panic with `"caller is not admin"`.

---

## Detailed Invariant Reference

Each invariant below maps 1-to-1 with annotated tests in
`contracts/attestation/src/access_control_test.rs`.

---

### SI-001 — initialize: one-time-only

**Applies to:** `initialize(admin, nonce)`

**Statement:**  
`initialize` may succeed at most once per contract instance. Any subsequent
call — from any address, with any nonce — **must panic** with
`"already initialized"`.

**Expected behavior:**

| Call | Outcome |
|---|---|
| First `initialize(admin, 0)` | Succeeds; admin stored |
| Second `initialize(any, any)` | Panics `"already initialized"` |

**Tests:** `test_initialize_succeeds_first_call`,
`test_initialize_rejects_second_call`,
`test_initialize_rejects_same_admin_different_nonce`

---

### SI-002 — configure_fees: admin only, uninitialized guard

**Applies to:** `configure_fees(token, collector, base_fee, enabled)`

**Statement:**  
Only the stored admin may call `configure_fees`. The admin must satisfy
`require_auth`. Any other caller, or a call before `initialize`, **must panic**.

**Tests:** `test_configure_fees_by_admin_succeeds`,
`test_configure_fees_by_non_admin_panics`,
`test_configure_fees_no_auth_panics`,
`test_configure_fees_before_initialize_panics`

---

### SI-003 — grant_role: admin only, no self-escalation, uninitialized guard

**Applies to:** `grant_role(caller, account, role, nonce)`

**Statement:**

1. Only the stored admin (passed as `caller`) may grant roles.
2. An account granting itself a higher-privilege role **must panic**.
3. Calling before `initialize` **must panic**.

**Role constants tested:** `ROLE_ADMIN`, `ROLE_ATTESTOR`, `ROLE_BUSINESS`,
`ROLE_OPERATOR`

**Tests:** `test_grant_role_attestor_by_admin_succeeds`,
`test_grant_role_business_by_admin_succeeds`,
`test_grant_role_operator_by_admin_succeeds`,
`test_grant_role_by_non_admin_panics`,
`test_grant_role_self_escalation_panics`,
`test_has_role_returns_false_for_unknown_address`,
`test_grant_role_before_initialize_panics`

---

### SI-004 — submit_attestation: business auth, no duplicates, no impersonation

**Applies to:** `submit_attestation(business, period, merkle_root, timestamp,
version, proof_hash, expiry_timestamp)`

**Statement:**

1. `business.require_auth()` is called — third parties cannot submit on behalf
   of a business.
2. Submitting a second attestation for the same `(business, period)` panics
   with `"attestation exists"`.
3. Calling before `initialize` panics.

**Tests:** `test_submit_attestation_by_business_succeeds`,
`test_submit_attestation_by_impersonator_panics`,
`test_submit_duplicate_attestation_panics`,
`test_submit_attestation_different_periods_both_succeed`,
`test_submit_attestation_different_businesses_same_period_both_succeed`,
`test_submit_attestation_before_initialize_panics`

---

### SI-005 — revoke_attestation: admin only, caller field validated

**Applies to:** `revoke_attestation(caller, business, period, reason, nonce)`

**Statement:**

1. Only the stored admin may revoke. The `caller` argument is validated against
   the stored admin — passing a non-admin as `caller` panics.
2. Calling before `initialize` panics.

**Tests:** `test_revoke_attestation_by_admin_succeeds`,
`test_revoke_attestation_by_non_admin_panics`,
`test_revoke_attestation_before_initialize_panics`

---

### SI-006 — migrate_attestation: admin only, version must increase

**Applies to:** `migrate_attestation(caller, business, period, new_root,
new_version)`

**Statement:**

1. Only the stored admin may migrate (via `access_control::require_admin`).
2. `new_version` must be strictly greater than the current version; equal or
   lower values panic with `"version too low"`.
3. Migrating a non-existent attestation panics with `"not found"`.

**Tests:** `test_migrate_attestation_by_admin_succeeds`,
`test_migrate_attestation_by_non_admin_panics`,
`test_migrate_attestation_same_version_panics`,
`test_migrate_attestation_lower_version_panics`,
`test_migrate_nonexistent_attestation_panics`

---

### SI-007 — submit_multi_period_attestation: business auth, no overlap

**Applies to:** `submit_multi_period_attestation(business, start_period,
end_period, merkle_root, timestamp, version)`

**Statement:**

1. `business.require_auth()` — impersonation panics.
2. A new range that intersects any existing non-revoked range for the same
   business panics with `"overlap"`.
3. Non-overlapping (including adjacent) ranges succeed.

**Tests:** `test_submit_multi_period_by_business_succeeds`,
`test_submit_multi_period_by_impersonator_panics`,
`test_submit_multi_period_overlap_panics`,
`test_submit_multi_period_non_overlapping_succeeds`

---

### SI-008 — is_expired: expiry semantics

**Applies to:** `is_expired(business, period)`

**Statement:**

| Condition | Return value |
|---|---|
| No `expiry_timestamp` set | `false` |
| `expiry_timestamp` in the far future | `false` |
| `expiry_timestamp ≤ ledger().timestamp()` | `true` |
| Attestation does not exist | `false` (no panic) |

**Tests:** `test_is_expired_no_expiry_returns_false`,
`test_is_expired_far_future_returns_false`,
`test_is_expired_past_expiry_returns_true`,
`test_is_expired_nonexistent_attestation_returns_false`

---

### SI-009 — open_dispute: challenger auth, unique ids

**Applies to:** `open_dispute(challenger, business, period, dispute_type,
evidence)`

**Statement:**

1. `challenger.require_auth()` — impersonation panics.
2. Every dispute receives a unique `id`; two calls on the same
   `(business, period)` yield different ids.
3. `get_dispute` returns `None` for an id that was never issued.

**Tests:** `test_open_dispute_by_challenger_succeeds`,
`test_open_dispute_multiple_have_unique_ids`,
`test_open_dispute_by_impersonator_panics`,
`test_get_dispute_nonexistent_returns_none`

---

### SI-010 — get_admin: read-only, immutable post-initialization

**Statement:**  
Once set, the admin address cannot be changed. No `set_admin` entry point
exists. `get_admin` is idempotent and always returns the address from
`initialize`.

**Rationale:**  
Mutable admin creates a social-engineering target. Immutability is a deliberate
trade-off (admin rotation via contract redeploy, not in-place key swap).

**Tests:** `test_get_admin_matches_initializer`,
`test_get_admin_is_idempotent`,
`test_get_admin_does_not_return_non_admin`

---

### SI-011 — Cross-feature role isolation

**Statement:**  
Roles are strictly scoped:

| Role | Can do | Cannot do |
|---|---|---|
| Admin | Fee config, role grants, revocation, migration | Bypasses `challenger.require_auth` on disputes |
| ATTESTOR / OPERATOR / BUSINESS | Domain-specific actions | `configure_fees`, `grant_role`, revoke, migrate |

**Tests:** `test_fee_token_has_no_role_after_configure_fees`,
`test_role_holder_cannot_configure_fees`

---

### SI-012 — Caller-field spoofing prevention

**Statement:**  
Methods accepting an explicit `caller: Address` argument validate it against
the stored admin using `require_admin`. Passing a non-admin `caller` panics
even when `mock_all_auths` is active, because the contract calls
`caller.require_auth()` first and then compares `caller` to the stored admin.

**Implementation pattern (correct):**
```rust
caller.require_auth();
let admin = read_admin(&env);
if caller != admin {
    panic!("unauthorized");
}
```

**Tests:** `test_caller_spoofing_revoke_panics`,
`test_caller_spoofing_migrate_panics`,
`test_caller_spoofing_grant_role_panics`

---

### SI-013 — get_attestation: read-only, returns correct stored tuple

**Applies to:** `get_attestation(business, period)`

**Statement:**

1. Returns `None` for a record that was never stored.
2. Returns the exact tuple `(merkle_root, timestamp, version, fee_paid,
   proof_hash, expiry_timestamp)` that was stored at submission.
3. Does not mutate any storage slot.

**Tests:** `test_get_attestation_nonexistent_returns_none`,
`test_get_attestation_returns_correct_fields`,
`test_get_attestation_with_proof_hash_returns_hash`,
`test_get_attestation_with_expiry_returns_expiry`

---

## Access Control Matrix

| Entry Point | Admin | ATTESTOR | BUSINESS | OPERATOR | Any |
|---|:---:|:---:|:---:|:---:|:---:|
| `initialize` | First call only | ✗ | ✗ | ✗ | ✗ |
| `configure_fees` | ✔ | ✗ | ✗ | ✗ | ✗ |
| `grant_role` | ✔ | ✗ | ✗ | ✗ | ✗ |
| `revoke_attestation` | ✔ | ✗ | ✗ | ✗ | ✗ |
| `migrate_attestation` | ✔ | ✗ | ✗ | ✗ | ✗ |
| `submit_attestation` | ✗ | ✗ | ✔ (own) | ✗ | ✗ |
| `submit_multi_period_attestation` | ✗ | ✗ | ✔ (own) | ✗ | ✗ |
| `open_dispute` | ✗ | ✗ | ✗ | ✗ | ✔ (self-auth) |
| `get_attestation` | ✔ | ✔ | ✔ | ✔ | ✔ |
| `get_admin` | ✔ | ✔ | ✔ | ✔ | ✔ |
| `has_role` | ✔ | ✔ | ✔ | ✔ | ✔ |
| `is_expired` | ✔ | ✔ | ✔ | ✔ | ✔ |
| `get_dispute` | ✔ | ✔ | ✔ | ✔ | ✔ |

> ✔ = permitted, ✗ = must panic if attempted.

---

## Adversarial Scenarios Considered

### Admin key leak

An attacker holding the admin key can revoke attestations and change fee
configuration. This is an operational risk, not a contract bug. Mitigation:
use a multisig or governance contract as the admin address.

### Front-running attestation submission

An attacker observing a pending `submit_attestation` could attempt to submit
the same `(business, period)` before the legitimate transaction lands. The
`business.require_auth()` requirement means only the business's own key can
sign, so a third-party front-run is impossible.

### Version roll-back via migration

`migrate_attestation` enforces `new_version > current_version`, preventing
an admin from rolling back to an earlier (potentially forged) root.

### Overlapping multi-period injection

The overlap check in `submit_multi_period_attestation` prevents an attacker
(or errant business) from inserting a range that would shadow or conflict with
an existing attestation.

### Dispute impersonation

`open_dispute` requires `challenger.require_auth()`. A third party cannot
file disputes on behalf of a challenger address they do not control.

---

## Regression Tests and Stress Cases

The invariant tests are written so that:

- They **assert** the behavior described above (e.g. second `initialize`
  panics, non-admin cannot call `grant_role`).
- `#[should_panic]` tests include an `expected` message where the panic string
  is deterministic, providing failure-mode assertions.
- Edge cases (empty portfolios, missing attestations, non-existent dispute ids)
  are covered in the respective contract test suites.
- New invariants can be added over time by appending tests in
  `access_control_test.rs` (or `security_invariant_test.rs` for cross-contract
  cases) and documenting them here.

---

## How to Add New Invariants

1. **Define the invariant**  
   State clearly what must always hold (e.g. "no unbounded growth of X",
   "only Y can write Z").

2. **Assign an SI-XXX id**  
   Take the next unused number. Add it to the [Detailed Invariant Reference]
   section above.

3. **Encode it in a test**  
   In `contracts/attestation/src/access_control_test.rs` (attestation contract)
   or `contracts/common/src/security_invariant_test.rs` (cross-contract), add
   a `#[test]` that:
   - Sets up the relevant contract(s).
   - Performs the action that should be forbidden or the condition that should
     never occur.
   - Asserts that the contract panics or returns an error, or that the state
     satisfies the invariant.

4. **Document**  
   Add a short bullet under the appropriate contract section in
   [Enforced Invariants](#enforced-invariants) and a full entry in
   [Detailed Invariant Reference](#detailed-invariant-reference).

5. **Run**  
   ```bash
   cargo test --all
   ```
   Invariant tests run with the rest of the suite and in CI.

---

## Gas and Performance Notes

Access control checks are O(1) for admin comparisons (single storage read)
and O(n) for role lookups where n is the number of assigned roles. Keep role
lists small to bound lookup cost.

For production gas benchmarks see `docs/contract-gas-benchmarks.md` and
`run_benchmarks.sh`.

---

## Retired Invariants

| ID | Description | Retired | Reason |
|---|---|---|---|
| SI-003 (old) | `set_tier_discount`: admin only, 0–10 000 bps | 2025-07 | Method removed from contract; superseded by `grant_role` RBAC |
| SI-004 (old) | `set_business_tier`: admin only | 2025-07 | Method removed; tier assignment replaced by RBAC roles |
| SI-005 (old) | `set_volume_brackets`: admin only, equal-length arrays | 2025-07 | Method removed from contract |
| SI-006 (old) | `set_fee_enabled`: admin only | 2025-07 | Folded into `configure_fees` `enabled` parameter |
| SI-008 (old) | `add/remove_authorized_analytics` | 2025-07 | Analytics oracle registry removed from this contract |
| SI-009 (old) | `set_anomaly`: authorized oracle only, score 0–100 | 2025-07 | Anomaly feature removed from this contract |
| SI-011 (old) | `verify_attestation`: read-only correctness | 2025-07 | Method removed; callers read via `get_attestation` + `is_expired` |
| SI-019 (old) | Pagination limit enforcement | 2025-07 | `get_attestations_page` removed from this contract |
| SI-020 (old) | Oracle revocation immediate and complete | 2025-07 | Oracle registry removed from this contract |

---

## Changelog

| Date | Author | Change |
|---|---|---|
| 2025-07 | Veritasor team | v2 — expanded to multi-contract scope; realigned all SI ids to actual `lib.rs` API; retired 9 stale invariants; added SI-006 (migrate), SI-007 (multi-period), SI-009 (dispute), SI-012 (spoofing), SI-013 (get_attestation) |
| 2025-07 | Veritasor team | v1 — initial draft (attestation contract only, 20 invariants) |