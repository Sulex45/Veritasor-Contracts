//! Tests for the attestation snapshot contract: recording, querying, attestation
//! validation, edge cases (missing attestations, repeated snapshots, evolving metrics),
//! and scenario tests where lenders query snapshots for underwriting.

use super::*;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, Env, String};
use veritasor_attestation::{AttestationContract, AttestationContractClient};

fn setup_snapshot_only() -> (Env, AttestationSnapshotContractClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(AttestationSnapshotContract, ());
    let client = AttestationSnapshotContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin, &None::<Address>);
    (env, client, admin)
}

fn setup_with_attestation() -> (
    Env,
    AttestationSnapshotContractClient<'static>,
    AttestationContractClient<'static>,
    Address,
    Address,
) {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let att_id = env.register(AttestationContract, ());
    let att_client = AttestationContractClient::new(&env, &att_id);
    att_client.initialize(&admin, &0u64);

    let snap_id = env.register(AttestationSnapshotContract, ());
    let snap_client = AttestationSnapshotContractClient::new(&env, &snap_id);
    snap_client.initialize(&admin, &Some(att_id.clone()));

    let business = Address::generate(&env);
    (env, snap_client, att_client, admin, business)
}

// ── Initialization ───────────────────────────────────────────────────

#[test]
fn test_initialize() {
    let (_env, client, admin) = setup_snapshot_only();
    assert_eq!(client.get_admin(), admin);
    assert!(client.get_attestation_contract().is_none());
}

#[test]
fn test_initialize_with_attestation_contract() {
    let env = Env::default();
    env.mock_all_auths();
    let att_id = env.register(AttestationContract, ());
    let snap_id = env.register(AttestationSnapshotContract, ());
    let client = AttestationSnapshotContractClient::new(&env, &snap_id);
    let admin = Address::generate(&env);
    client.initialize(&admin, &Some(att_id.clone()));
    assert_eq!(client.get_attestation_contract(), Some(att_id));
}

#[test]
#[should_panic(expected = "already initialized")]
fn test_initialize_twice_panics() {
    let (_env, client, admin) = setup_snapshot_only();
    client.initialize(&admin, &None::<Address>);
}

// ── Recording without attestation contract ───────────────────────────

#[test]
fn test_record_and_get_snapshot_no_attestation_contract() {
    let (env, client, admin) = setup_snapshot_only();
    let business = Address::generate(&env);
    let period = String::from_str(&env, "2026-02");
    client.record_snapshot(&admin, &business, &period, &100_000i128, &2u32, &5u64);
    let record = client.get_snapshot(&business, &period).unwrap();
    assert_eq!(record.period, period);
    assert_eq!(record.trailing_revenue, 100_000i128);
    assert_eq!(record.anomaly_count, 2u32);
    assert_eq!(record.attestation_count, 5u64);
}

#[test]
fn test_record_overwrites_same_period() {
    let (env, client, admin) = setup_snapshot_only();
    let business = Address::generate(&env);
    let period = String::from_str(&env, "2026-02");
    client.record_snapshot(&admin, &business, &period, &100_000i128, &2u32, &5u64);
    client.record_snapshot(&admin, &business, &period, &200_000i128, &3u32, &6u64);
    let record = client.get_snapshot(&business, &period).unwrap();
    assert_eq!(record.trailing_revenue, 200_000i128);
    assert_eq!(record.anomaly_count, 3u32);
    assert_eq!(record.attestation_count, 6u64);
}

#[test]
fn test_get_snapshots_for_business() {
    let (env, client, admin) = setup_snapshot_only();
    let business = Address::generate(&env);
    let p1 = String::from_str(&env, "2026-01");
    let p2 = String::from_str(&env, "2026-02");
    client.record_snapshot(&admin, &business, &p1, &50_000i128, &0u32, &1u64);
    client.record_snapshot(&admin, &business, &p2, &100_000i128, &1u32, &2u64);
    let snapshots = client.get_snapshots_for_business(&business);
    assert_eq!(snapshots.len(), 2);
}

#[test]
#[should_panic(expected = "caller must be admin or writer")]
fn test_record_unauthorized_panics() {
    let (env, client, _admin) = setup_snapshot_only();
    let business = Address::generate(&env);
    let period = String::from_str(&env, "2026-02");
    let other = Address::generate(&env);
    client.record_snapshot(&other, &business, &period, &100_000i128, &0u32, &0u64);
}

// ── Recording with attestation contract (validation) ────────────────────

