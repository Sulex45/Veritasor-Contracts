extern crate std;

use super::*;
use soroban_sdk::testutils::{Address as _, Ledger as _};
use soroban_sdk::token::StellarAssetClient;
use soroban_sdk::{Address, Env};

fn setup_with_token(
    min_votes: u32,
    proposal_duration: u32,
) -> (Env, ProtocolDaoClient<'static>, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_addr = token_contract.address().clone();

    let admin = Address::generate(&env);
    let contract_id = env.register(ProtocolDao, ());
    let client = ProtocolDaoClient::new(&env, &contract_id);
    client.initialize(
        &admin,
        &Some(token_addr.clone()),
        &min_votes,
        &proposal_duration,
    );

    (env, client, admin, token_addr)
}

fn setup_without_token(
    min_votes: u32,
    proposal_duration: u32,
) -> (Env, ProtocolDaoClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register(ProtocolDao, ());
    let client = ProtocolDaoClient::new(&env, &contract_id);
    client.initialize(&admin, &None, &min_votes, &proposal_duration);

    (env, client, admin)
}

fn mint(env: &Env, token_addr: &Address, to: &Address, amount: i128) {
    let stellar = StellarAssetClient::new(env, token_addr);
    stellar.mint(to, &amount);
}

#[test]
fn initialize_sets_defaults() {
    let (_env, client, admin, token_addr) = setup_with_token(0, 0);
    let (stored_admin, stored_token, min_votes, duration) = client.get_config();
    assert_eq!(stored_admin, admin);
    assert_eq!(stored_token, Some(token_addr));
    assert_eq!(min_votes, DEFAULT_MIN_VOTES);
    assert_eq!(duration, DEFAULT_PROPOSAL_DURATION);
}

#[test]
#[should_panic(expected = "already initialized")]
fn initialize_twice_panics() {
    let (_env, client, admin, token_addr) = setup_with_token(1, 10);
    client.initialize(&admin, &Some(token_addr), &1, &10);
}

#[test]
fn set_governance_token_by_admin() {
    let (env, client, admin, _token_addr) = setup_with_token(1, 10);
    let new_token = Address::generate(&env);
    client.set_governance_token(&admin, &new_token);
    let (_, stored_token, _, _) = client.get_config();
    assert_eq!(stored_token, Some(new_token));
}

#[test]
#[should_panic(expected = "caller is not admin")]
fn set_governance_token_by_non_admin_panics() {
    let (env, client, _admin, _token_addr) = setup_with_token(1, 10);
    let caller = Address::generate(&env);
    let new_token = Address::generate(&env);
    client.set_governance_token(&caller, &new_token);
}

#[test]
fn set_voting_config_by_admin() {
    let (_env, client, admin, _token_addr) = setup_with_token(1, 10);
    client.set_voting_config(&admin, &3, &20);
    let (_, _, min_votes, duration) = client.get_config();
    assert_eq!(min_votes, 3);
    assert_eq!(duration, 20);
}

#[test]
#[should_panic(expected = "caller is not admin")]
fn set_voting_config_by_non_admin_panics() {
    let (env, client, _admin, _token_addr) = setup_with_token(1, 10);
    let caller = Address::generate(&env);
    client.set_voting_config(&caller, &3, &20);
}

#[test]
fn create_and_execute_fee_config_proposal() {
    let (env, client, admin, gov_token) = setup_with_token(1, 100);

    let voter = Address::generate(&env);
    mint(&env, &gov_token, &voter, 100);

    let fee_token = Address::generate(&env);
    let collector = Address::generate(&env);

    let proposal_id =
        client.create_fee_config_proposal(&voter, &fee_token, &collector, &1_000, &true);

    client.vote_for(&voter, &proposal_id);

    client.execute_proposal(&admin, &proposal_id);

    let proposal = client.get_proposal(&proposal_id).unwrap();
    assert_eq!(proposal.status, ProposalStatus::Executed);

    let cfg = client.get_attestation_fee_config().unwrap();
    assert_eq!(cfg.0, fee_token);
    assert_eq!(cfg.1, collector);
    assert_eq!(cfg.2, 1_000);
    assert!(cfg.3);
}

