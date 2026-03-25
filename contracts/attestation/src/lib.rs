/// # Access Control Regression Tests
///
/// Regression tests targeting access control bypass attempts across every
/// admin-gated and authorization-gated entry point of the Veritasor attestation
/// contract as implemented in `lib.rs`.
///
/// ## Real contract entry points covered
/// - `initialize(env, admin, nonce)`
/// - `configure_fees(env, token, collector, base_fee, enabled)`
/// - `grant_role(env, caller, account, role, nonce)`
/// - `submit_attestation(env, business, period: String, merkle_root, timestamp,
///     version, proof_hash: Option<BytesN<32>>, expiry_timestamp: Option<u64>)`
/// - `revoke_attestation(env, caller, business, period: String, reason: String,
///     nonce)`
/// - `migrate_attestation(env, caller, business, period: String, new_root,
///     new_version)`
/// - `submit_multi_period_attestation(env, business, start_period: u32,
///     end_period: u32, merkle_root, timestamp, version)`
/// - `open_dispute(env, challenger, business, period: String, dispute_type,
///     evidence: String)`
/// - `get_admin`, `has_role`, `get_attestation`, `is_expired`, `get_dispute`
///
/// ## What changed from the previous version
/// - `initialize` now takes `(admin, nonce: u64)` — nonce param added
/// - `period` is `soroban_sdk::String` throughout, not `u32`
/// - `submit_attestation` has 8 params: business, period, merkle_root, timestamp,
///   version, proof_hash, expiry_timestamp
/// - `revoke_attestation` takes `(caller, business, period, reason: String, nonce)`
/// - `migrate_attestation` exists; enforces `new_version > current_version`
/// - `grant_role` / `has_role` expose role-based access control
/// - `open_dispute` requires challenger auth
/// - Removed: `init`, `add_authorized_analytics`, `remove_authorized_analytics`,
///   `set_anomaly`, `set_tier_discount`, `set_business_tier`,
///   `set_volume_brackets`, `set_fee_enabled`, `verify_attestation`,
///   `get_attestations_page` — not present on this contract
///
/// ## Security Invariants tested
/// See `docs/security-invariants.md`. Every test is annotated with its SI-XXX id.

#[cfg(test)]
mod access_control_tests {
    use soroban_sdk::{
        testutils::Address as _,
        Address, BytesN, Env, String,
    };

    use crate::{
        dispute::DisputeType,
        ROLE_ADMIN, ROLE_ATTESTOR, ROLE_BUSINESS, ROLE_OPERATOR,
    };

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Fresh environment with all auth mocked, plus an admin and a non-admin.
    fn setup() -> (Env, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let non_admin = Address::generate(&env);
        (env, admin, non_admin)
    }

    /// Register the contract and return its address together with a ready client.
    fn deploy(env: &Env) -> (Address, crate::AttestationContractClient) {
        let id = env.register_contract(None, crate::AttestationContract);
        let client = crate::AttestationContractClient::new(env, &id);
        (id, client)
    }

    /// Convenience: call `initialize` with nonce 0.
    fn init(client: &crate::AttestationContractClient, admin: &Address) {
        client.initialize(admin, &0u64);
    }

    /// A valid 32-byte Merkle root (all 0xAB).
    fn dummy_root(env: &Env) -> BytesN<32> {
        BytesN::from_array(env, &[0xABu8; 32])
    }

    /// An alternate 32-byte root (all 0xCD) for mismatch assertions.
    fn other_root(env: &Env) -> BytesN<32> {
        BytesN::from_array(env, &[0xCDu8; 32])
    }

    /// Convert a `&str` literal to `soroban_sdk::String`.
    fn s(env: &Env, v: &str) -> String {
        String::from_str(env, v)
    }

    /// Submit a bare single-period attestation (no proof hash, no expiry).
    fn submit(
        client: &crate::AttestationContractClient,
        business: &Address,
        period: &String,
        root: &BytesN<32>,
    ) {
        client.submit_attestation(
            business,
            period,
            root,
            &1_700_000_000u64,
            &1u32,
            &None::<BytesN<32>>,
            &None::<u64>,
        );
    }

    // -----------------------------------------------------------------------
    // SI-001 — initialize: one-time-only, admin stored correctly
    // -----------------------------------------------------------------------

    /// SI-001a: `initialize` succeeds on a fresh contract; `get_admin` returns
    /// the supplied address.
    #[test]
    fn test_initialize_succeeds_first_call() {
        let (env, admin, _) = setup();
        let (_, client) = deploy(&env);

        init(&client, &admin);
        assert_eq!(client.get_admin(), admin);
    }