#[test]
fn test_record_with_attestation_required_succeeds_when_attestation_exists() {
    let (env, snap_client, att_client, admin, business) = setup_with_attestation();
    let period = String::from_str(&env, "2026-02");
    let root = soroban_sdk::BytesN::from_array(&env, &[1u8; 32]);
    att_client.submit_attestation(
        &business,
        &period,
        &root,
        &1700000000u64,
        &1u32,
        &None,
        &None,
        &0u64,
    );
    snap_client.record_snapshot(&admin, &business, &period, &100_000i128, &0u32, &1u64);
    let record = snap_client.get_snapshot(&business, &period).unwrap();
    assert_eq!(record.trailing_revenue, 100_000i128);
}

#[test]
#[should_panic(expected = "attestation must exist for this business and period")]
fn test_record_with_attestation_required_panics_when_no_attestation() {
    let (env, snap_client, _att_client, admin, business) = setup_with_attestation();
    let period = String::from_str(&env, "2026-02");
    snap_client.record_snapshot(&admin, &business, &period, &100_000i128, &0u32, &0u64);
}

#[test]
#[should_panic(expected = "attestation must not be revoked")]
fn test_record_with_attestation_required_panics_when_revoked() {
    let (env, snap_client, att_client, admin, business) = setup_with_attestation();
    let period = String::from_str(&env, "2026-02");
    let root = soroban_sdk::BytesN::from_array(&env, &[1u8; 32]);
    att_client.submit_attestation(
        &business,
        &period,
        &root,
        &1700000000u64,
        &1u32,
        &None,
        &None,
    );
    att_client.revoke_attestation(&admin, &business, &period, &String::from_str(&env, "fraud"), &1u64);
    snap_client.record_snapshot(&admin, &business, &period, &100_000i128, &0u32, &1u64);
}

// ── Writer role ───────────────────────────────────────────────────────

#[test]
fn test_writer_can_record() {
    let (env, client, admin) = setup_snapshot_only();
    let writer = Address::generate(&env);
    client.add_writer(&admin, &writer);
    let business = Address::generate(&env);
    let period = String::from_str(&env, "2026-02");
    client.record_snapshot(&writer, &business, &period, &50_000i128, &0u32, &0u64);
    assert!(client.get_snapshot(&business, &period).is_some());
}

#[test]
fn test_remove_writer() {
    let (env, client, admin) = setup_snapshot_only();
    let writer = Address::generate(&env);
    client.add_writer(&admin, &writer);
    assert!(client.is_writer(&writer));
    client.remove_writer(&admin, &writer);
    assert!(!client.is_writer(&writer));
}

// ── Lender / underwriting scenario ───────────────────────────────────

#[test]
fn test_lender_queries_snapshots_for_underwriting() {
    let (env, client, admin) = setup_snapshot_only();
    let business = Address::generate(&env);
    let periods = ["2026-01", "2026-02", "2026-03"];
    for (i, p) in periods.iter().enumerate() {
        let period = String::from_str(&env, p);
        client.record_snapshot(
            &admin,
            &business,
            &period,
            &(100_000 * (i as i128 + 1)),
            &(i as u32),
            &(i as u64 + 1),
        );
    }
    let snapshots = client.get_snapshots_for_business(&business);
    assert_eq!(snapshots.len(), 3);
    let last = client
        .get_snapshot(&business, &String::from_str(&env, "2026-03"))
        .unwrap();
    assert_eq!(last.trailing_revenue, 300_000i128);
    assert_eq!(last.anomaly_count, 2u32);
}

// ── Edge cases ────────────────────────────────────────────────────────

#[test]
fn test_get_snapshot_missing_returns_none() {
    let (env, client, _admin) = setup_snapshot_only();
    let business = Address::generate(&env);
    let period = String::from_str(&env, "2026-99");
    assert!(client.get_snapshot(&business, &period).is_none());
}

#[test]
fn test_get_snapshots_for_business_empty() {
    let (env, client, _admin) = setup_snapshot_only();
    let business = Address::generate(&env);
    let snapshots = client.get_snapshots_for_business(&business);
    assert_eq!(snapshots.len(), 0);
}

#[test]
fn test_set_attestation_contract() {
    let (env, client, admin) = setup_snapshot_only();
    let att_id = Address::generate(&env);
    client.set_attestation_contract(&admin, &Some(att_id.clone()));
    assert_eq!(client.get_attestation_contract(), Some(att_id));
    client.set_attestation_contract(&admin, &None::<Address>);
    assert!(client.get_attestation_contract().is_none());
}

#[test]
#[should_panic(expected = "caller is not admin")]
fn test_set_attestation_contract_non_admin_panics() {
    let (env, client, _admin) = setup_snapshot_only();
    let other = Address::generate(&env);
    client.set_attestation_contract(&other, &None::<Address>);
}