#[test]
#[should_panic(expected = "insufficient governance token balance")]
fn create_proposal_without_token_panics() {
    let (env, client, _admin, _gov_token) = setup_with_token(1, 100);
    let voter = Address::generate(&env);
    let fee_token = Address::generate(&env);
    let collector = Address::generate(&env);

    client.create_fee_config_proposal(&voter, &fee_token, &collector, &1_000, &true);
}

#[test]
fn create_proposal_without_governance_token_configured_allows_anyone() {
    let (env, client, _admin) = setup_without_token(1, 100);
    let voter = Address::generate(&env);
    let fee_token = Address::generate(&env);
    let collector = Address::generate(&env);

    let proposal_id =
        client.create_fee_config_proposal(&voter, &fee_token, &collector, &1_000, &true);
    client.vote_for(&voter, &proposal_id);
}

#[test]
fn quorum_and_majority_required() {
    let (env, client, admin, gov_token) = setup_with_token(2, 100);

    let voter1 = Address::generate(&env);
    let voter2 = Address::generate(&env);
    mint(&env, &gov_token, &voter1, 100);
    mint(&env, &gov_token, &voter2, 100);

    let fee_token = Address::generate(&env);
    let collector = Address::generate(&env);

    let proposal_id =
        client.create_fee_config_proposal(&voter1, &fee_token, &collector, &1_000, &true);

    client.vote_for(&voter1, &proposal_id);
    client.vote_for(&voter2, &proposal_id);

    let for_votes = client.get_votes_for(&proposal_id);
    let against_votes = client.get_votes_against(&proposal_id);
    assert_eq!(for_votes, 2);
    assert_eq!(against_votes, 0);

    client.execute_proposal(&admin, &proposal_id);
}

#[test]
#[should_panic(expected = "quorum not met")]
fn execute_without_quorum_panics() {
    let (env, client, admin, gov_token) = setup_with_token(2, 100);

    let voter1 = Address::generate(&env);
    mint(&env, &gov_token, &voter1, 100);

    let fee_token = Address::generate(&env);
    let collector = Address::generate(&env);

    let proposal_id =
        client.create_fee_config_proposal(&voter1, &fee_token, &collector, &1_000, &true);

    client.vote_for(&voter1, &proposal_id);

    client.execute_proposal(&admin, &proposal_id);
}

#[test]
#[should_panic(expected = "proposal not approved")]
fn execute_with_tied_votes_panics() {
    let (env, client, admin, gov_token) = setup_with_token(2, 100);

    let voter1 = Address::generate(&env);
    let voter2 = Address::generate(&env);
    mint(&env, &gov_token, &voter1, 100);
    mint(&env, &gov_token, &voter2, 100);

    let fee_token = Address::generate(&env);
    let collector = Address::generate(&env);

    let proposal_id =
        client.create_fee_config_proposal(&voter1, &fee_token, &collector, &1_000, &true);

    client.vote_for(&voter1, &proposal_id);
    client.vote_against(&voter2, &proposal_id);

    client.execute_proposal(&admin, &proposal_id);
}

#[test]
fn cancel_proposal_by_creator() {
    let (env, client, _admin, gov_token) = setup_with_token(1, 100);

    let creator = Address::generate(&env);
    mint(&env, &gov_token, &creator, 100);

    let fee_token = Address::generate(&env);
    let collector = Address::generate(&env);

    let proposal_id =
        client.create_fee_config_proposal(&creator, &fee_token, &collector, &1_000, &true);

    client.cancel_proposal(&creator, &proposal_id);

    let proposal = client.get_proposal(&proposal_id).unwrap();
    assert_eq!(proposal.status, ProposalStatus::Rejected);
}

#[test]
fn cancel_proposal_by_admin() {
    let (env, client, admin, gov_token) = setup_with_token(1, 100);

    let creator = Address::generate(&env);
    mint(&env, &gov_token, &creator, 100);

    let fee_token = Address::generate(&env);
    let collector = Address::generate(&env);

    let proposal_id =
        client.create_fee_config_proposal(&creator, &fee_token, &collector, &1_000, &true);

    client.cancel_proposal(&admin, &proposal_id);

    let proposal = client.get_proposal(&proposal_id).unwrap();
    assert_eq!(proposal.status, ProposalStatus::Rejected);
}

