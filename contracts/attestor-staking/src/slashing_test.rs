use super::*;
use soroban_sdk::{testutils::{Address as _, Ledger as _}, token, Address, Env};
use soroban_sdk::{contract, contractimpl};
use proptest::prelude::*;

#[contract]
struct DummyDisputeContract;

#[contractimpl]
impl DummyDisputeContract {}

fn create_token_contract<'a>(
    env: &Env,
    admin: &Address,
) -> (Address, token::StellarAssetClient<'a>, token::Client<'a>) {
    let contract_id = env.register_stellar_asset_contract_v2(admin.clone());
    let addr = contract_id.address();
    (
        addr.clone(),
        token::StellarAssetClient::new(env, &addr),
        token::Client::new(env, &addr),
    )
}

/// Shared setup helper: initializes the contract and stakes `stake_amount` tokens for the attestor.
/// Returns `(attestor, treasury, dispute_contract, token_id, client)`.
fn setup(
    env: &Env,
    stake_amount: i128,
) -> (
    Address,
    Address,
    Address,
    Address,
    AttestorStakingContractClient<'_>,
) {
    let admin = Address::generate(env);
    let attestor = Address::generate(env);
    let treasury = Address::generate(env);
    let dispute_contract = env.register(DummyDisputeContract, ());

    let (token_id, token_admin, _token_client) = create_token_contract(env, &admin);
    token_admin.mint(&attestor, &stake_amount);

    let contract_id = env.register(AttestorStakingContract, ());
    let client = AttestorStakingContractClient::new(env, &contract_id);

    client.initialize(&admin, &token_id, &treasury, &1000, &dispute_contract, &0u64);
    client.stake(&attestor, &stake_amount);

    (attestor, treasury, dispute_contract, token_id, client)
}

#[test]
fn test_slash_success() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let attestor = Address::generate(&env);
    let treasury = Address::generate(&env);
    let dispute_contract = env.register(DummyDisputeContract, ());

    let (token_id, token_admin, token_client) = create_token_contract(&env, &admin);
    token_admin.mint(&attestor, &10000);

    let contract_id = env.register(AttestorStakingContract, ());
    let client = AttestorStakingContractClient::new(&env, &contract_id);

    client.initialize(&admin, &token_id, &treasury, &1000, &dispute_contract, &0u64);
    client.stake(&attestor, &5000);

    let initial_treasury_balance = token_client.balance(&treasury);

    // Slash 2000 tokens
    env.as_contract(&dispute_contract, || {
        let outcome = client.slash(&attestor, &2000, &1);
        assert_eq!(outcome, SlashOutcome::Slashed);
    });

    let stake = client.get_stake(&attestor).unwrap();
    assert_eq!(stake.amount, 3000);

    let treasury_balance = token_client.balance(&treasury);
    assert_eq!(treasury_balance, initial_treasury_balance + 2000);
}

#[test]
fn test_slash_partial_when_insufficient_stake() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let attestor = Address::generate(&env);
    let treasury = Address::generate(&env);
    let dispute_contract = env.register(DummyDisputeContract, ());

    let (token_id, token_admin, token_client) = create_token_contract(&env, &admin);
    token_admin.mint(&attestor, &10000);

    let contract_id = env.register(AttestorStakingContract, ());
    let client = AttestorStakingContractClient::new(&env, &contract_id);

    client.initialize(&admin, &token_id, &treasury, &1000, &dispute_contract, &0u64);
    client.stake(&attestor, &2000);

    let initial_treasury_balance = token_client.balance(&treasury);

    // Try to slash 5000 but only 2000 available
    env.as_contract(&dispute_contract, || {
        let outcome = client.slash(&attestor, &5000, &1);
        assert_eq!(outcome, SlashOutcome::Slashed);
    });

    let stake = client.get_stake(&attestor).unwrap();
    assert_eq!(stake.amount, 0);

    let treasury_balance = token_client.balance(&treasury);
    assert_eq!(treasury_balance, initial_treasury_balance + 2000);
}

#[test]
#[should_panic(expected = "dispute already processed")]
fn test_slash_double_slashing_prevented() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let attestor = Address::generate(&env);
    let treasury = Address::generate(&env);
    let dispute_contract = env.register(DummyDisputeContract, ());

    let (token_id, token_admin, _token_client) = create_token_contract(&env, &admin);
    token_admin.mint(&attestor, &10000);

    let contract_id = env.register(AttestorStakingContract, ());
    let client = AttestorStakingContractClient::new(&env, &contract_id);

    client.initialize(&admin, &token_id, &treasury, &1000, &dispute_contract, &0u64);
    client.stake(&attestor, &5000);

    env.as_contract(&dispute_contract, || {
        client.slash(&attestor, &2000, &1);
    });
    // Second slash with same dispute_id should panic
    env.as_contract(&dispute_contract, || {
        client.slash(&attestor, &1000, &1);
    });
}