    /// SI-001b: A second call to `initialize` — even with a different nonce or
    /// caller — must panic with "already initialized".
    #[test]
    #[should_panic(expected = "already initialized")]
    fn test_initialize_rejects_second_call() {
        let (env, admin, non_admin) = setup();
        let (_, client) = deploy(&env);

        init(&client, &admin);
        // Any address / any nonce — must still panic.
        client.initialize(&non_admin, &1u64);
    }

    /// SI-001c: Re-initializing with the same admin address and a different nonce
    /// must also panic.
    #[test]
    #[should_panic(expected = "already initialized")]
    fn test_initialize_rejects_same_admin_different_nonce() {
        let (env, admin, _) = setup();
        let (_, client) = deploy(&env);

        client.initialize(&admin, &0u64);
        client.initialize(&admin, &999u64);
    }

    // -----------------------------------------------------------------------
    // SI-002 — configure_fees: admin only, uninitialized guard
    // -----------------------------------------------------------------------

    /// SI-002a: Admin configures fees successfully; stored config is accessible.
    #[test]
    fn test_configure_fees_by_admin_succeeds() {
        let (env, admin, _) = setup();
        let (_, client) = deploy(&env);
        let token = Address::generate(&env);
        let collector = Address::generate(&env);

        init(&client, &admin);
        // This call must not panic.
        client.configure_fees(&token, &collector, &1_000_i128, &true);
    }

    /// SI-002b: Non-admin signing `configure_fees` must panic regardless of the
    /// auth mock.
    #[test]
    #[should_panic]
    fn test_configure_fees_by_non_admin_panics() {
        let (env, admin, non_admin) = setup();
        let (_, client) = deploy(&env);
        let token = Address::generate(&env);
        let collector = Address::generate(&env);

        init(&client, &admin);

        // Strip global mock; give only non_admin's signature.
        env.set_auths(&[]);
        env.mock_auths(&[soroban_sdk::testutils::MockAuth {
            address: &non_admin,
            invoke: &soroban_sdk::testutils::MockAuthInvoke {
                contract: &client.address,
                fn_name: "configure_fees",
                args: soroban_sdk::IntoVal::into_val(
                    &(&token, &collector, &1_000_i128, &true),
                    &env,
                ),
                sub_invokes: &[],
            },
        }]);
        client.configure_fees(&token, &collector, &1_000_i128, &true);
    }

    /// SI-002c: Stripping all auth before `configure_fees` must panic.
    #[test]
    #[should_panic]
    fn test_configure_fees_no_auth_panics() {
        let (env, admin, _) = setup();
        let (_, client) = deploy(&env);
        let token = Address::generate(&env);
        let collector = Address::generate(&env);

        init(&client, &admin);
        env.set_auths(&[]); // remove every auth
        client.configure_fees(&token, &collector, &500_i128, &true);
    }

    /// SI-002d: `configure_fees` before `initialize` must panic — no admin stored.
    #[test]
    #[should_panic]
    fn test_configure_fees_before_initialize_panics() {
        let (env, _, _) = setup();
        let (_, client) = deploy(&env);
        let token = Address::generate(&env);
        let collector = Address::generate(&env);

        client.configure_fees(&token, &collector, &1_000_i128, &true);
    }

    // -----------------------------------------------------------------------
    // SI-003 — grant_role: admin only, role constants correct, uninitialized guard
    // -----------------------------------------------------------------------

    /// SI-003a: Admin grants ROLE_ATTESTOR to a new account; `has_role` confirms.
    #[test]
    fn test_grant_role_attestor_by_admin_succeeds() {
        let (env, admin, _) = setup();
        let (_, client) = deploy(&env);
        let account = Address::generate(&env);

        init(&client, &admin);
        client.grant_role(&admin, &account, &ROLE_ATTESTOR, &0u64);
        assert!(client.has_role(&account, &ROLE_ATTESTOR));
    }

    /// SI-003b: Admin grants ROLE_BUSINESS; `has_role` confirms.
    #[test]
    fn test_grant_role_business_by_admin_succeeds() {
        let (env, admin, _) = setup();
        let (_, client) = deploy(&env);
        let account = Address::generate(&env);

        init(&client, &admin);
        client.grant_role(&admin, &account, &ROLE_BUSINESS, &0u64);
        assert!(client.has_role(&account, &ROLE_BUSINESS));
    }

    /// SI-003c: Admin grants ROLE_OPERATOR; `has_role` confirms.
    #[test]
    fn test_grant_role_operator_by_admin_succeeds() {
        let (env, admin, _) = setup();
        let (_, client) = deploy(&env);
        let account = Address::generate(&env);

        init(&client, &admin);
        client.grant_role(&admin, &account, &ROLE_OPERATOR, &0u64);
        assert!(client.has_role(&account, &ROLE_OPERATOR));
    }

