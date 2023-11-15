use substrate_fixed::types::{U64F64};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Deps, Order, StdResult, Storage};
use crate::staking::{get_total_stake_for_hotkey};
use crate::state::{DELEGATES, OWNER, STAKE, TEMPO};
use crate::uids::{get_registered_networks_for_hotkey, get_uid_for_net_and_hotkey};
use crate::utils::{get_emission_for_uid, get_validator_permit_for_uid};
extern crate alloc;
use alloc::vec::Vec;


#[cw_serde]
pub struct DelegateInfo {
    delegate_ss58: Addr,
    take: u16,
    nominators: Vec<(Addr, u64)>,
    // map of nominator_ss58 to stake amount
    owner_ss58: Addr,
    registrations: Vec<u16>,
    // Vec of netuid this delegate is registered on
    validator_permits: Vec<u16>,
    // Vec of netuid this delegate has validator permit on
    return_per_1000: u64,
    // Delegators current daily return per 1000 TAO staked minus take fee
    total_daily_return: u64, // Delegators current daily return
}

pub fn get_delegate_by_existing_account(
    store: &dyn Storage,
    delegate: Addr
) -> DelegateInfo {
    let mut nominators = Vec::<(Addr, u64)>::new();

    for item in STAKE
        .prefix(delegate.clone())
        .range(store, None, None, Order::Ascending)
    {
        let (nominator, stake) = item.unwrap();
        if stake == 0 { continue; }
        // Only add nominators with stake
        nominators.push((nominator.clone(), stake.into()));
    }

    let registrations = get_registered_networks_for_hotkey(store, delegate.clone());
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
            let tempo = U64F64::from_num(TEMPO.load(store, *netuid).unwrap());
            let epochs_per_day = U64F64::from_num(7200) / tempo;
            emissions_per_day += emission * epochs_per_day;
        }
    }

    let owner = OWNER.load(store, delegate.clone()).unwrap();
    let take: u16 = DELEGATES.load(store, delegate.clone()).unwrap();

    let total_stake = U64F64::from_num(get_total_stake_for_hotkey(store, delegate.clone()));

    let mut return_per_1000 = U64F64::from_num(0);

    if total_stake > U64F64::from_num(0) {
        return_per_1000 = (emissions_per_day * U64F64::from_num(0.82)) / (total_stake / U64F64::from_num(1000));
    }

    return DelegateInfo {
        delegate_ss58: delegate.clone(),
        take,
        nominators,
        owner_ss58: owner.clone(),
        registrations: registrations.iter().map(|x| *x).collect(),
        validator_permits,
        return_per_1000: U64F64::to_num::<u64>(return_per_1000).into(),
        total_daily_return: U64F64::to_num::<u64>(emissions_per_day).into(),
    };
}


pub fn get_delegate(deps: Deps, delegate: Addr) -> StdResult<Option<DelegateInfo>> {
    if !DELEGATES.has(deps.storage,delegate.clone()) {
        return Ok(None);
    }

    let delegate_info = get_delegate_by_existing_account(deps.storage,delegate.clone());
    return Ok(Some(delegate_info));
}

pub fn get_delegates(deps: Deps) -> StdResult<Vec<DelegateInfo>> {
    let mut delegates = Vec::<DelegateInfo>::new();
    for item in DELEGATES.range(deps.storage, None, None, Order::Ascending).into_iter() {
        let (delegate, _) = item?;
        let delegate_info = get_delegate_by_existing_account(deps.storage,delegate.clone());
        delegates.push(delegate_info);
    }

    return Ok(delegates);
}

pub fn get_delegated(deps: Deps, delegatee: Addr) -> StdResult<Vec<(DelegateInfo, u64)>> {
    let mut delegates: Vec<(DelegateInfo, u64)> = Vec::new();
    for item in DELEGATES.range(deps.storage, None, None, Order::Ascending).into_iter() {
        let (delegate, _) = item?;
        let staked_to_this_delegatee = STAKE.load(deps.storage, (delegatee.clone(), delegate.clone()))?;
        if staked_to_this_delegatee == 0 {
            continue; // No stake to this delegate
        }
        // Staked to this delegate, so add to list
        let delegate_info = get_delegate_by_existing_account(deps.storage,delegate.clone());
        delegates.push((delegate_info, staked_to_this_delegatee.into()));
    }

    return Ok(delegates);
}