#[test]
fn test_slash_multiple_disputes() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let attestor = Address::generate(&env);
    let treasury = Address::generate(&env);
    let dispute_contract = env.register(DummyDisputeContract, ());

    let (token_id, token_admin, token_client) = create_token_contract(&env, &admin);
    token_admin.mint(&attestor, &10000);

    let contract_id = env.register(AttestorStakingContract, ());
    let client = AttestorStakingContractClient::new(&env, &contract_id);

    client.initialize(&admin, &token_id, &treasury, &1000, &dispute_contract, &0u64);
    client.stake(&attestor, &5000);

    let initial_treasury_balance = token_client.balance(&treasury);

    // Slash for dispute 1
    env.as_contract(&dispute_contract, || {
        client.slash(&attestor, &1000, &1);
    });
    // Slash for dispute 2 (different dispute_id)
    env.as_contract(&dispute_contract, || {
        client.slash(&attestor, &1500, &2);
    });

    let stake = client.get_stake(&attestor).unwrap();
    assert_eq!(stake.amount, 2500);

    let treasury_balance = token_client.balance(&treasury);
    assert_eq!(treasury_balance, initial_treasury_balance + 2500);
}

#[test]
fn test_slash_no_stake() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let attestor = Address::generate(&env);
    let treasury = Address::generate(&env);
    let dispute_contract = env.register(DummyDisputeContract, ());

    let (token_id, _token_admin, _token_client) = create_token_contract(&env, &admin);

    let contract_id = env.register(AttestorStakingContract, ());
    let client = AttestorStakingContractClient::new(&env, &contract_id);

    client.initialize(&admin, &token_id, &treasury, &1000, &dispute_contract, &0u64);

    // Try to slash attestor with no stake - should panic
    let result = client.try_slash(&attestor, &1000, &1);
    assert!(result.is_err());
}

#[test]
fn test_slash_zero_stake_returns_no_slash() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let attestor = Address::generate(&env);
    let treasury = Address::generate(&env);
    let dispute_contract = env.register(DummyDisputeContract, ());

    let (token_id, token_admin, token_client) = create_token_contract(&env, &admin);
    token_admin.mint(&attestor, &10000);

    let contract_id = env.register(AttestorStakingContract, ());
    let client = AttestorStakingContractClient::new(&env, &contract_id);

    client.initialize(&admin, &token_id, &treasury, &1000, &dispute_contract, &0u64);
    client.stake(&attestor, &1000);

    // Slash all stake
    env.as_contract(&dispute_contract, || {
        client.slash(&attestor, &1000, &1);
    });

    // Capture treasury balance before the NoSlash call (Req 2.5, Req 11.2)
    let treasury_balance_before = token_client.balance(&treasury);

    // Try to slash again with different dispute_id - should return NoSlash
    env.as_contract(&dispute_contract, || {
        let outcome = client.slash(&attestor, &500, &2);
        // Req 2.4: NoSlash returned when stake.amount == 0
        assert_eq!(outcome, SlashOutcome::NoSlash);
    });

    // Req 2.5 / Req 11.2: treasury balance must be unchanged on NoSlash
    assert_eq!(
        token_client.balance(&treasury),
        treasury_balance_before,
        "treasury balance must not change when NoSlash is returned"
    );
}

/// Test scenario: Dispute resolved as Upheld -> Slashing triggered
#[test]
fn test_dispute_resolution_triggers_slashing() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let attestor = Address::generate(&env);
    let treasury = Address::generate(&env);
    let dispute_contract = env.register(DummyDisputeContract, ());

    let (token_id, token_admin, token_client) = create_token_contract(&env, &admin);
    token_admin.mint(&attestor, &10000);

    let contract_id = env.register(AttestorStakingContract, ());
    let client = AttestorStakingContractClient::new(&env, &contract_id);

    client.initialize(&admin, &token_id, &treasury, &1000, &dispute_contract, &0u64);
    client.stake(&attestor, &5000);

    let initial_treasury = token_client.balance(&treasury);

    // Simulate dispute resolution: dispute_id=42, slash 30% of stake
    let slash_amount = 1500;
    env.as_contract(&dispute_contract, || {
        let outcome = client.slash(&attestor, &slash_amount, &42);
        assert_eq!(outcome, SlashOutcome::Slashed);
    });
    assert_eq!(client.get_stake(&attestor).unwrap().amount, 3500);
    assert_eq!(
        token_client.balance(&treasury),
        initial_treasury + slash_amount
    );
}

/// Test scenario: Slash with amount = 0 panics with correct message (Req 7.1)
#[test]
#[should_panic(expected = "slash amount must be positive")]
fn test_slash_amount_zero_panics() {
    let env = Env::default();
    env.mock_all_auths();

    let (attestor, _treasury, dispute_contract, _token_id, client) = setup(&env, 5000);

    env.as_contract(&dispute_contract, || {
        client.slash(&attestor, &0, &1);
    });
}