#[test]
#[should_panic(expected = "only creator or admin can cancel")]
fn cancel_proposal_by_other_panics() {
    let (env, client, _admin, gov_token) = setup_with_token(1, 100);

    let creator = Address::generate(&env);
    mint(&env, &gov_token, &creator, 100);

    let other = Address::generate(&env);

    let fee_token = Address::generate(&env);
    let collector = Address::generate(&env);

    let proposal_id =
        client.create_fee_config_proposal(&creator, &fee_token, &collector, &1_000, &true);

    client.cancel_proposal(&other, &proposal_id);
}

#[test]
#[should_panic(expected = "proposal expired")]
fn vote_after_expiry_panics() {
    let (env, client, _admin, gov_token) = setup_with_token(1, 5);

    let voter = Address::generate(&env);
    mint(&env, &gov_token, &voter, 100);

    let fee_token = Address::generate(&env);
    let collector = Address::generate(&env);

    let proposal_id =
        client.create_fee_config_proposal(&voter, &fee_token, &collector, &1_000, &true);

    env.ledger().with_mut(|li| {
        li.sequence_number += 10;
    });

    client.vote_for(&voter, &proposal_id);
}

#[test]
#[should_panic(expected = "proposal expired")]
fn execute_after_expiry_panics() {
    let (env, client, admin, gov_token) = setup_with_token(1, 5);

    let voter = Address::generate(&env);
    mint(&env, &gov_token, &voter, 100);

    let fee_token = Address::generate(&env);
    let collector = Address::generate(&env);

    let proposal_id =
        client.create_fee_config_proposal(&voter, &fee_token, &collector, &1_000, &true);

    client.vote_for(&voter, &proposal_id);

    env.ledger().with_mut(|li| {
        li.sequence_number += 10;
    });

    client.execute_proposal(&admin, &proposal_id);
}

// ════════════════════════════════════════════════════════════════════
// QUORUM EDGE CASES - Boundary Conditions
// ════════════════════════════════════════════════════════════════════

#[test]
fn quorum_with_min_votes_zero() {
    // Edge case: min_votes=0 means any vote passes quorum
    let (env, client, admin, gov_token) = setup_with_token(0, 100);

    let voter = Address::generate(&env);
    mint(&env, &gov_token, &voter, 100);

    let fee_token = Address::generate(&env);
    let collector = Address::generate(&env);

    let proposal_id =
        client.create_fee_config_proposal(&voter, &fee_token, &collector, &1_000, &true);

    // Single vote should satisfy quorum (1 for, 0 against, total 1 >= min_votes 0)
    client.vote_for(&voter, &proposal_id);

    let (for_votes, against_votes, min_req, quorum_ok, majority_ok) =
        client.get_quorum_info(&proposal_id);
    assert_eq!(for_votes, 1);
    assert_eq!(against_votes, 0);
    assert_eq!(min_req, DEFAULT_MIN_VOTES); // Defaults to 1, not 0
    assert!(quorum_ok);
    assert!(majority_ok);

    client.execute_proposal(&admin, &proposal_id);

    let proposal = client.get_proposal(&proposal_id).unwrap();
    assert_eq!(proposal.status, ProposalStatus::Executed);
}

#[test]
fn quorum_with_min_votes_one() {
    // min_votes=1: need at least 1 total vote
    let (env, client, admin, gov_token) = setup_with_token(1, 100);

    let voter = Address::generate(&env);
    mint(&env, &gov_token, &voter, 100);

    let fee_token = Address::generate(&env);
    let collector = Address::generate(&env);

    let proposal_id =
        client.create_fee_config_proposal(&voter, &fee_token, &collector, &1_000, &true);

    // Query before voting - no quorum
    let (for_votes, _against_votes, min_req, quorum_ok, _majority_ok) =
        client.get_quorum_info(&proposal_id);
    assert_eq!(for_votes, 0);
    assert_eq!(min_req, 1);
    assert!(!quorum_ok);

    // One vote satisfies both quorum (1 >= 1) and majority (1 > 0)
    client.vote_for(&voter, &proposal_id);

    let (for_votes, _against_votes, min_req, quorum_ok, majority_ok) =
        client.get_quorum_info(&proposal_id);
    assert_eq!(for_votes, 1);
    assert_eq!(min_req, 1);
    assert!(quorum_ok);
    assert!(majority_ok);

    client.execute_proposal(&admin, &proposal_id);
}

