use std::ops::Deref;

use cosmwasm_std::{
    coins, ensure, Addr, BankMsg, CosmosMsg, DepsMut, Env, MessageInfo, Order, StdResult, Storage,
    Uint128,
};
use cw_utils::must_pay;

use crate::state::{
    DELEGATES, DENOM, OWNER, STAKE, TOTAL_COLDKEY_STAKE, TOTAL_HOTKEY_STAKE, TOTAL_ISSUANCE,
    TOTAL_STAKE,
};
use crate::utils::{exceeds_tx_rate_limit, get_last_tx_block, set_last_tx_block};
use crate::ContractError;
use cyber_std::Response;

// ---- The implementation for the extrinsic become_delegate: signals that this hotkey allows delegated stake.
//
// # Args:
// 	* 'origin': (<T as frame_system::Config>RuntimeOrigin):
// 		- The signature of the caller's coldkey.
//
// 	* 'hotkey' (T::AccountId):
// 		- The hotkey we are delegating (must be owned by the coldkey.)
//
// 	* 'take' (u16):
// 		- The stake proportion that this hotkey takes from delegations.
//
// # Event:
// 	* DelegateAdded;
// 		- On successfully setting a hotkey as a delegate.
//
// # Raises:
// 	* 'NotRegistered':
// 		- The hotkey we are delegating is not registered on the network.
//
// 	* 'NonAssociatedColdKey':
// 		- The hotkey we are delegating is not owned by the calling coldkey.
//
// 	* 'TxRateLimitExceeded':
// 		- Thrown if key has hit transaction rate limit
//
pub fn do_become_delegate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    hotkey_address: String,
    // take: u16,
) -> Result<Response, ContractError> {
    // TODO set get_default_take() of custom take
    let take = 11_796;

    // --- 1. We check the coldkey signuture.
    let coldkey = info.sender;
    let hotkey = deps.api.addr_validate(&hotkey_address)?;

    deps.api.debug(&format!(
        "🌐 do_become_delegate ( coldkey:{:?} hotkey:{:?}, take:{:?} )",
        coldkey, hotkey, take
    ));

    // --- 2. Ensure we are delegating an known key.
    ensure!(
        hotkey_account_exists(deps.storage, &hotkey),
        ContractError::NotRegistered {}
    );

    // --- 3. Ensure that the coldkey is the owner.
    ensure!(
        coldkey_owns_hotkey(deps.storage, &coldkey, &hotkey),
        ContractError::NonAssociatedColdKey {}
    );

    // --- 4. Ensure we are not already a delegate (dont allow changing delegate take.)
    ensure!(
        !hotkey_is_delegate(deps.storage, &hotkey),
        ContractError::AlreadyDelegate {}
    );

    // --- 5. Ensure we don't exceed tx rate limit
    ensure!(
        !exceeds_tx_rate_limit(
            deps.storage,
            get_last_tx_block(deps.storage, &coldkey),
            env.block.height
        ),
        ContractError::TxRateLimitExceeded {}
    );

    // --- 6. Delegate the key.
    delegate_hotkey(deps.storage, &hotkey, take);

    // Set last block for rate limiting
    set_last_tx_block(deps.storage, &coldkey, env.block.height);

    // --- 7. Emit the staking event.
    deps.api.debug(&format!(
        "🌐 DelegateAdded( coldkey:{:?}, hotkey:{:?}, take:{:?} )",
        coldkey,
        hotkey.clone(),
        take
    ));

    // --- 8. Ok and return.
    Ok(Response::default()
        .add_attribute("action", "delegate_added")
        .add_attribute("hotkey", hotkey)
        .add_attribute("take", format!("{}", take)))
}