/// Test scenario: Zero-amount slash does not consume the dispute ID (Req 7.2, 7.3)
///
/// A `slash` call with `amount = 0` must panic before recording the dispute ID.
/// After the failed call, `stake.amount` must be unchanged and the same `dispute_id`
/// must still be usable for a subsequent valid slash.
#[test]
fn test_slash_zero_amount_does_not_consume_dispute_id() {
    let env = Env::default();
    env.mock_all_auths();

    let stake_amount = 5000_i128;
    let (attestor, _treasury, dispute_contract, _token_id, client) = setup(&env, stake_amount);

    let dispute_id: u64 = 42;

    // Attempt slash with amount = 0 — must panic (Req 7.1).
    // Use try_slash so the test can continue after the expected failure.
    let result = env.as_contract(&dispute_contract, || {
        client.try_slash(&attestor, &0, &dispute_id)
    });
    assert!(result.is_err(), "slash with amount=0 should panic");

    // Req 7.3: stake.amount must be unchanged after the failed call.
    let stake_after_failed = client.get_stake(&attestor).unwrap();
    assert_eq!(
        stake_after_failed.amount, stake_amount,
        "stake.amount must be unchanged after zero-amount slash"
    );

    // Req 7.2: the dispute_id was NOT consumed, so a valid slash with the same
    // dispute_id must succeed.
    let outcome = env.as_contract(&dispute_contract, || {
        client.slash(&attestor, &1000, &dispute_id)
    });
    assert_eq!(
        outcome,
        SlashOutcome::Slashed,
        "valid slash with previously-failed dispute_id should succeed"
    );

    // Confirm the stake was actually reduced by the valid slash.
    let stake_after_valid = client.get_stake(&attestor).unwrap();
    assert_eq!(
        stake_after_valid.amount,
        stake_amount - 1000,
        "stake.amount should reflect the valid slash"
    );
}

/// Test scenario: Slash reduces pending unstake to zero but preserves the record (Req 4.5)
///
/// After staking and requesting unstake for the full amount, slashing the full amount
/// must reduce `pending.amount` to 0 while keeping the `PendingUnstake` record present
/// so that `withdraw_unstaked` can still be called to clean up state.
#[test]
fn test_slash_pending_reduced_to_zero_record_preserved() {
    let env = Env::default();
    env.mock_all_auths();

    let stake_amount = 5000_i128;
    let (attestor, _treasury, dispute_contract, _token_id, client) = setup(&env, stake_amount);

    // Request unstake for the full staked amount
    client.request_unstake(&attestor, &stake_amount);

    // Confirm pending unstake exists with the full amount
    let pending_before = client.get_pending_unstake(&attestor).unwrap();
    assert_eq!(pending_before.amount, stake_amount);

    // Slash the full amount — this should reduce pending.amount to 0
    env.as_contract(&dispute_contract, || {
        let outcome = client.slash(&attestor, &stake_amount, &1);
        assert_eq!(outcome, SlashOutcome::Slashed);
    });

    // Req 4.5: PendingUnstake record must still exist (Some), with amount == 0
    let pending_after = client.get_pending_unstake(&attestor);
    assert!(
        pending_after.is_some(),
        "PendingUnstake record must be preserved after slash reduces it to zero"
    );
    assert_eq!(
        pending_after.unwrap().amount,
        0,
        "pending.amount must be 0 after full slash"
    );

    // Confirm stake.amount is also 0
    let stake_after = client.get_stake(&attestor).unwrap();
    assert_eq!(stake_after.amount, 0);
    assert_eq!(stake_after.locked, 0);
}

/// Test scenario: Withdraw after pending unstake is slashed to zero transfers 0 tokens (Req 9.3)
///
/// After staking, requesting unstake for the full amount, and slashing the full amount,
/// calling `withdraw_unstaked` after the unbonding period must:
/// - Transfer exactly 0 tokens to the attestor
/// - Clean up the PendingUnstake record
#[test]
fn test_slash_pending_zero_then_withdraw() {
    let env = Env::default();
    env.mock_all_auths();

    let stake_amount = 5000_i128;
    let (attestor, _treasury, dispute_contract, token_id, client) = setup(&env, stake_amount);

    let token_client = token::Client::new(&env, &token_id);

    // Request unstake for the full staked amount
    client.request_unstake(&attestor, &stake_amount);

    // Slash the full amount — reduces pending.amount to 0
    env.as_contract(&dispute_contract, || {
        let outcome = client.slash(&attestor, &stake_amount, &1);
        assert_eq!(outcome, SlashOutcome::Slashed);
    });

    // Confirm pending record exists with amount == 0 (Req 4.5)
    let pending = client.get_pending_unstake(&attestor).unwrap();
    assert_eq!(pending.amount, 0);

    // Advance ledger past the unbonding period unlock timestamp
    let unlock_ts = pending.unlock_timestamp;
    env.ledger().set_timestamp(unlock_ts + 1);

    // Record attestor balance before withdrawal
    let attestor_balance_before = token_client.balance(&attestor);

    // Call withdraw_unstaked — should transfer 0 tokens and clean up the record
    client.withdraw_unstaked(&attestor);

    // Req 9.3: 0 tokens transferred to attestor
    let attestor_balance_after = token_client.balance(&attestor);
    assert_eq!(
        attestor_balance_after,
        attestor_balance_before,
        "attestor should receive 0 tokens when pending.amount is 0"
    );

    // Req 9.3: pending unstake record cleaned up
    assert!(
        client.get_pending_unstake(&attestor).is_none(),
        "PendingUnstake record must be removed after withdraw_unstaked"
    );
}

