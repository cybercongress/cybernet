use crate::epoch::epoch;
use crate::root::{get_root_netuid, root_epoch};
use crate::staking::{
    add_balance_to_coldkey_account, hotkey_is_delegate, increase_stake_on_coldkey_hotkey_account,
    increase_stake_on_hotkey_account, u64_to_balance,
};
use crate::state::{
    ADJUSTMENTS_ALPHA, ADJUSTMENT_INTERVAL, BLOCKS_SINCE_LAST_STEP, BURN,
    BURN_REGISTRATIONS_THIS_INTERVAL, DELEGATES, DIFFICULTY, EMISSION_VALUES,
    LAST_ADJUSTMENT_BLOCK, LAST_MECHANISM_STEP_BLOCK, LOADED_EMISSION, MAX_BURN, MAX_DIFFICULTY,
    MIN_BURN, MIN_DIFFICULTY, NETWORKS_ADDED, PENDING_EMISSION, POW_REGISTRATIONS_THIS_INTERVAL,
    REGISTRATIONS_THIS_BLOCK, REGISTRATIONS_THIS_INTERVAL, STAKE, SUBNET_OWNER, SUBNET_OWNER_CUT,
    TARGET_REGISTRATIONS_PER_INTERVAL, TEMPO, TOTAL_COLDKEY_STAKE, TOTAL_HOTKEY_STAKE,
    TOTAL_ISSUANCE, TOTAL_STAKE,
};
use crate::utils::get_blocks_since_last_step;
use crate::ContractError;
use cosmwasm_std::{Addr, Api, DepsMut, Env, Order, Response, StdResult, Storage};
use std::ops::Add;
use substrate_fixed::types::I110F18;
use substrate_fixed::types::I64F64;
use substrate_fixed::types::I96F32;

/// Executes the necessary operations for each block.
/// TODO make it msg and then do call from native layer as sudo
pub fn block_step(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    let block_number: u64 = env.block.height;
    deps.api
        .debug(&format!("block_step for block: {:?} ", block_number));
    // --- 1. Adjust difficulties.
    adjust_registration_terms_for_networks(deps.storage, deps.api, env.block.height)?;
    // --- 2. Calculate per-subnet emissions
    match root_epoch(deps.storage, deps.api, block_number) {
        Ok(_) => (),
        Err(e) => {
            // return Err(ContractError::Std(GenericErr {msg: format!("Error while running root epoch: {:?}", e)}))
            deps.api
                .debug(&format!("Error while running root epoch: {:?}", e));
        }
    }
    // --- 3. Drains emission tuples ( hotkey, amount ).
    drain_emission(deps.storage, deps.api, block_number)?;
    // --- 4. Generates emission tuples from epoch functions.
    generate_emission(deps.storage, deps.api, block_number)?;
    // Return ok.
    Ok(Response::default().add_attribute("action", "block_step"))
}

// Helper function which returns the number of blocks remaining before we will run the epoch on this
// network. Networks run their epoch when (block_number + netuid + 1 ) % (tempo + 1) = 0
//
pub fn blocks_until_next_epoch(netuid: u16, tempo: u16, block_number: u64) -> u64 {
    // tempo | netuid | # first epoch block
    //   1        0               0
    //   1        1               1
    //   2        0               1
    //   2        1               0
    //   100      0              99
    //   100      1              98
    // Special case: tempo = 0, the network never runs.
    if tempo == 0 {
        return 1000;
    }

    //TODO revisit this
    let blocks_until = tempo as u64 - (block_number + netuid as u64) % (tempo as u64 + 1);

    // let blocks_until = (block_number + netuid as u64) % (tempo as u64 + 1);
    // println!("until {:?} netuid {:?} tempo {:?} block {:?}", netuid, tempo, block_number, blocks_until);
    // return tempo as u64 - (block_number + netuid as u64 + 1) % (tempo as u64 + 1);

    blocks_until
}