    /// SI-003d: Non-admin as `caller` to `grant_role` must panic.
    #[test]
    #[should_panic]
    fn test_grant_role_by_non_admin_panics() {
        let (env, admin, non_admin) = setup();
        let (_, client) = deploy(&env);
        let account = Address::generate(&env);

        init(&client, &admin);
        // non_admin as caller — access_control::require_admin rejects this.
        client.grant_role(&non_admin, &account, &ROLE_ATTESTOR, &0u64);
    }

    /// SI-003e: An account attempting to grant itself a role (privilege escalation)
    /// must be rejected.
    #[test]
    #[should_panic]
    fn test_grant_role_self_escalation_panics() {
        let (env, admin, _) = setup();
        let (_, client) = deploy(&env);
        let account = Address::generate(&env);

        init(&client, &admin);
        // account is not the admin — trying to grant itself ROLE_ADMIN.
        client.grant_role(&account, &account, &ROLE_ADMIN, &0u64);
    }

    /// SI-003f: `has_role` returns false for every role on an address that was
    /// never assigned one.
    #[test]
    fn test_has_role_returns_false_for_unknown_address() {
        let (env, admin, _) = setup();
        let (_, client) = deploy(&env);
        let stranger = Address::generate(&env);

        init(&client, &admin);
        assert!(!client.has_role(&stranger, &ROLE_ATTESTOR));
        assert!(!client.has_role(&stranger, &ROLE_BUSINESS));
        assert!(!client.has_role(&stranger, &ROLE_OPERATOR));
    }

    /// SI-003g: `grant_role` before `initialize` must panic (no admin stored).
    #[test]
    #[should_panic]
    fn test_grant_role_before_initialize_panics() {
        let (env, _, _) = setup();
        let (_, client) = deploy(&env);
        let fake_admin = Address::generate(&env);
        let account = Address::generate(&env);

        client.grant_role(&fake_admin, &account, &ROLE_ATTESTOR, &0u64);
    }

    // -----------------------------------------------------------------------
    // SI-004 — submit_attestation: business auth required, duplicate rejected,
    //          impersonation blocked, uninitialized guard
    // -----------------------------------------------------------------------

    /// SI-004a: Business submits its own attestation — succeeds.
    #[test]
    fn test_submit_attestation_by_business_succeeds() {
        let (env, admin, _) = setup();
        let (_, client) = deploy(&env);
        let business = Address::generate(&env);

        init(&client, &admin);
        submit(&client, &business, &s(&env, "2024-01"), &dummy_root(&env));

        assert!(client
            .get_attestation(&business, &s(&env, "2024-01"))
            .is_some());
    }

    /// SI-004b: A third party cannot submit on behalf of a business (impersonation).
    #[test]
    #[should_panic]
    fn test_submit_attestation_by_impersonator_panics() {
        let (env, admin, impersonator) = setup();
        let (_, client) = deploy(&env);
        let business = Address::generate(&env);

        init(&client, &admin);

        // Only the impersonator signs — not the business address.
        env.set_auths(&[]);
        env.mock_auths(&[soroban_sdk::testutils::MockAuth {
            address: &impersonator,
            invoke: &soroban_sdk::testutils::MockAuthInvoke {
                contract: &client.address,
                fn_name: "submit_attestation",
                args: soroban_sdk::IntoVal::into_val(
                    &(
                        &business,
                        &s(&env, "2024-01"),
                        &dummy_root(&env),
                        &1_700_000_000u64,
                        &1u32,
                        &None::<BytesN<32>>,
                        &None::<u64>,
                    ),
                    &env,
                ),
                sub_invokes: &[],
            },
        }]);

        client.submit_attestation(
            &business,
            &s(&env, "2024-01"),
            &dummy_root(&env),
            &1_700_000_000u64,
            &1u32,
            &None::<BytesN<32>>,
            &None::<u64>,
        );
    }

    /// SI-004c: Duplicate (business, period) must panic with "attestation exists".
    #[test]
    #[should_panic(expected = "attestation exists")]
    fn test_submit_duplicate_attestation_panics() {
        let (env, admin, _) = setup();
        let (_, client) = deploy(&env);
        let business = Address::generate(&env);

        init(&client, &admin);
        submit(&client, &business, &s(&env, "2024-01"), &dummy_root(&env));
        // Same (business, period) with a different root — still must panic.
        submit(&client, &business, &s(&env, "2024-01"), &other_root(&env));
    }

    /// SI-004d: Same business, different periods — both submissions succeed.
    #[test]
    fn test_submit_attestation_different_periods_both_succeed() {
        let (env, admin, _) = setup();
        let (_, client) = deploy(&env);
        let business = Address::generate(&env);

        init(&client, &admin);
        submit(&client, &business, &s(&env, "2024-01"), &dummy_root(&env));
        submit(&client, &business, &s(&env, "2024-02"), &other_root(&env));

        assert!(client
            .get_attestation(&business, &s(&env, "2024-01"))
            .is_some());
        assert!(client
            .get_attestation(&business, &s(&env, "2024-02"))
            .is_some());
    }