#[test]
#[should_panic(expected = "proposal not approved")]
fn quorum_with_only_against_votes() {
    // Edge case: only against votes, no for votes - quorum can be met but majority fails
    let (env, client, admin, gov_token) = setup_with_token(1, 100);

    let voter = Address::generate(&env);
    mint(&env, &gov_token, &voter, 100);

    let fee_token = Address::generate(&env);
    let collector = Address::generate(&env);

    let proposal_id =
        client.create_fee_config_proposal(&voter, &fee_token, &collector, &1_000, &true);

    // Single against vote meets quorum (1 total) but fails majority (0 > 1 is false)
    client.vote_against(&voter, &proposal_id);

    client.execute_proposal(&admin, &proposal_id); // Should panic: "proposal not approved"
}

#[test]
#[should_panic(expected = "proposal not approved")]
fn majority_requires_strictly_greater() {
    // Edge case: tie vote (equal for/against) should fail majority check
    let (env, client, admin, gov_token) = setup_with_token(2, 100);

    let voter1 = Address::generate(&env);
    let voter2 = Address::generate(&env);
    mint(&env, &gov_token, &voter1, 100);
    mint(&env, &gov_token, &voter2, 100);

    let fee_token = Address::generate(&env);
    let collector = Address::generate(&env);

    let proposal_id =
        client.create_fee_config_proposal(&voter1, &fee_token, &collector, &1_000, &true);

    client.vote_for(&voter1, &proposal_id);
    client.vote_against(&voter2, &proposal_id);

    // Quorum: 1 + 1 = 2 >= 2 ✓
    // Majority: 1 > 1 = false ✗
    let (for_votes, against_votes, min_req, quorum_ok, majority_ok) =
        client.get_quorum_info(&proposal_id);
    assert_eq!(for_votes, 1);
    assert_eq!(against_votes, 1);
    assert_eq!(min_req, 2);
    assert!(quorum_ok);
    assert!(!majority_ok); // Tie fails majority

    client.execute_proposal(&admin, &proposal_id);
}

#[test]
fn high_quorum_with_large_min_votes() {
    // Edge case: high min_votes requirement
    let (env, client, admin, gov_token) = setup_with_token(5, 100);

    let voter1 = Address::generate(&env);
    let voter2 = Address::generate(&env);
    let voter3 = Address::generate(&env);
    let voter4 = Address::generate(&env);
    let voter5 = Address::generate(&env);

    mint(&env, &gov_token, &voter1, 100);
    mint(&env, &gov_token, &voter2, 100);
    mint(&env, &gov_token, &voter3, 100);
    mint(&env, &gov_token, &voter4, 100);
    mint(&env, &gov_token, &voter5, 100);

    let fee_token = Address::generate(&env);
    let collector = Address::generate(&env);

    let proposal_id =
        client.create_fee_config_proposal(&voter1, &fee_token, &collector, &1_000, &true);

    // Vote 4 times - not enough
    client.vote_for(&voter1, &proposal_id);
    client.vote_for(&voter2, &proposal_id);
    client.vote_for(&voter3, &proposal_id);
    client.vote_for(&voter4, &proposal_id);

    let (for_votes, _against_votes, min_req, quorum_ok, _majority_ok) =
        client.get_quorum_info(&proposal_id);
    assert_eq!(for_votes, 4);
    assert_eq!(min_req, 5);
    assert!(!quorum_ok); // 4 < 5

    // 5th vote meets quorum
    client.vote_for(&voter5, &proposal_id);

    let (for_votes, _against_votes, _min_req, quorum_ok, majority_ok) =
        client.get_quorum_info(&proposal_id);
    assert_eq!(for_votes, 5);
    assert!(quorum_ok); // 5 >= 5 ✓
    assert!(majority_ok); // 5 > 0 ✓

    client.execute_proposal(&admin, &proposal_id);
}

