//! # Interface Specification Consistency Check Tests
//!
//! These tests verify that the documented interface specification remains
//! consistent with the actual contract implementations.

use soroban_sdk::{Env, String};

// Import the module under test
use crate::interface_spec_check::{
    get_event_count, get_expected_events, get_expected_methods, get_expected_structs,
    get_method_count, get_struct_count, is_event_documented, is_method_documented,
    is_struct_documented, verify_interface_consistency, VerificationResult,
};

#[test]
fn test_verification_result_new() {
    let env = Env::default();
    let result = VerificationResult::new(&env);

    assert!(result.passed);
    assert_eq!(result.missing_methods.len(), 0);
    assert_eq!(result.undocumented_methods.len(), 0);
    assert_eq!(result.missing_events.len(), 0);
    assert_eq!(result.missing_structs.len(), 0);
    assert_eq!(result.errors.len(), 0);
}

#[test]
fn test_verification_result_add_missing_method() {
    let env = Env::default();
    let mut result = VerificationResult::new(&env);

    result.add_missing_method(&env, String::from_str(&env, "test_method"));

    assert!(!result.passed);
    assert_eq!(result.missing_methods.len(), 1);
}

#[test]
fn test_verification_result_add_undocumented_method() {
    let env = Env::default();
    let mut result = VerificationResult::new(&env);

    result.add_undocumented_method(&env, String::from_str(&env, "undoc_method"));

    assert!(!result.passed);
    assert_eq!(result.undocumented_methods.len(), 1);
}

#[test]
fn test_verification_result_add_missing_event() {
    let env = Env::default();
    let mut result = VerificationResult::new(&env);

    result.add_missing_event(&env, String::from_str(&env, "test_event"));

    assert!(!result.passed);
    assert_eq!(result.missing_events.len(), 1);
}

#[test]
fn test_verification_result_add_missing_struct() {
    let env = Env::default();
    let mut result = VerificationResult::new(&env);

    result.add_missing_struct(&env, String::from_str(&env, "TestStruct"));

    assert!(!result.passed);
    assert_eq!(result.missing_structs.len(), 1);
}

#[test]
fn test_verification_result_add_error() {
    let env = Env::default();
    let mut result = VerificationResult::new(&env);

    result.add_error(&env, String::from_str(&env, "test error"));

    assert!(!result.passed);
    assert_eq!(result.errors.len(), 1);
}

#[test]
fn test_get_expected_methods_non_empty() {
    let env = Env::default();
    let methods = get_expected_methods(&env);

    assert!(!methods.is_empty(), "Expected methods should not be empty");
}

#[test]
fn test_get_expected_events_non_empty() {
    let env = Env::default();
    let events = get_expected_events(&env);

    assert!(!events.is_empty(), "Expected events should not be empty");
}

#[test]
fn test_get_expected_structs_non_empty() {
    let env = Env::default();
    let structs = get_expected_structs(&env);

    assert!(!structs.is_empty(), "Expected structs should not be empty");
}

#[test]
fn test_method_count() {
    let env = Env::default();
    let count = get_method_count(&env);

    // Total: 83 methods across all contracts
    assert_eq!(count, 83, "Total method count should be 83");
}

#[test]
fn test_event_count() {
    let env = Env::default();
    let count = get_event_count(&env);

    // Total: 13 events
    assert_eq!(count, 13, "Total event count should be 13");
}

#[test]
fn test_struct_count() {
    let env = Env::default();
    let count = get_struct_count(&env);

    // Total: 17 structs
    assert_eq!(count, 17, "Total struct count should be 17");
}

#[test]
fn test_is_method_documented() {
    let env = Env::default();

    assert!(
        is_method_documented(&env, "AttestationContract", "initialize"),
        "initialize should be documented for AttestationContract"
    );
    assert!(
        is_method_documented(&env, "AttestationContract", "submit_attestation"),
        "submit_attestation should be documented"
    );
    assert!(
        is_method_documented(&env, "IntegrationRegistryContract", "register_provider"),
        "register_provider should be documented"
    );
    assert!(
        !is_method_documented(&env, "AttestationContract", "nonexistent_method"),
        "nonexistent_method should not be documented"
    );
}

#[test]
fn test_is_event_documented() {
    let env = Env::default();

    assert!(
        is_event_documented(&env, "AttestationContract", "AttestationSubmitted"),
        "AttestationSubmitted should be documented"
    );
    assert!(
        is_event_documented(&env, "AttestationContract", "RoleGranted"),
        "RoleGranted should be documented"
    );
    assert!(
        is_event_documented(&env, "IntegrationRegistryContract", "ProviderRegistered"),
        "ProviderRegistered should be documented"
    );
    assert!(
        !is_event_documented(&env, "AttestationContract", "NonexistentEvent"),
        "NonexistentEvent should not be documented"
    );
}