// Helper function returns the number of tuples to drain on a particular step based on
// the remaining tuples to sink and the block number
//
pub fn tuples_to_drain_this_block(
    netuid: u16,
    tempo: u16,
    block_number: u64,
    n_remaining: usize,
) -> usize {
    let blocks_until_epoch: u64 = blocks_until_next_epoch(netuid, tempo, block_number);
    if blocks_until_epoch / 2 == 0 {
        return n_remaining;
    } // drain all.
    if tempo / 2 == 0 {
        return n_remaining;
    } // drain all
    if n_remaining == 0 {
        return 0;
    } // nothing to drain at all.
      // Else return enough tuples to drain all within half the epoch length.
    let to_sink_via_tempo: usize = n_remaining / (tempo as usize / 2);
    let to_sink_via_blocks_until_epoch: usize = n_remaining / (blocks_until_epoch as usize / 2);
    if to_sink_via_tempo > to_sink_via_blocks_until_epoch {
        return to_sink_via_tempo;
    } else {
        return to_sink_via_blocks_until_epoch;
    }
}

// Iterates through networks queues more emission onto their pending storage.
// If a network has no blocks left until tempo, we run the epoch function and generate
// more token emission tuples for later draining onto accounts.
//
pub fn generate_emission(
    store: &mut dyn Storage,
    api: &dyn Api,
    block_number: u64,
) -> Result<(), ContractError> {
    // --- 1. Iterate across each network and add pending emission into stash.
    let netuid_tempo: Vec<(u16, u16)> = TEMPO
        .range(store, None, None, Order::Ascending)
        .map(|item| {
            let i = item.unwrap();
            (i.0, i.1)
        })
        .collect::<Vec<(u16, u16)>>();

    for (netuid, tempo) in netuid_tempo {
        // Skip the root network.
        if netuid == get_root_netuid() {
            // Root emission is burned.
            continue;
        }

        // --- 2. Queue the emission due to this network.
        let new_queued_emission = EMISSION_VALUES.load(store, netuid)?;
        api.debug(&format!(
            "generate_emission for netuid: {:?} with tempo: {:?} and emission: {:?}",
            netuid, tempo, new_queued_emission,
        ));

        let subnet_has_owner = SUBNET_OWNER.has(store, netuid);
        let mut remaining = I96F32::from_num(new_queued_emission);
        if subnet_has_owner {
            let subnet_owner_cut = SUBNET_OWNER_CUT.load(store)?;
            let cut = remaining
                .saturating_mul(I96F32::from_num(subnet_owner_cut))
                .saturating_div(I96F32::from_num(u16::MAX));

            remaining = remaining.saturating_sub(cut);

            let subnet_owner = SUBNET_OWNER.load(store, netuid)?;
            // TODO create messages here
            // add_balance_to_coldkey_account(
            //     &subnet_owner,
            //     u64_to_balance(cut.to_num::<u64>()).unwrap(),
            // );
            // let denom = DENOM.load(deps.storage)?;
            // let msg = CosmosMsg::Bank(BankMsg::Send {
            //     to_address: &subnet_owner.to_string(),
            //     amount: coins(Uint128::from(cut.to_num::<u64>()).u128(), denom),
            // });

            TOTAL_ISSUANCE.update(store, |a| -> StdResult<_> {
                Ok(a.saturating_add(cut.to_num::<u64>()))
            })?;
        }
        // --- 5. Add remaining amount to the network's pending emission.
        PENDING_EMISSION.update(store, netuid, |queued| -> StdResult<_> {
            let mut q = queued.unwrap();
            q += remaining.to_num::<u64>();
            Ok(q)
        })?;
        api.debug(&format!(
            "netuid_i: {:?} queued_emission: +{:?} ",
            netuid, new_queued_emission
        ));

        // --- 6. Check to see if this network has reached tempo.
        if blocks_until_next_epoch(netuid, tempo, block_number) != 0 {
            // --- 3.1 No epoch, increase blocks since last step and continue,
            // make update here
            let block_since = get_blocks_since_last_step(store, netuid);
            BLOCKS_SINCE_LAST_STEP.save(store, netuid, &(block_since + 1))?;
            continue;
        }

        // --- 7 This network is at tempo and we are running its epoch.
        // First drain the queued emission.
        let emission_to_drain: u64 = PENDING_EMISSION.load(store, netuid)?;
        PENDING_EMISSION.save(store, netuid, &0)?;

        // --- 8. Run the epoch mechanism and return emission tuples for hotkeys in the network.
        let emission_tuples_this_block: Vec<(Addr, u64, u64)> =
            epoch(store, api, netuid, emission_to_drain, block_number)?;
        api.debug(&format!(
            "netuid_i: {:?} emission_to_drain: {:?} ",
            netuid, emission_to_drain
        ));

        // --- 9. Check that the emission does not exceed the allowed total.
        let emission_sum: u128 = emission_tuples_this_block
            .iter()
            .map(|(_account_id, ve, se)| *ve as u128 + *se as u128)
            .sum();
        if emission_sum > emission_to_drain as u128 {
            continue;
        } // Saftey check.

        // --- 10. Sink the emission tuples onto the already loaded.
        let mut concat_emission_tuples: Vec<(Addr, u64, u64)> = emission_tuples_this_block.clone();
        if LOADED_EMISSION.has(store, netuid) {
            // 10.a We already have loaded emission tuples, so we concat the new ones.
            let mut current_emission_tuples: Vec<(Addr, u64, u64)> =
                LOADED_EMISSION.load(store, netuid)?;
            concat_emission_tuples.append(&mut current_emission_tuples);
        }
        LOADED_EMISSION.save(store, netuid, &concat_emission_tuples)?;

        // --- 11 Set counters.
        BLOCKS_SINCE_LAST_STEP.save(store, netuid, &0)?;
        LAST_MECHANISM_STEP_BLOCK.save(store, netuid, &block_number)?;
    }
    Ok(())
}