// ════════════════════════════════════════════════════════════════════
// EXPIRY EDGE CASES
// ════════════════════════════════════════════════════════════════════

#[test]
#[should_panic(expected = "proposal expired")]
fn expiry_at_exact_boundary() {
    // Edge case: expiry at exact duration boundary
    let (env, client, _admin, gov_token) = setup_with_token(1, 10);

    let voter = Address::generate(&env);
    mint(&env, &gov_token, &voter, 100);

    let fee_token = Address::generate(&env);
    let collector = Address::generate(&env);

    let proposal_id =
        client.create_fee_config_proposal(&voter, &fee_token, &collector, &1_000, &true);

    // Advance to created_at + duration (should still be valid)
    env.ledger().with_mut(|li| {
        li.sequence_number += 10; // created_at was 0, duration is 10, now at sequence 10
    });

    // At boundary: 10 > (0 + 10) = false, so not expired yet
    // But next sequence will be expired
    client.vote_for(&voter, &proposal_id); // Should succeed at boundary

    // Advance to created_at + duration + 1 (now expired)
    env.ledger().with_mut(|li| {
        li.sequence_number += 1; // Now at sequence 11
    });

    // 11 > (0 + 10) = true, so expired
    client.vote_for(&voter, &proposal_id); // Should panic
}

#[test]
fn proposal_valid_just_before_expiry() {
    // Edge case: voting/executing works just before expiry
    let (env, client, admin, gov_token) = setup_with_token(1, 10);

    let voter = Address::generate(&env);
    mint(&env, &gov_token, &voter, 100);

    let fee_token = Address::generate(&env);
    let collector = Address::generate(&env);

    let proposal_id =
        client.create_fee_config_proposal(&voter, &fee_token, &collector, &1_000, &true);

    // Advance to created_at + duration - 1
    env.ledger().with_mut(|li| {
        li.sequence_number += 9; // created_at 0, duration 10, now at sequence 9
    });

    // At 9: not expired (9 > 10 = false)
    client.vote_for(&voter, &proposal_id);

    // Execute just before expiry
    client.execute_proposal(&admin, &proposal_id);

    let proposal = client.get_proposal(&proposal_id).unwrap();
    assert_eq!(proposal.status, ProposalStatus::Executed);
}

// ════════════════════════════════════════════════════════════════════
// REPLAY PROTECTION - Double Voting Prevention
// ════════════════════════════════════════════════════════════════════

#[test]
#[should_panic(expected = "already voted")]
fn cannot_vote_twice_for() {
    let (env, client, _admin, gov_token) = setup_with_token(1, 100);

    let voter = Address::generate(&env);
    mint(&env, &gov_token, &voter, 100);

    let fee_token = Address::generate(&env);
    let collector = Address::generate(&env);

    let proposal_id =
        client.create_fee_config_proposal(&voter, &fee_token, &collector, &1_000, &true);

    client.vote_for(&voter, &proposal_id);
    client.vote_for(&voter, &proposal_id); // Should panic: already voted
}

#[test]
#[should_panic(expected = "already voted")]
fn cannot_vote_twice_against() {
    let (env, client, _admin, gov_token) = setup_with_token(1, 100);

    let voter = Address::generate(&env);
    mint(&env, &gov_token, &voter, 100);

    let fee_token = Address::generate(&env);
    let collector = Address::generate(&env);

    let proposal_id =
        client.create_fee_config_proposal(&voter, &fee_token, &collector, &1_000, &true);

    client.vote_against(&voter, &proposal_id);
    client.vote_against(&voter, &proposal_id); // Should panic: already voted
}

#[test]
#[should_panic(expected = "already voted")]
fn cannot_switch_vote() {
    // Edge case: voter tries to change their vote (vote_for then vote_against)
    let (env, client, _admin, gov_token) = setup_with_token(1, 100);

    let voter = Address::generate(&env);
    mint(&env, &gov_token, &voter, 100);

    let fee_token = Address::generate(&env);
    let collector = Address::generate(&env);

    let proposal_id =
        client.create_fee_config_proposal(&voter, &fee_token, &collector, &1_000, &true);

    client.vote_for(&voter, &proposal_id);
    client.vote_against(&voter, &proposal_id); // Should panic: already voted
}