/// Test scenario: Slash after withdraw_unstaked only affects remaining stake (Req 9.4)
///
/// After staking, requesting unstake for a partial amount, advancing the ledger,
/// and withdrawing the unstaked tokens, slashing the remaining stake must:
/// - Only reduce the remaining `stake.amount` (not the already-withdrawn amount)
/// - Leave the attestor's withdrawn balance unaffected
#[test]
fn test_slash_after_withdraw_unstaked() {
    let env = Env::default();
    env.mock_all_auths();

    let stake_amount = 5000_i128;
    let unstake_amount = 2000_i128;
    let remaining_stake = stake_amount - unstake_amount; // 3000
    let slash_amount = 1000_i128;

    let (attestor, _treasury, dispute_contract, token_id, client) = setup(&env, stake_amount);
    let token_client = token::Client::new(&env, &token_id);

    // Step 1: Request unstake for a partial amount
    client.request_unstake(&attestor, &unstake_amount);

    let pending = client.get_pending_unstake(&attestor).unwrap();
    assert_eq!(pending.amount, unstake_amount);

    // Step 2: Advance ledger past the unbonding period (unbonding_period = 0, so already unlocked)
    env.ledger().set_timestamp(pending.unlock_timestamp + 1);

    // Step 3: Withdraw the unstaked tokens
    let attestor_balance_before_withdraw = token_client.balance(&attestor);
    client.withdraw_unstaked(&attestor);
    let attestor_balance_after_withdraw = token_client.balance(&attestor);

    // Confirm withdrawal transferred the correct amount
    assert_eq!(
        attestor_balance_after_withdraw,
        attestor_balance_before_withdraw + unstake_amount,
        "withdraw_unstaked should transfer exactly unstake_amount to attestor"
    );

    // Confirm pending record is cleaned up
    assert!(
        client.get_pending_unstake(&attestor).is_none(),
        "PendingUnstake record must be removed after withdraw_unstaked"
    );

    // Confirm remaining stake
    let stake_after_withdraw = client.get_stake(&attestor).unwrap();
    assert_eq!(
        stake_after_withdraw.amount, remaining_stake,
        "stake.amount should equal stake_amount - unstake_amount after withdrawal"
    );
    assert_eq!(
        stake_after_withdraw.locked, 0,
        "stake.locked should be 0 after withdrawal"
    );

    // Step 4: Slash the remaining stake
    env.as_contract(&dispute_contract, || {
        let outcome = client.slash(&attestor, &slash_amount, &1);
        assert_eq!(outcome, SlashOutcome::Slashed);
    });

    // Req 9.4: slash only affects the remaining stake.amount
    let stake_after_slash = client.get_stake(&attestor).unwrap();
    assert_eq!(
        stake_after_slash.amount,
        remaining_stake - slash_amount,
        "slash must only reduce the remaining stake, not the already-withdrawn amount"
    );

    // Req 9.4: the already-withdrawn amount is unaffected — attestor's token balance unchanged
    assert_eq!(
        token_client.balance(&attestor),
        attestor_balance_after_withdraw,
        "attestor's withdrawn balance must not be affected by the subsequent slash"
    );
}

/// Test scenario: Frivolous slashing attempt (unauthorized caller)
#[test]
#[should_panic]
fn test_frivolous_slashing_blocked() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let attestor = Address::generate(&env);
    let treasury = Address::generate(&env);
    let dispute_contract = env.register(DummyDisputeContract, ());
    let malicious_caller = env.register(DummyDisputeContract, ());

    let (token_id, token_admin, _token_client) = create_token_contract(&env, &admin);
    token_admin.mint(&attestor, &10000);

    let contract_id = env.register(AttestorStakingContract, ());
    let client = AttestorStakingContractClient::new(&env, &contract_id);

    client.initialize(&admin, &token_id, &treasury, &1000, &dispute_contract, &0u64);
    client.stake(&attestor, &5000);

    // Malicious caller tries to slash by impersonating a non-dispute contract.
    env.as_contract(&malicious_caller, || {
        client.slash(&attestor, &2000, &99);
    });
}

/// Test scenario: Dispute contract update rejects old address and accepts new address (Req 6.3, 6.4)
///
/// After calling `set_dispute_contract` to update to a new dispute contract address:
/// - The old dispute contract address must be rejected when calling `slash`
/// - The new dispute contract address must be accepted when calling `slash`
#[test]
fn test_dispute_contract_update_rejects_old_accepts_new() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let attestor = Address::generate(&env);
    let treasury = Address::generate(&env);

    // Register two distinct dispute contracts
    let dispute_contract_a = env.register(DummyDisputeContract, ());
    let dispute_contract_b = env.register(DummyDisputeContract, ());

    let (token_id, token_admin, _token_client) = create_token_contract(&env, &admin);
    token_admin.mint(&attestor, &10000);

    let contract_id = env.register(AttestorStakingContract, ());
    let client = AttestorStakingContractClient::new(&env, &contract_id);

    // Initialize with dispute_contract_A
    client.initialize(&admin, &token_id, &treasury, &1000, &dispute_contract_a, &0u64);
    client.stake(&attestor, &5000);

    // Update dispute contract to dispute_contract_B (Req 6.3, 6.4)
    client.set_dispute_contract(&dispute_contract_b);

    // Req 6.3: old dispute contract (A) must now be rejected
    let result = env.as_contract(&dispute_contract_a, || {
        client.try_slash(&attestor, &1000, &1)
    });
    assert!(
        result.is_err(),
        "slash from old dispute_contract_A must be rejected after update"
    );

    // Req 6.4: new dispute contract (B) must be accepted
    let outcome = env.as_contract(&dispute_contract_b, || {
        client.slash(&attestor, &1000, &2)
    });
    assert_eq!(
        outcome,
        SlashOutcome::Slashed,
        "slash from new dispute_contract_B must succeed after update"
    );

    // Confirm stake was reduced by the successful slash
    let stake = client.get_stake(&attestor).unwrap();
    assert_eq!(stake.amount, 4000, "stake.amount should be reduced by the successful slash");
}