pub fn has_loaded_emission_tuples(store: &dyn Storage, netuid: u16) -> bool {
    LOADED_EMISSION.has(store, netuid)
}
pub fn get_loaded_emission_tuples(store: &dyn Storage, netuid: u16) -> Vec<(Addr, u64, u64)> {
    LOADED_EMISSION.load(store, netuid).unwrap()
}

// Reads from the loaded emission storage which contains lists of pending emission tuples ( hotkey, amount )
// and distributes small chunks of them at a time.
//
pub fn drain_emission(store: &mut dyn Storage, api: &dyn Api, _: u64) -> Result<(), ContractError> {
    // --- 1. We iterate across each network.
    let netuid_tempo: Vec<(u16, u16)> = TEMPO
        .range(store, None, None, Order::Ascending)
        .map(|item| {
            let i = item.unwrap();
            (i.0, i.1)
        })
        .collect::<Vec<(u16, u16)>>();

    for (netuid, _) in netuid_tempo {
        if !LOADED_EMISSION.has(store, netuid) {
            continue;
        } // There are no tuples to emit.
        let tuples_to_drain: Vec<(Addr, u64, u64)> = LOADED_EMISSION.load(store, netuid)?;
        let mut total_emitted: u64 = 0;
        for (hotkey, server_amount, validator_amount) in tuples_to_drain.iter() {
            emit_inflation_through_hotkey_account(
                store,
                api,
                &hotkey,
                *server_amount,
                *validator_amount,
            )?;
            total_emitted += *server_amount + *validator_amount;
        }
        LOADED_EMISSION.remove(store, netuid);
        TOTAL_ISSUANCE.update(store, |a| -> StdResult<_> {
            Ok(a.saturating_add(total_emitted))
        })?;
    }
    Ok(())
}