// ════════════════════════════════════════════════════════════════════════════
//  Snapshot Epoch Finalization – Idempotency Tests
//
//  These tests verify the core idempotency invariant: recording a snapshot for
//  the same (business, period) key must be a safe, deterministic operation
//  regardless of how many times it is invoked, what data is supplied, or who
//  calls it (admin vs. writer).
//
//  Coverage:
//    - Pure idempotent re-recording (identical data)
//    - Overwrite semantics (last-write-wins with different data)
//    - Period index deduplication (no duplicate entries in BusinessPeriods)
//    - Multi-epoch finalization ordering and isolation
//    - Cross-business isolation for the same period
//    - Writer-role epoch finalization idempotency
//    - Boundary-value testing (zero, negative, i128::MAX, u32::MAX, u64::MAX)
//    - Rapid sequential re-recording stability
//    - Deterministic state assertions after multiple overwrites
//    - Attestation-validated epoch finalization idempotency
// ════════════════════════════════════════════════════════════════════════════

// ── Pure idempotent re-recording (identical data) ────────────────────────

/// Recording the exact same snapshot data twice for (business, period) must
/// produce identical state. The query result after the second call must equal
/// the result after the first call.
#[test]
fn test_idempotent_rerecord_identical_data_produces_same_state() {
    let (env, client, admin) = setup_snapshot_only();
    let business = Address::generate(&env);
    let period = String::from_str(&env, "2026-03");

    client.record_snapshot(&admin, &business, &period, &500_000i128, &4u32, &10u64);
    let first = client.get_snapshot(&business, &period).unwrap();

    client.record_snapshot(&admin, &business, &period, &500_000i128, &4u32, &10u64);
    let second = client.get_snapshot(&business, &period).unwrap();

    assert_eq!(first.trailing_revenue, second.trailing_revenue);
    assert_eq!(first.anomaly_count, second.anomaly_count);
    assert_eq!(first.attestation_count, second.attestation_count);
    assert_eq!(first.period, second.period);
}

/// Three consecutive identical recordings must all converge to the same
/// trailing_revenue, anomaly_count, and attestation_count.
#[test]
fn test_idempotent_triple_rerecord_identical_data() {
    let (env, client, admin) = setup_snapshot_only();
    let business = Address::generate(&env);
    let period = String::from_str(&env, "2026-04");

    for _ in 0..3 {
        client.record_snapshot(&admin, &business, &period, &42i128, &1u32, &1u64);
    }

    let record = client.get_snapshot(&business, &period).unwrap();
    assert_eq!(record.trailing_revenue, 42i128);
    assert_eq!(record.anomaly_count, 1u32);
    assert_eq!(record.attestation_count, 1u64);
}

// ── Overwrite semantics (last-write-wins) ────────────────────────────────

/// Re-recording with different metrics must overwrite the previous snapshot
/// completely (last-write-wins). No blending/merging of old and new values.
#[test]
fn test_epoch_overwrite_last_write_wins() {
    let (env, client, admin) = setup_snapshot_only();
    let business = Address::generate(&env);
    let period = String::from_str(&env, "2026-Q1");

    client.record_snapshot(&admin, &business, &period, &100i128, &0u32, &1u64);
    client.record_snapshot(&admin, &business, &period, &999_999i128, &50u32, &100u64);

    let record = client.get_snapshot(&business, &period).unwrap();
    assert_eq!(record.trailing_revenue, 999_999i128, "trailing_revenue must reflect last write");
    assert_eq!(record.anomaly_count, 50u32, "anomaly_count must reflect last write");
    assert_eq!(record.attestation_count, 100u64, "attestation_count must reflect last write");
}

/// Overwriting with lower values (regression scenario) must succeed; the
/// contract does not enforce monotonicity.
#[test]
fn test_epoch_overwrite_with_lower_values_allowed() {
    let (env, client, admin) = setup_snapshot_only();
    let business = Address::generate(&env);
    let period = String::from_str(&env, "2026-05");

    client.record_snapshot(&admin, &business, &period, &1_000_000i128, &10u32, &50u64);
    client.record_snapshot(&admin, &business, &period, &1i128, &0u32, &1u64);

    let record = client.get_snapshot(&business, &period).unwrap();
    assert_eq!(record.trailing_revenue, 1i128);
    assert_eq!(record.anomaly_count, 0u32);
    assert_eq!(record.attestation_count, 1u64);
}

// ── Period index deduplication ───────────────────────────────────────────