// ---- The implementation for the extrinsic add_stake: Adds stake to a hotkey account.
//
// # Args:
// 	* 'origin': (<T as frame_system::Config>RuntimeOrigin):
// 		- The signature of the caller's coldkey.
//
// 	* 'hotkey' (T::AccountId):
// 		- The associated hotkey account.
//
// 	* 'stake_to_be_added' (u64):
// 		- The amount of stake to be added to the hotkey staking account.
//
// # Event:
// 	* StakeAdded;
// 		- On the successfully adding stake to a global account.
//
// # Raises:
// 	* 'CouldNotConvertToBalance':
// 		- Unable to convert the passed stake value to a balance.
//
// 	* 'NotEnoughBalanceToStake':
// 		- Not enough balance on the coldkey to add onto the global account.
//
// 	* 'NonAssociatedColdKey':
// 		- The calling coldkey is not associated with this hotkey.
//
// 	* 'BalanceWithdrawalError':
// 		- Errors stemming from transaction pallet.
//
// 	* 'TxRateLimitExceeded':
// 		- Thrown if key has hit transaction rate limit
//
pub fn do_add_stake(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    hotkey_address: String,
) -> Result<Response, ContractError> {
    // --- 1. We check that the transaction is signed by the caller and retrieve the T::AccountId coldkey information.
    let coldkey = info.clone().sender;
    let hotkey = deps.api.addr_validate(&hotkey_address)?;

    let denom = DENOM.load(deps.storage)?;
    let stake_to_be_added =
        must_pay(&info, &denom).map_err(|_| ContractError::CouldNotConvertToBalance {})?;

    deps.api.debug(&format!(
        "🌐 do_add_stake ( coldkey:{:?}, hotkey:{:?}, stake_to_be_added:{:?} )",
        coldkey, hotkey, stake_to_be_added
    ));

    // --- 4. Ensure that the hotkey account exists this is only possible through registration.
    ensure!(
        hotkey_account_exists(deps.storage, &hotkey),
        ContractError::NotRegistered {}
    );

    // --- 5. Ensure that the hotkey allows delegation or that the hotkey is owned by the calling coldkey.
    ensure!(
        hotkey_is_delegate(deps.storage, &hotkey)
            || coldkey_owns_hotkey(deps.storage, &coldkey, &hotkey),
        ContractError::NonAssociatedColdKey {}
    );

    ensure!(
        !exceeds_tx_rate_limit(
            deps.storage,
            get_last_tx_block(deps.storage, &coldkey),
            env.block.height
        ),
        ContractError::TxRateLimitExceeded {}
    );

    // --- 8. If we reach here, add the balance to the hotkey.
    increase_stake_on_coldkey_hotkey_account(
        deps.storage,
        &coldkey,
        &hotkey,
        stake_to_be_added.u128() as u64,
    );

    // --- 9. Emit the staking event.
    deps.api.debug(&format!(
        "🌐 StakeAdded ( hotkey:{:?}, stake_to_be_added:{:?} )",
        hotkey.clone(),
        stake_to_be_added
    ));

    // --- 10. Ok and return.
    Ok(Response::default()
        .add_attribute("action", "stake_added")
        .add_attribute("hotkey", hotkey)
        .add_attribute("take", format!("{:?}", stake_to_be_added)))
}