// Distributes token inflation through the hotkey based on emission. The call ensures that the inflation
// is distributed onto the accounts in proportion of the stake delegated minus the take. This function
// is called after an epoch to distribute the newly minted stake according to delegation.
//
pub fn emit_inflation_through_hotkey_account(
    store: &mut dyn Storage,
    api: &dyn Api,
    hotkey: &Addr,
    server_emission: u64,
    validator_emission: u64,
) -> Result<(), ContractError> {
    // --- 1. Check if the hotkey is a delegate. If not, we simply pass the stake through to the
    // coldkey - hotkey account as normal.
    if !hotkey_is_delegate(store, &hotkey) {
        increase_stake_on_hotkey_account(store, &hotkey, server_emission + validator_emission);
        return Ok(());
    }
    // Then this is a delegate, we distribute validator_emission, then server_emission.

    // --- 2. The hotkey is a delegate. We first distribute a proportion of the validator_emission to the hotkey
    // directly as a function of its 'take'
    let total_hotkey_stake: u64 = TOTAL_HOTKEY_STAKE.load(store, &hotkey)?;
    let delegate_take: u64 =
        calculate_delegate_proportional_take(store, &hotkey, validator_emission);
    let validator_emission_minus_take: u64 = validator_emission - delegate_take;
    let mut remaining_validator_emission: u64 = validator_emission_minus_take;

    // 3. -- The remaining emission goes to the owners in proportion to the stake delegated.

    let stake: Vec<(Addr, u64)> = STAKE
        .prefix(&hotkey)
        .range(store, None, None, Order::Ascending)
        .map(|item| {
            let i = item.unwrap();
            (i.0, i.1)
        })
        .collect::<Vec<(Addr, u64)>>();

    for (owning_coldkey_i, stake_i) in stake {
        // --- 4. The emission proportion is remaining_emission * ( stake / total_stake ).
        let stake_proportion: u64 = calculate_stake_proportional_emission(
            stake_i,
            total_hotkey_stake,
            validator_emission_minus_take,
        );
        increase_stake_on_coldkey_hotkey_account(
            store,
            &owning_coldkey_i,
            &hotkey,
            stake_proportion,
        );
        api.debug(&format!(
            "owning_coldkey_i: {:?} hotkey: {:?} emission: +{:?} ",
            owning_coldkey_i,
            hotkey.clone(),
            stake_proportion
        ));
        remaining_validator_emission -= stake_proportion;
    }

    // --- 5. Last increase final account balance of delegate after 4, since 5 will change the stake proportion of
    // the delegate and effect calculation in 4.
    increase_stake_on_hotkey_account(store, &hotkey, delegate_take + remaining_validator_emission);
    api.debug(&format!(
        "delkey: {:?} delegate_take: +{:?} ",
        hotkey, delegate_take
    ));
    // Also emit the server_emission to the hotkey
    // The server emission is distributed in-full to the delegate owner.
    // We do this after 4. for the same reason as above.
    increase_stake_on_hotkey_account(store, &hotkey, server_emission);

    Ok(())
}

// Increases the stake on the cold - hot pairing by increment while also incrementing other counters.
// This function should be called rather than set_stake under account.
//
// TODO revisit
pub fn block_step_increase_stake_on_coldkey_hotkey_account(
    store: &mut dyn Storage,
    coldkey: &Addr,
    hotkey: &Addr,
    increment: u64,
) -> StdResult<()> {
    TOTAL_COLDKEY_STAKE.update(store, coldkey, |s| -> StdResult<_> {
        Ok(s.unwrap_or_default().saturating_add(increment))
    })?;
    TOTAL_HOTKEY_STAKE.update(store, hotkey, |s| -> StdResult<_> {
        Ok(s.unwrap_or_default().saturating_add(increment))
    })?;
    STAKE.update(store, (hotkey, coldkey), |s| -> StdResult<_> {
        Ok(s.unwrap_or_default().saturating_add(increment))
    })?;
    TOTAL_STAKE.update(store, |a| -> StdResult<_> {
        Ok(a.saturating_add(increment))
    })?;
    TOTAL_ISSUANCE.update(store, |a| -> StdResult<_> {
        Ok(a.saturating_add(increment))
    })?;
    Ok(())
}