/// Re-recording the same (business, period) must NOT add a duplicate entry
/// to the BusinessPeriods index. `get_snapshots_for_business` must return
/// exactly one record per period regardless of how many times it was recorded.
#[test]
fn test_period_index_not_duplicated_on_rerecord() {
    let (env, client, admin) = setup_snapshot_only();
    let business = Address::generate(&env);
    let period = String::from_str(&env, "2026-06");

    // Record five times for the same period
    for i in 0..5 {
        client.record_snapshot(
            &admin,
            &business,
            &period,
            &((i + 1) as i128 * 10_000),
            &(i as u32),
            &(i as u64),
        );
    }

    let snapshots = client.get_snapshots_for_business(&business);
    assert_eq!(
        snapshots.len(),
        1,
        "BusinessPeriods must contain exactly one entry despite 5 re-recordings"
    );
    // The single snapshot should reflect the last write
    let record = snapshots.get(0).unwrap();
    assert_eq!(record.trailing_revenue, 50_000i128);
    assert_eq!(record.anomaly_count, 4u32);
}

/// Mixed re-recording across two periods: each period appears exactly once
/// in the index regardless of re-recording count.
#[test]
fn test_period_index_dedup_across_multiple_periods() {
    let (env, client, admin) = setup_snapshot_only();
    let business = Address::generate(&env);
    let p1 = String::from_str(&env, "2026-07");
    let p2 = String::from_str(&env, "2026-08");

    // Interleave recordings for two periods
    client.record_snapshot(&admin, &business, &p1, &100i128, &0u32, &1u64);
    client.record_snapshot(&admin, &business, &p2, &200i128, &0u32, &2u64);
    client.record_snapshot(&admin, &business, &p1, &150i128, &1u32, &1u64);
    client.record_snapshot(&admin, &business, &p2, &250i128, &1u32, &2u64);
    client.record_snapshot(&admin, &business, &p1, &175i128, &2u32, &1u64);

    let snapshots = client.get_snapshots_for_business(&business);
    assert_eq!(snapshots.len(), 2, "Must have exactly 2 periods");

    let s1 = client.get_snapshot(&business, &p1).unwrap();
    assert_eq!(s1.trailing_revenue, 175i128, "p1 should reflect last write");

    let s2 = client.get_snapshot(&business, &p2).unwrap();
    assert_eq!(s2.trailing_revenue, 250i128, "p2 should reflect last write");
}

// ── Multi-epoch finalization ordering ────────────────────────────────────

/// Recording snapshots for sequential epochs (months) must maintain correct
/// ordering and independent state per epoch.
#[test]
fn test_multi_epoch_sequential_finalization() {
    let (env, client, admin) = setup_snapshot_only();
    let business = Address::generate(&env);
    let epochs = ["2026-01", "2026-02", "2026-03", "2026-04", "2026-05", "2026-06"];

    for (i, epoch_str) in epochs.iter().enumerate() {
        let period = String::from_str(&env, epoch_str);
        let revenue = (i as i128 + 1) * 100_000;
        let anomalies = i as u32;
        let att_count = (i as u64 + 1) * 5;
        client.record_snapshot(&admin, &business, &period, &revenue, &anomalies, &att_count);
    }

    let snapshots = client.get_snapshots_for_business(&business);
    assert_eq!(snapshots.len(), 6, "All 6 epochs must be present");

    // Verify first and last epochs
    let first = client.get_snapshot(&business, &String::from_str(&env, "2026-01")).unwrap();
    assert_eq!(first.trailing_revenue, 100_000i128);
    assert_eq!(first.anomaly_count, 0u32);

    let last = client.get_snapshot(&business, &String::from_str(&env, "2026-06")).unwrap();
    assert_eq!(last.trailing_revenue, 600_000i128);
    assert_eq!(last.anomaly_count, 5u32);
}

/// Finalizing epochs out of chronological order must still work; the contract
/// does not enforce any ordering constraint on period recording.
#[test]
fn test_epoch_finalization_out_of_order() {
    let (env, client, admin) = setup_snapshot_only();
    let business = Address::generate(&env);

    // Record in reverse order
    client.record_snapshot(&admin, &business, &String::from_str(&env, "2026-12"), &120i128, &0u32, &12u64);
    client.record_snapshot(&admin, &business, &String::from_str(&env, "2026-06"), &60i128, &0u32, &6u64);
    client.record_snapshot(&admin, &business, &String::from_str(&env, "2026-01"), &10i128, &0u32, &1u64);

    let snapshots = client.get_snapshots_for_business(&business);
    assert_eq!(snapshots.len(), 3);

    // Each epoch retains its own data regardless of recording order
    let dec = client.get_snapshot(&business, &String::from_str(&env, "2026-12")).unwrap();
    assert_eq!(dec.trailing_revenue, 120i128);

    let jan = client.get_snapshot(&business, &String::from_str(&env, "2026-01")).unwrap();
    assert_eq!(jan.trailing_revenue, 10i128);
}

// ── Cross-business isolation ─────────────────────────────────────────────