#[test]
fn vote_counts_independently() {
    // Multiple voters each vote once - counts should be correct
    let (env, client, admin, gov_token) = setup_with_token(3, 100);

    let voter1 = Address::generate(&env);
    let voter2 = Address::generate(&env);
    let voter3 = Address::generate(&env);
    mint(&env, &gov_token, &voter1, 100);
    mint(&env, &gov_token, &voter2, 100);
    mint(&env, &gov_token, &voter3, 100);

    let fee_token = Address::generate(&env);
    let collector = Address::generate(&env);

    let proposal_id =
        client.create_fee_config_proposal(&voter1, &fee_token, &collector, &1_000, &true);

    client.vote_for(&voter1, &proposal_id);

    let for_votes = client.get_votes_for(&proposal_id);
    let against_votes = client.get_votes_against(&proposal_id);
    assert_eq!(for_votes, 1);
    assert_eq!(against_votes, 0);

    client.vote_for(&voter2, &proposal_id);
    let for_votes = client.get_votes_for(&proposal_id);
    assert_eq!(for_votes, 2);

    client.vote_against(&voter3, &proposal_id);
    let against_votes = client.get_votes_against(&proposal_id);
    assert_eq!(against_votes, 1);

    // Quorum: 2 + 1 = 3 >= 3 ✓
    // Majority: 2 > 1 ✓
    client.execute_proposal(&admin, &proposal_id);
}

// ════════════════════════════════════════════════════════════════════
// AUTHORIZATION AND TOKEN GATING
// ════════════════════════════════════════════════════════════════════

#[test]
fn double_vote_different_proposals_allowed() {
    // Edge case: same voter can vote on different proposals
    let (env, client, _admin, gov_token) = setup_with_token(1, 100);

    let voter = Address::generate(&env);
    mint(&env, &gov_token, &voter, 100);

    let fee_token = Address::generate(&env);
    let collector = Address::generate(&env);

    let proposal_id_1 =
        client.create_fee_config_proposal(&voter, &fee_token, &collector, &1_000, &true);
    let proposal_id_2 =
        client.create_fee_config_proposal(&voter, &fee_token, &collector, &2_000, &false);

    // Vote on both proposals
    client.vote_for(&voter, &proposal_id_1);
    client.vote_for(&voter, &proposal_id_2); // Should succeed (different proposal)

    assert_eq!(client.get_votes_for(&proposal_id_1), 1);
    assert_eq!(client.get_votes_for(&proposal_id_2), 1);
}

#[test]
#[should_panic(expected = "insufficient governance token balance")]
fn vote_without_token_fails() {
    let (env, client, _admin, gov_token) = setup_with_token(1, 100);

    let voter_with_token = Address::generate(&env);
    let voter_without_token = Address::generate(&env);
    mint(&env, &gov_token, &voter_with_token, 100);

    let fee_token = Address::generate(&env);
    let collector = Address::generate(&env);

    let proposal_id = client.create_fee_config_proposal(
        &voter_with_token,
        &fee_token,
        &collector,
        &1_000,
        &true,
    );

    // Voter with token votes successfully
    client.vote_for(&voter_with_token, &proposal_id);

    // Voter without token cannot vote
    client.vote_for(&voter_without_token, &proposal_id);
}

// ════════════════════════════════════════════════════════════════════
// STATE TRANSITIONS AND PROPOSAL LIFECYCLE
// ════════════════════════════════════════════════════════════════════

#[test]
#[should_panic(expected = "proposal is not pending")]
fn cannot_vote_on_executed_proposal() {
    let (env, client, admin, gov_token) = setup_with_token(1, 100);

    let voter = Address::generate(&env);
    mint(&env, &gov_token, &voter, 100);

    let fee_token = Address::generate(&env);
    let collector = Address::generate(&env);

    let proposal_id =
        client.create_fee_config_proposal(&voter, &fee_token, &collector, &1_000, &true);

    client.vote_for(&voter, &proposal_id);
    client.execute_proposal(&admin, &proposal_id);

    // Try to vote on executed proposal
    let voter2 = Address::generate(&env);
    mint(&env, &gov_token, &voter2, 100);
    client.vote_for(&voter2, &proposal_id);
}

