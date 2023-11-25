#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::{get_contract_version, set_contract_version, ContractVersion};

use crate::block_step::block_step;
use crate::delegate_info::{get_delegate, get_delegated, get_delegates};
use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, SudoMsg};
use crate::neuron_info::{get_neuron, get_neuron_lite, get_neurons, get_neurons_lite};
use crate::registration::{do_burned_registration, do_registration, do_sudo_registration};
use crate::root::{do_root_register, get_network_lock_cost, user_add_network, user_remove_network};
use crate::serving::{do_serve_axon, do_serve_prometheus};
use crate::stake_info::{get_stake_info_for_coldkey, get_stake_info_for_coldkeys};
use crate::staking::{
    do_add_stake, do_become_delegate, do_remove_stake, increase_stake_on_coldkey_hotkey_account,
};
use crate::state::{ACTIVE, ACTIVITY_CUTOFF, ADJUSTMENTS_ALPHA, ADJUSTMENT_INTERVAL, ALLOW_FAUCET, BLOCKS_SINCE_LAST_STEP, BLOCK_AT_REGISTRATION, BLOCK_EMISSION, BONDS_MOVING_AVERAGE, BURN, BURN_REGISTRATIONS_THIS_INTERVAL, CONSENSUS, DEFAULT_TAKE, DIFFICULTY, DIVIDENDS, EMISSION, EMISSION_VALUES, IMMUNITY_PERIOD, INCENTIVE, IS_NETWORK_MEMBER, KAPPA, KEYS, LAST_ADJUSTMENT_BLOCK, LAST_UPDATE, MAX_ALLOWED_UIDS, MAX_ALLOWED_VALIDATORS, MAX_BURN, MAX_DIFFICULTY, MAX_REGISTRATION_PER_BLOCK, MAX_WEIGHTS_LIMIT, MIN_ALLOWED_WEIGHTS, MIN_BURN, MIN_DIFFICULTY, NETWORKS_ADDED, NETWORK_IMMUNITY_PERIOD, NETWORK_LAST_LOCK_COST, NETWORK_LAST_REGISTERED, NETWORK_LOCK_REDUCTION_INTERVAL, NETWORK_MIN_ALLOWED_UIDS, NETWORK_MIN_LOCK_COST, NETWORK_MODALITY, NETWORK_RATE_LIMIT, NETWORK_REGISTERED_AT, NETWORK_REGISTRATION_ALLOWED, OWNER, PENDING_EMISSION, POW_REGISTRATIONS_THIS_INTERVAL, PRUNING_SCORES, RANK, RAO_RECYCLED_FOR_REGISTRATION, REGISTRATIONS_THIS_BLOCK, REGISTRATIONS_THIS_INTERVAL, RHO, ROOT, SERVING_RATE_LIMIT, SUBNETWORK_N, SUBNET_LIMIT, SUBNET_LOCKED, SUBNET_OWNER, SUBNET_OWNER_CUT, TARGET_REGISTRATIONS_PER_INTERVAL, TEMPO, TOTAL_ISSUANCE, TOTAL_NETWORKS, TOTAL_STAKE, TRUST, TX_RATE_LIMIT, UIDS, VALIDATOR_PERMIT, VALIDATOR_TRUST, WEIGHTS_SET_RATE_LIMIT, WEIGHTS_VERSION_KEY, DENOM};
use crate::state_info::get_state_info;
use crate::subnet_info::{get_subnet_hyperparams, get_subnet_info, get_subnets_info};
use crate::utils::{
    do_sudo_set_activity_cutoff, do_sudo_set_adjustment_alpha, do_sudo_set_adjustment_interval,
    do_sudo_set_block_emission, do_sudo_set_bonds_moving_average, do_sudo_set_default_take,
    do_sudo_set_difficulty, do_sudo_set_immunity_period, do_sudo_set_kappa,
    do_sudo_set_lock_reduction_interval, do_sudo_set_max_allowed_uids,
    do_sudo_set_max_allowed_validators, do_sudo_set_max_burn, do_sudo_set_max_difficulty,
    do_sudo_set_max_registrations_per_block, do_sudo_set_max_weight_limit,
    do_sudo_set_min_allowed_weights, do_sudo_set_min_burn, do_sudo_set_min_difficulty,
    do_sudo_set_network_immunity_period, do_sudo_set_network_min_lock_cost,
    do_sudo_set_network_rate_limit, do_sudo_set_network_registration_allowed,
    do_sudo_set_rao_recycled, do_sudo_set_rho, do_sudo_set_scaling_law_power,
    do_sudo_set_serving_rate_limit, do_sudo_set_subnet_limit, do_sudo_set_subnet_owner_cut,
    do_sudo_set_target_registrations_per_interval, do_sudo_set_tempo, do_sudo_set_total_issuance,
    do_sudo_set_tx_rate_limit, do_sudo_set_validator_permit_for_uid,
    do_sudo_set_validator_prune_len, do_sudo_set_weights_set_rate_limit,
    do_sudo_set_weights_version_key,
};
use crate::weights::{do_set_weights, get_network_weights};