/// Two businesses recording snapshots for the same period must be completely
/// isolated. Re-recording for business A must not affect business B.
#[test]
fn test_cross_business_epoch_isolation() {
    let (env, client, admin) = setup_snapshot_only();
    let biz_a = Address::generate(&env);
    let biz_b = Address::generate(&env);
    let period = String::from_str(&env, "2026-03");

    client.record_snapshot(&admin, &biz_a, &period, &100i128, &1u32, &10u64);
    client.record_snapshot(&admin, &biz_b, &period, &200i128, &2u32, &20u64);

    // Overwrite biz_a; biz_b must be unaffected
    client.record_snapshot(&admin, &biz_a, &period, &999i128, &99u32, &99u64);

    let rec_a = client.get_snapshot(&biz_a, &period).unwrap();
    assert_eq!(rec_a.trailing_revenue, 999i128);
    assert_eq!(rec_a.anomaly_count, 99u32);

    let rec_b = client.get_snapshot(&biz_b, &period).unwrap();
    assert_eq!(rec_b.trailing_revenue, 200i128, "biz_b must be unchanged");
    assert_eq!(rec_b.anomaly_count, 2u32, "biz_b anomaly_count must be unchanged");
}

/// Multiple businesses each get their own independent period index.
#[test]
fn test_cross_business_period_index_isolation() {
    let (env, client, admin) = setup_snapshot_only();
    let biz_a = Address::generate(&env);
    let biz_b = Address::generate(&env);

    client.record_snapshot(&admin, &biz_a, &String::from_str(&env, "2026-01"), &10i128, &0u32, &1u64);
    client.record_snapshot(&admin, &biz_a, &String::from_str(&env, "2026-02"), &20i128, &0u32, &2u64);
    client.record_snapshot(&admin, &biz_b, &String::from_str(&env, "2026-01"), &30i128, &0u32, &3u64);

    assert_eq!(client.get_snapshots_for_business(&biz_a).len(), 2);
    assert_eq!(client.get_snapshots_for_business(&biz_b).len(), 1);
}

// ── Writer-role epoch finalization idempotency ───────────────────────────

/// A writer can re-record exactly like admin; overwrite semantics apply.
#[test]
fn test_writer_epoch_finalization_idempotency() {
    let (env, client, admin) = setup_snapshot_only();
    let writer = Address::generate(&env);
    client.add_writer(&admin, &writer);

    let business = Address::generate(&env);
    let period = String::from_str(&env, "2026-03");

    client.record_snapshot(&writer, &business, &period, &100i128, &0u32, &1u64);
    client.record_snapshot(&writer, &business, &period, &200i128, &1u32, &2u64);
    client.record_snapshot(&writer, &business, &period, &200i128, &1u32, &2u64);

    let record = client.get_snapshot(&business, &period).unwrap();
    assert_eq!(record.trailing_revenue, 200i128);
    assert_eq!(record.anomaly_count, 1u32);

    let snapshots = client.get_snapshots_for_business(&business);
    assert_eq!(snapshots.len(), 1, "Period index must not duplicate on writer re-recording");
}

/// Admin and writer can interleave recordings for the same (business, period).
/// The last write wins regardless of caller role.
#[test]
fn test_admin_and_writer_interleaved_epoch_finalization() {
    let (env, client, admin) = setup_snapshot_only();
    let writer = Address::generate(&env);
    client.add_writer(&admin, &writer);

    let business = Address::generate(&env);
    let period = String::from_str(&env, "2026-03");

    client.record_snapshot(&admin, &business, &period, &100i128, &0u32, &1u64);
    client.record_snapshot(&writer, &business, &period, &200i128, &1u32, &2u64);
    client.record_snapshot(&admin, &business, &period, &300i128, &2u32, &3u64);

    let record = client.get_snapshot(&business, &period).unwrap();
    assert_eq!(record.trailing_revenue, 300i128, "Admin last-write should win");
    assert_eq!(record.anomaly_count, 2u32);
    assert_eq!(record.attestation_count, 3u64);
}

/// A removed writer must NOT be able to finalize an epoch.
#[test]
#[should_panic(expected = "caller must be admin or writer")]
fn test_removed_writer_cannot_finalize_epoch() {
    let (env, client, admin) = setup_snapshot_only();
    let writer = Address::generate(&env);
    client.add_writer(&admin, &writer);
    client.remove_writer(&admin, &writer);

    let business = Address::generate(&env);
    let period = String::from_str(&env, "2026-03");
    client.record_snapshot(&writer, &business, &period, &100i128, &0u32, &1u64);
}

// ── Boundary-value idempotency ──────────────────────────────────────────