#[test]
fn test_is_struct_documented() {
    let env = Env::default();

    assert!(
        is_struct_documented(&env, "AttestationContract", "FeeConfig"),
        "FeeConfig should be documented"
    );
    assert!(
        is_struct_documented(&env, "AttestationContract", "Proposal"),
        "Proposal should be documented"
    );
    assert!(
        is_struct_documented(&env, "IntegrationRegistryContract", "Provider"),
        "Provider should be documented"
    );
    assert!(
        !is_struct_documented(&env, "AttestationContract", "NonexistentStruct"),
        "NonexistentStruct should not be documented"
    );
}

#[test]
fn test_verify_interface_consistency() {
    let env = Env::default();
    let result = verify_interface_consistency(&env);

    assert!(
        result.passed,
        "Interface consistency verification should pass"
    );
}

#[test]
fn test_all_contracts_have_initialize() {
    let env = Env::default();
    let methods = get_expected_methods(&env);

    let contracts = [
        "AttestationContract",
        "AggregatedAttestationsContract",
        "AttestationSnapshotContract",
        "AuditLogContract",
        "IntegrationRegistryContract",
        "RevenueStreamContract",
    ];

    for contract in contracts.iter() {
        let has_initialize = methods.iter().any(|m| {
            m.contract == String::from_str(&env, contract)
                && m.name == String::from_str(&env, "initialize")
        });
        assert!(has_initialize, "{} should have initialize", contract);
    }
}

#[test]
fn test_all_contracts_have_get_admin() {
    let env = Env::default();
    let methods = get_expected_methods(&env);

    let contracts = [
        "AttestationContract",
        "AggregatedAttestationsContract",
        "AttestationSnapshotContract",
        "AuditLogContract",
        "IntegrationRegistryContract",
        "RevenueStreamContract",
    ];

    for contract in contracts.iter() {
        let has_get_admin = methods.iter().any(|m| {
            m.contract == String::from_str(&env, contract)
                && m.name == String::from_str(&env, "get_admin")
        });
        assert!(has_get_admin, "{} should have get_admin", contract);
    }
}

#[test]
fn test_attestation_events_have_correct_topics() {
    let env = Env::default();
    let events = get_expected_events(&env);

    let expected_topics = [
        ("AttestationSubmitted", "att_sub"),
        ("AttestationRevoked", "att_rev"),
        ("AttestationMigrated", "att_mig"),
        ("RoleGranted", "role_gr"),
        ("RoleRevoked", "role_rv"),
        ("ContractPaused", "paused"),
        ("ContractUnpaused", "unpaus"),
        ("FeeConfigChanged", "fee_cfg"),
    ];

    for (name, expected_topic) in expected_topics.iter() {
        let event = events.iter().find(|e| {
            e.name == String::from_str(&env, name)
                && e.contract == String::from_str(&env, "AttestationContract")
        });
        assert!(event.is_some(), "Event {} should exist", name);
        assert_eq!(
            event.unwrap().topic,
            String::from_str(&env, expected_topic),
            "Event {} should have topic {}",
            name,
            expected_topic
        );
    }
}

#[test]
fn test_provider_events_have_correct_topics() {
    let env = Env::default();
    let events = get_expected_events(&env);

    let expected_topics = [
        ("ProviderRegistered", "prv_reg"),
        ("ProviderEnabled", "prv_ena"),
        ("ProviderDeprecated", "prv_dep"),
        ("ProviderDisabled", "prv_dis"),
        ("ProviderUpdated", "prv_upd"),
    ];

    for (name, expected_topic) in expected_topics.iter() {
        let event = events.iter().find(|e| {
            e.name == String::from_str(&env, name)
                && e.contract == String::from_str(&env, "IntegrationRegistryContract")
        });
        assert!(event.is_some(), "Event {} should exist", name);
        assert_eq!(
            event.unwrap().topic,
            String::from_str(&env, expected_topic),
            "Event {} should have topic {}",
            name,
            expected_topic
        );
    }
}