/// Test scenario: Slash attestor down to exactly min_stake; is_eligible returns true (Req 10.2)
///
/// The setup helper uses min_stake = 1000. We stake 2000 and slash 1000,
/// leaving exactly min_stake = 1000. is_eligible must return true.
#[test]
fn test_eligibility_boundary_exactly_min_stake() {
    let env = Env::default();
    env.mock_all_auths();

    // setup uses min_stake = 1000; stake 2000 so we can slash down to exactly 1000
    let stake_amount = 2000_i128;
    let min_stake = 1000_i128;
    let (attestor, _treasury, dispute_contract, _token_id, client) = setup(&env, stake_amount);

    // Slash exactly (stake_amount - min_stake) to land on min_stake
    let slash_amount = stake_amount - min_stake; // 1000
    env.as_contract(&dispute_contract, || {
        let outcome = client.slash(&attestor, &slash_amount, &1);
        assert_eq!(outcome, SlashOutcome::Slashed);
    });

    // Confirm stake.amount is exactly min_stake
    let stake = client.get_stake(&attestor).unwrap();
    assert_eq!(
        stake.amount, min_stake,
        "stake.amount must equal min_stake after slash"
    );

    // Req 10.2: is_eligible must return true when stake.amount == min_stake
    assert!(
        client.is_eligible(&attestor),
        "is_eligible must return true when stake.amount == min_stake"
    );
}

/// Test scenario: Slash attestor down to min_stake - 1; is_eligible returns false (Req 10.3)
///
/// The setup helper uses min_stake = 1000. We stake 2000 and slash 1001,
/// leaving min_stake - 1 = 999. is_eligible must return false.
#[test]
fn test_eligibility_boundary_min_stake_minus_one() {
    let env = Env::default();
    env.mock_all_auths();

    let stake_amount = 2000_i128;
    let min_stake = 1000_i128;
    let (attestor, _treasury, dispute_contract, _token_id, client) = setup(&env, stake_amount);

    // Slash exactly (stake_amount - (min_stake - 1)) to land on min_stake - 1
    let slash_amount = stake_amount - (min_stake - 1); // 1001
    env.as_contract(&dispute_contract, || {
        let outcome = client.slash(&attestor, &slash_amount, &1);
        assert_eq!(outcome, SlashOutcome::Slashed);
    });

    // Confirm stake.amount is exactly min_stake - 1
    let stake = client.get_stake(&attestor).unwrap();
    assert_eq!(
        stake.amount,
        min_stake - 1,
        "stake.amount must equal min_stake - 1 after slash"
    );

    // Req 10.3: is_eligible must return false when stake.amount == min_stake - 1
    assert!(
        !client.is_eligible(&attestor),
        "is_eligible must return false when stake.amount == min_stake - 1"
    );
}

/// Test scenario: Slash attestorA with dispute_id=N, then slash attestorB with the same
/// dispute_id=N; the second call must panic with "dispute already processed" (Req 5.3)
///
/// This verifies that dispute_id uniqueness is global (not per-attestor).
#[test]
#[should_panic(expected = "dispute already processed")]
fn test_double_slash_different_attestor_same_dispute_id() {
    let env = Env::default();
    env.mock_all_auths();

    // Set up attestorA using the shared helper
    let stake_amount = 5000_i128;
    let (attestor_a, _treasury, dispute_contract, token_id, client) = setup(&env, stake_amount);

    // Register and fund attestorB separately (same contract, same token)
    let attestor_b = Address::generate(&env);
    let token_admin_addr = Address::generate(&env);
    let (_, _token_admin_client, _) = create_token_contract(&env, &token_admin_addr);
    // Mint tokens for attestorB using the existing token contract
    let token_admin_client_existing = token::StellarAssetClient::new(&env, &token_id);
    token_admin_client_existing.mint(&attestor_b, &stake_amount);
    client.stake(&attestor_b, &stake_amount);

    let dispute_id: u64 = 42;

    // First slash: attestorA with dispute_id=42 — must succeed
    env.as_contract(&dispute_contract, || {
        let outcome = client.slash(&attestor_a, &1000, &dispute_id);
        assert_eq!(outcome, SlashOutcome::Slashed);
    });

    // Second slash: attestorB with the same dispute_id=42 — must panic
    env.as_contract(&dispute_contract, || {
        client.slash(&attestor_b, &1000, &dispute_id);
    });
}