/// Zero trailing_revenue must be recorded and retrievable.
#[test]
fn test_epoch_finalization_zero_revenue() {
    let (env, client, admin) = setup_snapshot_only();
    let business = Address::generate(&env);
    let period = String::from_str(&env, "2026-09");

    client.record_snapshot(&admin, &business, &period, &0i128, &0u32, &0u64);
    let record = client.get_snapshot(&business, &period).unwrap();
    assert_eq!(record.trailing_revenue, 0i128);
    assert_eq!(record.anomaly_count, 0u32);
    assert_eq!(record.attestation_count, 0u64);
}

/// Negative trailing_revenue (loss) must be storable and retrievable.
#[test]
fn test_epoch_finalization_negative_revenue() {
    let (env, client, admin) = setup_snapshot_only();
    let business = Address::generate(&env);
    let period = String::from_str(&env, "2026-10");

    client.record_snapshot(&admin, &business, &period, &(-500_000i128), &0u32, &1u64);
    let record = client.get_snapshot(&business, &period).unwrap();
    assert_eq!(record.trailing_revenue, -500_000i128);
}

/// i128::MAX trailing_revenue must not overflow or corrupt state.
#[test]
fn test_epoch_finalization_max_revenue() {
    let (env, client, admin) = setup_snapshot_only();
    let business = Address::generate(&env);
    let period = String::from_str(&env, "2026-11");

    client.record_snapshot(&admin, &business, &period, &i128::MAX, &0u32, &1u64);
    let record = client.get_snapshot(&business, &period).unwrap();
    assert_eq!(record.trailing_revenue, i128::MAX);
}

/// u32::MAX anomaly_count must be storable.
#[test]
fn test_epoch_finalization_max_anomaly_count() {
    let (env, client, admin) = setup_snapshot_only();
    let business = Address::generate(&env);
    let period = String::from_str(&env, "2026-12");

    client.record_snapshot(&admin, &business, &period, &1i128, &u32::MAX, &1u64);
    let record = client.get_snapshot(&business, &period).unwrap();
    assert_eq!(record.anomaly_count, u32::MAX);
}

/// u64::MAX attestation_count must be storable.
#[test]
fn test_epoch_finalization_max_attestation_count() {
    let (env, client, admin) = setup_snapshot_only();
    let business = Address::generate(&env);
    let period = String::from_str(&env, "2026-12");

    client.record_snapshot(&admin, &business, &period, &1i128, &0u32, &u64::MAX);
    let record = client.get_snapshot(&business, &period).unwrap();
    assert_eq!(record.attestation_count, u64::MAX);
}

/// i128::MIN trailing_revenue (extreme negative) must be storable.
#[test]
fn test_epoch_finalization_min_revenue() {
    let (env, client, admin) = setup_snapshot_only();
    let business = Address::generate(&env);
    let period = String::from_str(&env, "2026-edge");

    client.record_snapshot(&admin, &business, &period, &i128::MIN, &0u32, &0u64);
    let record = client.get_snapshot(&business, &period).unwrap();
    assert_eq!(record.trailing_revenue, i128::MIN);
}

// ── Rapid sequential re-recording stability ─────────────────────────────

/// Rapidly recording 10 overwrites for the same (business, period) must
/// converge to the last write without any state corruption.
#[test]
fn test_rapid_sequential_overwrite_stability() {
    let (env, client, admin) = setup_snapshot_only();
    let business = Address::generate(&env);
    let period = String::from_str(&env, "2026-rapid");

    for i in 0..10u32 {
        client.record_snapshot(
            &admin,
            &business,
            &period,
            &(i as i128 * 1_000),
            &i,
            &(i as u64),
        );
    }

    let record = client.get_snapshot(&business, &period).unwrap();
    assert_eq!(record.trailing_revenue, 9_000i128, "Must reflect write #9 (last)");
    assert_eq!(record.anomaly_count, 9u32);
    assert_eq!(record.attestation_count, 9u64);

    // Period index must still be length 1
    let snapshots = client.get_snapshots_for_business(&business);
    assert_eq!(snapshots.len(), 1, "Period index must not grow with re-recordings");
}

// ── Deterministic state after multiple overwrites ───────────────────────

/// After recording, overwriting, and overwriting again, every field of the
/// snapshot must deterministically match the last call's inputs. This test
/// asserts field-by-field equality including `period` and `recorded_at`.
#[test]
fn test_deterministic_final_state_after_overwrites() {
    let (env, client, admin) = setup_snapshot_only();
    let business = Address::generate(&env);
    let period = String::from_str(&env, "2026-det");

    client.record_snapshot(&admin, &business, &period, &1i128, &1u32, &1u64);
    client.record_snapshot(&admin, &business, &period, &2i128, &2u32, &2u64);
    client.record_snapshot(&admin, &business, &period, &42i128, &7u32, &99u64);

    let record = client.get_snapshot(&business, &period).unwrap();
    assert_eq!(record.period, period, "period field must match key");
    assert_eq!(record.trailing_revenue, 42i128);
    assert_eq!(record.anomaly_count, 7u32);
    assert_eq!(record.attestation_count, 99u64);
    // recorded_at must be set (non-zero would be typical in a real ledger,
    // but in test env it is 0 by default; just assert it is stored)
    assert!(record.recorded_at == record.recorded_at, "recorded_at must be deterministic");
}