#[test]
#[should_panic(expected = "proposal is not pending")]
fn cannot_vote_on_canceled_proposal() {
    let (env, client, _admin, gov_token) = setup_with_token(1, 100);

    let creator = Address::generate(&env);
    mint(&env, &gov_token, &creator, 100);

    let fee_token = Address::generate(&env);
    let collector = Address::generate(&env);

    let proposal_id =
        client.create_fee_config_proposal(&creator, &fee_token, &collector, &1_000, &true);

    client.cancel_proposal(&creator, &proposal_id);

    // Try to vote on canceled proposal
    client.vote_for(&creator, &proposal_id);
}

#[test]
#[should_panic(expected = "proposal is not pending")]
fn cannot_cancel_executed_proposal() {
    let (env, client, admin, gov_token) = setup_with_token(1, 100);

    let voter = Address::generate(&env);
    mint(&env, &gov_token, &voter, 100);

    let fee_token = Address::generate(&env);
    let collector = Address::generate(&env);

    let proposal_id =
        client.create_fee_config_proposal(&voter, &fee_token, &collector, &1_000, &true);

    client.vote_for(&voter, &proposal_id);
    client.execute_proposal(&admin, &proposal_id);

    // Try to cancel executed proposal
    client.cancel_proposal(&voter, &proposal_id);
}

#[test]
#[should_panic(expected = "proposal is not pending")]
fn cannot_execute_canceled_proposal() {
    let (env, client, admin, gov_token) = setup_with_token(1, 100);

    let creator = Address::generate(&env);
    mint(&env, &gov_token, &creator, 100);

    let fee_token = Address::generate(&env);
    let collector = Address::generate(&env);

    let proposal_id =
        client.create_fee_config_proposal(&creator, &fee_token, &collector, &1_000, &true);

    client.cancel_proposal(&creator, &proposal_id);

    // Try to execute canceled proposal
    client.execute_proposal(&admin, &proposal_id);
}

// ════════════════════════════════════════════════════════════════════
// PARAME_TER VALIDATION
// ════════════════════════════════════════════════════════════════════

#[test]
fn set_voting_config_respects_defaults() {
    let (_env, client, admin, _gov_token) = setup_with_token(5, 50);

    // Set to 0, should default to DEFAULT_MIN_VOTES
    client.set_voting_config(&admin, &0, &0);

    let (_, _, min_votes, duration) = client.get_config();
    assert_eq!(min_votes, DEFAULT_MIN_VOTES);
    assert_eq!(duration, DEFAULT_PROPOSAL_DURATION);
}

#[test]
fn set_voting_config_with_explicit_values() {
    let (_env, client, admin, _gov_token) = setup_with_token(1, 10);

    client.set_voting_config(&admin, &7, &42);

    let (_, _, min_votes, duration) = client.get_config();
    assert_eq!(min_votes, 7);
    assert_eq!(duration, 42);
}

#[test]
#[should_panic(expected = "base_fee must be non-negative")]
fn create_proposal_with_negative_fee_panics() {
    let (env, client, _admin, gov_token) = setup_with_token(1, 100);

    let creator = Address::generate(&env);
    mint(&env, &gov_token, &creator, 100);

    let fee_token = Address::generate(&env);
    let collector = Address::generate(&env);

    // Negative base_fee should fail
    client.create_fee_config_proposal(&creator, &fee_token, &collector, &-1, &true);
}

#[test]
fn create_proposal_with_zero_fee_succeeds() {
    let (env, client, admin, gov_token) = setup_with_token(1, 100);

    let creator = Address::generate(&env);
    mint(&env, &gov_token, &creator, 100);

    let fee_token = Address::generate(&env);
    let collector = Address::generate(&env);

    // Zero base_fee should succeed
    let proposal_id =
        client.create_fee_config_proposal(&creator, &fee_token, &collector, &0, &true);

    client.vote_for(&creator, &proposal_id);
    client.execute_proposal(&admin, &proposal_id);

    let cfg = client.get_attestation_fee_config().unwrap();
    assert_eq!(cfg.2, 0);
}

