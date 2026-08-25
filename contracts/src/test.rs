#![cfg(test)]
#![allow(deprecated)]

use super::*;
use soroban_sdk::token;
use soroban_sdk::{
    symbol_short, testutils::Address as _, testutils::Events, vec, Address, Env, IntoVal, String,
};

#[test]
fn test_initialize() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let name = String::from_str(&env, "Test Group");
    let contribution_amount = 1000;

    client.initialize(
        &admin,
        &token,
        &name,
        &contribution_amount,
        &GroupType::Rotational,
        &None,
        &false,
        &None,
    );
}

#[test]
#[should_panic(expected = "Already initialized")]
fn test_double_initialize() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let name = String::from_str(&env, "Test Group");
    let contribution_amount = 1000;

    client.initialize(
        &admin,
        &token,
        &name,
        &contribution_amount,
        &GroupType::Rotational,
        &None,
        &false,
        &None,
    );
    client.initialize(
        &admin,
        &token,
        &name,
        &contribution_amount,
        &GroupType::Rotational,
        &None,
        &false,
        &None,
    );
}

#[test]
fn test_add_member() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let name = String::from_str(&env, "Test Group");
    let contribution_amount = 1000;

    client.initialize(
        &admin,
        &token,
        &name,
        &contribution_amount,
        &GroupType::Rotational,
        &None,
        &false,
        &None,
    );

    env.mock_all_auths();
    let member1 = Address::generate(&env);
    client.add_member(&member1);
}

#[test]
#[should_panic(expected = "Not a member")]
fn test_contribute_not_member() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract(token_admin.clone());
    let name = String::from_str(&env, "Test Group");
    let contribution_amount = 1000;

    client.initialize(
        &admin,
        &token,
        &name,
        &contribution_amount,
        &GroupType::Rotational,
        &None,
        &false,
        &None,
    );

    env.mock_all_auths();
    let not_member = Address::generate(&env);
    client.contribute(&not_member, &1000);
}

#[test]
#[should_panic(expected = "Already contributed this cycle")]
fn test_contribute_twice_same_cycle_is_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token);
    let name = String::from_str(&env, "Test Group");

    client.initialize(
        &admin,
        &token,
        &name,
        &1000i128,
        &GroupType::Rotational,
        &None,
        &false,
        &None,
    );

    let member = Address::generate(&env);
    client.add_member(&member);
    token_client.mint(&member, &5000);

    client.contribute(&member, &1000);
    client.contribute(&member, &1000);
}

#[test]
fn test_contribute_allowed_after_reset() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token);
    let name = String::from_str(&env, "Test Group");

    client.initialize(
        &admin,
        &token,
        &name,
        &1000i128,
        &GroupType::Rotational,
        &None,
        &false,
        &None,
    );

    let member = Address::generate(&env);
    client.add_member(&member);
    token_client.mint(&member, &5000);

    client.contribute(&member, &1000);
    client.reset_cycle();
    client.contribute(&member, &1000);

    assert_eq!(client.get_contribution(&member), 2000);
}

#[test]
fn test_events() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let name = String::from_str(&env, "Test Group");
    let contribution_amount = 1000;

    // 1. Test Initialize Event
    client.initialize(
        &admin,
        &token,
        &name,
        &contribution_amount,
        &GroupType::Rotational,
        &None,
        &false,
        &None,
    );

    assert_eq!(
        env.events().all(),
        vec![
            &env,
            (
                contract_id.clone(),
                soroban_sdk::vec![&env, symbol_short!("init").into_val(&env)],
                (
                    admin.clone(),
                    token.clone(),
                    name.clone(),
                    contribution_amount
                )
                    .into_val(&env)
            )
        ]
    );

    // 2. Test Add Member Event
    let member1 = Address::generate(&env);
    client.add_member(&member1);

    assert_eq!(
        env.events().all(),
        vec![
            &env,
            (
                contract_id.clone(),
                soroban_sdk::vec![
                    &env,
                    symbol_short!("add_mem").into_val(&env),
                    member1.clone().into_val(&env)
                ],
                ().into_val(&env)
            )
        ]
    );
}

#[test]
fn test_goalbased_flexible_contributions() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token);
    let name = String::from_str(&env, "Goal Group");

    client.initialize(
        &admin,
        &token,
        &name,
        &1000i128,
        &GroupType::GoalBased,
        &None,
        &false,
        &None,
    );

    let member = Address::generate(&env);
    client.add_member(&member);
    token_client.mint(&member, &5000);

    // Can contribute varying amounts
    client.contribute(&member, &500);

    // Wait, the test above calls contribute twice in a row, but HasContributedThisCycle is still active.
    // So we need to call reset_cycle() or it will panic.
    client.reset_cycle();
    client.contribute(&member, &1500);

    assert_eq!(client.get_contribution(&member), 2000);
}

