#![no_std]
#![allow(deprecated)]

use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, token, Address, Env, String, Vec,
};

mod test;

const LEDGERS_TO_LIVE: u32 = 518_400; // ~30 days at 5s/ledger

fn extend_instance_ttl(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(LEDGERS_TO_LIVE / 2, LEDGERS_TO_LIVE);
}

#[contracttype]
#[derive(Clone, PartialEq, Eq)]
pub enum GroupType {
    Rotational,
    GoalBased,
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    Token,
    Name,
    ContributionAmount,
    Members,
    Contributions(Address),
    NextPayoutIndex,
    HasContributedThisCycle(Address),
    CycleMemberCount,
    CurrentCycleContributions,
    User(Address),
    GroupType,
    TargetAmount,
    LockUntilTarget,
}

#[contracttype]
#[derive(Clone)]
pub struct User {
    pub wallet_address: Address,
    pub joined_groups: Vec<u32>,
}

#[contract]
pub struct KoloSavingsContract;

#[contractimpl]
impl KoloSavingsContract {
    /// Initialize the savings group
    #[allow(clippy::too_many_arguments)]
    pub fn initialize(
        env: Env,
        admin: Address,
        token: Address,
        name: String,
        contribution_amount: i128,
        group_type: GroupType,
        target_amount: Option<i128>,
        lock_until_target: bool,
    ) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("Already initialized");
        }

        admin.require_auth();
        extend_instance_ttl(&env);

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Token, &token);
        env.storage().instance().set(&DataKey::Name, &name);
        env.storage()
            .instance()
            .set(&DataKey::ContributionAmount, &contribution_amount);
        env.storage()
            .instance()
            .set(&DataKey::GroupType, &group_type);
        if let Some(target) = target_amount {
            env.storage()
                .instance()
                .set(&DataKey::TargetAmount, &target);
        }
        env.storage()
            .instance()
            .set(&DataKey::LockUntilTarget, &lock_until_target);

        let empty_members: Vec<Address> = Vec::new(&env);
        env.storage()
            .instance()
            .set(&DataKey::Members, &empty_members);

        env.events().publish(
            (symbol_short!("init"),),
            (admin, token, name, contribution_amount),
        );
    }

    /// Add a member to the group (Admin only)
    pub fn add_member(env: Env, new_member: Address) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        extend_instance_ttl(&env);

        let mut members: Vec<Address> = env.storage().instance().get(&DataKey::Members).unwrap();
        if !members.contains(&new_member) {
            members.push_back(new_member.clone());
            env.storage().instance().set(&DataKey::Members, &members);
            env.storage()
                .persistent()
                .set(&DataKey::Contributions(new_member.clone()), &0i128);
            if !env.storage().instance().has(&DataKey::NextPayoutIndex) {
                env.storage()
                    .instance()
                    .set(&DataKey::NextPayoutIndex, &0u32);
            }
            env.storage().persistent().set(
                &DataKey::HasContributedThisCycle(new_member.clone()),
                &false,
            );

            env.events()
                .publish((symbol_short!("add_mem"), new_member), ());
        }
    }

    /// Remove a member from the group (Admin only)
    /// Refunds current cycle contribution if applicable. Panics if member already received payout.
    pub fn remove_member(env: Env, member_to_remove: Address) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        extend_instance_ttl(&env);

        let mut members: Vec<Address> = env.storage().instance().get(&DataKey::Members).unwrap();
        if !members.contains(&member_to_remove) {
            panic!("Not a member");
        }

        let members_list: Vec<Address> = env.storage().instance().get(&DataKey::Members).unwrap();
        let remove_index = members_list
            .iter()
            .position(|m| m == member_to_remove)
            .unwrap() as u32;
        let next_payout_index: u32 = env
            .storage()
            .instance()
            .get(&DataKey::NextPayoutIndex)
            .unwrap_or(0);
        if remove_index < next_payout_index {
            panic!("Cannot remove member after their payout turn");
        }

        let has_contributed: bool = env
            .storage()
            .persistent()
            .get(&DataKey::HasContributedThisCycle(member_to_remove.clone()))
            .unwrap_or(false);
        if has_contributed {
            let contribution_amount: i128 = env
                .storage()
                .instance()
                .get(&DataKey::ContributionAmount)
                .unwrap();
            let token: Address = env.storage().instance().get(&DataKey::Token).unwrap();
            let token_client = token::Client::new(&env, &token);
            token_client.transfer(
                &env.current_contract_address(),
                &member_to_remove,
                &contribution_amount,
            );
        }

        env.storage()
            .persistent()
            .remove(&DataKey::HasContributedThisCycle(member_to_remove.clone()));

        if env.storage().instance().has(&DataKey::CycleMemberCount) {
            let current_count: i128 = env
                .storage()
                .instance()
                .get(&DataKey::CycleMemberCount)
                .unwrap();
            if current_count <= 1 {
                env.storage().instance().remove(&DataKey::CycleMemberCount);
            } else {
                env.storage()
                    .instance()
                    .set(&DataKey::CycleMemberCount, &(current_count - 1));
            }
        }

        let index = members.iter().position(|m| m == member_to_remove).unwrap() as u32;
        members.remove(index);
        env.storage().instance().set(&DataKey::Members, &members);

        env.events()
            .publish((symbol_short!("rm_member"), member_to_remove), ());
    }

    /// Contribute to the pool
    pub fn contribute(env: Env, member: Address, amount: i128) {
        member.require_auth();
        extend_instance_ttl(&env);

        let expected_amount: i128 = env
            .storage()
            .instance()
            .get(&DataKey::ContributionAmount)
            .unwrap();
        let group_type: GroupType = env
            .storage()
            .instance()
            .get(&DataKey::GroupType)
            .unwrap_or(GroupType::Rotational);

        if group_type == GroupType::Rotational && amount != expected_amount {
            panic!("Must contribute the exact amount");
        }

        if amount <= 0 {
            panic!("Amount must be positive");
        }

        let members: Vec<Address> = env.storage().instance().get(&DataKey::Members).unwrap();
        if !members.contains(&member) {
            panic!("Not a member");
        }

        // Freeze the member count at the start of a cycle on the first contribution
        if group_type == GroupType::Rotational
            && !env.storage().instance().has(&DataKey::CycleMemberCount)
        {
            let count = members.len() as i128;
            env.storage().instance().set(&DataKey::CycleMemberCount, &count);
            env.storage().instance().set(&DataKey::CurrentCycleContributions, &0i128);
        }

        let token: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let token_client = token::Client::new(&env, &token);

        // Transfer tokens from the member to this contract
        token_client.transfer(&member, env.current_contract_address(), &amount);

        env.storage()
            .persistent()
            .set(&DataKey::HasContributedThisCycle(member.clone()), &true);

        let current_contribution: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Contributions(member.clone()))
            .unwrap_or(0);
        env.storage().persistent().set(
            &DataKey::Contributions(member.clone()),
            &(current_contribution + amount),
        );

        // Increment current cycle contributions counter
        let cycle_contributions: i128 = env.storage().instance().get(&DataKey::CurrentCycleContributions).unwrap_or(0);
        env.storage().instance().set(&DataKey::CurrentCycleContributions, &(cycle_contributions + 1));

        env.events().publish((symbol_short!("contrib"), member), amount);
    }

    /// Withdraw payout (Admin triggers payout to the next member in queue)
    /// Enforces strictly deterministic rotational payout (Ajo/Esusu) order.
    pub fn payout(env: Env) {
        let group_type: GroupType = env
            .storage()
            .instance()
            .get(&DataKey::GroupType)
            .unwrap_or(GroupType::Rotational);
        if group_type == GroupType::GoalBased {
            panic!("Payouts not allowed in GoalBased groups");
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        extend_instance_ttl(&env);

        let members: Vec<Address> = env.storage().instance().get(&DataKey::Members).unwrap();
        let next_index: u32 = env
            .storage()
            .instance()
            .get(&DataKey::NextPayoutIndex)
            .unwrap_or(0);

        if next_index >= members.len() {
            panic!("All members have received payouts this cycle");
        }

        let recipient: Address = members.get(next_index).unwrap();

        let contribution_amount: i128 = env
            .storage()
            .instance()
            .get(&DataKey::ContributionAmount)
            .unwrap();
        let frozen_count: i128 = env
            .storage()
            .instance()
            .get(&DataKey::CycleMemberCount)
            .expect("No active cycle");
        let pool_size = contribution_amount * frozen_count;

        let token: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let token_client = token::Client::new(&env, &token);

        let contract_balance = token_client.balance(&env.current_contract_address());
        if pool_size > contract_balance {
            panic!("Insufficient funds in contract for full payout");
        }

        env.storage()
            .instance()
            .set(&DataKey::NextPayoutIndex, &(next_index + 1));
        token_client.transfer(&env.current_contract_address(), &recipient, &pool_size);

        env.events()
            .publish((symbol_short!("payout"), recipient), pool_size);
    }

    /// Returns the address of the next member in line for a payout.
    pub fn get_next_payout_recipient(env: Env) -> Address {
        let members: Vec<Address> = env.storage().instance().get(&DataKey::Members).unwrap();
        let next_index: u32 = env
            .storage()
            .instance()
            .get(&DataKey::NextPayoutIndex)
            .unwrap_or(0);
        members
            .get(next_index)
            .expect("No members or cycle complete")
    }

    /// Member-initiated payout trigger
    /// Allows any member to trigger payout once the pool is fully funded
    pub fn trigger_payout(env: Env, caller: Address, recipient: Address) {
        caller.require_auth();

        // Verify caller is a member
        let members: Vec<Address> = env.storage().instance().get(&DataKey::Members).unwrap();
        if !members.contains(&caller) {
            panic!("Caller is not a member");
        }

        // Verify recipient is a member
        if !members.contains(&recipient) {
            panic!("Recipient is not a member");
        }

        // Verify recipient has not already received payout this cycle
        let has_received: bool = env.storage().persistent().get(&DataKey::HasReceivedPayout(recipient.clone())).unwrap_or(false);
        if has_received {
            panic!("Recipient has already received a payout this cycle");
        }

        // Check if pool is full (all members have contributed)
        let cycle_member_count: i128 = env.storage().instance()
            .get(&DataKey::CycleMemberCount)
            .expect("No active cycle");
        let current_cycle_contributions: i128 = env.storage().instance()
            .get(&DataKey::CurrentCycleContributions)
            .unwrap_or(0);
        
        if current_cycle_contributions < cycle_member_count {
            panic!("Pool is not full");
        }

        // Execute payout
        let contribution_amount: i128 = env.storage().instance().get(&DataKey::ContributionAmount).unwrap();
        let pool_size = contribution_amount * cycle_member_count;

        let token: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let token_client = token::Client::new(&env, &token);
        
        let contract_balance = token_client.balance(&env.current_contract_address());
        if pool_size > contract_balance {
            panic!("Insufficient funds in contract for full payout");
        }

        env.storage().persistent().set(&DataKey::HasReceivedPayout(recipient.clone()), &true);
        token_client.transfer(&env.current_contract_address(), &recipient, &pool_size);

        env.events().publish((symbol_short!("trg_pay"), recipient), pool_size);
    }

    /// Resets the payout cycle so members can receive payouts again.
    pub fn reset_cycle(env: Env) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        extend_instance_ttl(&env);

        let group_type: GroupType = env
            .storage()
            .instance()
            .get(&DataKey::GroupType)
            .unwrap_or(GroupType::Rotational);
        let members: Vec<Address> = env.storage().instance().get(&DataKey::Members).unwrap();

        for member in members.iter() {
            env.storage()
                .persistent()
                .set(&DataKey::HasContributedThisCycle(member.clone()), &false);
            env.storage().persistent().extend_ttl(
                &DataKey::HasContributedThisCycle(member.clone()),
                LEDGERS_TO_LIVE / 2,
                LEDGERS_TO_LIVE,
            );
        }

        // Clear the frozen member count so it is re-established at the next cycle's first contribution
        env.storage().instance().remove(&DataKey::CycleMemberCount);
        // Reset the current cycle contributions counter
        env.storage().instance().set(&DataKey::CurrentCycleContributions, &0i128);

        env.events().publish((symbol_short!("reset"),), ());
    }

    /// Resets the payout queue so the rotation starts from the first member again.
    /// Call this after all members have received their payout to begin a new rotation.
    pub fn reset_rotation(env: Env) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        extend_instance_ttl(&env);

        env.storage()
            .instance()
            .set(&DataKey::NextPayoutIndex, &0u32);

        env.events().publish((symbol_short!("new_rot"),), ());
    }

    /// Get contract balance
    pub fn get_balance(env: Env) -> i128 {
        extend_instance_ttl(&env);
        let token: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let token_client = token::Client::new(&env, &token);
        token_client.balance(&env.current_contract_address())
    }

    pub fn get_contribution(env: Env, member: Address) -> i128 {
        env.storage().persistent().extend_ttl(
            &DataKey::Contributions(member.clone()),
            LEDGERS_TO_LIVE / 2,
            LEDGERS_TO_LIVE,
        );
        env.storage()
            .persistent()
            .get(&DataKey::Contributions(member))
            .unwrap_or(0)
    }

    pub fn has_received_payout(env: Env, member: Address) -> bool {
        let members: Vec<Address> = env.storage().instance().get(&DataKey::Members).unwrap();
        let next_payout_index: u32 = env
            .storage()
            .instance()
            .get(&DataKey::NextPayoutIndex)
            .unwrap_or(0);
        match members.iter().position(|m| m == member) {
            Some(idx) => (idx as u32) < next_payout_index,
            None => false,
        }
    }
}