// Feature: stake-slashing-precision-rounding-tests, Property 1: Exact Slash Conservation
//
// Validates: Requirements 1.1, 1.2, 1.3, 1.4, 1.5, 11.1, 11.4
proptest! {
    #[test]
    fn prop_exact_slash_conservation(
        // Generate stake_amount in [1, i128::MAX/2] and slash_amount in [1, stake_amount]
        stake_amount in 1_i128..=(i128::MAX / 2),
        slash_amount in 1_i128..=(i128::MAX / 2),
    ) {
        // Constrain: slash_amount <= stake_amount
        let slash_amount = slash_amount.min(stake_amount);

        let env = Env::default();
        env.mock_all_auths();

        // --- Inline setup (proptest closures can't use the `setup` fn due to lifetime issues) ---
        let admin = Address::generate(&env);
        let attestor = Address::generate(&env);
        let treasury = Address::generate(&env);
        let dispute_contract = env.register(DummyDisputeContract, ());

        let (token_id, token_admin, token_client) = create_token_contract(&env, &admin);
        token_admin.mint(&attestor, &stake_amount);

        let contract_id = env.register(AttestorStakingContract, ());
        let client = AttestorStakingContractClient::new(&env, &contract_id);

        // min_stake = 1 so any stake_amount is valid; unbonding_period = 0
        client.initialize(&admin, &token_id, &treasury, &1, &dispute_contract, &0u64);
        client.stake(&attestor, &stake_amount);
        // --- End inline setup ---

        let treasury_balance_before = token_client.balance(&treasury);

        // Perform the slash
        let outcome = env.as_contract(&dispute_contract, || {
            client.slash(&attestor, &slash_amount, &1u64)
        });

        // Req 1.1 / 1.3 / 1.4: post-slash stake.amount == stake_amount - slash_amount
        let stake_after = client.get_stake(&attestor).unwrap();
        prop_assert_eq!(
            stake_after.amount,
            stake_amount - slash_amount,
            "stake.amount must equal stake_amount - slash_amount"
        );

        // Req 1.2 / 1.5 / 11.1 / 11.4: treasury increased by exactly slash_amount
        let treasury_balance_after = token_client.balance(&treasury);
        prop_assert_eq!(
            treasury_balance_after - treasury_balance_before,
            slash_amount,
            "treasury must increase by exactly slash_amount"
        );

        // Outcome must be Slashed (slash_amount >= 1 and stake_amount >= slash_amount)
        prop_assert_eq!(outcome, SlashOutcome::Slashed);
    }
}

// Feature: stake-slashing-precision-rounding-tests, Property 2: Over-Slash Clamping
//
// Validates: Requirements 2.1, 2.2, 2.3
proptest! {
    #[test]
    fn prop_over_slash_clamping(
        stake_amount in 1_i128..=(i128::MAX / 2),
        extra in 1_i128..=(i128::MAX / 2),
    ) {
        // slash_amount > stake_amount
        let slash_amount = stake_amount.saturating_add(extra);

        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let attestor = Address::generate(&env);
        let treasury = Address::generate(&env);
        let dispute_contract = env.register(DummyDisputeContract, ());

        let (token_id, token_admin, token_client) = create_token_contract(&env, &admin);
        token_admin.mint(&attestor, &stake_amount);

        let contract_id = env.register(AttestorStakingContract, ());
        let client = AttestorStakingContractClient::new(&env, &contract_id);

        client.initialize(&admin, &token_id, &treasury, &1, &dispute_contract, &0u64);
        client.stake(&attestor, &stake_amount);

        let treasury_before = token_client.balance(&treasury);

        let outcome = env.as_contract(&dispute_contract, || {
            client.slash(&attestor, &slash_amount, &1u64)
        });

        // Req 2.1: stake.amount reduced to exactly 0
        let stake_after = client.get_stake(&attestor).unwrap();
        prop_assert_eq!(stake_after.amount, 0, "stake.amount must be 0 after over-slash");

        // Req 2.2: treasury increased by exactly the pre-slash stake_amount
        let treasury_after = token_client.balance(&treasury);
        prop_assert_eq!(
            treasury_after - treasury_before,
            stake_amount,
            "treasury must increase by exactly the original stake_amount"
        );

        // Req 2.3: outcome is Slashed
        prop_assert_eq!(outcome, SlashOutcome::Slashed);
    }
}

// Feature: stake-slashing-precision-rounding-tests, Property 3: Locked Invariant Preservation
//
// Validates: Requirements 3.1, 3.2, 3.3, 3.4
proptest! {
    #[test]
    fn prop_locked_invariant_preserved(
        stake_amount in 1_i128..=(i128::MAX / 2),
        locked_amount in 0_i128..=(i128::MAX / 2),
        slash_amount in 1_i128..=(i128::MAX / 2),
    ) {
        // Constrain: locked_amount <= stake_amount
        let locked_amount = locked_amount.min(stake_amount);

        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let attestor = Address::generate(&env);
        let treasury = Address::generate(&env);
        let dispute_contract = env.register(DummyDisputeContract, ());

        let (token_id, token_admin, _token_client) = create_token_contract(&env, &admin);
        token_admin.mint(&attestor, &stake_amount);

        let contract_id = env.register(AttestorStakingContract, ());
        let client = AttestorStakingContractClient::new(&env, &contract_id);

        client.initialize(&admin, &token_id, &treasury, &1, &dispute_contract, &0u64);
        client.stake(&attestor, &stake_amount);

        // Set up locked by calling request_unstake (if locked_amount > 0)
        if locked_amount > 0 {
            client.request_unstake(&attestor, &locked_amount);
        }

        env.as_contract(&dispute_contract, || {
            client.slash(&attestor, &slash_amount, &1u64)
        });

        let post_slash_stake = client.get_stake(&attestor).unwrap();

        // Req 3.1: locked <= amount invariant must hold after slash
        prop_assert!(
            post_slash_stake.locked <= post_slash_stake.amount,
            "locked ({}) must be <= amount ({}) after slash",
            post_slash_stake.locked,
            post_slash_stake.amount
        );
    }
}