// ---- The implementation for the extrinsic remove_stake: Removes stake from a hotkey account and adds it onto a coldkey.
//
// # Args:
// 	* 'origin': (<T as frame_system::Config>RuntimeOrigin):
// 		- The signature of the caller's coldkey.
//
// 	* 'hotkey' (T::AccountId):
// 		- The associated hotkey account.
//
// 	* 'stake_to_be_added' (u64):
// 		- The amount of stake to be added to the hotkey staking account.
//
// # Event:
// 	* StakeRemoved;
// 		- On the successfully removing stake from the hotkey account.
//
// # Raises:
// 	* 'NotRegistered':
// 		- Thrown if the account we are attempting to unstake from is non existent.
//
// 	* 'NonAssociatedColdKey':
// 		- Thrown if the coldkey does not own the hotkey we are unstaking from.
//
// 	* 'NotEnoughStaketoWithdraw':
// 		- Thrown if there is not enough stake on the hotkey to withdwraw this amount.
//
// 	* 'CouldNotConvertToBalance':
// 		- Thrown if we could not convert this amount to a balance.
//
// 	* 'TxRateLimitExceeded':
// 		- Thrown if key has hit transaction rate limit
//
//
pub fn do_remove_stake(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    hotkey_address: String,
    stake_to_be_removed: u64,
) -> Result<Response, ContractError> {
    // --- 1. We check the transaction is signed by the caller and retrieve the T::AccountId coldkey information.
    let coldkey = info.clone().sender;
    let hotkey = deps.api.addr_validate(&hotkey_address)?;

    deps.api.debug(&format!(
        "🌐 do_remove_stake ( coldkey:{:?}, hotkey:{:?}, stake_to_be_removed:{:?} )",
        coldkey, hotkey, stake_to_be_removed
    ));

    // --- 2. Ensure that the hotkey account exists this is only possible through registration.
    ensure!(
        hotkey_account_exists(deps.storage, &hotkey),
        ContractError::NotRegistered {}
    );

    // --- 3. Ensure that the hotkey allows delegation or that the hotkey is owned by the calling coldkey.
    ensure!(
        hotkey_is_delegate(deps.storage, &hotkey)
            || coldkey_owns_hotkey(deps.storage, &coldkey, &hotkey),
        ContractError::NonAssociatedColdKey {}
    );

    // --- Ensure that the stake amount to be removed is above zero.
    ensure!(
        stake_to_be_removed > 0,
        ContractError::NotEnoughStaketoWithdraw {}
    );

    // --- 4. Ensure that the hotkey has enough stake to withdraw.
    ensure!(
        has_enough_stake(deps.storage, &coldkey, &hotkey, stake_to_be_removed),
        ContractError::NotEnoughStaketoWithdraw {}
    );

    // --- 6. Ensure we don't exceed tx rate limit
    ensure!(
        !exceeds_tx_rate_limit(
            deps.storage,
            get_last_tx_block(deps.storage, &coldkey),
            env.block.height
        ),
        ContractError::TxRateLimitExceeded {}
    );

    // --- 7. We remove the balance from the hotkey.
    decrease_stake_on_coldkey_hotkey_account(deps.storage, &coldkey, &hotkey, stake_to_be_removed)?;

    // --- 8. We add the balance to the coldkey
    let denom = DENOM.load(deps.storage)?;
    let msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: coins(Uint128::from(stake_to_be_removed).u128(), denom),
    });

    // --- 9. Emit the unstaking event.
    deps.api.debug(&format!(
        "🌐 StakeRemoved ( hotkey:{:?}, stake_to_be_removed:{:?} )",
        hotkey, stake_to_be_removed
    ));

    // --- 10. Done and ok.
    Ok(Response::default()
        .add_message(msg)
        .add_attribute("action", "stake_removed")
        .add_attribute("hotkey", hotkey.clone())
        .add_attribute("stake_to_be_removed", format!("{}", stake_to_be_removed)))
}

// Returns true if the passed hotkey allow delegative staking.
//
pub fn hotkey_is_delegate(store: &dyn Storage, hotkey: &Addr) -> bool {
    DELEGATES.has(store, hotkey)
}

// Sets the hotkey as a delegate with take.
//
pub fn delegate_hotkey(store: &mut dyn Storage, hotkey: &Addr, take: u16) {
    DELEGATES.save(store, hotkey, &take).unwrap();
}

// Returns the total amount of stake in the staking table.
#[cfg(test)]
pub fn get_total_stake(store: &dyn Storage) -> u64 {
    return TOTAL_STAKE.load(store).unwrap();
}

// Increases the total amount of stake by the passed amount.
#[cfg(test)]
pub fn increase_total_stake(store: &mut dyn Storage, increment: u64) {
    TOTAL_STAKE
        .update(store, |s| -> StdResult<_> {
            Ok(s.saturating_add(increment))
        })
        .unwrap();
}

// Decreases the total amount of stake by the passed amount.
#[cfg(test)]
pub fn decrease_total_stake(store: &mut dyn Storage, decrement: u64) {
    TOTAL_STAKE
        .update(store, |s| -> StdResult<_> {
            Ok(s.saturating_sub(decrement))
        })
        .unwrap();
}