// Decreases the stake on the cold - hot pairing by the decrement while decreasing other counters.
//
// TODO revisit
pub fn block_step_decrease_stake_on_coldkey_hotkey_account(
    store: &mut dyn Storage,
    coldkey: &Addr,
    hotkey: &Addr,
    decrement: u64,
) -> StdResult<()> {
    TOTAL_COLDKEY_STAKE.update(store, coldkey, |s| -> StdResult<_> {
        Ok(s.unwrap().saturating_sub(decrement))
    })?;
    TOTAL_HOTKEY_STAKE.update(store, hotkey, |s| -> StdResult<_> {
        Ok(s.unwrap().saturating_sub(decrement))
    })?;
    STAKE.update(store, (hotkey, coldkey), |s| -> StdResult<_> {
        Ok(s.unwrap().saturating_sub(decrement))
    })?;
    TOTAL_STAKE.update(store, |a| -> StdResult<_> {
        Ok(a.saturating_sub(decrement))
    })?;
    TOTAL_ISSUANCE.update(store, |a| -> StdResult<_> {
        Ok(a.saturating_sub(decrement))
    })?;

    Ok(())
}

// Returns emission awarded to a hotkey as a function of its proportion of the total stake.
//
pub fn calculate_stake_proportional_emission(stake: u64, total_stake: u64, emission: u64) -> u64 {
    if total_stake == 0 {
        return 0;
    };
    let stake_proportion = I64F64::from_num(stake) / I64F64::from_num(total_stake);
    let proportional_emission = I64F64::from_num(emission) * stake_proportion;
    return proportional_emission.to_num::<u64>();
}

// Returns the delegated stake 'take' assigned to this key. (If exists, otherwise 0)
//
pub fn calculate_delegate_proportional_take(
    store: &dyn Storage,
    hotkey: &Addr,
    emission: u64,
) -> u64 {
    if hotkey_is_delegate(store, hotkey) {
        let take_proportion =
            I64F64::from_num(DELEGATES.load(store, hotkey).unwrap()) / I64F64::from_num(u16::MAX);
        let take_emission: I64F64 = take_proportion * I64F64::from_num(emission);
        return take_emission.to_num::<u64>();
    } else {
        return 0;
    }
}