    /// SI-004e: Different businesses can submit the same period independently.
    #[test]
    fn test_submit_attestation_different_businesses_same_period_both_succeed() {
        let (env, admin, _) = setup();
        let (_, client) = deploy(&env);
        let biz_a = Address::generate(&env);
        let biz_b = Address::generate(&env);

        init(&client, &admin);
        submit(&client, &biz_a, &s(&env, "2024-01"), &dummy_root(&env));
        submit(&client, &biz_b, &s(&env, "2024-01"), &other_root(&env));

        assert!(client
            .get_attestation(&biz_a, &s(&env, "2024-01"))
            .is_some());
        assert!(client
            .get_attestation(&biz_b, &s(&env, "2024-01"))
            .is_some());
    }

    /// SI-004f: `submit_attestation` before `initialize` must panic.
    #[test]
    #[should_panic]
    fn test_submit_attestation_before_initialize_panics() {
        let (env, _, _) = setup();
        let (_, client) = deploy(&env);
        let business = Address::generate(&env);

        submit(&client, &business, &s(&env, "2024-01"), &dummy_root(&env));
    }

    // -----------------------------------------------------------------------
    // SI-005 — revoke_attestation: admin only, caller field validated,
    //          uninitialized guard
    // -----------------------------------------------------------------------

    /// SI-005a: Admin revokes an existing attestation successfully.
    #[test]
    fn test_revoke_attestation_by_admin_succeeds() {
        let (env, admin, _) = setup();
        let (_, client) = deploy(&env);
        let business = Address::generate(&env);

        init(&client, &admin);
        submit(&client, &business, &s(&env, "2024-01"), &dummy_root(&env));

        client.revoke_attestation(
            &admin,
            &business,
            &s(&env, "2024-01"),
            &s(&env, "fraud detected"),
            &0u64,
        );
    }

    /// SI-005b: Non-admin address passed as `caller` must panic.
    #[test]
    #[should_panic]
    fn test_revoke_attestation_by_non_admin_panics() {
        let (env, admin, non_admin) = setup();
        let (_, client) = deploy(&env);
        let business = Address::generate(&env);

        init(&client, &admin);
        submit(&client, &business, &s(&env, "2024-01"), &dummy_root(&env));

        // non_admin as caller — dynamic_fees::require_admin must reject.
        client.revoke_attestation(
            &non_admin,
            &business,
            &s(&env, "2024-01"),
            &s(&env, "bypass attempt"),
            &0u64,
        );
    }

    /// SI-005c: `revoke_attestation` before `initialize` must panic.
    #[test]
    #[should_panic]
    fn test_revoke_attestation_before_initialize_panics() {
        let (env, _, _) = setup();
        let (_, client) = deploy(&env);
        let fake_admin = Address::generate(&env);
        let business = Address::generate(&env);

        client.revoke_attestation(
            &fake_admin,
            &business,
            &s(&env, "2024-01"),
            &s(&env, "no admin set"),
            &0u64,
        );
    }

    // -----------------------------------------------------------------------
    // SI-006 — migrate_attestation: admin only (access_control::require_admin),
    //          new_version strictly greater, record must exist
    // -----------------------------------------------------------------------

    /// SI-006a: Admin migrates an existing attestation to a higher version;
    /// stored root and version change.
    #[test]
    fn test_migrate_attestation_by_admin_succeeds() {
        let (env, admin, _) = setup();
        let (_, client) = deploy(&env);
        let business = Address::generate(&env);

        init(&client, &admin);
        submit(&client, &business, &s(&env, "2024-01"), &dummy_root(&env));

        client.migrate_attestation(
            &admin,
            &business,
            &s(&env, "2024-01"),
            &other_root(&env),
            &2u32,
        );

        let (root, _, ver, _, _, _) = client
            .get_attestation(&business, &s(&env, "2024-01"))
            .expect("attestation must still exist after migration");

        assert_eq!(root, other_root(&env));
        assert_eq!(ver, 2u32);
    }

    /// SI-006b: Non-admin as `caller` to `migrate_attestation` must panic.
    #[test]
    #[should_panic]
    fn test_migrate_attestation_by_non_admin_panics() {
        let (env, admin, non_admin) = setup();
        let (_, client) = deploy(&env);
        let business = Address::generate(&env);

        init(&client, &admin);
        submit(&client, &business, &s(&env, "2024-01"), &dummy_root(&env));

        client.migrate_attestation(
            &non_admin,
            &business,
            &s(&env, "2024-01"),
            &other_root(&env),
            &2u32,
        );
    }