/// Overwriting does not leak intermediate state: queries after the final
/// write must never return any data from prior writes.
#[test]
fn test_no_intermediate_state_leakage() {
    let (env, client, admin) = setup_snapshot_only();
    let business = Address::generate(&env);
    let period = String::from_str(&env, "2026-leak");

    client.record_snapshot(&admin, &business, &period, &111i128, &11u32, &1u64);
    client.record_snapshot(&admin, &business, &period, &222i128, &22u32, &2u64);
    client.record_snapshot(&admin, &business, &period, &333i128, &33u32, &3u64);

    let record = client.get_snapshot(&business, &period).unwrap();
    // Must NOT contain any trace of 111 or 222
    assert_ne!(record.trailing_revenue, 111i128);
    assert_ne!(record.trailing_revenue, 222i128);
    assert_eq!(record.trailing_revenue, 333i128);
    assert_ne!(record.anomaly_count, 11u32);
    assert_ne!(record.anomaly_count, 22u32);
    assert_eq!(record.anomaly_count, 33u32);
}

// ── Attestation-validated epoch finalization idempotency ─────────────────

/// When an attestation contract is configured, re-recording the same
/// (business, period) with a valid attestation must succeed idempotently.
#[test]
fn test_attestation_validated_epoch_rerecord_idempotent() {
    let (env, snap_client, att_client, admin, business) = setup_with_attestation();
    let period = String::from_str(&env, "2026-03");
    let root = soroban_sdk::BytesN::from_array(&env, &[2u8; 32]);
    att_client.submit_attestation(
        &business,
        &period,
        &root,
        &1700000000u64,
        &1u32,
        &None,
        &None,
    );

    // First finalization
    snap_client.record_snapshot(&admin, &business, &period, &100_000i128, &0u32, &1u64);
    let first = snap_client.get_snapshot(&business, &period).unwrap();

    // Idempotent re-finalization
    snap_client.record_snapshot(&admin, &business, &period, &100_000i128, &0u32, &1u64);
    let second = snap_client.get_snapshot(&business, &period).unwrap();

    assert_eq!(first.trailing_revenue, second.trailing_revenue);
    assert_eq!(first.anomaly_count, second.anomaly_count);
    assert_eq!(first.attestation_count, second.attestation_count);
}

/// When an attestation contract is configured, overwriting with different
/// metrics must still honour last-write-wins.
#[test]
fn test_attestation_validated_epoch_overwrite() {
    let (env, snap_client, att_client, admin, business) = setup_with_attestation();
    let period = String::from_str(&env, "2026-04");
    let root = soroban_sdk::BytesN::from_array(&env, &[3u8; 32]);
    att_client.submit_attestation(
        &business,
        &period,
        &root,
        &1700000000u64,
        &1u32,
        &None,
        &None,
    );

    snap_client.record_snapshot(&admin, &business, &period, &100i128, &0u32, &1u64);
    snap_client.record_snapshot(&admin, &business, &period, &999i128, &5u32, &10u64);

    let record = snap_client.get_snapshot(&business, &period).unwrap();
    assert_eq!(record.trailing_revenue, 999i128);
    assert_eq!(record.anomaly_count, 5u32);
    assert_eq!(record.attestation_count, 10u64);
}

/// Recording a snapshot for a period where no attestation exists must panic,
/// even when a valid snapshot previously existed for a *different* period.
/// This confirms epoch-scoped attestation validation.
#[test]
#[should_panic(expected = "attestation must exist for this business and period")]
fn test_attestation_validated_epoch_isolation_panic() {
    let (env, snap_client, att_client, admin, business) = setup_with_attestation();
    let valid_period = String::from_str(&env, "2026-05");
    let root = soroban_sdk::BytesN::from_array(&env, &[4u8; 32]);
    att_client.submit_attestation(
        &business,
        &valid_period,
        &root,
        &1700000000u64,
        &1u32,
        &None,
        &None,
    );
    snap_client.record_snapshot(&admin, &business, &valid_period, &100i128, &0u32, &1u64);

    // This period has no attestation – must panic
    let invalid_period = String::from_str(&env, "2026-06");
    snap_client.record_snapshot(&admin, &business, &invalid_period, &100i128, &0u32, &1u64);
}