// Adjusts the network difficulties/burns of every active network. Resetting state parameters.
//
pub fn adjust_registration_terms_for_networks(
    store: &mut dyn Storage,
    api: &dyn Api,
    current_block: u64,
) -> Result<(), ContractError> {
    api.debug(&format!("adjust_registration_terms_for_networks"));

    let networks_added: Vec<(u16, bool)> = NETWORKS_ADDED
        .range(store, None, None, Order::Ascending)
        .map(|item| {
            let i = item.unwrap();
            (i.0, i.1)
        })
        .collect::<Vec<(u16, bool)>>();

    // --- 1. Iterate through each network.
    for (netuid, _) in networks_added {
        // --- 2. Pull counters for network difficulty.
        let last_adjustment_block: u64 = LAST_ADJUSTMENT_BLOCK.load(store, netuid)?;
        let adjustment_interval: u16 = ADJUSTMENT_INTERVAL.load(store, netuid)?;
        api.debug(&format!("netuid: {:?} last_adjustment_block: {:?} adjustment_interval: {:?} current_block: {:?}",
                           netuid,
                           last_adjustment_block,
                           adjustment_interval,
                           current_block
        ));

        // --- 3. Check if we are at the adjustment interval for this network.
        // If so, we need to adjust the registration difficulty based on target and actual registrations.
        if (current_block - last_adjustment_block) >= adjustment_interval as u64 {
            api.debug(&format!("interval reached."));

            // --- 4. Get the current counters for this network w.r.t burn and difficulty values.
            let current_burn: u64 = BURN.load(store, netuid)?;
            let current_difficulty: u64 = DIFFICULTY.load(store, netuid)?;
            let registrations_this_interval: u16 =
                REGISTRATIONS_THIS_INTERVAL.load(store, netuid)?;
            let pow_registrations_this_interval: u16 =
                POW_REGISTRATIONS_THIS_INTERVAL.load(store, netuid)?;
            let burn_registrations_this_interval: u16 =
                BURN_REGISTRATIONS_THIS_INTERVAL.load(store, netuid)?;
            let target_registrations_this_interval: u16 =
                TARGET_REGISTRATIONS_PER_INTERVAL.load(store, netuid)?;

            // --- 5. Adjust burn + pow
            // There are six cases to consider. A, B, C, D, E, F
            if registrations_this_interval > target_registrations_this_interval {
                if pow_registrations_this_interval > burn_registrations_this_interval {
                    // A. There are too many registrations this interval and most of them are pow registrations
                    // this triggers an increase in the pow difficulty.
                    // pow_difficulty ++
                    let difficulty = adjust_difficulty(
                        store,
                        netuid,
                        current_difficulty,
                        registrations_this_interval,
                        target_registrations_this_interval,
                    )?;
                    DIFFICULTY.save(store, netuid, &difficulty)?;
                } else if pow_registrations_this_interval < burn_registrations_this_interval {
                    // B. There are too many registrations this interval and most of them are burn registrations
                    // this triggers an increase in the burn cost.
                    // burn_cost ++
                    let burn = adjust_burn(
                        store,
                        netuid,
                        current_burn,
                        registrations_this_interval,
                        target_registrations_this_interval,
                    )?;
                    BURN.save(store, netuid, &burn)?;
                } else {
                    // F. There are too many registrations this interval and the pow and burn registrations are equal
                    // this triggers an increase in the burn cost and pow difficulty
                    // burn_cost ++
                    let burn = adjust_burn(
                        store,
                        netuid,
                        current_burn,
                        registrations_this_interval,
                        target_registrations_this_interval,
                    )?;
                    BURN.save(store, netuid, &burn)?;
                    // pow_difficulty ++
                    let difficulty = adjust_difficulty(
                        store,
                        netuid,
                        current_difficulty,
                        registrations_this_interval,
                        target_registrations_this_interval,
                    )?;
                    DIFFICULTY.save(store, netuid, &difficulty)?;
                }
            } else {
                // Not enough registrations this interval.
                if pow_registrations_this_interval > burn_registrations_this_interval {
                    // C. There are not enough registrations this interval and most of them are pow registrations
                    // this triggers a decrease in the burn cost
                    // burn_cost --
                    let burn = adjust_burn(
                        store,
                        netuid,
                        current_burn,
                        registrations_this_interval,
                        target_registrations_this_interval,
                    )?;
                    BURN.save(store, netuid, &burn)?;
                } else if pow_registrations_this_interval < burn_registrations_this_interval {
                    // D. There are not enough registrations this interval and most of them are burn registrations
                    // this triggers a decrease in the pow difficulty
                    // pow_difficulty --
                    let diffuculty = adjust_difficulty(
                        store,
                        netuid,
                        current_difficulty,
                        registrations_this_interval,
                        target_registrations_this_interval,
                    )?;
                    DIFFICULTY.save(store, netuid, &diffuculty)?;
                } else {
                    // E. There are not enough registrations this interval and the pow and burn registrations are equal
                    // this triggers a decrease in the burn cost and pow difficulty
                    // burn_cost --
                    let burn = adjust_burn(
                        store,
                        netuid,
                        current_burn,
                        registrations_this_interval,
                        target_registrations_this_interval,
                    )?;
                    BURN.save(store, netuid, &burn)?;
                    // pow_difficulty --
                    let difficulty = adjust_difficulty(
                        store,
                        netuid,
                        current_difficulty,
                        registrations_this_interval,
                        target_registrations_this_interval,
                    )?;
                    DIFFICULTY.save(store, netuid, &difficulty)?;
                }
            }

            // --- 6. Drain all counters for this network for this interval.
            LAST_ADJUSTMENT_BLOCK.save(store, netuid, &current_block)?;
            REGISTRATIONS_THIS_INTERVAL.save(store, netuid, &0)?;
            POW_REGISTRATIONS_THIS_INTERVAL.save(store, netuid, &0)?;
            BURN_REGISTRATIONS_THIS_INTERVAL.save(store, netuid, &0)?;
        } else {
            api.debug(&format!("interval not reached."));
        }

        // --- 7. Drain block registrations for each network. Needed for registration rate limits.
        REGISTRATIONS_THIS_BLOCK.save(store, netuid, &0)?;
    }

    Ok(())
}