#[test]
fn test_goalbased_partial_withdrawal() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token);
    let name = String::from_str(&env, "Goal Group");

    client.initialize(
        &admin,
        &token,
        &name,
        &1000i128,
        &GroupType::GoalBased,
        &None,
        &false,
        &None,
    );

    let member = Address::generate(&env);
    client.add_member(&member);
    token_client.mint(&member, &5000);

    client.contribute(&member, &2000);

    assert_eq!(token_client.balance(&member), 3000);

    client.withdraw_savings(&member, &500);

    assert_eq!(client.get_contribution(&member), 1500);
    assert_eq!(token_client.balance(&member), 3500);
}

#[test]
#[should_panic(expected = "Insufficient savings to withdraw")]
fn test_goalbased_over_withdrawal() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token);
    let name = String::from_str(&env, "Goal Group");

    client.initialize(
        &admin,
        &token,
        &name,
        &1000i128,
        &GroupType::GoalBased,
        &None,
        &false,
        &None,
    );

    let member = Address::generate(&env);
    client.add_member(&member);
    token_client.mint(&member, &5000);

    client.contribute(&member, &1000);
    client.withdraw_savings(&member, &1500);
}

#[test]
#[should_panic(expected = "Target amount not reached yet")]
fn test_goalbased_locked_until_target() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token);
    let name = String::from_str(&env, "Goal Group");

    client.initialize(
        &admin,
        &token,
        &name,
        &1000i128,
        &GroupType::GoalBased,
        &Some(2000), // target amount
        &true,       // lock until target
        &None,       // expected_cycle_days
    );

    let member = Address::generate(&env);
    client.add_member(&member);
    token_client.mint(&member, &5000);

    client.contribute(&member, &1000);
    // Should panic because 1000 < 2000
    client.withdraw_savings(&member, &500);
}

#[test]
fn test_remove_member_no_contribution() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract(token_admin.clone());
    let name = String::from_str(&env, "Test Group");

    client.initialize(
        &admin,
        &token,
        &name,
        &1000i128,
        &GroupType::Rotational,
        &None,
        &false,
        &None,
    );

    let member1 = Address::generate(&env);
    let member2 = Address::generate(&env);
    client.add_member(&member1);
    client.add_member(&member2);

    client.remove_member(&member2);

    env.as_contract(&contract_id, || {
        let members: Vec<Address> = env.storage().instance().get(&DataKey::Members).unwrap();
        assert_eq!(members.len(), 1);
        assert!(members.contains(&member1));
        assert!(!members.contains(&member2));
    });
}

#[test]
fn test_remove_member_with_contribution_refund() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token);
    let name = String::from_str(&env, "Test Group");

    client.initialize(
        &admin,
        &token,
        &name,
        &1000i128,
        &GroupType::Rotational,
        &None,
        &false,
        &None,
    );

    let member1 = Address::generate(&env);
    let member2 = Address::generate(&env);
    client.add_member(&member1);
    client.add_member(&member2);
    token_client.mint(&member1, &5000);
    token_client.mint(&member2, &5000);

    client.contribute(&member1, &1000);
    client.contribute(&member2, &1000);

    assert_eq!(token_client.balance(&member2), 4000);

    client.remove_member(&member2);

    assert_eq!(token_client.balance(&member2), 5000);

    env.as_contract(&contract_id, || {
        // MemberState entry removed entirely during remove_member (O(1) design)
        assert!(!env
            .storage()
            .instance()
            .has(&DataKey::Member(member2.clone())));
    });
}

#[test]
#[should_panic(expected = "Cannot remove member after their payout turn")]
fn test_remove_member_after_payout_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token);
    let name = String::from_str(&env, "Test Group");

    client.initialize(
        &admin,
        &token,
        &name,
        &1000i128,
        &GroupType::Rotational,
        &None,
        &false,
        &None,
    );

    let member1 = Address::generate(&env);
    let member2 = Address::generate(&env);
    client.add_member(&member1);
    client.add_member(&member2);
    token_client.mint(&member1, &5000);
    token_client.mint(&member2, &5000);

    client.contribute(&member1, &1000);
    client.contribute(&member2, &1000);

    client.payout();

    client.remove_member(&member1);
}

