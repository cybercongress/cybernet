use crate::staking::{
    get_owning_coldkey_for_hotkey, get_stake_for_coldkey_and_hotkey, get_total_stake_for_hotkey,
};
use crate::state::{DELEGATES, DENOM, STAKE};
use crate::uids::{get_registered_networks_for_hotkey, get_uid_for_net_and_hotkey};
use crate::utils::{get_emission_for_uid, get_tempo, get_validator_permit_for_uid};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Deps, Order, StdResult, Storage};
use substrate_fixed::types::U64F64;
extern crate alloc;
use alloc::vec::Vec;

#[cw_serde]
pub struct DelegateInfo {
    delegate: Addr,
    take: u16,
    nominators: Vec<(Addr, u64)>,
    // map of nominator to stake amount
    owner: Addr,
    registrations: Vec<u16>,
    // Vec of netuid this delegate is registered on
    validator_permits: Vec<u16>,
    // Vec of netuid this delegate has validator permit on
    return_per_giga: Coin,
    // Delegators current daily return per X tokens staked minus take fee
    total_daily_return: Coin, // Delegators current daily return
}

pub fn get_delegate_by_existing_account(store: &dyn Storage, delegate: &Addr) -> DelegateInfo {
    let mut nominators = Vec::<(Addr, u64)>::new();

    for item in STAKE
        .prefix(&delegate)
        .range(store, None, None, Order::Ascending)
    {
        let (nominator, stake) = item.unwrap();
        if stake == 0 {
            continue;
        }
        // Only add nominators with stake
        nominators.push((nominator.clone(), stake.into()));
    }

    let registrations = get_registered_networks_for_hotkey(store, &delegate);
    let mut validator_permits = Vec::<u16>::new();
    let mut emissions_per_day: U64F64 = U64F64::from_num(0);

    for netuid in registrations.iter() {
        let _uid = get_uid_for_net_and_hotkey(store, *netuid, &delegate.clone());
        if !_uid.is_ok() {
            continue; // this should never happen
        } else {
            let uid = _uid.expect("Delegate's UID should be ok");
            let validator_permit = get_validator_permit_for_uid(store, *netuid, uid);
            if validator_permit {
                validator_permits.push((*netuid).into());
            }

            let emission = U64F64::from_num(get_emission_for_uid(store, *netuid, uid));
            let tempo = U64F64::from_num(get_tempo(store, *netuid));
            let epochs_per_day = U64F64::from_num(14400) / tempo;
            emissions_per_day += emission * epochs_per_day;
        }
    }

    let owner = get_owning_coldkey_for_hotkey(store, &delegate);
    let take: u16 = DELEGATES.load(store, &delegate).unwrap();

    let total_stake = U64F64::from_num(get_total_stake_for_hotkey(store, &delegate));

    let mut return_per_giga = U64F64::from_num(0);

    if total_stake > U64F64::from_num(0) {
        // TODO rewrite this to Decimal and load take from store
        return_per_giga =
            (emissions_per_day * U64F64::from_num(0.8)) / (total_stake / U64F64::from_num(1000000000));
    }

    let denom = DENOM.load(store).unwrap();

    return DelegateInfo {
        delegate: delegate.clone(),
        take,
        nominators,
        owner: owner.clone(),
        registrations: registrations.iter().map(|x| *x).collect(),
        validator_permits,
        return_per_giga: Coin::new(U64F64::to_num::<u128>(return_per_giga), denom.clone()),
        total_daily_return: Coin::new(U64F64::to_num::<u128>(emissions_per_day), denom),
    };
}

pub fn get_delegate(deps: Deps, delegate: String) -> StdResult<Option<DelegateInfo>> {
    let delegate = deps.api.addr_validate(&delegate)?;
    if !DELEGATES.has(deps.storage, &delegate) {
        return Ok(None);
    }

    let delegate_info = get_delegate_by_existing_account(deps.storage, &delegate);
    return Ok(Some(delegate_info));
}

// TODO add pagination and limit
pub fn get_delegates(deps: Deps) -> StdResult<Vec<DelegateInfo>> {
    let mut delegates = Vec::<DelegateInfo>::new();
    for item in DELEGATES
        .range(deps.storage, None, None, Order::Ascending)
        .into_iter()
    {
        let (delegate, _) = item?;
        let delegate_info = get_delegate_by_existing_account(deps.storage, &delegate);
        delegates.push(delegate_info);
    }

    return Ok(delegates);
}

// TODO add pagination and limit
pub fn get_delegated(deps: Deps, delegatee: String) -> StdResult<Vec<(DelegateInfo, u64)>> {
    let delegatee = deps.api.addr_validate(&delegatee)?;
    let mut delegates: Vec<(DelegateInfo, u64)> = Vec::new();
    // TODO iterator over all delegates?
    for item in DELEGATES
        .range(deps.storage, None, None, Order::Ascending)
        .into_iter()
    {
        let (delegate, _) = item?;
        let staked_to_this_delegatee =
            get_stake_for_coldkey_and_hotkey(deps.storage, &delegatee, &delegate);
        if staked_to_this_delegatee == 0 {
            continue; // No stake to this delegate
        }
        // Staked to this delegate, so add to list
        let delegate_info = get_delegate_by_existing_account(deps.storage, &delegate);
        delegates.push((delegate_info, staked_to_this_delegatee.into()));
    }

    return Ok(delegates);
}