// Performs the difficulty adjustment by multiplying the current difficulty by the ratio ( reg_actual + reg_target / reg_target * reg_target )
// We use I110F18 to avoid any overflows on u64. Also min_difficulty and max_difficulty bound the range.
//
pub fn adjust_difficulty(
    store: &dyn Storage,
    netuid: u16,
    current_difficulty: u64,
    registrations_this_interval: u16,
    target_registrations_per_interval: u16,
) -> Result<u64, ContractError> {
    let updated_difficulty: I110F18 = I110F18::from_num(current_difficulty)
        * I110F18::from_num(registrations_this_interval + target_registrations_per_interval)
        / I110F18::from_num(target_registrations_per_interval + target_registrations_per_interval);

    let adjustment_alpha = ADJUSTMENTS_ALPHA.load(store, netuid)?;
    let alpha: I110F18 = I110F18::from_num(adjustment_alpha) / I110F18::from_num(u64::MAX);
    let next_value: I110F18 = alpha * I110F18::from_num(current_difficulty)
        + (I110F18::from_num(1.0) - alpha) * updated_difficulty;

    let max_difficulty = MAX_DIFFICULTY.load(store, netuid)?;
    let min_difficulty = MIN_DIFFICULTY.load(store, netuid)?;
    if next_value >= I110F18::from_num(max_difficulty) {
        return Ok(max_difficulty);
    } else if next_value <= I110F18::from_num(min_difficulty) {
        return Ok(min_difficulty);
    } else {
        return Ok(next_value.to_num::<u64>());
    }
}

// Performs the burn adjustment by multiplying the current difficulty by the ratio ( reg_actual + reg_target / reg_target * reg_target )
// We use I110F18 to avoid any overflows on u64. Also min_burn and max_burn bound the range.
//
pub fn adjust_burn(
    store: &dyn Storage,
    netuid: u16,
    current_burn: u64,
    registrations_this_interval: u16,
    target_registrations_per_interval: u16,
) -> Result<u64, ContractError> {
    let updated_burn: I110F18 = I110F18::from_num(current_burn)
        * I110F18::from_num(registrations_this_interval + target_registrations_per_interval)
        / I110F18::from_num(target_registrations_per_interval + target_registrations_per_interval);

    let adjustment_alpha = ADJUSTMENTS_ALPHA.load(store, netuid)?;
    let alpha: I110F18 = I110F18::from_num(adjustment_alpha) / I110F18::from_num(u64::MAX);
    let next_value: I110F18 =
        alpha * I110F18::from_num(current_burn) + (I110F18::from_num(1.0) - alpha) * updated_burn;

    let max_burn = MAX_BURN.load(store, netuid)?;
    let min_burn = MIN_BURN.load(store, netuid)?;
    if next_value >= I110F18::from_num(max_burn) {
        return Ok(max_burn);
    } else if next_value <= I110F18::from_num(min_burn) {
        return Ok(min_burn);
    } else {
        return Ok(next_value.to_num::<u64>());
    }
}
