#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, to_json_binary};
use crate::delegate_info::{get_delegate, get_delegated, get_delegates};
// use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::neuron_info::{get_neuron, get_neuron_lite, get_neurons, get_neurons_lite};
use crate::registration::{do_burned_registration, do_sudo_registration};
use crate::root::{do_root_register, get_network_lock_cost, user_add_network, user_remove_network};
use crate::serving::{do_serve_axon, do_serve_prometheus};
use crate::stake_info::{get_stake_info_for_coldkey, get_stake_info_for_coldkeys};
use crate::staking::{do_add_stake, do_become_delegate, do_remove_stake};
use crate::state::{ACTIVE, ACTIVITY_CUTOFF, ALLOW_FAUCET, BLOCK_AT_REGISTRATION, BURN_REGISTRATIONS_THIS_INTERVAL, CONSENSUS, DIFFICULTY, DIVIDENDS, EMISSION, EMISSION_VALUES, IMMUNITY_PERIOD, INCENTIVE, IS_NETWORK_MEMBER, KAPPA, KEYS, LAST_UPDATE, MAX_ALLOWED_UIDS, MAX_ALLOWED_VALIDATORS, MAX_WEIGHTS_LIMIT, MIN_ALLOWED_WEIGHTS, NETWORK_MODALITY, NETWORK_REGISTRATION_ALLOWED, NETWORKS_ADDED, OWNER, POW_REGISTRATIONS_THIS_INTERVAL, PRUNING_SCORES, RANK, REGISTRATIONS_THIS_INTERVAL, ROOT, STAKE, SUBNETWORK_N, TARGET_REGISTRATIONS_PER_INTERVAL, TEMPO, TOTAL_COLDKEY_STAKE, TOTAL_HOTKEY_STAKE, TOTAL_ISSUANCE, TOTAL_NETWORKS, TRUST, UIDS, VALIDATOR_PERMIT, VALIDATOR_TRUST};
use crate::subnet_info::{get_subnet_hyperparams, get_subnet_info, get_subnets_info};
use crate::utils::{do_set_rao_recycled, do_set_total_issuance, do_sudo_set_activity_cutoff, do_sudo_set_adjustment_alpha, do_sudo_set_adjustment_interval, do_sudo_set_bonds_moving_average, do_sudo_set_default_take, do_sudo_set_difficulty, do_sudo_set_immunity_period, do_sudo_set_kappa, do_sudo_set_lock_reduction_interval, do_sudo_set_max_allowed_uids, do_sudo_set_max_allowed_validators, do_sudo_set_max_burn, do_sudo_set_max_difficulty, do_sudo_set_max_registrations_per_block, do_sudo_set_max_weight_limit, do_sudo_set_min_allowed_weights, do_sudo_set_min_burn, do_sudo_set_min_difficulty, do_sudo_set_network_immunity_period, do_sudo_set_network_min_lock_cost, do_sudo_set_network_rate_limit, do_sudo_set_network_registration_allowed, do_sudo_set_rho, do_sudo_set_scaling_law_power, do_sudo_set_serving_rate_limit, do_sudo_set_subnet_limit, do_sudo_set_subnet_owner_cut, do_sudo_set_target_registrations_per_interval, do_sudo_set_tempo, do_sudo_set_tx_rate_limit, do_sudo_set_validator_prune_len, do_sudo_set_weights_set_rate_limit, do_sudo_set_weights_version_key};
use crate::weights::do_set_weights;