    /// SI-006c: Migration with `new_version` equal to the current version must
    /// panic with "version too low".
    #[test]
    #[should_panic(expected = "version too low")]
    fn test_migrate_attestation_same_version_panics() {
        let (env, admin, _) = setup();
        let (_, client) = deploy(&env);
        let business = Address::generate(&env);

        init(&client, &admin);
        // Submitted at version 1.
        submit(&client, &business, &s(&env, "2024-01"), &dummy_root(&env));

        // Attempt migration to version 1 — not strictly greater.
        client.migrate_attestation(
            &admin,
            &business,
            &s(&env, "2024-01"),
            &other_root(&env),
            &1u32,
        );
    }

    /// SI-006d: Migration to a lower version must panic with "version too low".
    #[test]
    #[should_panic(expected = "version too low")]
    fn test_migrate_attestation_lower_version_panics() {
        let (env, admin, _) = setup();
        let (_, client) = deploy(&env);
        let business = Address::generate(&env);

        init(&client, &admin);
        submit(&client, &business, &s(&env, "2024-01"), &dummy_root(&env));

        // First migrate forward to version 5.
        client.migrate_attestation(
            &admin,
            &business,
            &s(&env, "2024-01"),
            &other_root(&env),
            &5u32,
        );
        // Now attempt to step back to version 3 — must panic.
        client.migrate_attestation(
            &admin,
            &business,
            &s(&env, "2024-01"),
            &dummy_root(&env),
            &3u32,
        );
    }

    /// SI-006e: Migrating a non-existent attestation must panic with "not found".
    #[test]
    #[should_panic(expected = "not found")]
    fn test_migrate_nonexistent_attestation_panics() {
        let (env, admin, _) = setup();
        let (_, client) = deploy(&env);
        let business = Address::generate(&env);

        init(&client, &admin);
        // No attestation submitted for "9999-01".
        client.migrate_attestation(
            &admin,
            &business,
            &s(&env, "9999-01"),
            &other_root(&env),
            &2u32,
        );
    }

    // -----------------------------------------------------------------------
    // SI-007 — submit_multi_period_attestation: business auth required,
    //          period ranges must not overlap
    // -----------------------------------------------------------------------

    /// SI-007a: Business submits a valid multi-period range — succeeds.
    #[test]
    fn test_submit_multi_period_by_business_succeeds() {
        let (env, admin, _) = setup();
        let (_, client) = deploy(&env);
        let business = Address::generate(&env);

        init(&client, &admin);
        client.submit_multi_period_attestation(
            &business,
            &202401u32,
            &202412u32,
            &dummy_root(&env),
            &1_700_000_000u64,
            &1u32,
        );
    }

    /// SI-007b: Third party cannot submit a multi-period attestation on behalf of
    /// a business (impersonation).
    #[test]
    #[should_panic]
    fn test_submit_multi_period_by_impersonator_panics() {
        let (env, admin, impersonator) = setup();
        let (_, client) = deploy(&env);
        let business = Address::generate(&env);

        init(&client, &admin);

        env.set_auths(&[]);
        env.mock_auths(&[soroban_sdk::testutils::MockAuth {
            address: &impersonator,
            invoke: &soroban_sdk::testutils::MockAuthInvoke {
                contract: &client.address,
                fn_name: "submit_multi_period_attestation",
                args: soroban_sdk::IntoVal::into_val(
                    &(
                        &business,
                        &202401u32,
                        &202412u32,
                        &dummy_root(&env),
                        &1_700_000_000u64,
                        &1u32,
                    ),
                    &env,
                ),
                sub_invokes: &[],
            },
        }]);

        client.submit_multi_period_attestation(
            &business,
            &202401u32,
            &202412u32,
            &dummy_root(&env),
            &1_700_000_000u64,
            &1u32,
        );
    }

    /// SI-007c: Overlapping period range must panic with "overlap".
    #[test]
    #[should_panic(expected = "overlap")]
    fn test_submit_multi_period_overlap_panics() {
        let (env, admin, _) = setup();
        let (_, client) = deploy(&env);
        let business = Address::generate(&env);

        init(&client, &admin);
        // First range: Jan–Jun 2024.
        client.submit_multi_period_attestation(
            &business,
            &202401u32,
            &202406u32,
            &dummy_root(&env),
            &1_700_000_000u64,
            &1u32,
        );
        // Second range overlaps (Apr–Sep 2024) — must panic.
        client.submit_multi_period_attestation(
            &business,
            &202404u32,
            &202409u32,
            &other_root(&env),
            &1_700_000_001u64,
            &1u32,
        );
    }