#[test]
fn test_remove_member_adjusts_cycle_count() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token);
    let name = String::from_str(&env, "Test Group");

    client.initialize(
        &admin,
        &token,
        &name,
        &1000i128,
        &GroupType::Rotational,
        &None,
        &false,
        &None,
    );

    let member1 = Address::generate(&env);
    let member2 = Address::generate(&env);
    let member3 = Address::generate(&env);
    client.add_member(&member1);
    client.add_member(&member2);
    client.add_member(&member3);
    token_client.mint(&member1, &5000);
    token_client.mint(&member2, &5000);
    token_client.mint(&member3, &5000);

    client.contribute(&member1, &1000);

    env.as_contract(&contract_id, || {
        let count: i128 = env
            .storage()
            .instance()
            .get(&DataKey::CycleMemberCount)
            .unwrap();
        assert_eq!(count, 3);
    });

    client.remove_member(&member3);

    env.as_contract(&contract_id, || {
        let count_after: i128 = env
            .storage()
            .instance()
            .get(&DataKey::CycleMemberCount)
            .unwrap();
        assert_eq!(count_after, 2);
    });

    client.contribute(&member2, &1000);

    client.payout();

    let contract_balance = token_client.balance(&contract_id);
    assert_eq!(contract_balance, 0);

    client.reset_cycle();

    env.as_contract(&contract_id, || {
        assert!(!env.storage().instance().has(&DataKey::CycleMemberCount));
    });
}

#[test]
fn test_remove_last_member_clears_cycle_count() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token);
    let name = String::from_str(&env, "Test Group");

    client.initialize(
        &admin,
        &token,
        &name,
        &1000i128,
        &GroupType::Rotational,
        &None,
        &false,
        &None,
    );

    let member = Address::generate(&env);
    client.add_member(&member);
    token_client.mint(&member, &5000);

    client.contribute(&member, &1000);

    env.as_contract(&contract_id, || {
        let count: i128 = env
            .storage()
            .instance()
            .get(&DataKey::CycleMemberCount)
            .unwrap();
        assert_eq!(count, 1);
    });

    client.remove_member(&member);

    env.as_contract(&contract_id, || {
        assert!(!env.storage().instance().has(&DataKey::CycleMemberCount));
    });
}

#[test]
fn test_deterministic_payout_order() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token);

    client.initialize(
        &admin,
        &token,
        &String::from_str(&env, "Test Group"),
        &1000i128,
        &GroupType::Rotational,
        &None,
        &false,
        &None,
    );

    let member0 = Address::generate(&env);
    let member1 = Address::generate(&env);
    let member2 = Address::generate(&env);
    client.add_member(&member0);
    client.add_member(&member1);
    client.add_member(&member2);

    token_client.mint(&member0, &10000);
    token_client.mint(&member1, &10000);
    token_client.mint(&member2, &10000);

    // Payout 1 goes to member0 (index 0 in join order)
    client.contribute(&member0, &1000);
    client.contribute(&member1, &1000);
    client.contribute(&member2, &1000);
    assert_eq!(client.get_next_payout_recipient(), member0);
    client.payout();
    assert!(client.has_received_payout(&member0));
    assert!(!client.has_received_payout(&member1));
    assert!(!client.has_received_payout(&member2));
    // member0 gets the full pool: 1000 * 3 = 3000
    // balance = 10000 - 1000 (contrib) + 3000 (payout) = 12000
    assert_eq!(token_client.balance(&member0), 12000);

    // After payout, NextPayoutIndex advanced to 1 — member1 is next
    assert_eq!(client.get_next_payout_recipient(), member1);

    // Payout 2 goes to member1 (index 1) after reset + re-contribute
    client.reset_cycle();
    client.contribute(&member0, &1000);
    client.contribute(&member1, &1000);
    client.contribute(&member2, &1000);
    assert_eq!(client.get_next_payout_recipient(), member1);
    client.payout();
    assert!(client.has_received_payout(&member1));
    assert!(!client.has_received_payout(&member2));
    // 10000 - 1000 (round1) - 1000 (round2) + 3000 (payout) = 11000
    assert_eq!(token_client.balance(&member1), 11000);

    // Payout 3 goes to member2 (index 2)
    client.reset_cycle();
    client.contribute(&member0, &1000);
    client.contribute(&member1, &1000);
    client.contribute(&member2, &1000);
    assert_eq!(client.get_next_payout_recipient(), member2);
    client.payout();
    assert!(client.has_received_payout(&member2));
    // 10000 - 1000*3 (3 rounds) + 3000 (payout) = 10000
    assert_eq!(token_client.balance(&member2), 10000);
}