// Feature: stake-slashing-precision-rounding-tests, Property 4: Pending Unstake Adjustment
//
// Validates: Requirements 4.1, 4.2, 4.4, 4.5
proptest! {
    #[test]
    fn prop_pending_adjustment(
        stake_amount in 1_i128..=(i128::MAX / 2),
        locked_amount in 1_i128..=(i128::MAX / 2),
        slash_amount in 1_i128..=(i128::MAX / 2),
    ) {
        // Constrain: 0 < locked_amount <= stake_amount
        let locked_amount = locked_amount.min(stake_amount);

        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let attestor = Address::generate(&env);
        let treasury = Address::generate(&env);
        let dispute_contract = env.register(DummyDisputeContract, ());

        let (token_id, token_admin, _token_client) = create_token_contract(&env, &admin);
        token_admin.mint(&attestor, &stake_amount);

        let contract_id = env.register(AttestorStakingContract, ());
        let client = AttestorStakingContractClient::new(&env, &contract_id);

        client.initialize(&admin, &token_id, &treasury, &1, &dispute_contract, &0u64);
        client.stake(&attestor, &stake_amount);

        // Set up pending unstake
        client.request_unstake(&attestor, &locked_amount);

        env.as_contract(&dispute_contract, || {
            client.slash(&attestor, &slash_amount, &1u64)
        });

        let post_slash_stake = client.get_stake(&attestor).unwrap();
        let pending = client.get_pending_unstake(&attestor).unwrap();

        // Req 4.1, 4.2, 4.4, 4.5: pending.amount == min(locked_amount, post_slash_stake.locked)
        let expected_pending = locked_amount.min(post_slash_stake.locked);
        prop_assert_eq!(
            pending.amount,
            expected_pending,
            "pending.amount ({}) must equal min(locked_amount={}, post_slash_locked={})",
            pending.amount,
            locked_amount,
            post_slash_stake.locked
        );
    }
}

// Feature: stake-slashing-precision-rounding-tests, Property 5: No Phantom Pending Unstake
//
// Validates: Requirements 4.3
proptest! {
    #[test]
    fn prop_no_phantom_pending(
        stake_amount in 1_i128..=(i128::MAX / 2),
        slash_amount in 1_i128..=(i128::MAX / 2),
    ) {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let attestor = Address::generate(&env);
        let treasury = Address::generate(&env);
        let dispute_contract = env.register(DummyDisputeContract, ());

        let (token_id, token_admin, _token_client) = create_token_contract(&env, &admin);
        token_admin.mint(&attestor, &stake_amount);

        let contract_id = env.register(AttestorStakingContract, ());
        let client = AttestorStakingContractClient::new(&env, &contract_id);

        client.initialize(&admin, &token_id, &treasury, &1, &dispute_contract, &0u64);
        client.stake(&attestor, &stake_amount);

        // No request_unstake call — no pending unstake exists

        env.as_contract(&dispute_contract, || {
            client.slash(&attestor, &slash_amount, &1u64)
        });

        // Req 4.3: no PendingUnstake record must exist after slash
        let pending = client.get_pending_unstake(&attestor);
        prop_assert!(
            pending.is_none(),
            "get_pending_unstake must return None when no request_unstake was called"
        );
    }
}

// Feature: stake-slashing-precision-rounding-tests, Property 6: Double-Slash Prevention
//
// Validates: Requirements 5.1, 5.2, 5.3
proptest! {
    #[test]
    fn prop_double_slash_prevention(
        stake_amount in 1_i128..=(i128::MAX / 2),
        slash_amount in 1_i128..=(i128::MAX / 2),
        dispute_id in 0_u64..=u64::MAX,
    ) {
        let slash_amount = slash_amount.min(stake_amount);

        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let attestor = Address::generate(&env);
        let treasury = Address::generate(&env);
        let dispute_contract = env.register(DummyDisputeContract, ());

        let (token_id, token_admin, _token_client) = create_token_contract(&env, &admin);
        token_admin.mint(&attestor, &stake_amount);

        let contract_id = env.register(AttestorStakingContract, ());
        let client = AttestorStakingContractClient::new(&env, &contract_id);

        client.initialize(&admin, &token_id, &treasury, &1, &dispute_contract, &0u64);
        client.stake(&attestor, &stake_amount);

        // First slash must succeed
        let outcome = env.as_contract(&dispute_contract, || {
            client.slash(&attestor, &slash_amount, &dispute_id)
        });
        prop_assert_eq!(outcome, SlashOutcome::Slashed);

        // Second slash with same dispute_id must fail (Req 5.1, 5.2, 5.3)
        let result = env.as_contract(&dispute_contract, || {
            client.try_slash(&attestor, &slash_amount, &dispute_id)
        });
        prop_assert!(
            result.is_err(),
            "second slash with same dispute_id must return an error"
        );
    }
}