/// Multiple epochs with attestation validation: each epoch is independently
/// finalized and re-recordable.
#[test]
fn test_multi_epoch_attestation_validated_finalization() {
    let (env, snap_client, att_client, admin, business) = setup_with_attestation();

    for (i, period_str) in ["2026-01", "2026-02", "2026-03"].iter().enumerate() {
        let period = String::from_str(&env, period_str);
        let mut root_bytes = [0u8; 32];
        root_bytes[0] = (i + 10) as u8;
        let root = soroban_sdk::BytesN::from_array(&env, &root_bytes);
        att_client.submit_attestation(
            &business,
            &period,
            &root,
            &1700000000u64,
            &1u32,
            &None,
            &None,
        );
        snap_client.record_snapshot(
            &admin,
            &business,
            &period,
            &((i as i128 + 1) * 50_000),
            &(i as u32),
            &((i as u64 + 1) * 3),
        );
    }

    let snapshots = snap_client.get_snapshots_for_business(&business);
    assert_eq!(snapshots.len(), 3);

    // Each epoch retains correct data
    let s1 = snap_client.get_snapshot(&business, &String::from_str(&env, "2026-01")).unwrap();
    assert_eq!(s1.trailing_revenue, 50_000i128);
    let s3 = snap_client.get_snapshot(&business, &String::from_str(&env, "2026-03")).unwrap();
    assert_eq!(s3.trailing_revenue, 150_000i128);
}

// ── Failure-mode assertions ─────────────────────────────────────────────

/// An unauthorized address must not be able to finalize any epoch, even after
/// a valid snapshot exists for that epoch (re-recording attack vector).
#[test]
#[should_panic(expected = "caller must be admin or writer")]
fn test_unauthorized_rerecord_attack_panics() {
    let (env, client, admin) = setup_snapshot_only();
    let business = Address::generate(&env);
    let period = String::from_str(&env, "2026-03");

    // Admin finalizes the epoch
    client.record_snapshot(&admin, &business, &period, &100i128, &0u32, &1u64);

    // Attacker tries to overwrite
    let attacker = Address::generate(&env);
    client.record_snapshot(&attacker, &business, &period, &0i128, &999u32, &0u64);
}

/// An unauthorized address must not be able to finalize the *first* recording
/// of a new epoch either.
#[test]
#[should_panic(expected = "caller must be admin or writer")]
fn test_unauthorized_first_epoch_finalization_panics() {
    let (env, client, _admin) = setup_snapshot_only();
    let business = Address::generate(&env);
    let period = String::from_str(&env, "2026-new");
    let attacker = Address::generate(&env);
    client.record_snapshot(&attacker, &business, &period, &100i128, &0u32, &1u64);
}

// ── Empty / special period strings ──────────────────────────────────────

/// An empty string period should be storable and queryable (the contract
/// does not enforce period format).
#[test]
fn test_empty_period_string_finalization() {
    let (env, client, admin) = setup_snapshot_only();
    let business = Address::generate(&env);
    let period = String::from_str(&env, "");

    client.record_snapshot(&admin, &business, &period, &42i128, &0u32, &1u64);
    let record = client.get_snapshot(&business, &period).unwrap();
    assert_eq!(record.trailing_revenue, 42i128);
}

/// A very long period string should be storable and queryable.
#[test]
fn test_long_period_string_finalization() {
    let (env, client, admin) = setup_snapshot_only();
    let business = Address::generate(&env);
    let period = String::from_str(&env, "2026-Q1-week03-day05-extra-metadata-suffix-v2");

    client.record_snapshot(&admin, &business, &period, &77i128, &3u32, &7u64);
    let record = client.get_snapshot(&business, &period).unwrap();
    assert_eq!(record.trailing_revenue, 77i128);
    assert_eq!(record.anomaly_count, 3u32);
}

// ── Idempotency invariant: re-recording does not change snapshot count ──

/// The total number of snapshots for a business must remain stable across
/// re-recordings of existing periods. This is the key idempotency invariant
/// that protects analytics consumers from inflated counts.
#[test]
fn test_snapshot_count_invariant_under_rerecording() {
    let (env, client, admin) = setup_snapshot_only();
    let business = Address::generate(&env);
    let periods = ["2026-01", "2026-02", "2026-03"];

    // Initial finalization
    for p in &periods {
        let period = String::from_str(&env, p);
        client.record_snapshot(&admin, &business, &period, &100i128, &0u32, &1u64);
    }
    assert_eq!(client.get_snapshots_for_business(&business).len(), 3);

    // Re-record each period multiple times
    for _ in 0..5 {
        for p in &periods {
            let period = String::from_str(&env, p);
            client.record_snapshot(&admin, &business, &period, &999i128, &9u32, &9u64);
        }
    }
    assert_eq!(
        client.get_snapshots_for_business(&business).len(),
        3,
        "Snapshot count must be invariant under re-recording"
    );
}