    /// SI-007d: Non-overlapping adjacent ranges for the same business both succeed.
    #[test]
    fn test_submit_multi_period_non_overlapping_succeeds() {
        let (env, admin, _) = setup();
        let (_, client) = deploy(&env);
        let business = Address::generate(&env);

        init(&client, &admin);
        // First half of 2024.
        client.submit_multi_period_attestation(
            &business,
            &202401u32,
            &202406u32,
            &dummy_root(&env),
            &1_700_000_000u64,
            &1u32,
        );
        // Second half of 2024 — starts immediately after first ends.
        client.submit_multi_period_attestation(
            &business,
            &202407u32,
            &202412u32,
            &other_root(&env),
            &1_700_000_001u64,
            &1u32,
        );
    }

    // -----------------------------------------------------------------------
    // SI-008 — is_expired: semantics verified against known timestamps
    // -----------------------------------------------------------------------

    /// SI-008a: Attestation submitted without an expiry timestamp is never expired.
    #[test]
    fn test_is_expired_no_expiry_returns_false() {
        let (env, admin, _) = setup();
        let (_, client) = deploy(&env);
        let business = Address::generate(&env);

        init(&client, &admin);
        client.submit_attestation(
            &business,
            &s(&env, "2024-01"),
            &dummy_root(&env),
            &1_700_000_000u64,
            &1u32,
            &None::<BytesN<32>>,
            &None::<u64>,
        );

        assert!(!client.is_expired(&business, &s(&env, "2024-01")));
    }

    /// SI-008b: Far-future expiry (year 2100) is not expired.
    #[test]
    fn test_is_expired_far_future_returns_false() {
        let (env, admin, _) = setup();
        let (_, client) = deploy(&env);
        let business = Address::generate(&env);

        init(&client, &admin);
        // 2100-01-01 00:00:00 UTC in unix seconds.
        let far_future: u64 = 4_102_444_800u64;
        client.submit_attestation(
            &business,
            &s(&env, "2024-01"),
            &dummy_root(&env),
            &1_700_000_000u64,
            &1u32,
            &None::<BytesN<32>>,
            &Some(far_future),
        );

        assert!(!client.is_expired(&business, &s(&env, "2024-01")));
    }

    /// SI-008c: Expiry set to unix timestamp 1 (deep past) is already expired.
    #[test]
    fn test_is_expired_past_expiry_returns_true() {
        let (env, admin, _) = setup();
        let (_, client) = deploy(&env);
        let business = Address::generate(&env);

        init(&client, &admin);
        client.submit_attestation(
            &business,
            &s(&env, "2024-01"),
            &dummy_root(&env),
            &1_700_000_000u64,
            &1u32,
            &None::<BytesN<32>>,
            &Some(1u64), // epoch second 1 — long expired
        );

        assert!(client.is_expired(&business, &s(&env, "2024-01")));
    }

    /// SI-008d: `is_expired` on a non-existent attestation returns false without
    /// panicking.
    #[test]
    fn test_is_expired_nonexistent_attestation_returns_false() {
        let (env, admin, _) = setup();
        let (_, client) = deploy(&env);
        let business = Address::generate(&env);

        init(&client, &admin);
        assert!(!client.is_expired(&business, &s(&env, "9999-99")));
    }

    // -----------------------------------------------------------------------
    // SI-009 — open_dispute: challenger auth required, unique ids assigned
    // -----------------------------------------------------------------------

    /// SI-009a: Challenger opens a dispute with valid auth — dispute stored.
    #[test]
    fn test_open_dispute_by_challenger_succeeds() {
        let (env, admin, _) = setup();
        let (_, client) = deploy(&env);
        let challenger = Address::generate(&env);
        let business = Address::generate(&env);

        init(&client, &admin);
        let id = client.open_dispute(
            &challenger,
            &business,
            &s(&env, "2024-01"),
            &DisputeType::DataIntegrity,
            &s(&env, "hash mismatch on revenue record"),
        );

        let d = client.get_dispute(&id).expect("dispute must be stored");
        assert_eq!(d.challenger, challenger);
        assert_eq!(d.business, business);
    }

    /// SI-009b: Two disputes on the same (business, period) each get a unique id.
    #[test]
    fn test_open_dispute_multiple_have_unique_ids() {
        let (env, admin, _) = setup();
        let (_, client) = deploy(&env);
        let challenger = Address::generate(&env);
        let business = Address::generate(&env);

        init(&client, &admin);
        let id1 = client.open_dispute(
            &challenger,
            &business,
            &s(&env, "2024-01"),
            &DisputeType::DataIntegrity,
            &s(&env, "first evidence"),
        );
        let id2 = client.open_dispute(
            &challenger,
            &business,
            &s(&env, "2024-01"),
            &DisputeType::DataIntegrity,
            &s(&env, "second evidence"),
        );

        assert_ne!(id1, id2);
    }