// Feature: stake-slashing-precision-rounding-tests, Property 8: Sequential Multi-Slash Accumulation
//
// Validates: Requirements 8.1, 8.2, 8.3, 8.4
proptest! {
    #[test]
    fn prop_sequential_accumulation(
        initial_stake in 3_i128..=(i128::MAX / 4),
        s1 in 1_i128..=(i128::MAX / 4),
        s2 in 1_i128..=(i128::MAX / 4),
        s3 in 1_i128..=(i128::MAX / 4),
    ) {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let attestor = Address::generate(&env);
        let treasury = Address::generate(&env);
        let dispute_contract = env.register(DummyDisputeContract, ());

        let (token_id, token_admin, token_client) = create_token_contract(&env, &admin);
        token_admin.mint(&attestor, &initial_stake);

        let contract_id = env.register(AttestorStakingContract, ());
        let client = AttestorStakingContractClient::new(&env, &contract_id);

        client.initialize(&admin, &token_id, &treasury, &1, &dispute_contract, &0u64);
        client.stake(&attestor, &initial_stake);

        let treasury_before = token_client.balance(&treasury);

        // Perform 3 sequential slashes with distinct dispute IDs
        env.as_contract(&dispute_contract, || {
            client.slash(&attestor, &s1, &1u64)
        });
        let stake_after_1 = client.get_stake(&attestor).unwrap();
        prop_assert!(stake_after_1.locked <= stake_after_1.amount, "locked <= amount after slash 1");

        env.as_contract(&dispute_contract, || {
            client.slash(&attestor, &s2, &2u64)
        });
        let stake_after_2 = client.get_stake(&attestor).unwrap();
        prop_assert!(stake_after_2.locked <= stake_after_2.amount, "locked <= amount after slash 2");

        env.as_contract(&dispute_contract, || {
            client.slash(&attestor, &s3, &3u64)
        });
        let stake_after_3 = client.get_stake(&attestor).unwrap();
        prop_assert!(stake_after_3.locked <= stake_after_3.amount, "locked <= amount after slash 3");

        // Req 8.1, 8.2: final stake.amount == max(0, initial_stake - sum)
        let sum = s1.saturating_add(s2).saturating_add(s3);
        let expected_stake = (initial_stake - sum).max(0);
        prop_assert_eq!(
            stake_after_3.amount,
            expected_stake,
            "final stake.amount must equal max(0, initial_stake - sum)"
        );

        // Req 8.2: total treasury increase == min(initial_stake, sum)
        let expected_treasury_increase = initial_stake.min(sum);
        let treasury_after = token_client.balance(&treasury);
        prop_assert_eq!(
            treasury_after - treasury_before,
            expected_treasury_increase,
            "total treasury increase must equal min(initial_stake, sum)"
        );
    }
}

// Feature: stake-slashing-precision-rounding-tests, Property 10: Eligibility Reflects Post-Slash Stake
//
// Validates: Requirements 10.1, 10.2, 10.3, 10.4
proptest! {
    #[test]
    fn prop_eligibility_reflects_post_slash_stake(
        stake_amount in 1_i128..=(i128::MAX / 2),
        min_stake in 1_i128..=(i128::MAX / 2),
        slash_amount in 1_i128..=(i128::MAX / 2),
    ) {
        // Constrain: stake_amount >= min_stake so the attestor starts eligible
        let stake_amount = stake_amount.max(min_stake);
        // Clamp slash_amount to stake_amount (over-slash is fine, just clamps to 0)
        let slash_amount = slash_amount.min(stake_amount);

        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let attestor = Address::generate(&env);
        let treasury = Address::generate(&env);
        let dispute_contract = env.register(DummyDisputeContract, ());

        let (token_id, token_admin, _token_client) = create_token_contract(&env, &admin);
        token_admin.mint(&attestor, &stake_amount);

        let contract_id = env.register(AttestorStakingContract, ());
        let client = AttestorStakingContractClient::new(&env, &contract_id);

        // Initialize with the generated min_stake
        client.initialize(&admin, &token_id, &treasury, &min_stake, &dispute_contract, &0u64);
        client.stake(&attestor, &stake_amount);

        env.as_contract(&dispute_contract, || {
            client.slash(&attestor, &slash_amount, &1u64)
        });

        let post_slash_stake = client.get_stake(&attestor).unwrap();
        let is_eligible = client.is_eligible(&attestor);

        // Req 10.1, 10.2, 10.3, 10.4: is_eligible == (post_slash_stake.amount >= min_stake)
        let expected_eligible = post_slash_stake.amount >= min_stake;
        prop_assert_eq!(
            is_eligible,
            expected_eligible,
            "is_eligible ({}) must match post_slash_stake.amount ({}) >= min_stake ({})",
            is_eligible,
            post_slash_stake.amount,
            min_stake
        );
    }
}