#[test]
fn test_queue_enforced_payout_order() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token);

    client.initialize(
        &admin,
        &token,
        &String::from_str(&env, "Test Group"),
        &1000i128,
        &GroupType::Rotational,
        &None,
        &false,
        &None,
    );

    let member0 = Address::generate(&env);
    let member1 = Address::generate(&env);
    let member2 = Address::generate(&env);
    client.add_member(&member0);
    client.add_member(&member1);
    client.add_member(&member2);

    token_client.mint(&member0, &10000);
    token_client.mint(&member1, &10000);
    token_client.mint(&member2, &10000);

    // Payout must go to member0 (index 0), admin cannot choose
    client.contribute(&member0, &1000);
    client.contribute(&member1, &1000);
    client.contribute(&member2, &1000);

    assert_eq!(client.get_next_payout_recipient(), member0);
    client.payout();
    assert!(client.has_received_payout(&member0));
    assert!(!client.has_received_payout(&member2));

    // After reset, queue advances to member1 (NextPayoutIndex persists)
    client.reset_cycle();
    client.contribute(&member0, &1000);
    client.contribute(&member1, &1000);
    client.contribute(&member2, &1000);

    assert_eq!(client.get_next_payout_recipient(), member1);
    client.payout();
    assert!(client.has_received_payout(&member1));
    assert!(!client.has_received_payout(&member2));

    // After another reset, finally member2
    client.reset_cycle();
    client.contribute(&member0, &1000);
    client.contribute(&member1, &1000);
    client.contribute(&member2, &1000);

    assert_eq!(client.get_next_payout_recipient(), member2);
    client.payout();
    assert!(client.has_received_payout(&member2));
}

#[test]
fn test_cycle_resets_and_starts_again() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token);

    client.initialize(
        &admin,
        &token,
        &String::from_str(&env, "Test Group"),
        &1000i128,
        &GroupType::Rotational,
        &None,
        &false,
        &None,
    );

    let member0 = Address::generate(&env);
    let member1 = Address::generate(&env);
    let member2 = Address::generate(&env);
    client.add_member(&member0);
    client.add_member(&member1);
    client.add_member(&member2);

    token_client.mint(&member0, &10000);
    token_client.mint(&member1, &10000);
    token_client.mint(&member2, &10000);

    // --- Full rotation: member0 → member1 → member2 ---
    // Round 1: payout to member0
    client.contribute(&member0, &1000);
    client.contribute(&member1, &1000);
    client.contribute(&member2, &1000);
    assert_eq!(client.get_next_payout_recipient(), member0);
    client.payout();
    assert!(client.has_received_payout(&member0));

    // Round 2: payout to member1
    client.reset_cycle();
    client.contribute(&member0, &1000);
    client.contribute(&member1, &1000);
    client.contribute(&member2, &1000);
    assert_eq!(client.get_next_payout_recipient(), member1);
    client.payout();
    assert!(client.has_received_payout(&member1));

    // Round 3: payout to member2
    client.reset_cycle();
    client.contribute(&member0, &1000);
    client.contribute(&member1, &1000);
    client.contribute(&member2, &1000);
    assert_eq!(client.get_next_payout_recipient(), member2);
    client.payout();
    assert!(client.has_received_payout(&member2));

    // --- Full reset: reset_rotation() resets NextPayoutIndex to 0 ---
    client.reset_cycle();
    client.reset_rotation();

    // After full reset, all has_received_payout flags are cleared
    assert!(!client.has_received_payout(&member0));
    assert!(!client.has_received_payout(&member1));
    assert!(!client.has_received_payout(&member2));

    // New rotation starts with member0 again
    client.contribute(&member0, &1000);
    client.contribute(&member1, &1000);
    client.contribute(&member2, &1000);
    assert_eq!(client.get_next_payout_recipient(), member0);
    client.payout();
    assert!(client.has_received_payout(&member0));
}

#[test]
#[should_panic(expected = "Contribution amount must be positive")]
fn test_initialize_with_negative_amount_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(
        &admin,
        &token,
        &String::from_str(&env, "Test"),
        &-1000i128,
        &GroupType::Rotational,
        &None,
        &false,
        &None,
    );
}

#[test]
#[should_panic(expected = "Contribution amount must be positive")]
fn test_initialize_with_zero_amount_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(
        &admin,
        &token,
        &String::from_str(&env, "Test"),
        &0i128,
        &GroupType::Rotational,
        &None,
        &false,
        &None,
    );
}

#[test]
#[should_panic(expected = "Contribution amount exceeds maximum limit")]
fn test_initialize_with_exceeding_max_amount_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(
        &admin,
        &token,
        &String::from_str(&env, "Test"),
        &1_000_000_000_000_001i128,
        &GroupType::Rotational,
        &None,
        &false,
        &None,
    );
}

#[test]
#[should_panic(expected = "Math overflow in pool calculation")]
fn test_payout_pool_size_overflow_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, KoloSavingsContract);
    let client = KoloSavingsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(
        &admin,
        &token,
        &String::from_str(&env, "Test"),
        &1000i128,
        &GroupType::Rotational,
        &None,
        &false,
        &None,
    );

    let member0 = Address::generate(&env);
    client.add_member(&member0);

    env.as_contract(&contract_id, || {
        // Force an overflow scenario
        env.storage()
            .instance()
            .set(&DataKey::ContributionAmount, &i128::MAX);
        env.storage()
            .instance()
            .set(&DataKey::CycleMemberCount, &2i128);
    });

    client.payout();
}