// use cw2::set_contract_version;

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

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    ROOT.save(deps.storage, &info.sender)?;
    ALLOW_FAUCET.save(deps.storage, &false)?;
    DENOM.save(deps.storage, &"boot".to_string())?;

    // TODO remove from InstantiateMsg
    // // Set initial total issuance from balances
    TOTAL_ISSUANCE.save(deps.storage, &0)?;
    TOTAL_STAKE.save(deps.storage, &0)?;

    // -- Cybertensor parameters initialization --

    SUBNET_LIMIT.save(deps.storage, &16)?;
    NETWORK_IMMUNITY_PERIOD.save(deps.storage, &7200)?;
    BLOCK_EMISSION.save(deps.storage, &1_000_000_000)?;

    NETWORK_MIN_ALLOWED_UIDS.save(deps.storage, &0)?;
    SUBNET_OWNER_CUT.save(deps.storage, &0)?;
    NETWORK_RATE_LIMIT.save(deps.storage, &0)?;

    DEFAULT_TAKE.save(deps.storage, &11_796)?;
    TX_RATE_LIMIT.save(deps.storage, &1000)?;

    NETWORK_LAST_LOCK_COST.save(deps.storage, &100_000_000_000)?;
    NETWORK_MIN_LOCK_COST.save(deps.storage, &100_000_000_000)?;
    NETWORK_LOCK_REDUCTION_INTERVAL.save(deps.storage, &2)?; // test value, change to 14 * 7200;

    // -- Root network initialization --

    // Get the root network uid.
    let root_netuid: u16 = 0;

    SUBNET_OWNER.save(deps.storage, root_netuid, &info.sender)?;
    // TODO revisit set of 1
    SUBNETWORK_N.save(deps.storage, root_netuid, &0)?;
    NETWORKS_ADDED.save(deps.storage, root_netuid, &true)?;
    NETWORK_MODALITY.save(deps.storage, root_netuid, &u16::MAX)?;
    MAX_ALLOWED_UIDS.save(deps.storage, root_netuid, &64)?;
    MAX_ALLOWED_VALIDATORS.save(deps.storage, root_netuid, &64)?;
    MIN_ALLOWED_WEIGHTS.save(deps.storage, root_netuid, &1)?;
    MAX_WEIGHTS_LIMIT.save(deps.storage, root_netuid, &u16::MAX)?;
    TEMPO.save(deps.storage, root_netuid, &1)?;
    NETWORK_REGISTRATION_ALLOWED.save(deps.storage, root_netuid, &true)?;
    TARGET_REGISTRATIONS_PER_INTERVAL.save(deps.storage, root_netuid, &1)?;
    WEIGHTS_VERSION_KEY.save(deps.storage, root_netuid, &0)?;
    SUBNET_OWNER.save(deps.storage, root_netuid, &info.sender)?;
    NETWORK_REGISTERED_AT.save(deps.storage, root_netuid, &env.block.height)?;
    WEIGHTS_SET_RATE_LIMIT.save(deps.storage, root_netuid, &100)?;

    PENDING_EMISSION.save(deps.storage, root_netuid, &0)?;
    BLOCKS_SINCE_LAST_STEP.save(deps.storage, root_netuid, &0)?;
    BONDS_MOVING_AVERAGE.save(deps.storage, root_netuid, &900_000)?;
    LAST_ADJUSTMENT_BLOCK.save(deps.storage, root_netuid, &0)?;
    ADJUSTMENT_INTERVAL.save(deps.storage, root_netuid, &100)?;
    BURN.save(deps.storage, root_netuid, &0)?;
    MIN_BURN.save(deps.storage, root_netuid, &0)?;
    MAX_BURN.save(deps.storage, root_netuid, &1_000_000_000)?;
    REGISTRATIONS_THIS_BLOCK.save(deps.storage, root_netuid, &0)?;
    MAX_REGISTRATION_PER_BLOCK.save(deps.storage, root_netuid, &1)?;
    REGISTRATIONS_THIS_INTERVAL.save(deps.storage, root_netuid, &0)?;
    KAPPA.save(deps.storage, root_netuid, &32_767)?;
    RHO.save(deps.storage, root_netuid, &30)?;
    RAO_RECYCLED_FOR_REGISTRATION.save(deps.storage, root_netuid, &0)?;
    ACTIVITY_CUTOFF.save(deps.storage, root_netuid, &5000)?;
    SERVING_RATE_LIMIT.save(deps.storage, root_netuid, &50)?;
    DIFFICULTY.save(deps.storage, root_netuid, &10_000_000)?;
    IMMUNITY_PERIOD.save(deps.storage, root_netuid, &7200)?;
    POW_REGISTRATIONS_THIS_INTERVAL.save(deps.storage, root_netuid, &0)?;
    BURN_REGISTRATIONS_THIS_INTERVAL.save(deps.storage, root_netuid, &0)?;
    ADJUSTMENTS_ALPHA.save(deps.storage, root_netuid, &0)?;
    MIN_DIFFICULTY.save(deps.storage, root_netuid, &1)?;
    MAX_DIFFICULTY.save(deps.storage, root_netuid, &1000000)?;
    EMISSION_VALUES.save(deps.storage, root_netuid, &0)?;
    NETWORK_LAST_REGISTERED.save(deps.storage, &0)?;
    TOTAL_NETWORKS.save(deps.storage, &1)?;

    // -- Subnetwork 1 initialization --

    // Subnet config values
    let netuid: u16 = 1;
    let tempo = 10;
    let max_uids = 4096;

    SUBNET_OWNER.save(deps.storage, netuid, &info.sender)?;
    NETWORKS_ADDED.save(deps.storage, netuid, &true)?;
    TEMPO.save(deps.storage, netuid, &tempo)?;
    NETWORK_MODALITY.save(deps.storage, netuid, &0)?;
    // TEMPO.save(deps.storage, netuid, &0)?;
    KAPPA.save(deps.storage, netuid, &0)?;
    DIFFICULTY.save(deps.storage, netuid, &10_000_000)?;
    IMMUNITY_PERIOD.save(deps.storage, netuid, &7200)?;
    ACTIVITY_CUTOFF.save(deps.storage, netuid, &5000)?;
    EMISSION_VALUES.save(deps.storage, netuid, &0)?;
    MAX_WEIGHTS_LIMIT.save(deps.storage, netuid, &u16::MAX)?;
    MIN_ALLOWED_WEIGHTS.save(deps.storage, netuid, &0)?;
    REGISTRATIONS_THIS_INTERVAL.save(deps.storage, netuid, &0)?;
    POW_REGISTRATIONS_THIS_INTERVAL.save(deps.storage, netuid, &0)?;
    BURN_REGISTRATIONS_THIS_INTERVAL.save(deps.storage, netuid, &0)?;
    MAX_ALLOWED_VALIDATORS.save(deps.storage, netuid, &64)?;
    MAX_ALLOWED_UIDS.save(deps.storage, netuid, &max_uids)?;
    WEIGHTS_VERSION_KEY.save(deps.storage, netuid, &0)?;
    WEIGHTS_SET_RATE_LIMIT.save(deps.storage, netuid, &100)?;

    PENDING_EMISSION.save(deps.storage, netuid, &0)?;
    BLOCKS_SINCE_LAST_STEP.save(deps.storage, netuid, &0)?;
    BONDS_MOVING_AVERAGE.save(deps.storage, netuid, &900_000)?;
    LAST_ADJUSTMENT_BLOCK.save(deps.storage, netuid, &0)?;
    ADJUSTMENT_INTERVAL.save(deps.storage, netuid, &100)?;
    BURN.save(deps.storage, netuid, &0)?;
    MIN_BURN.save(deps.storage, netuid, &0)?;
    MAX_BURN.save(deps.storage, netuid, &1_000_000_000)?;
    REGISTRATIONS_THIS_BLOCK.save(deps.storage, netuid, &0)?;
    MAX_REGISTRATION_PER_BLOCK.save(deps.storage, netuid, &3)?;
    KAPPA.save(deps.storage, netuid, &32_767)?;
    RHO.save(deps.storage, netuid, &30)?;
    RAO_RECYCLED_FOR_REGISTRATION.save(deps.storage, netuid, &0)?;
    SERVING_RATE_LIMIT.save(deps.storage, netuid, &50)?;
    ADJUSTMENTS_ALPHA.save(deps.storage, netuid, &0)?;
    MIN_DIFFICULTY.save(deps.storage, root_netuid, &1)?;
    MAX_DIFFICULTY.save(deps.storage, root_netuid, &1000000)?;
    SUBNET_LOCKED.save(deps.storage, root_netuid, &0)?;
    NETWORK_REGISTERED_AT.save(deps.storage, netuid, &env.block.height)?;
    SUBNETWORK_N.save(deps.storage, netuid, &0)?;
    SUBNET_LOCKED.save(deps.storage, netuid, &0)?;

    RANK.save(deps.storage, netuid, &vec![])?;
    TRUST.save(deps.storage, netuid, &vec![])?;
    ACTIVE.save(deps.storage, netuid, &vec![])?;
    EMISSION.save(deps.storage, netuid, &vec![])?;
    CONSENSUS.save(deps.storage, netuid, &vec![])?;
    INCENTIVE.save(deps.storage, netuid, &vec![])?;
    DIVIDENDS.save(deps.storage, netuid, &vec![])?;
    LAST_UPDATE.save(deps.storage, netuid, &vec![])?;
    PRUNING_SCORES.save(deps.storage, netuid, &vec![])?;
    VALIDATOR_TRUST.save(deps.storage, netuid, &vec![])?;
    VALIDATOR_PERMIT.save(deps.storage, netuid, &vec![])?;

    TOTAL_NETWORKS.update(deps.storage, |mut n| -> StdResult<_> {
        n += 1;
        Ok(n)
    })?;

    Ok(Response::default().add_attribute("action", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Empty {} => Ok(Response::default()),
        ExecuteMsg::BlockStep {} => block_step(deps, env),

        ExecuteMsg::SetWeights {
            netuid,
            dests,
            weights,
            version_key,
        } => do_set_weights(deps, env, info, netuid, dests, weights, version_key),
        ExecuteMsg::BecomeDelegate { hotkey, take } => {
            do_become_delegate(deps, env, info, hotkey, take)
        }
        ExecuteMsg::AddStake {
            hotkey,
            amount_staked,
        } => do_add_stake(deps, env, info, hotkey, amount_staked),
        ExecuteMsg::RemoveStake {
            hotkey,
            amount_unstaked,
        } => do_remove_stake(deps, env, info, hotkey, amount_unstaked),
        ExecuteMsg::ServeAxon {
            netuid,
            version,
            ip,
            port,
            ip_type,
            protocol,
            placeholder1,
            placeholder2,
        } => do_serve_axon(
            deps,
            env,
            info,
            netuid,
            version,
            ip,
            port,
            ip_type,
            protocol,
            placeholder1,
            placeholder2,
        ),
        ExecuteMsg::ServePrometheus {
            netuid,
            version,
            ip,
            port,
            ip_type,
        } => do_serve_prometheus(deps, env, info, netuid, version, ip, port, ip_type),
        ExecuteMsg::Register {
            netuid,
            block_number,
            nonce,
            work,
            hotkey,
            coldkey,
        } => do_registration(
            deps,
            env,
            info,
            netuid,
            block_number,
            nonce,
            work,
            hotkey,
            coldkey,
        ),
        ExecuteMsg::RootRegister { hotkey } => do_root_register(deps, env, info, hotkey),
        ExecuteMsg::BurnedRegister { netuid, hotkey } => {
            do_burned_registration(deps, env, info, netuid, hotkey)
        }

        ExecuteMsg::RegisterNetwork {} => user_add_network(deps, env, info),
        ExecuteMsg::DissolveNetwork { netuid } => user_remove_network(deps, env, info, netuid),
        // ExecuteMsg::Faucet { block_number, nonce, work } => do_faucet(deps, env, info, block_number, nonce, work),
        ExecuteMsg::SudoRegister {
            netuid,
            hotkey,
            coldkey,
            stake,
            balance,
        } => do_sudo_registration(deps, env, info, netuid, hotkey, coldkey, stake, balance),
        ExecuteMsg::SudoSetDefaultTake { default_take } => {
            do_sudo_set_default_take(deps, env, info, default_take)
        }
        ExecuteMsg::SudoSetServingRateLimit {
            netuid,
            serving_rate_limit,
        } => do_sudo_set_serving_rate_limit(deps, env, info, netuid, serving_rate_limit),
        ExecuteMsg::SudoSetTxRateLimit { tx_rate_limit } => {
            do_sudo_set_tx_rate_limit(deps, env, info, tx_rate_limit)
        }
        ExecuteMsg::SudoSetMaxBurn { netuid, max_burn } => {
            do_sudo_set_max_burn(deps, env, info, netuid, max_burn)
        }
        ExecuteMsg::SudoSetMinBurn { netuid, min_burn } => {
            do_sudo_set_min_burn(deps, env, info, netuid, min_burn)
        }
        ExecuteMsg::SudoSetMaxDifficulty {
            netuid,
            max_difficulty,
        } => do_sudo_set_max_difficulty(deps, env, info, netuid, max_difficulty),
        ExecuteMsg::SudoSetMinDifficulty {
            netuid,
            min_difficulty,
        } => do_sudo_set_min_difficulty(deps, env, info, netuid, min_difficulty),
        ExecuteMsg::SudoSetWeightsSetRateLimit {
            netuid,
            weights_set_rate_limit,
        } => do_sudo_set_weights_set_rate_limit(deps, env, info, netuid, weights_set_rate_limit),
        ExecuteMsg::SudoSetWeightsVersionKey {
            netuid,
            weights_version_key,
        } => do_sudo_set_weights_version_key(deps, env, info, netuid, weights_version_key),
        ExecuteMsg::SudoSetBondsMovingAverage {
            netuid,
            bonds_moving_average,
        } => do_sudo_set_bonds_moving_average(deps, env, info, netuid, bonds_moving_average),
        ExecuteMsg::SudoSetMaxAllowedValidators {
            netuid,
            max_allowed_validators,
        } => do_sudo_set_max_allowed_validators(deps, env, info, netuid, max_allowed_validators),
        ExecuteMsg::SudoSetDifficulty { netuid, difficulty } => {
            do_sudo_set_difficulty(deps, env, info, netuid, difficulty)
        }
        ExecuteMsg::SudoSetAdjustmentInterval {
            netuid,
            adjustment_interval,
        } => do_sudo_set_adjustment_interval(deps, env, info, netuid, adjustment_interval),
        ExecuteMsg::SudoSetTargetRegistrationsPerInterval {
            netuid,
            target_registrations_per_interval,
        } => do_sudo_set_target_registrations_per_interval(
            deps,
            env,
            info,
            netuid,
            target_registrations_per_interval,
        ),
        ExecuteMsg::SudoSetActivityCutoff {
            netuid,
            activity_cutoff,
        } => do_sudo_set_activity_cutoff(deps, env, info, netuid, activity_cutoff),
        ExecuteMsg::SudoSetRho { netuid, rho } => do_sudo_set_rho(deps, env, info, netuid, rho),
        ExecuteMsg::SudoSetKappa { netuid, kappa } => {
            do_sudo_set_kappa(deps, env, info, netuid, kappa)
        }
        ExecuteMsg::SudoSetMaxAllowedUids {
            netuid,
            max_allowed_uids,
        } => do_sudo_set_max_allowed_uids(deps, env, info, netuid, max_allowed_uids),
        ExecuteMsg::SudoSetMinAllowedWeights {
            netuid,
            min_allowed_weights,
        } => do_sudo_set_min_allowed_weights(deps, env, info, netuid, min_allowed_weights),
        ExecuteMsg::SudoSetValidatorPruneLen {
            netuid,
            validator_prune_len,
        } => do_sudo_set_validator_prune_len(deps, env, info, netuid, validator_prune_len),
        ExecuteMsg::SudoSetScalingLawPower {
            netuid,
            scaling_law_power,
        } => do_sudo_set_scaling_law_power(deps, env, info, netuid, scaling_law_power),
        ExecuteMsg::SudoSetImmunityPeriod {
            netuid,
            immunity_period,
        } => do_sudo_set_immunity_period(deps, env, info, netuid, immunity_period),
        ExecuteMsg::SudoSetMaxWeightLimit {
            netuid,
            max_weight_limit,
        } => do_sudo_set_max_weight_limit(deps, env, info, netuid, max_weight_limit),
        ExecuteMsg::SudoSetMaxRegistrationsPerBlock {
            netuid,
            max_registrations_per_block,
        } => do_sudo_set_max_registrations_per_block(
            deps,
            env,
            info,
            netuid,
            max_registrations_per_block,
        ),
        ExecuteMsg::SudoSetTotalIssuance { total_issuance } => {
            do_sudo_set_total_issuance(deps, env, info, total_issuance)
        }
        ExecuteMsg::SudoSetTempo { netuid, tempo } => {
            do_sudo_set_tempo(deps, env, info, netuid, tempo)
        }
        ExecuteMsg::SudoSetRaoRecycled {
            netuid,
            rao_recycled,
        } => do_sudo_set_rao_recycled(deps, env, info, netuid, rao_recycled),
        // ExecuteMsg::Sudo { .. } => {}
        ExecuteMsg::SudoSetRegistrationAllowed {
            netuid,
            registration_allowed,
        } => {
            do_sudo_set_network_registration_allowed(deps, env, info, netuid, registration_allowed)
        }
        ExecuteMsg::SudoSetAdjustmentAlpha {
            netuid,
            adjustment_alpha,
        } => do_sudo_set_adjustment_alpha(deps, env, info, netuid, adjustment_alpha),
        ExecuteMsg::SudoSetSubnetOwnerCut { cut } => {
            do_sudo_set_subnet_owner_cut(deps, env, info, cut)
        }
        ExecuteMsg::SudoSetNetworkRateLimit { rate_limit } => {
            do_sudo_set_network_rate_limit(deps, env, info, rate_limit)
        }
        ExecuteMsg::SudoSetNetworkImmunityPeriod { immunity_period } => {
            do_sudo_set_network_immunity_period(deps, env, info, immunity_period)
        }
        ExecuteMsg::SudoSetNetworkMinLockCost { lock_cost } => {
            do_sudo_set_network_min_lock_cost(deps, env, info, lock_cost)
        }
        ExecuteMsg::SudoSetSubnetLimit { max_subnets } => {
            do_sudo_set_subnet_limit(deps, env, info, max_subnets)
        }
        ExecuteMsg::SudoSetLockReductionInterval { interval } => {
            do_sudo_set_lock_reduction_interval(deps, env, info, interval)
        }
        ExecuteMsg::SudoSetValidatorPermitForUid {
            netuid,
            uid,
            permit,
        } => do_sudo_set_validator_permit_for_uid(deps, env, info, netuid, uid, permit),
        ExecuteMsg::SudoSetBlockEmission { emission } => {
            do_sudo_set_block_emission(deps, env, info, emission)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn sudo(deps: DepsMut, env: Env, msg: SudoMsg) -> Result<Response, ContractError> {
    match msg {
        SudoMsg::BlockStep {} => block_step(deps, env),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetDelegates {} => to_json_binary(&get_delegates(deps)?),
        QueryMsg::GetDelegate { delegate_account } => {
            to_json_binary(&get_delegate(deps, delegate_account)?)
        }
        QueryMsg::GetDelegated { delegatee_account } => {
            to_json_binary(&get_delegated(deps, delegatee_account)?)
        }
        QueryMsg::GetNeuronsLite { netuid } => {
            to_json_binary(&get_neurons_lite(deps.storage, netuid)?)
        }
        QueryMsg::GetNeuronLite { netuid, uid } => {
            to_json_binary(&get_neuron_lite(deps.storage, netuid, uid)?)
        }
        QueryMsg::GetNeurons { netuid } => to_json_binary(&get_neurons(deps.storage, netuid)?),
        QueryMsg::GetNeuron { netuid, uid } => {
            to_json_binary(&get_neuron(deps.storage, netuid, uid)?)
        }
        QueryMsg::GetSubnetInfo { netuid } => to_json_binary(&get_subnet_info(deps, netuid)?),
        QueryMsg::GetSubnetsInfo {} => to_json_binary(&get_subnets_info(deps)?),
        QueryMsg::GetSubnetHyperparams { netuid } => {
            to_json_binary(&get_subnet_hyperparams(deps, netuid)?)
        }
        QueryMsg::GetStakeInfoForColdkey { coldkey_account } => {
            to_json_binary(&get_stake_info_for_coldkey(deps, coldkey_account)?)
        }
        QueryMsg::GetStakeInfoForColdkeys { coldkey_accounts } => {
            to_json_binary(&get_stake_info_for_coldkeys(deps, coldkey_accounts)?)
        }
        QueryMsg::GetNetworkRegistrationCost {} => to_json_binary(&get_network_lock_cost(
            deps.storage,
            deps.api,
            env.block.height,
        )?),

        // TODO added for debugging, remove later
        QueryMsg::GetWeights { netuid } => {
            to_json_binary(&get_network_weights(deps.storage, netuid)?)
        }
        QueryMsg::GetState {} => to_json_binary(&get_state_info(deps.storage)?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let storage_version: ContractVersion = get_contract_version(deps.storage)?;

    // Only migrate if newer
    if storage_version.version.as_str() < CONTRACT_VERSION {
        // Set contract to version to latest
        set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    }

    Ok(Response::new().add_attribute("action", "migrate"))
}