// Returns the total amount of stake under a hotkey (delegative or otherwise)
//
pub fn get_total_stake_for_hotkey(store: &dyn Storage, hotkey: &Addr) -> u64 {
    // TODO revisit and delete default
    return TOTAL_HOTKEY_STAKE.load(store, hotkey).unwrap();
}

// Returns the total amount of stake held by the coldkey (delegative or otherwise)
#[cfg(test)]
pub fn get_total_stake_for_coldkey(store: &dyn Storage, coldkey: &Addr) -> u64 {
    return TOTAL_COLDKEY_STAKE.load(store, coldkey).unwrap();
}

// Returns the stake under the cold - hot pairing in the staking table.
//
pub fn get_stake_for_coldkey_and_hotkey(store: &dyn Storage, coldkey: &Addr, hotkey: &Addr) -> u64 {
    // Added default, see delegate_info:125
    STAKE.load(store, (hotkey, coldkey)).unwrap_or_default()
}

// Creates a cold - hot pairing account if the hotkey is not already an active account.
//
pub fn create_account_if_non_existent(store: &mut dyn Storage, coldkey: &Addr, hotkey: &Addr) {
    if !hotkey_account_exists(store, hotkey) {
        STAKE.save(store, (hotkey, coldkey), &0).unwrap();
        OWNER.save(store, hotkey, coldkey).unwrap();
        TOTAL_HOTKEY_STAKE.save(store, hotkey, &0u64).unwrap();
        TOTAL_COLDKEY_STAKE.save(store, coldkey, &0u64).unwrap();
    }
}

// Returns the coldkey owning this hotkey. This function should only be called for active accounts.
//
pub fn get_owning_coldkey_for_hotkey(store: &dyn Storage, hotkey: &Addr) -> Addr {
    return OWNER.load(store, hotkey).unwrap();
}

pub fn hotkey_account_exists(store: &dyn Storage, hotkey: &Addr) -> bool {
    return OWNER.has(store, hotkey);
}

// Return true if the passed coldkey owns the hotkey.
//
pub fn coldkey_owns_hotkey(store: &dyn Storage, coldkey: &Addr, hotkey: &Addr) -> bool {
    if OWNER.has(store, hotkey) {
        return OWNER.load(store, hotkey).unwrap().eq(coldkey);
    } else {
        return false;
    }
}

// Returns true if the cold-hot staking account has enough balance to fufil the decrement.
//
pub fn has_enough_stake(
    store: &dyn Storage,
    coldkey: &Addr,
    hotkey: &Addr,
    decrement: u64,
) -> bool {
    return get_stake_for_coldkey_and_hotkey(store, coldkey, hotkey) >= decrement;
}

// Increases the stake on the hotkey account under its owning coldkey.
//
pub fn increase_stake_on_hotkey_account(store: &mut dyn Storage, hotkey: &Addr, increment: u64) {
    let coldkey = get_owning_coldkey_for_hotkey(store, hotkey);
    increase_stake_on_coldkey_hotkey_account(store, &coldkey, hotkey, increment);
}

// Decreases the stake on the hotkey account under its owning coldkey.
#[cfg(test)]
pub fn decrease_stake_on_hotkey_account(
    store: &mut dyn Storage,
    hotkey: &Addr,
    decrement: u64,
) -> Result<(), ContractError> {
    let coldkey = get_owning_coldkey_for_hotkey(store, hotkey);
    decrease_stake_on_coldkey_hotkey_account(store, &coldkey, &hotkey, decrement)?;

    Ok(())
}