// version info for migration info
const CONTRACT_NAME: &str = "cybernet";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    ROOT.save(deps.storage, &info.sender)?;
    ALLOW_FAUCET.save(deps.storage, &true)?;

    // Set initial total issuance from balances
    TOTAL_ISSUANCE.save(deps.storage, &msg.balances_issuance)?;

    // Subnet config values
    let netuid: u16 = 1;
    let tempo = 1;
    let max_uids = 4096;

    // --- Set this network uid to alive.
    NETWORKS_ADDED.save(deps.storage, netuid, &true)?;

    // --- Fill tempo memory item.
    TEMPO.save(deps.storage, netuid, &tempo)?;

    // --- Fill modality item.
    // Only modality 0 exists (text)
    NETWORK_MODALITY.save(deps.storage, netuid, &0)?;

    // Make network parameters explicit.
    TEMPO.save(deps.storage, netuid, &0)?;
    KAPPA.save(deps.storage, netuid, &0)?;
    DIFFICULTY.save(deps.storage, netuid, &0)?;
    IMMUNITY_PERIOD.save(deps.storage, netuid, &0)?;
    ACTIVITY_CUTOFF.save(deps.storage, netuid, &0)?;
    EMISSION_VALUES.save(deps.storage, netuid, &0)?;
    MAX_WEIGHTS_LIMIT.save(deps.storage, netuid, &0)?;
    MIN_ALLOWED_WEIGHTS.save(deps.storage, netuid, &0)?;
    REGISTRATIONS_THIS_INTERVAL.save(deps.storage, netuid, &0)?;
    POW_REGISTRATIONS_THIS_INTERVAL.save(deps.storage, netuid, &0)?;
    BURN_REGISTRATIONS_THIS_INTERVAL.save(deps.storage, netuid, &0)?;

    // Set max allowed uids
    MAX_ALLOWED_UIDS.save(deps.storage, netuid, &max_uids)?;

    let mut next_uid = 0;

    let action = |vec: Option<Vec<u16>>| -> StdResult<_> {
        match vec {
            Some(mut v) => {
                v.push(0);
                Ok(v)
            },
            None => Ok(vec!(0)),
        }
    };
    for (coldkey, hotkeys) in msg.stakes.iter() {
        for (hotkey, stake_uid) in hotkeys.iter() {
            let (stake, uid) = stake_uid;

            // Expand Yuma Consensus with new position.
            RANK.update(deps.storage, netuid.clone(), action)?;
            TRUST.update(deps.storage, netuid.clone(), action)?;
            ACTIVE.update(deps.storage, netuid.clone(), |vec| -> StdResult<_> {
                match vec {
                    Some(mut v) => {
                        v.push(true);
                        Ok(v)
                    },
                    None => Ok(vec!(true)),
                }
            })?;
            EMISSION.update(deps.storage, netuid.clone(), |vec| -> StdResult<_> {
                match vec {
                    Some(mut v) => {
                        v.push(0);
                        Ok(v)
                    },
                    None => Ok(vec!(0)),
                }
            })?;
            CONSENSUS.update(deps.storage, netuid.clone(), action)?;
            INCENTIVE.update(deps.storage, netuid.clone(), action)?;
            DIVIDENDS.update(deps.storage, netuid.clone(), action)?;
            LAST_UPDATE.update(deps.storage, netuid.clone(), |vec| -> StdResult<_> {
                match vec {
                    Some(mut v) => {
                        v.push(0);
                        Ok(v)
                    },
                    None => Ok(vec!(0)),
                }
            })?;
            PRUNING_SCORES.update(deps.storage, netuid.clone(), action)?;
            VALIDATOR_TRUST.update(deps.storage, netuid.clone(), action)?;
            VALIDATOR_PERMIT.update(deps.storage, netuid.clone(), |vec| -> StdResult<_> {
                match vec {
                    Some(mut v) => {
                        v.push(true);
                        Ok(v)
                    },
                    None => Ok(vec!(true)),
                }
            })?;

            // Insert account information.
            KEYS.save(deps.storage, (netuid.clone(), uid.clone()), &hotkey.clone())?; // Make hotkey - uid association.
            UIDS.save(deps.storage, (netuid.clone(), hotkey), &uid.clone())?; // Make uid - hotkey association.
            BLOCK_AT_REGISTRATION.save(deps.storage, (netuid.clone(), uid.clone()), &env.block.height)?; // Fill block at registration.
            IS_NETWORK_MEMBER.save(deps.storage, (hotkey, netuid), &true )?; // Fill network is member.

            // Fill stake information.
            OWNER.save(deps.storage, hotkey.clone(), &coldkey.clone())?;

            TOTAL_HOTKEY_STAKE.save(deps.storage, hotkey.clone(), stake)?;
            TOTAL_COLDKEY_STAKE.update(deps.storage, coldkey.clone(), |s| -> StdResult<_> {
                match s {
                    Some(s) => Ok(s.saturating_add(stake.clone())),
                    None => Ok(stake.clone()),
                }
            })?;

            // Update total issuance value
            TOTAL_ISSUANCE.update(deps.storage, |a| -> StdResult<_> { Ok(a.saturating_add(stake.clone())) })?;

            STAKE.save(deps.storage,(hotkey.clone(), coldkey.clone()), stake)?;

            next_uid += 1;
        }
    }

    // Set correct length for Subnet neurons
    SUBNETWORK_N.save(deps.storage, netuid, &next_uid)?;

    // --- Increase total network count.
    TOTAL_NETWORKS.save(deps.storage, &1)?;

    // Get the root network uid.
    let root_netuid: u16 = 0;

    // Set the root network as added.
    NETWORKS_ADDED.save(deps.storage, root_netuid, &true)?;

    // Increment the number of total networks.
    TOTAL_NETWORKS.update(deps.storage, |mut n| -> StdResult<_> {
        n += 1;
        Ok(n)
    })?;

    // Set the number of validators to 1.
    SUBNETWORK_N.save(deps.storage, root_netuid, &0)?;

    // Set the maximum number to the number of senate members.
    MAX_ALLOWED_UIDS.save(deps.storage, root_netuid, &64)?;

    // Set the maximum number to the number of validators to all members.
    MAX_ALLOWED_VALIDATORS.save(deps.storage, root_netuid, &64)?;

    // Set the min allowed weights to zero, no weights restrictions.
    MIN_ALLOWED_WEIGHTS.save(deps.storage, root_netuid, &0)?;

    // Set the max weight limit to infitiy, no weight restrictions.
    MAX_WEIGHTS_LIMIT.save(deps.storage, root_netuid, &u16::MAX)?;

    // Add default root tempo.
    TEMPO.save(deps.storage, root_netuid, &100)?;

    // Set the root network as open.
    NETWORK_REGISTRATION_ALLOWED.save(deps.storage, root_netuid, &true)?;

    // Set target registrations for validators as 1 per block.
    TARGET_REGISTRATIONS_PER_INTERVAL.save(deps.storage, root_netuid, &1)?;

    Ok(Response::default()
        .add_attribute("action", "instantiate")
    )
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::SetWeights { netuid, dests, weights, version_key } => do_set_weights(deps, env, info, netuid, dests, weights, version_key),
        ExecuteMsg::BecomeDelegate { hotkey, take } => do_become_delegate(deps, env, info, hotkey, take),
        ExecuteMsg::AddStake { hotkey, amount_staked } => do_add_stake(deps, env, info, hotkey, amount_staked),
        ExecuteMsg::RemoveStake { hotkey, amount_unstaked } => do_remove_stake(deps, env, info, hotkey, amount_unstaked),
        ExecuteMsg::ServeAxon {
            netuid,
            version,
            ip,
            port,
            ip_type,
            protocol,
            placeholder1,
            placeholder2
        } => do_serve_axon(deps, env, info,
                           netuid,
                           version,
                           ip,
                           port,
                           ip_type,
                           protocol,
                           placeholder1,
                           placeholder2,
        ),
        ExecuteMsg::ServePrometheus { netuid, version, ip, port, ip_type } => do_serve_prometheus(deps, env, info, netuid, version, ip, port, ip_type),
        // ExecuteMsg::Register { netuid, block_number, nonce, work, hotkey, coldkey } => do_registration(deps, env, info, netuid, block_number, nonce, work, hotkey, coldkey),
        ExecuteMsg::RootRegister { hotkey } => do_root_register(deps, env, info, hotkey),
        ExecuteMsg::BurnedRegister { netuid, hotkey } => do_burned_registration(deps, env, info, netuid, hotkey),

        ExecuteMsg::RegisterNetwork {} => user_add_network(deps, env, info),
        ExecuteMsg::DissolveNetwork { netuid } => user_remove_network(deps, env, info, netuid),
        // ExecuteMsg::Faucet { block_number, nonce, work } => do_faucet(deps, env, info, block_number, nonce, work),

        ExecuteMsg::SudoRegister { netuid, hotkey, coldkey, stake, balance } => do_sudo_registration(deps, env, info, netuid, hotkey, coldkey, stake, balance),
        ExecuteMsg::SudoSetDefaultTake { default_take } => do_sudo_set_default_take(deps, env, info, default_take),
        ExecuteMsg::SudoSetServingRateLimit { netuid, serving_rate_limit } => do_sudo_set_serving_rate_limit(deps, env, info, netuid, serving_rate_limit),
        ExecuteMsg::SudoSetTxRateLimit { tx_rate_limit } => do_sudo_set_tx_rate_limit(deps, env, info, tx_rate_limit),
        ExecuteMsg::SudoSetMaxBurn { netuid, max_burn } => do_sudo_set_max_burn(deps, env, info, netuid, max_burn),
        ExecuteMsg::SudoSetMinBurn { netuid, min_burn } => do_sudo_set_min_burn(deps, env, info, netuid, min_burn),
        ExecuteMsg::SudoSetMaxDifficulty { netuid, max_difficulty } => do_sudo_set_max_difficulty(deps, env, info, netuid, max_difficulty),
        ExecuteMsg::SudoSetMinDifficulty { netuid, min_difficulty } => do_sudo_set_min_difficulty(deps, env, info, netuid, min_difficulty),
        ExecuteMsg::SudoSetWeightsSetRateLimit { netuid, weights_set_rate_limit } => do_sudo_set_weights_set_rate_limit(deps, env, info, netuid, weights_set_rate_limit),
        ExecuteMsg::SudoSetWeightsVersionKey { netuid, weights_version_key } => do_sudo_set_weights_version_key(deps, env, info, netuid, weights_version_key),
        ExecuteMsg::SudoSetBondsMovingAverage { netuid, bonds_moving_average } => do_sudo_set_bonds_moving_average(deps, env, info, netuid, bonds_moving_average),
        ExecuteMsg::SudoSetMaxAllowedValidators { netuid, max_allowed_validators } => do_sudo_set_max_allowed_validators(deps, env, info, netuid, max_allowed_validators),
        ExecuteMsg::SudoSetDifficulty { netuid, difficulty } => do_sudo_set_difficulty(deps, env, info, netuid, difficulty),
        ExecuteMsg::SudoSetAdjustmentInterval { netuid, adjustment_interval } => do_sudo_set_adjustment_interval(deps, env, info, netuid, adjustment_interval),
        ExecuteMsg::SudoSetTargetRegistrationsPerInterval { netuid, target_registrations_per_interval } => do_sudo_set_target_registrations_per_interval(deps, env, info, netuid, target_registrations_per_interval),
        ExecuteMsg::SudoSetActivityCutoff { netuid, activity_cutoff } => do_sudo_set_activity_cutoff(deps, env, info, netuid, activity_cutoff),
        ExecuteMsg::SudoSetRho { netuid, rho } => do_sudo_set_rho(deps, env, info, netuid, rho),
        ExecuteMsg::SudoSetKappa { netuid, kappa } => do_sudo_set_kappa(deps, env, info, netuid, kappa),
        ExecuteMsg::SudoSetMaxAllowedUids { netuid, max_allowed_uids } => do_sudo_set_max_allowed_uids(deps, env, info, netuid, max_allowed_uids),
        ExecuteMsg::SudoSetMinAllowedWeights { netuid, min_allowed_weights } => do_sudo_set_min_allowed_weights(deps, env, info, netuid, min_allowed_weights),
        ExecuteMsg::SudoSetValidatorPruneLen { netuid, validator_prune_len } => do_sudo_set_validator_prune_len(deps, env, info, netuid, validator_prune_len),
        ExecuteMsg::SudoSetScalingLawPower { netuid, scaling_law_power } => do_sudo_set_scaling_law_power(deps, env, info, netuid, scaling_law_power),
        ExecuteMsg::SudoSetImmunityPeriod { netuid, immunity_period } => do_sudo_set_immunity_period(deps, env, info, netuid, immunity_period),
        ExecuteMsg::SudoSetMaxWeightLimit { netuid, max_weight_limit } => do_sudo_set_max_weight_limit(deps, env, info, netuid, max_weight_limit),
        ExecuteMsg::SudoSetMaxRegistrationsPerBlock { netuid, max_registrations_per_block } => do_sudo_set_max_registrations_per_block(deps, env, info, netuid, max_registrations_per_block),
        ExecuteMsg::SudoSetTotalIssuance { total_issuance } => do_set_total_issuance(deps, env, info, total_issuance),
        ExecuteMsg::SudoSetTempo { netuid, tempo } => do_sudo_set_tempo(deps, env, info, netuid, tempo),
        ExecuteMsg::SudoSetRaoRecycled { netuid, rao_recycled } => do_set_rao_recycled(deps, env, info, netuid, rao_recycled),
        // ExecuteMsg::Sudo { .. } => {}
        ExecuteMsg::SudoSetRegistrationAllowed { netuid, registration_allowed } => do_sudo_set_network_registration_allowed(deps, env, info, netuid, registration_allowed),
        ExecuteMsg::SudoSetAdjustmentAlpha { netuid, adjustment_alpha } => do_sudo_set_adjustment_alpha(deps, env, info, netuid, adjustment_alpha),
        ExecuteMsg::SudoSetSubnetOwnerCut { cut } => do_sudo_set_subnet_owner_cut(deps, env, info, cut),
        ExecuteMsg::SudoSetNetworkRateLimit { rate_limit } => do_sudo_set_network_rate_limit(deps, env, info, rate_limit),
        ExecuteMsg::SudoSetNetworkImmunityPeriod { immunity_period } => do_sudo_set_network_immunity_period(deps, env, info, immunity_period),
        ExecuteMsg::SudoSetNetworkMinLockCost { lock_cost } => do_sudo_set_network_min_lock_cost(deps, env, info, lock_cost),
        ExecuteMsg::SudoSetSubnetLimit { max_subnets } => do_sudo_set_subnet_limit(deps, env, info, max_subnets),
        ExecuteMsg::SudoSetLockReductionInterval { interval } => do_sudo_set_lock_reduction_interval(deps, env, info, interval),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetDelegates {} => to_json_binary(&get_delegates(deps)?),
        QueryMsg::GetDelegate { delegate_account } => to_json_binary(&get_delegate(deps, delegate_account)?),
        QueryMsg::GetDelegated { delegatee_account } => to_json_binary(&get_delegated(deps, delegatee_account)?),
        QueryMsg::GetNeuronsLite { netuid } => to_json_binary(&get_neurons_lite(deps, netuid)?),
        QueryMsg::GetNeuronLite { netuid, uid } => to_json_binary(&get_neuron_lite(deps, netuid, uid)?),
        QueryMsg::GetNeurons { netuid } => to_json_binary(&get_neurons(deps, netuid)?),
        QueryMsg::GetNeuron { netuid, uid } => to_json_binary(&get_neuron(deps, netuid, uid)?),
        QueryMsg::GetSubnetInfo { netuid } => to_json_binary(&get_subnet_info(deps, netuid)?),
        QueryMsg::GetSubnetsInfo {} => to_json_binary(&get_subnets_info(deps)?),
        QueryMsg::GetSubnetHyperparams { netuid } => to_json_binary(&get_subnet_hyperparams(deps, netuid)?),
        QueryMsg::GetStakeInfoForColdkey { coldkey_account } => to_json_binary(&get_stake_info_for_coldkey(deps, coldkey_account)?),
        QueryMsg::GetStakeInfoForColdkeys { coldkey_accounts } => to_json_binary(&get_stake_info_for_coldkeys(deps, coldkey_accounts)?),
        QueryMsg::GetNetworkRegistrationCost { } => to_json_binary(&get_network_lock_cost(deps.storage, deps.api, env.block.height)?),
    }
}