#[test]
fn test_method_documentation_completeness() {
    let env = Env::default();

    let required_methods = [
        ("AttestationContract", "initialize"),
        ("AttestationContract", "initialize_multisig"),
        ("AttestationContract", "configure_fees"),
        ("AttestationContract", "set_tier_discount"),
        ("AttestationContract", "set_business_tier"),
        ("AttestationContract", "set_volume_brackets"),
        ("AttestationContract", "set_fee_enabled"),
        ("AttestationContract", "grant_role"),
        ("AttestationContract", "revoke_role"),
        ("AttestationContract", "has_role"),
        ("AttestationContract", "get_roles"),
        ("AttestationContract", "get_role_holders"),
        ("AttestationContract", "pause"),
        ("AttestationContract", "unpause"),
        ("AttestationContract", "is_paused"),
        ("AttestationContract", "submit_attestation"),
        ("AttestationContract", "submit_attestation_with_metadata"),
        ("AttestationContract", "revoke_attestation"),
        ("AttestationContract", "migrate_attestation"),
        ("AttestationContract", "is_revoked"),
        ("AttestationContract", "get_attestation"),
        ("AttestationContract", "get_attestation_metadata"),
        ("AttestationContract", "verify_attestation"),
        ("AttestationContract", "create_proposal"),
        ("AttestationContract", "approve_proposal"),
        ("AttestationContract", "reject_proposal"),
        ("AttestationContract", "execute_proposal"),
        ("AttestationContract", "get_proposal"),
        ("AttestationContract", "get_approval_count"),
        ("AttestationContract", "is_proposal_approved"),
        ("AttestationContract", "get_multisig_owners"),
        ("AttestationContract", "get_multisig_threshold"),
        ("AttestationContract", "is_multisig_owner"),
        ("AttestationContract", "get_fee_config"),
        ("AttestationContract", "get_fee_quote"),
        ("AttestationContract", "get_business_tier"),
        ("AttestationContract", "get_business_count"),
        ("AttestationContract", "get_admin"),
    ];

    for (contract, method) in required_methods.iter() {
        assert!(
            is_method_documented(&env, contract, method),
            "Method {}::{} should be documented",
            contract,
            method
        );
    }
}

#[test]
fn test_spec_document_exists() {
    let env = Env::default();
    let method_count = get_method_count(&env);
    assert!(method_count > 0, "Spec should define methods");
}

// =============================================================================
// Governance Gating Cross-Role Authorization Tests
//
// These tests verify the cross-role authorization behavior documented in
// docs/protocol-dao-governance.md. They ensure governance token gating and
// role-based permissions are correctly enforced across all governance operations.
// =============================================================================

mod governance_gating_cross_role_authorization {
    use soroban_sdk::testutils::{Address as _, Ledger as _};
    use soroban_sdk::token::StellarAssetClient;
    use soroban_sdk::{Address, Env};
    use veritasor_protocol_dao::{ProposalStatus, ProtocolDao, ProtocolDaoClient};

    const DEFAULT_MIN_VOTES: u32 = 1;
    const DEFAULT_PROPOSAL_DURATION: u32 = 120_960;

    fn setup_dao_with_token(
        min_votes: u32,
        proposal_duration: u32,
    ) -> (Env, ProtocolDaoClient<'static>, Address, Address, Address) {
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

        (env, client, admin, token_addr, token_admin)
    }

