use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Deps, Order, StdResult, Storage};

use crate::state::STAKE;

#[cw_serde]
pub struct StakeInfo {
    hotkey: Addr,
    coldkey: Addr,
    stake: u64,
}

fn _get_stake_info_for_coldkeys(
    store: &dyn Storage,
    coldkeys: Vec<Addr>,
) -> StdResult<Vec<(Addr, Vec<StakeInfo>)>> {
    if coldkeys.len() == 0 {
        return Ok(Vec::new()); // No coldkeys to check
    }

    let mut stake_info: Vec<(Addr, Vec<StakeInfo>)> = Vec::new();
    for coldkey_ in coldkeys {
        let mut stake_info_for_coldkey: Vec<StakeInfo> = Vec::new();

        for item in STAKE.range(store, None, None, Order::Ascending) {
            let ((hotkey, coldkey), stake) = item?;
            if coldkey == coldkey_ {
                stake_info_for_coldkey.push(StakeInfo {
                    hotkey,
                    coldkey,
                    stake,
                });
            }
        }

        stake_info.push((coldkey_, stake_info_for_coldkey));
    }

    Ok(stake_info)
}

pub fn get_stake_info_for_coldkeys(
    deps: Deps,
    coldkey_accounts: Vec<String>,
) -> StdResult<Vec<(Addr, Vec<StakeInfo>)>> {
    if coldkey_accounts.len() == 0 {
        return Ok(Vec::new()); // Invalid coldkey
    }

    let mut coldkeys: Vec<Addr> = Vec::new();
    for coldkey_account in coldkey_accounts {
        let coldkey = deps.api.addr_validate(&coldkey_account)?;
        coldkeys.push(coldkey);
    }

    let stake_info = _get_stake_info_for_coldkeys(deps.storage, coldkeys)?;

    Ok(stake_info)
}

pub fn get_stake_info_for_coldkey(
    deps: Deps,
    coldkey_account: String,
) -> StdResult<Vec<StakeInfo>> {
    let coldkey = deps.api.addr_validate(&coldkey_account)?;

    let stake_info = _get_stake_info_for_coldkeys(deps.storage, vec![coldkey])?;

    return if stake_info.len() == 0 {
        Ok(Vec::new()) // Invalid coldkey
    } else {
        Ok(stake_info.get(0).unwrap().1.clone())
    };
}