    /// SI-009c: Impersonator cannot open a dispute on behalf of a real challenger.
    #[test]
    #[should_panic]
    fn test_open_dispute_by_impersonator_panics() {
        let (env, admin, impersonator) = setup();
        let (_, client) = deploy(&env);
        let challenger = Address::generate(&env);
        let business = Address::generate(&env);

        init(&client, &admin);

        env.set_auths(&[]);
        env.mock_auths(&[soroban_sdk::testutils::MockAuth {
            address: &impersonator,
            invoke: &soroban_sdk::testutils::MockAuthInvoke {
                contract: &client.address,
                fn_name: "open_dispute",
                args: soroban_sdk::IntoVal::into_val(
                    &(
                        &challenger,
                        &business,
                        &s(&env, "2024-01"),
                        &DisputeType::DataIntegrity,
                        &s(&env, "fake evidence"),
                    ),
                    &env,
                ),
                sub_invokes: &[],
            },
        }]);

        client.open_dispute(
            &challenger,
            &business,
            &s(&env, "2024-01"),
            &DisputeType::DataIntegrity,
            &s(&env, "fake evidence"),
        );
    }

    /// SI-009d: `get_dispute` returns None for an id that was never issued.
    #[test]
    fn test_get_dispute_nonexistent_returns_none() {
        let (env, admin, _) = setup();
        let (_, client) = deploy(&env);

        init(&client, &admin);
        assert!(client.get_dispute(&u64::MAX).is_none());
    }

    // -----------------------------------------------------------------------
    // SI-010 — get_admin: read-only correctness and immutability
    // -----------------------------------------------------------------------

    /// SI-010a: `get_admin` returns the address passed to `initialize`.
    #[test]
    fn test_get_admin_matches_initializer() {
        let (env, admin, _) = setup();
        let (_, client) = deploy(&env);

        init(&client, &admin);
        assert_eq!(client.get_admin(), admin);
    }

    /// SI-010b: Repeated reads of `get_admin` are idempotent.
    #[test]
    fn test_get_admin_is_idempotent() {
        let (env, admin, _) = setup();
        let (_, client) = deploy(&env);

        init(&client, &admin);
        assert_eq!(client.get_admin(), client.get_admin());
    }

    /// SI-010c: `get_admin` never returns a non-admin address.
    #[test]
    fn test_get_admin_does_not_return_non_admin() {
        let (env, admin, non_admin) = setup();
        let (_, client) = deploy(&env);

        init(&client, &admin);
        assert_ne!(client.get_admin(), non_admin);
    }

    // -----------------------------------------------------------------------
    // SI-011 — Cross-feature: role assignment does not grant fee-admin rights;
    //          fee config does not grant role rights
    // -----------------------------------------------------------------------

    /// SI-011a: Configuring fees does not automatically grant any role to the
    /// token address or the collector.
    #[test]
    fn test_fee_token_has_no_role_after_configure_fees() {
        let (env, admin, _) = setup();
        let (_, client) = deploy(&env);
        let token = Address::generate(&env);
        let collector = Address::generate(&env);

        init(&client, &admin);
        client.configure_fees(&token, &collector, &1_000_i128, &true);

        assert!(!client.has_role(&token, &ROLE_ADMIN));
        assert!(!client.has_role(&token, &ROLE_ATTESTOR));
        assert!(!client.has_role(&collector, &ROLE_ADMIN));
    }

    /// SI-011b: A ROLE_OPERATOR holder cannot call admin-only `configure_fees`.
    #[test]
    #[should_panic]
    fn test_role_holder_cannot_configure_fees() {
        let (env, admin, _) = setup();
        let (_, client) = deploy(&env);
        let operator = Address::generate(&env);
        let token = Address::generate(&env);
        let collector = Address::generate(&env);

        init(&client, &admin);
        client.grant_role(&admin, &operator, &ROLE_OPERATOR, &0u64);

        // operator has a role but is not the stored admin.
        env.set_auths(&[]);
        env.mock_auths(&[soroban_sdk::testutils::MockAuth {
            address: &operator,
            invoke: &soroban_sdk::testutils::MockAuthInvoke {
                contract: &client.address,
                fn_name: "configure_fees",
                args: soroban_sdk::IntoVal::into_val(
                    &(&token, &collector, &500_i128, &true),
                    &env,
                ),
                sub_invokes: &[],
            },
        }]);
        client.configure_fees(&token, &collector, &500_i128, &true);
    }

    // -----------------------------------------------------------------------
    // SI-012 — Caller-field spoofing: revoke, migrate, grant_role
    // -----------------------------------------------------------------------