    fn setup_dao_without_token(
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

    fn mint_governance_token(env: &Env, token_addr: &Address, to: &Address, amount: i128) {
        let stellar = StellarAssetClient::new(env, token_addr);
        stellar.mint(to, &amount);
    }

    // -------------------------------------------------------------------------
    // Token Gating: Proposal Creation
    // -------------------------------------------------------------------------

    #[test]
    fn gating_token_holder_can_create_fee_config_proposal() {
        let (env, client, _admin, gov_token, _) = setup_dao_with_token(1, 100);
        let creator = Address::generate(&env);
        mint_governance_token(&env, &gov_token, &creator, 1);

        let fee_token = Address::generate(&env);
        let collector = Address::generate(&env);
        let proposal_id =
            client.create_fee_config_proposal(&creator, &fee_token, &collector, &500, &true);

        let proposal = client.get_proposal(&proposal_id).unwrap();
        assert_eq!(proposal.creator, creator);
        assert_eq!(proposal.status, ProposalStatus::Pending);
    }

    #[test]
    #[should_panic(expected = "insufficient governance token balance")]
    fn gating_non_token_holder_cannot_create_fee_config_proposal() {
        let (env, client, _admin, _gov_token, _) = setup_dao_with_token(1, 100);
        let non_holder = Address::generate(&env);

        let fee_token = Address::generate(&env);
        let collector = Address::generate(&env);
        client.create_fee_config_proposal(&non_holder, &fee_token, &collector, &500, &true);
    }

    #[test]
    fn gating_token_holder_can_create_fee_toggle_proposal() {
        let (env, client, _admin, gov_token, _) = setup_dao_with_token(1, 100);
        let creator = Address::generate(&env);
        mint_governance_token(&env, &gov_token, &creator, 1);

        let proposal_id = client.create_fee_toggle_proposal(&creator, &false);

        let proposal = client.get_proposal(&proposal_id).unwrap();
        assert_eq!(proposal.creator, creator);
        assert_eq!(proposal.status, ProposalStatus::Pending);
    }

    #[test]
    #[should_panic(expected = "insufficient governance token balance")]
    fn gating_non_token_holder_cannot_create_fee_toggle_proposal() {
        let (env, client, _admin, _gov_token, _) = setup_dao_with_token(1, 100);
        let non_holder = Address::generate(&env);
        client.create_fee_toggle_proposal(&non_holder, &true);
    }

    #[test]
    fn gating_token_holder_can_create_gov_config_proposal() {
        let (env, client, _admin, gov_token, _) = setup_dao_with_token(1, 100);
        let creator = Address::generate(&env);
        mint_governance_token(&env, &gov_token, &creator, 1);

        let proposal_id = client.create_gov_config_proposal(&creator, &5, &200);

        let proposal = client.get_proposal(&proposal_id).unwrap();
        assert_eq!(proposal.creator, creator);
        assert_eq!(proposal.status, ProposalStatus::Pending);
    }

    #[test]
    #[should_panic(expected = "insufficient governance token balance")]
    fn gating_non_token_holder_cannot_create_gov_config_proposal() {
        let (env, client, _admin, _gov_token, _) = setup_dao_with_token(1, 100);
        let non_holder = Address::generate(&env);
        client.create_gov_config_proposal(&non_holder, &5, &200);
    }

    // -------------------------------------------------------------------------
    // Token Gating: Voting
    // -------------------------------------------------------------------------

    #[test]
    fn gating_token_holder_can_vote_for_proposal() {
        let (env, client, _admin, gov_token, _) = setup_dao_with_token(1, 100);
        let creator = Address::generate(&env);
        let voter = Address::generate(&env);
        mint_governance_token(&env, &gov_token, &creator, 1);
        mint_governance_token(&env, &gov_token, &voter, 1);

        let fee_token = Address::generate(&env);
        let collector = Address::generate(&env);
        let proposal_id =
            client.create_fee_config_proposal(&creator, &fee_token, &collector, &500, &true);

        client.vote_for(&voter, &proposal_id);

        assert_eq!(client.get_votes_for(&proposal_id), 1);
    }

    #[test]
    #[should_panic(expected = "insufficient governance token balance")]
    fn gating_non_token_holder_cannot_vote_for_proposal() {
        let (env, client, _admin, gov_token, _) = setup_dao_with_token(1, 100);
        let creator = Address::generate(&env);
        let non_holder = Address::generate(&env);
        mint_governance_token(&env, &gov_token, &creator, 1);

        let fee_token = Address::generate(&env);
        let collector = Address::generate(&env);
        let proposal_id =
            client.create_fee_config_proposal(&creator, &fee_token, &collector, &500, &true);

        client.vote_for(&non_holder, &proposal_id);
    }

    #[test]
    fn gating_token_holder_can_vote_against_proposal() {
        let (env, client, _admin, gov_token, _) = setup_dao_with_token(1, 100);
        let creator = Address::generate(&env);
        let voter = Address::generate(&env);
        mint_governance_token(&env, &gov_token, &creator, 1);
        mint_governance_token(&env, &gov_token, &voter, 1);

        let fee_token = Address::generate(&env);
        let collector = Address::generate(&env);
        let proposal_id =
            client.create_fee_config_proposal(&creator, &fee_token, &collector, &500, &true);

        client.vote_against(&voter, &proposal_id);

        assert_eq!(client.get_votes_against(&proposal_id), 1);
    }

    #[test]
    #[should_panic(expected = "insufficient governance token balance")]
    fn gating_non_token_holder_cannot_vote_against_proposal() {
        let (env, client, _admin, gov_token, _) = setup_dao_with_token(1, 100);
        let creator = Address::generate(&env);
        let non_holder = Address::generate(&env);
        mint_governance_token(&env, &gov_token, &creator, 1);

        let fee_token = Address::generate(&env);
        let collector = Address::generate(&env);
        let proposal_id =
            client.create_fee_config_proposal(&creator, &fee_token, &collector, &500, &true);

        client.vote_against(&non_holder, &proposal_id);
    }

    // -------------------------------------------------------------------------
    // No Token Configured: Permissionless Operations
    // -------------------------------------------------------------------------

    #[test]
    fn no_token_anyone_can_create_proposal() {
        let (env, client, _admin) = setup_dao_without_token(1, 100);
        let anyone = Address::generate(&env);

        let fee_token = Address::generate(&env);
        let collector = Address::generate(&env);
        let proposal_id =
            client.create_fee_config_proposal(&anyone, &fee_token, &collector, &500, &true);

        let proposal = client.get_proposal(&proposal_id).unwrap();
        assert_eq!(proposal.creator, anyone);
    }

    #[test]
    fn no_token_anyone_can_vote() {
        let (env, client, _admin) = setup_dao_without_token(1, 100);
        let creator = Address::generate(&env);
        let voter = Address::generate(&env);

        let fee_token = Address::generate(&env);
        let collector = Address::generate(&env);
        let proposal_id =
            client.create_fee_config_proposal(&creator, &fee_token, &collector, &500, &true);

        client.vote_for(&voter, &proposal_id);

        assert_eq!(client.get_votes_for(&proposal_id), 1);
    }

    // -------------------------------------------------------------------------
    // Cross-Role: Admin Operations
    // -------------------------------------------------------------------------

    #[test]
    fn cross_role_admin_can_set_governance_token() {
        let (env, client, admin, _gov_token, _) = setup_dao_with_token(1, 100);
        let new_token = Address::generate(&env);

        client.set_governance_token(&admin, &new_token);

        let (_, stored_token, _, _) = client.get_config();
        assert_eq!(stored_token, Some(new_token));
    }

    #[test]
    #[should_panic(expected = "caller is not admin")]
    fn cross_role_non_admin_cannot_set_governance_token() {
        let (env, client, _admin, gov_token, _) = setup_dao_with_token(1, 100);
        let token_holder = Address::generate(&env);
        mint_governance_token(&env, &gov_token, &token_holder, 100);

        let new_token = Address::generate(&env);
        client.set_governance_token(&token_holder, &new_token);
    }

    #[test]
    fn cross_role_admin_can_set_voting_config() {
        let (_, client, admin, _, _) = setup_dao_with_token(1, 100);

        client.set_voting_config(&admin, &10, &500);

        let (_, _, min_votes, duration) = client.get_config();
        assert_eq!(min_votes, 10);
        assert_eq!(duration, 500);
    }

    #[test]
    #[should_panic(expected = "caller is not admin")]
    fn cross_role_non_admin_cannot_set_voting_config() {
        let (env, client, _admin, gov_token, _) = setup_dao_with_token(1, 100);
        let token_holder = Address::generate(&env);
        mint_governance_token(&env, &gov_token, &token_holder, 100);

        client.set_voting_config(&token_holder, &10, &500);
    }

    // -------------------------------------------------------------------------
    // Cross-Role: Cancel Authorization
    // -------------------------------------------------------------------------

    #[test]
    fn cross_role_creator_can_cancel_own_proposal() {
        let (env, client, _admin, gov_token, _) = setup_dao_with_token(1, 100);
        let creator = Address::generate(&env);
        mint_governance_token(&env, &gov_token, &creator, 1);

        let fee_token = Address::generate(&env);
        let collector = Address::generate(&env);
        let proposal_id =
            client.create_fee_config_proposal(&creator, &fee_token, &collector, &500, &true);

        client.cancel_proposal(&creator, &proposal_id);

        let proposal = client.get_proposal(&proposal_id).unwrap();
        assert_eq!(proposal.status, ProposalStatus::Rejected);
    }

    #[test]
    fn cross_role_admin_can_cancel_any_proposal() {
        let (env, client, admin, gov_token, _) = setup_dao_with_token(1, 100);
        let creator = Address::generate(&env);
        mint_governance_token(&env, &gov_token, &creator, 1);

        let fee_token = Address::generate(&env);
        let collector = Address::generate(&env);
        let proposal_id =
            client.create_fee_config_proposal(&creator, &fee_token, &collector, &500, &true);

        client.cancel_proposal(&admin, &proposal_id);

        let proposal = client.get_proposal(&proposal_id).unwrap();
        assert_eq!(proposal.status, ProposalStatus::Rejected);
    }

    #[test]
    #[should_panic(expected = "only creator or admin can cancel")]
    fn cross_role_other_token_holder_cannot_cancel_proposal() {
        let (env, client, _admin, gov_token, _) = setup_dao_with_token(1, 100);
        let creator = Address::generate(&env);
        let other_holder = Address::generate(&env);
        mint_governance_token(&env, &gov_token, &creator, 1);
        mint_governance_token(&env, &gov_token, &other_holder, 1);

        let fee_token = Address::generate(&env);
        let collector = Address::generate(&env);
        let proposal_id =
            client.create_fee_config_proposal(&creator, &fee_token, &collector, &500, &true);

        client.cancel_proposal(&other_holder, &proposal_id);
    }

    #[test]
    #[should_panic(expected = "only creator or admin can cancel")]
    fn cross_role_voter_cannot_cancel_proposal() {
        let (env, client, _admin, gov_token, _) = setup_dao_with_token(2, 100);
        let creator = Address::generate(&env);
        let voter = Address::generate(&env);
        mint_governance_token(&env, &gov_token, &creator, 1);
        mint_governance_token(&env, &gov_token, &voter, 1);

        let fee_token = Address::generate(&env);
        let collector = Address::generate(&env);
        let proposal_id =
            client.create_fee_config_proposal(&creator, &fee_token, &collector, &500, &true);

        client.vote_for(&voter, &proposal_id);

        client.cancel_proposal(&voter, &proposal_id);
    }

    // -------------------------------------------------------------------------
    // Cross-Role: Execution
    // -------------------------------------------------------------------------

    #[test]
    fn cross_role_any_address_can_execute_approved_proposal() {
        let (env, client, _admin, gov_token, _) = setup_dao_with_token(1, 100);
        let creator = Address::generate(&env);
        let voter = Address::generate(&env);
        let executor = Address::generate(&env);
        mint_governance_token(&env, &gov_token, &creator, 1);
        mint_governance_token(&env, &gov_token, &voter, 1);

        let fee_token = Address::generate(&env);
        let collector = Address::generate(&env);
        let proposal_id =
            client.create_fee_config_proposal(&creator, &fee_token, &collector, &500, &true);

        client.vote_for(&voter, &proposal_id);

        client.execute_proposal(&executor, &proposal_id);

        let proposal = client.get_proposal(&proposal_id).unwrap();
        assert_eq!(proposal.status, ProposalStatus::Executed);
    }

    #[test]
    fn cross_role_non_token_holder_can_execute_approved_proposal() {
        let (env, client, _admin, gov_token, _) = setup_dao_with_token(1, 100);
        let creator = Address::generate(&env);
        let voter = Address::generate(&env);
        mint_governance_token(&env, &gov_token, &creator, 1);
        mint_governance_token(&env, &gov_token, &voter, 1);

        let non_holder_executor = Address::generate(&env);

        let fee_token = Address::generate(&env);
        let collector = Address::generate(&env);
        let proposal_id =
            client.create_fee_config_proposal(&creator, &fee_token, &collector, &500, &true);

        client.vote_for(&voter, &proposal_id);

        client.execute_proposal(&non_holder_executor, &proposal_id);

        let proposal = client.get_proposal(&proposal_id).unwrap();
        assert_eq!(proposal.status, ProposalStatus::Executed);
    }

    // -------------------------------------------------------------------------
    // Cross-Role: Double Vote Prevention
    // -------------------------------------------------------------------------

    #[test]
    #[should_panic(expected = "already voted")]
    fn cross_role_same_voter_cannot_vote_twice() {
        let (env, client, _admin, gov_token, _) = setup_dao_with_token(2, 100);
        let creator = Address::generate(&env);
        let voter = Address::generate(&env);
        mint_governance_token(&env, &gov_token, &creator, 1);
        mint_governance_token(&env, &gov_token, &voter, 1);

        let fee_token = Address::generate(&env);
        let collector = Address::generate(&env);
        let proposal_id =
            client.create_fee_config_proposal(&creator, &fee_token, &collector, &500, &true);

        client.vote_for(&voter, &proposal_id);
        client.vote_for(&voter, &proposal_id);
    }

    #[test]
    #[should_panic(expected = "already voted")]
    fn cross_role_voter_cannot_vote_for_then_against() {
        let (env, client, _admin, gov_token, _) = setup_dao_with_token(2, 100);
        let creator = Address::generate(&env);
        let voter = Address::generate(&env);
        mint_governance_token(&env, &gov_token, &creator, 1);
        mint_governance_token(&env, &gov_token, &voter, 1);

        let fee_token = Address::generate(&env);
        let collector = Address::generate(&env);
        let proposal_id =
            client.create_fee_config_proposal(&creator, &fee_token, &collector, &500, &true);

        client.vote_for(&voter, &proposal_id);
        client.vote_against(&voter, &proposal_id);
    }

    // -------------------------------------------------------------------------
    // Cross-Role: Proposal Lifecycle Transitions
    // -------------------------------------------------------------------------

    #[test]
    #[should_panic(expected = "proposal is not pending")]
    fn cross_role_cannot_vote_on_executed_proposal() {
        let (env, client, _admin, gov_token, _) = setup_dao_with_token(1, 100);
        let creator = Address::generate(&env);
        let voter1 = Address::generate(&env);
        let voter2 = Address::generate(&env);
        mint_governance_token(&env, &gov_token, &creator, 1);
        mint_governance_token(&env, &gov_token, &voter1, 1);
        mint_governance_token(&env, &gov_token, &voter2, 1);

        let fee_token = Address::generate(&env);
        let collector = Address::generate(&env);
        let proposal_id =
            client.create_fee_config_proposal(&creator, &fee_token, &collector, &500, &true);

        client.vote_for(&voter1, &proposal_id);
        client.execute_proposal(&creator, &proposal_id);

        client.vote_for(&voter2, &proposal_id);
    }

    #[test]
    #[should_panic(expected = "proposal is not pending")]
    fn cross_role_cannot_vote_on_rejected_proposal() {
        let (env, client, _admin, gov_token, _) = setup_dao_with_token(1, 100);
        let creator = Address::generate(&env);
        let voter = Address::generate(&env);
        mint_governance_token(&env, &gov_token, &creator, 1);
        mint_governance_token(&env, &gov_token, &voter, 1);

        let fee_token = Address::generate(&env);
        let collector = Address::generate(&env);
        let proposal_id =
            client.create_fee_config_proposal(&creator, &fee_token, &collector, &500, &true);

        client.cancel_proposal(&creator, &proposal_id);

        client.vote_for(&voter, &proposal_id);
    }

    #[test]
    #[should_panic(expected = "proposal is not pending")]
    fn cross_role_cannot_execute_rejected_proposal() {
        let (env, client, _admin, gov_token, _) = setup_dao_with_token(1, 100);
        let creator = Address::generate(&env);
        let voter = Address::generate(&env);
        mint_governance_token(&env, &gov_token, &creator, 1);
        mint_governance_token(&env, &gov_token, &voter, 1);

        let fee_token = Address::generate(&env);
        let collector = Address::generate(&env);
        let proposal_id =
            client.create_fee_config_proposal(&creator, &fee_token, &collector, &500, &true);

        client.vote_for(&voter, &proposal_id);
        client.cancel_proposal(&creator, &proposal_id);

        client.execute_proposal(&voter, &proposal_id);
    }

    #[test]
    #[should_panic(expected = "proposal is not pending")]
    fn cross_role_cannot_cancel_executed_proposal() {
        let (env, client, admin, gov_token, _) = setup_dao_with_token(1, 100);
        let creator = Address::generate(&env);
        let voter = Address::generate(&env);
        mint_governance_token(&env, &gov_token, &creator, 1);
        mint_governance_token(&env, &gov_token, &voter, 1);

        let fee_token = Address::generate(&env);
        let collector = Address::generate(&env);
        let proposal_id =
            client.create_fee_config_proposal(&creator, &fee_token, &collector, &500, &true);

        client.vote_for(&voter, &proposal_id);
        client.execute_proposal(&creator, &proposal_id);

        client.cancel_proposal(&admin, &proposal_id);
    }

    // -------------------------------------------------------------------------
    // Cross-Role: Expiry Behavior
    // -------------------------------------------------------------------------

    #[test]
    #[should_panic(expected = "proposal expired")]
    fn cross_role_cannot_vote_on_expired_proposal() {
        let (env, client, _admin, gov_token, _) = setup_dao_with_token(1, 5);
        let creator = Address::generate(&env);
        let voter = Address::generate(&env);
        mint_governance_token(&env, &gov_token, &creator, 1);
        mint_governance_token(&env, &gov_token, &voter, 1);

        let fee_token = Address::generate(&env);
        let collector = Address::generate(&env);
        let proposal_id =
            client.create_fee_config_proposal(&creator, &fee_token, &collector, &500, &true);

        env.ledger().with_mut(|li| {
            li.sequence_number += 10;
        });

        client.vote_for(&voter, &proposal_id);
    }

    #[test]
    #[should_panic(expected = "proposal expired")]
    fn cross_role_cannot_execute_expired_proposal() {
        let (env, client, _admin, gov_token, _) = setup_dao_with_token(1, 5);
        let creator = Address::generate(&env);
        let voter = Address::generate(&env);
        mint_governance_token(&env, &gov_token, &creator, 1);
        mint_governance_token(&env, &gov_token, &voter, 1);

        let fee_token = Address::generate(&env);
        let collector = Address::generate(&env);
        let proposal_id =
            client.create_fee_config_proposal(&creator, &fee_token, &collector, &500, &true);

        client.vote_for(&voter, &proposal_id);

        env.ledger().with_mut(|li| {
            li.sequence_number += 10;
        });

        client.execute_proposal(&creator, &proposal_id);
    }

    // -------------------------------------------------------------------------
    // Cross-Role: Quorum and Majority Enforcement
    // -------------------------------------------------------------------------

    #[test]
    #[should_panic(expected = "quorum not met")]
    fn cross_role_execution_requires_quorum() {
        let (env, client, _admin, gov_token, _) = setup_dao_with_token(3, 100);
        let creator = Address::generate(&env);
        let voter1 = Address::generate(&env);
        let voter2 = Address::generate(&env);
        mint_governance_token(&env, &gov_token, &creator, 1);
        mint_governance_token(&env, &gov_token, &voter1, 1);
        mint_governance_token(&env, &gov_token, &voter2, 1);

        let fee_token = Address::generate(&env);
        let collector = Address::generate(&env);
        let proposal_id =
            client.create_fee_config_proposal(&creator, &fee_token, &collector, &500, &true);

        client.vote_for(&voter1, &proposal_id);
        client.vote_for(&voter2, &proposal_id);

        client.execute_proposal(&creator, &proposal_id);
    }

    #[test]
    #[should_panic(expected = "proposal not approved")]
    fn cross_role_execution_requires_majority() {
        let (env, client, _admin, gov_token, _) = setup_dao_with_token(2, 100);
        let creator = Address::generate(&env);
        let voter1 = Address::generate(&env);
        let voter2 = Address::generate(&env);
        mint_governance_token(&env, &gov_token, &creator, 1);
        mint_governance_token(&env, &gov_token, &voter1, 1);
        mint_governance_token(&env, &gov_token, &voter2, 1);

        let fee_token = Address::generate(&env);
        let collector = Address::generate(&env);
        let proposal_id =
            client.create_fee_config_proposal(&creator, &fee_token, &collector, &500, &true);

        client.vote_for(&voter1, &proposal_id);
        client.vote_against(&voter2, &proposal_id);

        client.execute_proposal(&creator, &proposal_id);
    }

    // -------------------------------------------------------------------------
    // Cross-Role: Multiple Voters and Role Isolation
    // -------------------------------------------------------------------------

    #[test]
    fn cross_role_multiple_independent_voters() {
        let (env, client, _admin, gov_token, _) = setup_dao_with_token(3, 100);
        let creator = Address::generate(&env);
        let voter1 = Address::generate(&env);
        let voter2 = Address::generate(&env);
        let voter3 = Address::generate(&env);
        mint_governance_token(&env, &gov_token, &creator, 1);
        mint_governance_token(&env, &gov_token, &voter1, 1);
        mint_governance_token(&env, &gov_token, &voter2, 1);
        mint_governance_token(&env, &gov_token, &voter3, 1);

        let fee_token = Address::generate(&env);
        let collector = Address::generate(&env);
        let proposal_id =
            client.create_fee_config_proposal(&creator, &fee_token, &collector, &500, &true);

        client.vote_for(&voter1, &proposal_id);
        client.vote_for(&voter2, &proposal_id);
        client.vote_against(&voter3, &proposal_id);

        assert_eq!(client.get_votes_for(&proposal_id), 2);
        assert_eq!(client.get_votes_against(&proposal_id), 1);
    }

    #[test]
    fn cross_role_creator_can_also_vote() {
        let (env, client, _admin, gov_token, _) = setup_dao_with_token(1, 100);
        let creator = Address::generate(&env);
        mint_governance_token(&env, &gov_token, &creator, 1);

        let fee_token = Address::generate(&env);
        let collector = Address::generate(&env);
        let proposal_id =
            client.create_fee_config_proposal(&creator, &fee_token, &collector, &500, &true);

        client.vote_for(&creator, &proposal_id);

        assert_eq!(client.get_votes_for(&proposal_id), 1);
        client.execute_proposal(&creator, &proposal_id);

        let proposal = client.get_proposal(&proposal_id).unwrap();
        assert_eq!(proposal.status, ProposalStatus::Executed);
    }

    #[test]
    fn cross_role_admin_can_also_be_voter_if_token_holder() {
        let (env, client, admin, gov_token, _) = setup_dao_with_token(1, 100);
        let creator = Address::generate(&env);
        mint_governance_token(&env, &gov_token, &creator, 1);
        mint_governance_token(&env, &gov_token, &admin, 1);

        let fee_token = Address::generate(&env);
        let collector = Address::generate(&env);
        let proposal_id =
            client.create_fee_config_proposal(&creator, &fee_token, &collector, &500, &true);

        client.vote_for(&admin, &proposal_id);

        assert_eq!(client.get_votes_for(&proposal_id), 1);
    }

    #[test]
    #[should_panic(expected = "insufficient governance token balance")]
    fn cross_role_admin_without_token_cannot_vote() {
        let (env, client, admin, gov_token, _) = setup_dao_with_token(1, 100);
        let creator = Address::generate(&env);
        mint_governance_token(&env, &gov_token, &creator, 1);

        let fee_token = Address::generate(&env);
        let collector = Address::generate(&env);
        let proposal_id =
            client.create_fee_config_proposal(&creator, &fee_token, &collector, &500, &true);

        client.vote_for(&admin, &proposal_id);
    }

    // -------------------------------------------------------------------------
    // Cross-Role: Config Defaults
    // -------------------------------------------------------------------------

    #[test]
    fn cross_role_default_config_applied_when_zero() {
        let (_, client, admin, _, _) = setup_dao_with_token(0, 0);

        let (stored_admin, _, min_votes, duration) = client.get_config();
        assert_eq!(stored_admin, admin);
        assert_eq!(min_votes, DEFAULT_MIN_VOTES);
        assert_eq!(duration, DEFAULT_PROPOSAL_DURATION);
    }

    #[test]
    fn cross_role_custom_config_applied() {
        let (_, client, admin, _, _) = setup_dao_with_token(5, 1000);

        let (stored_admin, _, min_votes, duration) = client.get_config();
        assert_eq!(stored_admin, admin);
        assert_eq!(min_votes, 5);
        assert_eq!(duration, 1000);
    }
}