// Increases the stake on the cold - hot pairing by increment while also incrementing other counters.
// This function should be called rather than set_stake under account.
//
pub fn increase_stake_on_coldkey_hotkey_account(
    store: &mut dyn Storage,
    coldkey: &Addr,
    hotkey: &Addr,
    increment: u64,
) {
    TOTAL_COLDKEY_STAKE
        .update(store, coldkey, |s| -> StdResult<_> {
            let stake = s.unwrap_or_default();
            Ok(stake.saturating_add(increment))
        })
        .unwrap();
    TOTAL_HOTKEY_STAKE
        .update(store, hotkey, |s| -> StdResult<_> {
            let stake = s.unwrap_or_default();
            Ok(stake.saturating_add(increment))
        })
        .unwrap();
    STAKE
        .update(store, (hotkey, coldkey), |s| -> StdResult<_> {
            let stake = s.unwrap_or_default();
            Ok(stake.saturating_add(increment))
        })
        .unwrap();
    TOTAL_STAKE
        .update(store, |s| -> StdResult<_> {
            Ok(s.saturating_add(increment))
        })
        .unwrap();
    TOTAL_ISSUANCE
        .update(store, |s| -> StdResult<_> {
            Ok(s.saturating_add(increment))
        })
        .unwrap();
}

// Decreases the stake on the cold - hot pairing by the decrement while decreasing other counters.
//
pub fn decrease_stake_on_coldkey_hotkey_account(
    store: &mut dyn Storage,
    coldkey: &Addr,
    hotkey: &Addr,
    decrement: u64,
) -> Result<(), ContractError> {
    TOTAL_COLDKEY_STAKE.update(store, coldkey, |s| -> StdResult<_> {
        let stake = s.unwrap();
        Ok(stake.saturating_sub(decrement))
    })?;
    TOTAL_HOTKEY_STAKE.update(store, hotkey, |s| -> StdResult<_> {
        let stake = s.unwrap();
        Ok(stake.saturating_sub(decrement))
    })?;
    STAKE.update(store, (hotkey, coldkey), |s| -> StdResult<_> {
        let stake = s.unwrap();
        Ok(stake.saturating_sub(decrement))
    })?;
    TOTAL_STAKE.update(store, |s| -> StdResult<_> {
        Ok(s.saturating_sub(decrement))
    })?;
    TOTAL_ISSUANCE.update(store, |s| -> StdResult<_> {
        Ok(s.saturating_sub(decrement))
    })?;

    Ok(())
}

#[cfg(test)]
pub fn add_balance_to_coldkey_account(_coldkey: &Addr, _amount: u64) {}

#[cfg(test)]
pub fn can_remove_balance_from_coldkey_account(_coldkey: &Addr, _amount: u64) -> bool {
    true
}

#[cfg(test)]
pub fn get_coldkey_balance(_coldkey: &Addr) -> u64 {
    return 0;
}

#[cfg(test)]
pub fn remove_balance_from_coldkey_account(_coldkey: &Addr, _amount: u64) -> bool {
    true
}

pub fn unstake_all_coldkeys_from_hotkey_account(
    store: &mut dyn Storage,
    hotkey: &Addr,
) -> Result<Vec<CosmosMsg>, ContractError> {
    // TODO can be issue when there are a lot of stakers (and messages) on account on replacement
    // Iterate through all coldkeys that have a stake on this hotkey account.
    let mut msgs: Vec<CosmosMsg> = Vec::new();

    let stakes = STAKE
        .prefix(hotkey)
        .range(store.deref(), None, None, Order::Ascending)
        .map(|item| {
            let i = item.unwrap();
            (i.0, i.1)
        })
        .collect::<Vec<(Addr, u64)>>();

    for (delegate_coldkey_i, stake_i) in stakes {
        if stake_i == 0 {
            continue;
        } else {
            // Stake is successfully converted to balance.

            // Remove the stake from the coldkey - hotkey pairing.
            decrease_stake_on_coldkey_hotkey_account(store, &delegate_coldkey_i, &hotkey, stake_i)?;

            let denom = DENOM.load(store)?;
            msgs.push(CosmosMsg::Bank(BankMsg::Send {
                to_address: delegate_coldkey_i.to_string(),
                amount: coins(Uint128::from(stake_i).u128(), denom),
            }));
        }
    }

    Ok(msgs)
}