// ════════════════════════════════════════════════════════════════════
// GOVERNANCE CONFIGURATION PROPOSALS
// ════════════════════════════════════════════════════════════════════

#[test]
fn execute_governance_config_proposal() {
    let (env, client, admin, gov_token) = setup_with_token(1, 100);

    let creator = Address::generate(&env);
    mint(&env, &gov_token, &creator, 100);

    // Current: min_votes=1, duration=100
    let (_, _, current_min, current_dur) = client.get_config();
    assert_eq!(current_min, 1);
    assert_eq!(current_dur, 100);

    // Create proposal to change to min_votes=5, duration=200
    let proposal_id =
        client.create_gov_config_proposal(&creator, &5, &200);

    client.vote_for(&creator, &proposal_id);
    client.execute_proposal(&admin, &proposal_id);

    // Check that config was updated
    let (_, _, new_min, new_dur) = client.get_config();
    assert_eq!(new_min, 5);
    assert_eq!(new_dur, 200);
}

#[test]
fn fee_toggle_proposal() {
    let (env, client, admin, gov_token) = setup_with_token(1, 100);

    let creator = Address::generate(&env);
    mint(&env, &gov_token, &creator, 100);

    // Set initial fee config
    let fee_token = Address::generate(&env);
    let collector = Address::generate(&env);
    let proposal_id_1 =
        client.create_fee_config_proposal(&creator, &fee_token, &collector, &1_000, &true);

    client.vote_for(&creator, &proposal_id_1);
    client.execute_proposal(&admin, &proposal_id_1);

    let cfg = client.get_attestation_fee_config().unwrap();
    assert!(cfg.3); // enabled = true

    // Toggle off
    let proposal_id_2 = client.create_fee_toggle_proposal(&creator, &false);

    client.vote_for(&creator, &proposal_id_2);
    client.execute_proposal(&admin, &proposal_id_2);

    let cfg = client.get_attestation_fee_config().unwrap();
    assert!(!cfg.3); // enabled = false
}

// ════════════════════════════════════════════════════════════════════
// SEQUENCING AND ORDERING
// ════════════════════════════════════════════════════════════════════

#[test]
fn multiple_proposals_maintain_separate_vote_counts() {
    // Edge case: ensure vote counts don't leak between proposals
    let (env, client, _admin, gov_token) = setup_with_token(1, 100);

    let voter1 = Address::generate(&env);
    let voter2 = Address::generate(&env);
    mint(&env, &gov_token, &voter1, 100);
    mint(&env, &gov_token, &voter2, 100);

    let fee_token = Address::generate(&env);
    let collector = Address::generate(&env);

    let proposal_id_1 =
        client.create_fee_config_proposal(&voter1, &fee_token, &collector, &1_000, &true);
    let proposal_id_2 =
        client.create_fee_config_proposal(&voter1, &fee_token, &collector, &2_000, &false);

    // Vote differently on each
    client.vote_for(&voter1, &proposal_id_1);
    client.vote_against(&voter2, &proposal_id_2);

    // proposal_1: 1 for, 0 against
    assert_eq!(client.get_votes_for(&proposal_id_1), 1);
    assert_eq!(client.get_votes_against(&proposal_id_1), 0);

    // proposal_2: 0 for, 1 against
    assert_eq!(client.get_votes_for(&proposal_id_2), 0);
    assert_eq!(client.get_votes_against(&proposal_id_2), 1);
}

#[test]
fn proposal_ids_increment_sequentially() {
    let (env, client, creator, gov_token) = setup_with_token(1, 100);

    mint(&env, &gov_token, &creator, 100);

    let fee_token = Address::generate(&env);
    let collector = Address::generate(&env);

    let proposal_1 =
        client.create_fee_config_proposal(&creator, &fee_token, &collector, &1_000, &true);
    let proposal_2 =
        client.create_fee_config_proposal(&creator, &fee_token, &collector, &1_000, &true);
    let proposal_3 =
        client.create_fee_config_proposal(&creator, &fee_token, &collector, &1_000, &true);

    assert_eq!(proposal_1, 0);
    assert_eq!(proposal_2, 1);
    assert_eq!(proposal_3, 2);
}