    /// SI-012a: Passing non_admin as `caller` to `revoke_attestation` must panic
    /// because `dynamic_fees::require_admin` validates against the stored admin.
    #[test]
    #[should_panic]
    fn test_caller_spoofing_revoke_panics() {
        let (env, admin, non_admin) = setup();
        let (_, client) = deploy(&env);
        let business = Address::generate(&env);

        init(&client, &admin);
        submit(&client, &business, &s(&env, "2024-01"), &dummy_root(&env));

        client.revoke_attestation(
            &non_admin, // spoofed caller
            &business,
            &s(&env, "2024-01"),
            &s(&env, "spoofed caller attempt"),
            &0u64,
        );
    }

    /// SI-012b: Passing non_admin as `caller` to `migrate_attestation` must panic.
    #[test]
    #[should_panic]
    fn test_caller_spoofing_migrate_panics() {
        let (env, admin, non_admin) = setup();
        let (_, client) = deploy(&env);
        let business = Address::generate(&env);

        init(&client, &admin);
        submit(&client, &business, &s(&env, "2024-01"), &dummy_root(&env));

        client.migrate_attestation(
            &non_admin, // spoofed caller
            &business,
            &s(&env, "2024-01"),
            &other_root(&env),
            &2u32,
        );
    }

    /// SI-012c: Passing non_admin as `caller` to `grant_role` must panic.
    #[test]
    #[should_panic]
    fn test_caller_spoofing_grant_role_panics() {
        let (env, admin, non_admin) = setup();
        let (_, client) = deploy(&env);
        let account = Address::generate(&env);

        init(&client, &admin);
        client.grant_role(&non_admin, &account, &ROLE_ATTESTOR, &0u64);
    }

    // -----------------------------------------------------------------------
    // SI-013 — get_attestation: read-only; returns correct stored data
    // -----------------------------------------------------------------------

    /// SI-013a: `get_attestation` returns None for a record that was never stored.
    #[test]
    fn test_get_attestation_nonexistent_returns_none() {
        let (env, admin, _) = setup();
        let (_, client) = deploy(&env);
        let business = Address::generate(&env);

        init(&client, &admin);
        assert!(client
            .get_attestation(&business, &s(&env, "9999-01"))
            .is_none());
    }

    /// SI-013b: `get_attestation` returns the correct tuple fields after
    /// submission — root, timestamp, version, fee_paid, proof_hash, expiry.
    #[test]
    fn test_get_attestation_returns_correct_fields() {
        let (env, admin, _) = setup();
        let (_, client) = deploy(&env);
        let business = Address::generate(&env);
        let root = dummy_root(&env);

        init(&client, &admin);
        client.submit_attestation(
            &business,
            &s(&env, "2024-01"),
            &root,
            &1_700_000_000u64,
            &3u32,
            &None::<BytesN<32>>,
            &None::<u64>,
        );

        let (stored_root, ts, ver, _fee, proof_hash, expiry) = client
            .get_attestation(&business, &s(&env, "2024-01"))
            .expect("attestation must exist");

        assert_eq!(stored_root, root);
        assert_eq!(ts, 1_700_000_000u64);
        assert_eq!(ver, 3u32);
        assert!(proof_hash.is_none());
        assert!(expiry.is_none());
    }

    /// SI-013c: An optional proof hash is stored and returned correctly.
    #[test]
    fn test_get_attestation_with_proof_hash_returns_hash() {
        let (env, admin, _) = setup();
        let (_, client) = deploy(&env);
        let business = Address::generate(&env);
        let root = dummy_root(&env);
        let proof = Some(other_root(&env));

        init(&client, &admin);
        client.submit_attestation(
            &business,
            &s(&env, "2024-01"),
            &root,
            &1_700_000_000u64,
            &1u32,
            &proof,
            &None::<u64>,
        );

        let (_, _, _, _, stored_proof, _) = client
            .get_attestation(&business, &s(&env, "2024-01"))
            .unwrap();

        assert_eq!(stored_proof, proof);
    }

    /// SI-013d: An optional expiry timestamp is stored and returned correctly.
    #[test]
    fn test_get_attestation_with_expiry_returns_expiry() {
        let (env, admin, _) = setup();
        let (_, client) = deploy(&env);
        let business = Address::generate(&env);
        let far_future: u64 = 4_102_444_800u64;

        init(&client, &admin);
        client.submit_attestation(
            &business,
            &s(&env, "2024-01"),
            &dummy_root(&env),
            &1_700_000_000u64,
            &1u32,
            &None::<BytesN<32>>,
            &Some(far_future),
        );

        let (_, _, _, _, _, expiry) = client
            .get_attestation(&business, &s(&env, "2024-01"))
            .unwrap();

        assert_eq!(expiry, Some(far_future));
    }
}