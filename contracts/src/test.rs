#![cfg(test)]
#![allow(deprecated)]

use super::*;
use soroban_sdk::token;
use soroban_sdk::{
    symbol_short, testutils::Address as _, testutils::Events, vec, Address, Env, FromVal, IntoVal,
    String,
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
    );
    client.initialize(
        &admin,
        &token,
        &name,
        &contribution_amount,
        &GroupType::Rotational,
        &None,
        &false,
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
    );

    assert_eq!(
        env.events().all(),
        vec![
            &env,
            (
                contract_id.clone(),
                soroban_sdk::vec![&env, symbol_short!("init").into_val(&env)],
                (admin.clone(), token.clone(), name.clone(), contribution_amount).into_val(&env)
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
                soroban_sdk::vec![&env, symbol_short!("add_mem").into_val(&env), member1.clone().into_val(&env)],
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
    );

    let member = Address::generate(&env);
    client.add_member(&member);
    token_client.mint(&member, &5000);

    client.contribute(&member, &1000);
    // Should panic because 1000 < 2000
    client.withdraw_savings(&member, &500);
}

