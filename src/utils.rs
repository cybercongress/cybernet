use std::ops::Deref;
use cosmwasm_std::{Addr, Api, DepsMut, ensure, Env, MessageInfo, Response, Storage};
use primitive_types::U256;
use crate::ContractError;
use crate::root::if_subnet_exist;
use crate::state::{ACTIVE, BLOCKS_SINCE_LAST_STEP, LAST_ADJUSTMENT_BLOCK, LAST_MECHANISM_STEP_BLOCK, REGISTRATIONS_THIS_BLOCK, TEMPO, EMISSION, CONSENSUS, INCENTIVE, DIVIDENDS, LAST_UPDATE, TRUST, RANK, PRUNING_SCORES, VALIDATOR_TRUST, VALIDATOR_PERMIT, EMISSION_VALUES, PENDING_EMISSION, REGISTRATIONS_THIS_INTERVAL, POW_REGISTRATIONS_THIS_INTERVAL, BURN_REGISTRATIONS_THIS_INTERVAL, BLOCK_AT_REGISTRATION, SERVING_RATE_LIMIT, MIN_DIFFICULTY, MAX_DIFFICULTY, DEFAULT_TAKE, WEIGHTS_VERSION_KEY, RAO_RECYCLED_FOR_REGISTRATION, TOTAL_ISSUANCE, NETWORK_LOCK_REDUCTION_INTERVAL, SUBNET_LIMIT, NETWORK_MIN_LOCK_COST, NETWORK_IMMUNITY_PERIOD, NETWORK_RATE_LIMIT, SUBNET_OWNER_CUT, MAX_REGISTRATION_PER_BLOCK, BONDS_MOVING_AVERAGE, MAX_ALLOWED_VALIDATORS, MAX_ALLOWED_UIDS, DIFFICULTY, MAX_BURN, MIN_BURN, TARGET_REGISTRATIONS_PER_INTERVAL, NETWORK_REGISTRATION_ALLOWED, ACTIVITY_CUTOFF, RHO, KAPPA, MIN_ALLOWED_WEIGHTS, IMMUNITY_PERIOD, MAX_WEIGHTS_LIMIT, SCALING_LAW_POWER, VALIDATOR_PRUNE_LEN, ADJUSTMENT_INTERVAL, WEIGHTS_SET_RATE_LIMIT, TX_RATE_LIMIT, BLOCK_EMISSION, BURN, ADJUSTMENTS_ALPHA, SUBNETWORK_N, SUBNET_OWNER, LAST_TX_BLOCK, SUBNET_LOCKED, ROOT};
use crate::uids::get_subnetwork_n;

pub fn ensure_subnet_owner_or_root(store: &dyn Storage, coldkey: Addr, netuid: u16) -> Result<(), ContractError> {
    ensure!(if_subnet_exist(store, netuid), ContractError::NetworkDoesNotExist {});
    ensure!(SUBNET_OWNER.load(store, netuid).unwrap() == coldkey, ContractError::Unauthorized {});
    Ok(())
}

pub fn ensure_root(store: &dyn Storage, address: Addr) -> Result<(), ContractError> {
    let root = ROOT.load(store)?;
    ensure!(root == address, ContractError::Unauthorized {});
    Ok(())
}

// ========================
// ==== Global Setters ====
// ========================
pub fn set_tempo(store: &mut dyn Storage, netuid: u16, tempo: u16) {
    TEMPO.save(store, netuid, &tempo).unwrap();
}

pub fn set_last_adjustment_block(store: &mut dyn Storage, netuid: u16, last_adjustment_block: u64) {
    LAST_ADJUSTMENT_BLOCK.save(store, netuid, &last_adjustment_block).unwrap();
}

pub fn set_blocks_since_last_step(store: &mut dyn Storage, netuid: u16, blocks_since_last_step: u64) {
    BLOCKS_SINCE_LAST_STEP.save(store, netuid, &blocks_since_last_step).unwrap();
}

pub fn set_registrations_this_block(store: &mut dyn Storage, netuid: u16, registrations_this_block: u16) {
    REGISTRATIONS_THIS_BLOCK.save(store, netuid, &registrations_this_block).unwrap();
}

pub fn set_last_mechanism_step_block(store: &mut dyn Storage, netuid: u16, last_mechanism_step_block: u64) {
    LAST_MECHANISM_STEP_BLOCK.save(store, netuid, &last_mechanism_step_block).unwrap();
}

pub fn set_registrations_this_interval(store: &mut dyn Storage, netuid: u16, registrations_this_interval: u16) {
    REGISTRATIONS_THIS_INTERVAL.save(store, netuid, &registrations_this_interval).unwrap();
}

pub fn set_pow_registrations_this_interval(store: &mut dyn Storage, netuid: u16, pow_registrations_this_interval: u16) {
    POW_REGISTRATIONS_THIS_INTERVAL.save(store, netuid, &pow_registrations_this_interval).unwrap();
}

pub fn set_burn_registrations_this_interval(
    store: &mut dyn Storage,
    netuid: u16,
    burn_registrations_this_interval: u16,
) {
    BURN_REGISTRATIONS_THIS_INTERVAL.save(store, netuid, &burn_registrations_this_interval).unwrap();
}

// ========================
// ==== Global Getters ====
// ========================
pub fn get_total_issuance(store: &dyn Storage) -> u64 {
    TOTAL_ISSUANCE.load(store).unwrap()
}

pub fn get_block_emission(store: &dyn Storage) -> u64 { BLOCK_EMISSION.load(store).unwrap() }

// ==============================
// ==== YumaConsensus params ====
// ==============================
pub fn get_rank(store: &dyn Storage, netuid: u16) -> Vec<u16> {
    RANK.load(store, netuid).unwrap()
}

pub fn get_trust(store: &dyn Storage, netuid: u16) -> Vec<u16> { TRUST.load(store, netuid).unwrap() }

pub fn get_active(store: &dyn Storage, netuid: u16) -> Vec<bool> { ACTIVE.load(store, netuid).unwrap() }

pub fn get_emission(store: &dyn Storage, netuid: u16) -> Vec<u64> { EMISSION.load(store, netuid).unwrap() }

pub fn get_consensus(store: &dyn Storage, netuid: u16) -> Vec<u16> { CONSENSUS.load(store, netuid).unwrap() }

pub fn get_incentive(store: &dyn Storage, netuid: u16) -> Vec<u16> { INCENTIVE.load(store, netuid).unwrap() }

pub fn get_dividends(store: &dyn Storage, netuid: u16) -> Vec<u16> { DIVIDENDS.load(store, netuid).unwrap() }

pub fn get_last_update(store: &dyn Storage, netuid: u16) -> Vec<u64> { LAST_UPDATE.load(store, netuid).unwrap() }

pub fn get_pruning_score(store: &dyn Storage, netuid: u16) -> Vec<u16> { PRUNING_SCORES.load(store, netuid).unwrap() }

pub fn get_validator_trust(store: &dyn Storage, netuid: u16) -> Vec<u16> { VALIDATOR_TRUST.load(store, netuid).unwrap() }

pub fn get_validator_permit(store: &dyn Storage, netuid: u16) -> Vec<bool> { VALIDATOR_PERMIT.load(store, netuid).unwrap() }

// ==================================
// ==== YumaConsensus UID params ====
// ==================================
pub fn set_last_update_for_uid(
    store: &mut dyn Storage,
    netuid: u16,
    uid: u16,
    last_update: u64,
) {
    let mut updated_last_update_vec = get_last_update(store, netuid);
    if (uid as usize) < updated_last_update_vec.len() {
        updated_last_update_vec[uid as usize] = last_update;
        LAST_UPDATE.save(store, netuid, &updated_last_update_vec).unwrap();
    }
}

pub fn set_active_for_uid(
    store: &mut dyn Storage,
    netuid: u16,
    uid: u16,
    active: bool,
) {
    let mut updated_active_vec = get_active(store.deref(), netuid);
    if (uid as usize) < updated_active_vec.len() {
        updated_active_vec[uid as usize] = active;
        ACTIVE.save(store, netuid, &updated_active_vec).unwrap();
    }
}

pub fn set_pruning_score_for_uid(
    store: &mut dyn Storage,
    api: &dyn Api,
    netuid: u16,
    uid: u16,
    pruning_score: u16,
) {
    api.debug(&format!("netuid = {:?}", netuid));
    api.debug(&format!(
        "SubnetworkN.load( netuid ) = {:?}",
        SUBNETWORK_N.load(store, netuid).unwrap()
    ));
    api.debug(&format!("uid = {:?}", uid));
    // assert!(uid < SubnetworkN.load(netuid));
    PRUNING_SCORES.update::<_, ContractError>(store, netuid, |v| {
        let mut v = v.unwrap();
        v[uid as usize] = pruning_score;
        Ok(v)
    }).unwrap();
}

pub fn set_validator_permit_for_uid(store: &mut dyn Storage, netuid: u16, uid: u16, validator_permit: bool) {
    let mut updated_validator_permit = get_validator_permit(store, netuid);
    if (uid as usize) < updated_validator_permit.len() {
        updated_validator_permit[uid as usize] = validator_permit;
        VALIDATOR_PERMIT.save(store, netuid, &updated_validator_permit).unwrap();
    }
}

pub fn get_rank_for_uid(store: &dyn Storage, netuid: u16, uid: u16) -> u16 {
    let vec = RANK.load(store, netuid).unwrap();
    if (uid as usize) < vec.len() {
        return vec[uid as usize];
    } else {
        return 0;
    }
}

pub fn get_trust_for_uid(store: &dyn Storage, netuid: u16, uid: u16) -> u16 {
    let vec = TRUST.load(store, netuid).unwrap();
    if (uid as usize) < vec.len() {
        return vec[uid as usize];
    } else {
        return 0;
    }
}

pub fn get_emission_for_uid(store: &dyn Storage, netuid: u16, uid: u16) -> u64 {
    let vec = EMISSION.load(store, netuid).unwrap();
    if (uid as usize) < vec.len() {
        return vec[uid as usize];
    } else {
        return 0;
    }
}

pub fn get_active_for_uid(store: &dyn Storage, netuid: u16, uid: u16) -> bool {
    let vec = ACTIVE.load(store, netuid).unwrap();
    if (uid as usize) < vec.len() {
        return vec[uid as usize];
    } else {
        return false;
    }
}

pub fn get_consensus_for_uid(store: &dyn Storage, netuid: u16, uid: u16) -> u16 {
    let vec = CONSENSUS.load(store, netuid).unwrap();
    if (uid as usize) < vec.len() {
        return vec[uid as usize];
    } else {
        return 0;
    }
}

pub fn get_incentive_for_uid(store: &dyn Storage, netuid: u16, uid: u16) -> u16 {
    let vec = INCENTIVE.load(store, netuid).unwrap();
    if (uid as usize) < vec.len() {
        return vec[uid as usize];
    } else {
        return 0;
    }
}

pub fn get_dividends_for_uid(store: &dyn Storage, netuid: u16, uid: u16) -> u16 {
    let vec = DIVIDENDS.load(store, netuid).unwrap();
    if (uid as usize) < vec.len() {
        return vec[uid as usize];
    } else {
        return 0;
    }
}

pub fn get_last_update_for_uid(store: &dyn Storage, netuid: u16, uid: u16) -> u64 {
    let vec = LAST_UPDATE.load(store, netuid).unwrap();
    if (uid as usize) < vec.len() {
        return vec[uid as usize];
    } else {
        return 0;
    }
}

pub fn get_pruning_score_for_uid(store: &dyn Storage, netuid: u16, uid: u16) -> u16 {
    let vec = PRUNING_SCORES.load(store, netuid).unwrap();
    if (uid as usize) < vec.len() {
        return vec[uid as usize];
    } else {
        return u16::MAX;
    }
}

pub fn get_validator_trust_for_uid(store: &dyn Storage, netuid: u16, uid: u16) -> u16 {
    let vec = VALIDATOR_TRUST.load(store, netuid).unwrap();
    if (uid as usize) < vec.len() {
        return vec[uid as usize];
    } else {
        return 0;
    }
}

pub fn get_validator_permit_for_uid(store: &dyn Storage, netuid: u16, uid: u16) -> bool {
    let vec = VALIDATOR_PERMIT.load(store, netuid).unwrap();
    if (uid as usize) < vec.len() {
        return vec[uid as usize];
    } else {
        return false;
    }
}

// ============================
// ==== Subnetwork Getters ====
// ============================
pub fn get_tempo(store: &dyn Storage, netuid: u16) -> u16 {
    TEMPO.load(store, netuid).unwrap()
}

pub fn get_emission_value(store: &dyn Storage, netuid: u16) -> u64 {
    EMISSION_VALUES.load(store, netuid).unwrap()
}

pub fn get_pending_emission(store: &dyn Storage, netuid: u16) -> u64 {
    PENDING_EMISSION.load(store, netuid).unwrap()
}

pub fn get_last_adjustment_block(store: &dyn Storage, netuid: u16) -> u64 {
    LAST_ADJUSTMENT_BLOCK.load(store, netuid).unwrap()
}

pub fn get_blocks_since_last_step(store: &dyn Storage, netuid: u16) -> u64 {
    BLOCKS_SINCE_LAST_STEP.load(store, netuid).unwrap()
}

pub fn get_difficulty(store: &dyn Storage, netuid: u16) -> U256 {
    U256::from(get_difficulty_as_u64(store, netuid))
}

pub fn get_registrations_this_block(store: &dyn Storage, netuid: u16) -> u16 {
    REGISTRATIONS_THIS_BLOCK.load(store, netuid).unwrap_or_default()
}

pub fn get_last_mechanism_step_block(store: &dyn Storage, netuid: u16) -> u64 {
    LAST_MECHANISM_STEP_BLOCK.load(store, netuid).unwrap()
}

pub fn get_registrations_this_interval(store: &dyn Storage, netuid: u16) -> u16 {
    REGISTRATIONS_THIS_INTERVAL.load(store, netuid).unwrap()
}

pub fn get_pow_registrations_this_interval(store: &dyn Storage, netuid: u16) -> u16 {
    POW_REGISTRATIONS_THIS_INTERVAL.load(store, netuid).unwrap()
}

pub fn get_burn_registrations_this_interval(store: &dyn Storage, netuid: u16) -> u16 {
    BURN_REGISTRATIONS_THIS_INTERVAL.load(store, netuid).unwrap()
}

pub fn get_neuron_block_at_registration(store: &dyn Storage, netuid: u16, neuron_uid: u16) -> u64 {
    BLOCK_AT_REGISTRATION.load(store, (netuid, neuron_uid)).unwrap()
}

// ========================
// ==== Rate Limiting =====
// ========================
pub fn set_last_tx_block(store: &mut dyn Storage, key: Addr, block: u64) {
    LAST_TX_BLOCK.save(store, key, &block).unwrap();
}

pub fn get_last_tx_block(store: &dyn Storage, key: Addr) -> u64 {
    LAST_TX_BLOCK.load(store, key).unwrap()
}

pub fn exceeds_tx_rate_limit(store: &dyn Storage, prev_tx_block: u64, current_block: u64) -> bool {
    let rate_limit: u64 = get_tx_rate_limit(store);
    if rate_limit == 0 || prev_tx_block == 0 {
        return false;
    }

    return current_block - prev_tx_block <= rate_limit;
}

// ========================
// === Token Management ===
// ========================
pub fn burn_tokens(store: &mut dyn Storage, amount: u64) -> Result<u64, ContractError> {
    TOTAL_ISSUANCE.update(store, |v| { Ok(v.saturating_sub(amount)) })
}

pub fn set_subnet_locked_balance(store: &mut dyn Storage, netuid: u16, amount: u64) {
    SUBNET_LOCKED.save(store, netuid, &amount).unwrap();
}

pub fn get_subnet_locked_balance(store: &mut dyn Storage, netuid: u16) -> u64 {
    SUBNET_LOCKED.load(store, netuid).unwrap()
}

pub fn get_default_take(store: &dyn Storage) -> u16 { DEFAULT_TAKE.load(store).unwrap() }

pub fn do_sudo_set_default_take(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    default_take: u16,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, info.sender)?;

    DEFAULT_TAKE.save(deps.storage, &default_take)?;
    deps.api.debug(&format!("DefaultTakeSet( default_take: {:?} ) ", default_take));

    Ok(Response::default()
        .add_attribute("action", "default_take_set")
        .add_attribute("default_take", format!("{}", default_take))
    )
}


// ========================
// ========= Sudo =========
// ========================

// Configure tx rate limiting
pub fn get_tx_rate_limit(store: &dyn Storage) -> u64 {
    TX_RATE_LIMIT.load(store).unwrap()
}

pub fn do_sudo_set_tx_rate_limit(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    tx_rate_limit: u64,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, info.sender)?;

    TX_RATE_LIMIT.save(deps.storage, &tx_rate_limit)?;

    deps.api.debug(&format!("TxRateLimitSet( tx_rate_limit: {:?} ) ", tx_rate_limit));

    Ok(Response::default()
        .add_attribute("action", "tx_rate_limit_set")
        .add_attribute("tx_rate_limit", format!("{}", tx_rate_limit))
    )
}

pub fn get_serving_rate_limit(store: &dyn Storage, netuid: u16) -> u64 {
    SERVING_RATE_LIMIT.load(store, netuid).unwrap()
}

pub fn do_sudo_set_serving_rate_limit(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    netuid: u16,
    serving_rate_limit: u64,
) -> Result<Response, ContractError> {
    ensure_subnet_owner_or_root(deps.storage, info.sender, netuid)?;

    SERVING_RATE_LIMIT.save(deps.storage, netuid, &serving_rate_limit)?;

    deps.api.debug(&format!(
        "ServingRateLimitSet( serving_rate_limit: {:?} ) ",
        serving_rate_limit
    ));

    Ok(Response::default()
        .add_attribute("active", "serving_rate_limit_set")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute("serving_rate_limit", format!("{}", serving_rate_limit))
    )
}

pub fn get_min_difficulty(store: &dyn Storage, netuid: u16) -> u64 {
    MIN_DIFFICULTY.load(store, netuid).unwrap()
}

pub fn do_sudo_set_min_difficulty(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    netuid: u16,
    min_difficulty: u64,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, info.sender)?;

    ensure!(if_subnet_exist(deps.storage, netuid), ContractError::NetworkDoesNotExist {});

    MIN_DIFFICULTY.save(deps.storage, netuid, &min_difficulty)?;

    deps.api.debug(&format!(
        "MinDifficultySet( netuid: {:?} min_difficulty: {:?} ) ",
        netuid,
        min_difficulty
    ));

    Ok(Response::default()
        .add_attribute("action", "min_difficulty_set")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute("min_diffuculty", format!("{}", min_difficulty))
    )
}

pub fn get_max_difficulty(store: &dyn Storage, netuid: u16) -> u64 {
    MAX_DIFFICULTY.load(store, netuid).unwrap()
}

pub fn do_sudo_set_max_difficulty(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    netuid: u16,
    max_difficulty: u64,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, info.sender)?;

    ensure!(if_subnet_exist(deps.storage, netuid), ContractError::NetworkDoesNotExist {});

    MAX_DIFFICULTY.save(deps.storage, netuid, &max_difficulty)?;

    deps.api.debug(&format!(
        "MaxDifficultySet( netuid: {:?} max_difficulty: {:?} ) ",
        netuid,
        max_difficulty
    ));

    Ok(Response::default()
        .add_attribute("active", "max_difficulty_set")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute("max_difficulty", format!("{}", max_difficulty))
    )
}

pub fn get_weights_version_key(store: &dyn Storage, netuid: u16) -> u64 {
    WEIGHTS_VERSION_KEY.load(store, netuid).unwrap()
}

pub fn do_sudo_set_weights_version_key(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    netuid: u16,
    weights_version_key: u64,
) -> Result<Response, ContractError> {
    ensure_subnet_owner_or_root(deps.storage, info.sender, netuid)?;

    ensure!(if_subnet_exist(deps.storage, netuid), ContractError::NetworkDoesNotExist {});

    WEIGHTS_VERSION_KEY.save(deps.storage, netuid, &weights_version_key)?;
    deps.api.debug(&format!(
        "WeightsVersionKeySet( netuid: {:?} weights_version_key: {:?} ) ",
        netuid,
        weights_version_key
    ));

    Ok(Response::default()
        .add_attribute("action", "weights_version_key_set")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute("weights_version_key", format!("{}", weights_version_key))
    )
}

pub fn get_weights_set_rate_limit(store: &dyn Storage, netuid: u16) -> u64 {
    WEIGHTS_SET_RATE_LIMIT.load(store, netuid).unwrap()
}

pub fn do_sudo_set_weights_set_rate_limit(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    netuid: u16,
    weights_set_rate_limit: u64,
) -> Result<Response, ContractError> {
    ensure_subnet_owner_or_root(deps.storage, info.sender, netuid)?;

    ensure!(if_subnet_exist(deps.storage, netuid), ContractError::NetworkDoesNotExist {});

    WEIGHTS_SET_RATE_LIMIT.save(deps.storage, netuid, &weights_set_rate_limit)?;

    deps.api.debug(&format!(
        "WeightsSetRateLimitSet( netuid: {:?} weights_set_rate_limit: {:?} ) ",
        netuid,
        weights_set_rate_limit
    ));

    Ok(Response::default()
        .add_attribute("active", "weights_set_rate_limit_set")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute("weights_set_rate_limit", format!("{}", weights_set_rate_limit))
    )
}

pub fn get_adjustment_interval(store: &dyn Storage, netuid: u16) -> u16 {
    ADJUSTMENT_INTERVAL.load(store, netuid).unwrap()
}

pub fn do_sudo_set_adjustment_interval(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    netuid: u16,
    adjustment_interval: u16,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, info.sender)?;

    ensure!(if_subnet_exist(deps.storage, netuid), ContractError::NetworkDoesNotExist {});

    ADJUSTMENT_INTERVAL.save(deps.storage, netuid, &adjustment_interval)?;

    deps.api.debug(&format!(
        "AdjustmentIntervalSet( netuid: {:?} adjustment_interval: {:?} ) ",
        netuid,
        adjustment_interval
    ));

    Ok(Response::default()
        .add_attribute("active", "adjustments_interval_set")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute("adjustment_interval", format!("{}", adjustment_interval))
    )
}

pub fn get_adjustment_alpha(store: &dyn Storage, netuid: u16) -> u64 {
    ADJUSTMENTS_ALPHA.load(store, netuid).unwrap()
}

pub fn do_sudo_set_adjustment_alpha(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    netuid: u16,
    adjustment_alpha: u64,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, info.sender)?;

    ensure!(if_subnet_exist(deps.storage, netuid), ContractError::NetworkDoesNotExist {});

    ADJUSTMENTS_ALPHA.save(deps.storage, netuid, &adjustment_alpha)?;

    deps.api.debug(&format!(
        "AdjustmentAlphaSet( adjustment_alpha: {:?} ) ",
        adjustment_alpha
    ));

    Ok(Response::default()
        .add_attribute("active", "adjustment_alpha_set")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute("adjustment_alpha", format!("{}", adjustment_alpha))
    )
}

pub fn get_validator_prune_len(store: &dyn Storage, netuid: u16) -> u64 {
    VALIDATOR_PRUNE_LEN.load(store, netuid).unwrap()
}

pub fn do_sudo_set_validator_prune_len(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    netuid: u16,
    validator_prune_len: u64,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, info.sender)?;

    ensure!(if_subnet_exist(deps.storage, netuid), ContractError::NetworkDoesNotExist {});

    VALIDATOR_PRUNE_LEN.save(deps.storage, netuid, &validator_prune_len)?;

    deps.api.debug(&format!(
        "ValidatorPruneLenSet( netuid: {:?} validator_prune_len: {:?} ) ",
        netuid,
        validator_prune_len
    ));

    Ok(Response::default()
        .add_attribute("active", "validator_prune_len_set")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute("validator_prune_len", format!("{}", validator_prune_len))
    )
}

pub fn get_scaling_law_power(store: &dyn Storage, netuid: u16) -> u16 {
    SCALING_LAW_POWER.load(store, netuid).unwrap()
}

pub fn do_sudo_set_scaling_law_power(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    netuid: u16,
    scaling_law_power: u16,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, info.sender)?;

    ensure!(if_subnet_exist(deps.storage, netuid), ContractError::NetworkDoesNotExist {});

    // The scaling law power must be between 0 and 100 => 0% and 100%
    ensure!(scaling_law_power > 100, ContractError::StorageValueOutOfRange {});

    SCALING_LAW_POWER.save(deps.storage, netuid, &scaling_law_power)?;

    deps.api.debug(&format!(
        "ScalingLawPowerSet( netuid: {:?} scaling_law_power: {:?} ) ",
        netuid,
        scaling_law_power
    ));

    Ok(Response::default()
        .add_attribute("active", "scaling_law_power_set")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute("scaling_law_power", format!("{}", scaling_law_power))
    )
}

pub fn get_max_weight_limit(store: &dyn Storage, netuid: u16) -> u16 {
    MAX_WEIGHTS_LIMIT.load(store, netuid).unwrap()
}

pub fn do_sudo_set_max_weight_limit(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    netuid: u16,
    max_weight_limit: u16,
) -> Result<Response, ContractError> {
    ensure_subnet_owner_or_root(deps.storage, info.sender, netuid)?;

    ensure!(if_subnet_exist(deps.storage, netuid), ContractError::NetworkDoesNotExist {});

    MAX_WEIGHTS_LIMIT.save(deps.storage, netuid, &max_weight_limit)?;

    deps.api.debug(&format!(
        "MaxWeightLimitSet( netuid: {:?} max_weight_limit: {:?} ) ",
        netuid,
        max_weight_limit
    ));

    Ok(Response::default()
        .add_attribute("active", "max_weight_limit_set")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute("max_weight_limit", format!("{}", max_weight_limit))
    )
}

pub fn get_immunity_period(store: &dyn Storage, netuid: u16) -> u16 {
    IMMUNITY_PERIOD.load(store, netuid).unwrap()
}

pub fn do_sudo_set_immunity_period(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    netuid: u16,
    immunity_period: u16,
) -> Result<Response, ContractError> {
    ensure_subnet_owner_or_root(deps.storage, info.sender, netuid)?;

    ensure!(if_subnet_exist(deps.storage, netuid), ContractError::NetworkDoesNotExist {});

    ensure!(immunity_period <= 7200 , ContractError::StorageValueOutOfRange {});

    IMMUNITY_PERIOD.save(deps.storage, netuid, &immunity_period)?;

    deps.api.debug(&format!(
        "ImmunityPeriodSet( netuid: {:?} immunity_period: {:?} ) ",
        netuid,
        immunity_period
    ));

    Ok(Response::default()
        .add_attribute("active", "immunity_period_set")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute("immunity_period", format!("{}", immunity_period))
    )
}

pub fn get_min_allowed_weights(store: &dyn Storage, netuid: u16) -> u16 {
    MIN_ALLOWED_WEIGHTS.load(store, netuid).unwrap()
}

pub fn do_sudo_set_min_allowed_weights(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    netuid: u16,
    min_allowed_weights: u16,
) -> Result<Response, ContractError> {
    ensure_subnet_owner_or_root(deps.storage, info.sender, netuid)?;

    ensure!(if_subnet_exist(deps.storage, netuid), ContractError::NetworkDoesNotExist {});

    MIN_ALLOWED_WEIGHTS.save(deps.storage, netuid, &min_allowed_weights)?;

    deps.api.debug(&format!(
        "MinAllowedWeightSet( netuid: {:?} min_allowed_weights: {:?} ) ",
        netuid,
        min_allowed_weights
    ));

    Ok(Response::default()
        .add_attribute("active", "min_allowed_weights_set")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute("min_allowed_weights", format!("{}", min_allowed_weights))
    )
}

pub fn get_max_allowed_uids(store: &dyn Storage, netuid: u16) -> u16 {
    MAX_ALLOWED_UIDS.load(store, netuid).unwrap()
}

pub fn do_sudo_set_max_allowed_uids(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    netuid: u16,
    max_allowed_uids: u16,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, info.sender)?;

    ensure!(if_subnet_exist(deps.storage, netuid), ContractError::NetworkDoesNotExist {});

    ensure!(get_subnetwork_n(deps.storage, netuid) < max_allowed_uids, ContractError::MaxAllowedUIdsNotAllowed {});

    MAX_ALLOWED_UIDS.save(deps.storage, netuid, &max_allowed_uids)?;

    deps.api.debug(&format!(
        "MaxAllowedUidsSet( netuid: {:?} max_allowed_uids: {:?} ) ",
        netuid,
        max_allowed_uids
    ));

    Ok(Response::default()
        .add_attribute("active", "max_allowed_uids_set")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute("max_allowed_uids", format!("{}", max_allowed_uids))
    )
}

pub fn get_kappa(store: &dyn Storage, netuid: u16) -> u16 {
    KAPPA.load(store, netuid).unwrap()
}

pub fn do_sudo_set_kappa(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    netuid: u16,
    kappa: u16,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, info.sender)?;

    ensure!(if_subnet_exist(deps.storage, netuid), ContractError::NetworkDoesNotExist {});

    KAPPA.save(deps.storage, netuid, &kappa)?;

    deps.api.debug(&format!("KappaSet( netuid: {:?} kappa: {:?} ) ", netuid, kappa));

    Ok(Response::default()
        .add_attribute("active", "kappa_set")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute("kappa", format!("{}", kappa))
    )
}

pub fn get_rho(store: &dyn Storage, netuid: u16) -> u16 {
    RHO.load(store, netuid).unwrap()
}

pub fn do_sudo_set_rho(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    netuid: u16,
    rho: u16,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, info.sender)?;

    ensure!(if_subnet_exist(deps.storage, netuid), ContractError::NetworkDoesNotExist {});

    RHO.save(deps.storage, netuid, &rho)?;

    deps.api.debug(&format!("RhoSet( netuid: {:?} rho: {:?} ) ", netuid, rho));

    Ok(Response::default()
        .add_attribute("active", "rho_set")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute("rho", format!("{}", rho))
    )
}

pub fn get_activity_cutoff(store: &dyn Storage, netuid: u16) -> u16 {
    ACTIVITY_CUTOFF.load(store, netuid).unwrap()
}

pub fn do_sudo_set_activity_cutoff(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    netuid: u16,
    activity_cutoff: u16,
) -> Result<Response, ContractError> {
    ensure_subnet_owner_or_root(deps.storage, info.sender, netuid)?;

    ensure!(if_subnet_exist(deps.storage, netuid), ContractError::NetworkDoesNotExist {});

    ACTIVITY_CUTOFF.save(deps.storage, netuid, &activity_cutoff)?;

    deps.api.debug(&format!(
        "ActivityCutoffSet( netuid: {:?} activity_cutoff: {:?} ) ",
        netuid,
        activity_cutoff
    ));

    Ok(Response::default()
        .add_attribute("active", "activity_cutoff_set")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute("activity_cutoff", format!("{}", activity_cutoff))
    )
}

pub fn get_network_registration_allowed(store: &dyn Storage, netuid: u16) -> bool {
    NETWORK_REGISTRATION_ALLOWED.load(store, netuid).unwrap()
}

pub fn do_sudo_set_network_registration_allowed(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    netuid: u16,
    registration_allowed: bool,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, info.sender)?;

    NETWORK_REGISTRATION_ALLOWED.save(deps.storage, netuid, &registration_allowed)?;

    deps.api.debug(&format!(
        "NetworkRegistrationAllowed( registration_allowed: {:?} ) ",
        registration_allowed
    ));

    Ok(Response::default()
        .add_attribute("active", "registration_allowed")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute("registration_allowed", format!("{}", registration_allowed))
    )
}

pub fn get_target_registrations_per_interval(store: &dyn Storage, netuid: u16) -> u16 {
    TARGET_REGISTRATIONS_PER_INTERVAL.load(store, netuid).unwrap()
}

pub fn do_sudo_set_target_registrations_per_interval(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    netuid: u16,
    target_registrations_per_interval: u16,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, info.sender)?;

    ensure!(if_subnet_exist(deps.storage, netuid), ContractError::NetworkDoesNotExist {});

    TARGET_REGISTRATIONS_PER_INTERVAL.save(deps.storage, netuid, &target_registrations_per_interval)?;

    deps.api.debug(&format!(
        "RegistrationPerIntervalSet( netuid: {:?} target_registrations_per_interval: {:?} ) ",
        netuid,
        target_registrations_per_interval
    ));

    Ok(Response::default()
        .add_attribute("active", "registratoin_per_interval_set")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute("target_registrations_per_interval", format!("{}", target_registrations_per_interval))
    )
}

pub fn get_burn_as_u64(store: &dyn Storage, netuid: u16) -> u64 {
    BURN.load(store, netuid).unwrap()
}

pub fn set_burn(store: &mut dyn Storage, netuid: u16, burn: u64) {
    BURN.save(store, netuid, &burn).unwrap();
}

pub fn get_min_burn_as_u64(store: &dyn Storage, netuid: u16) -> u64 {
    MIN_BURN.load(store, netuid).unwrap()
}

pub fn do_sudo_set_min_burn(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    netuid: u16,
    min_burn: u64,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, info.sender)?;

    ensure!(if_subnet_exist(deps.storage, netuid), ContractError::NetworkDoesNotExist {});

    MIN_BURN.save(deps.storage, netuid, &min_burn)?;

    deps.api.debug(&format!(
        "MinBurnSet( netuid: {:?} min_burn: {:?} ) ",
        netuid,
        min_burn
    ));

    Ok(Response::default()
        .add_attribute("active", "min_burn_set")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute("min_burn", format!("{}", min_burn))
    )
}

pub fn get_max_burn_as_u64(store: &dyn Storage, netuid: u16) -> u64 {
    MAX_BURN.load(store, netuid).unwrap()
}

pub fn do_sudo_set_max_burn(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    netuid: u16,
    max_burn: u64,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, info.sender)?;

    ensure!(if_subnet_exist(deps.storage, netuid), ContractError::NetworkDoesNotExist {});

    MAX_BURN.save(deps.storage, netuid, &max_burn)?;

    deps.api.debug(&format!(
        "MaxBurnSet( netuid: {:?} max_burn: {:?} ) ",
        netuid,
        max_burn
    ));

    Ok(Response::default()
        .add_attribute("acitve", "max_burn_set")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute("max_burn", format!("{}", max_burn))
    )
}

pub fn get_difficulty_as_u64(store: &dyn Storage, netuid: u16) -> u64 {
    DIFFICULTY.load(store, netuid).unwrap()
}

pub fn set_difficulty(store: &mut dyn Storage, netuid: u16, difficulty: u64) {
    DIFFICULTY.save(store, netuid, &difficulty).unwrap();
}

pub fn do_sudo_set_difficulty(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    netuid: u16,
    difficulty: u64,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, info.sender)?;

    ensure!(if_subnet_exist(deps.storage, netuid), ContractError::NetworkDoesNotExist {});

    DIFFICULTY.save(deps.storage, netuid, &difficulty)?;

    deps.api.debug(&format!(
        "DifficultySet( netuid: {:?} difficulty: {:?} ) ",
        netuid,
        difficulty
    ));

    Ok(Response::default()
        .add_attribute("active", "diffuculty_set")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute("difficulty", format!("{}", difficulty))
    )
}

pub fn get_max_allowed_validators(store: &dyn Storage, netuid: u16) -> u16 {
    MAX_ALLOWED_VALIDATORS.load(store, netuid).unwrap()
}

pub fn do_sudo_set_max_allowed_validators(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    netuid: u16,
    max_allowed_validators: u16,
) -> Result<Response, ContractError> {
    ensure_subnet_owner_or_root(deps.storage, info.sender, netuid)?;

    ensure!(if_subnet_exist(deps.storage, netuid), ContractError::NetworkDoesNotExist {});

    ensure!(max_allowed_validators <= get_max_allowed_uids(deps.storage, netuid), ContractError::StorageValueOutOfRange {});

    MAX_ALLOWED_VALIDATORS.save(deps.storage, netuid, &max_allowed_validators)?;

    deps.api.debug(&format!(
        "MaxAllowedValidatorsSet( netuid: {:?} max_allowed_validators: {:?} ) ",
        netuid,
        max_allowed_validators
    ));

    Ok(Response::default()
        .add_attribute("active", "max_allowed_validator_set")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute("max_allowed_validators", format!("{}", max_allowed_validators))
    )
}

pub fn get_bonds_moving_average(store: &dyn Storage, netuid: u16) -> u64 {
    BONDS_MOVING_AVERAGE.load(store, netuid).unwrap()
}

pub fn do_sudo_set_bonds_moving_average(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    netuid: u16,
    bonds_moving_average: u64,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, info.sender)?;

    ensure!(if_subnet_exist(deps.storage, netuid), ContractError::NetworkDoesNotExist {});

    BONDS_MOVING_AVERAGE.save(deps.storage, netuid, &bonds_moving_average)?;

    deps.api.debug(&format!(
        "BondsMovingAverageSet( netuid: {:?} bonds_moving_average: {:?} ) ",
        netuid,
        bonds_moving_average
    ));

    Ok(Response::default()
        .add_attribute("active", "bonds_moving_average_set")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute("bonds_moving_average", format!("{}", bonds_moving_average))
    )
}

pub fn get_max_registrations_per_block(store: &dyn Storage, netuid: u16) -> u16 {
    MAX_REGISTRATION_PER_BLOCK.load(store, netuid).unwrap()
}

pub fn do_sudo_set_max_registrations_per_block(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    netuid: u16,
    max_registrations_per_block: u16,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, info.sender)?;

    ensure!(if_subnet_exist(deps.storage, netuid), ContractError::NetworkDoesNotExist {});

    MAX_REGISTRATION_PER_BLOCK.save(deps.storage, netuid, &max_registrations_per_block)?;

    deps.api.debug(&format!(
        "MaxRegistrationsPerBlock( netuid: {:?} max_registrations_per_block: {:?} ) ",
        netuid,
        max_registrations_per_block
    ));

    Ok(Response::default()
        .add_attribute("acitve", "max_registration_per_block_set")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute("max_registration_per_block", format!("{}", max_registrations_per_block))
    )
}

pub fn get_subnet_owner(store: &dyn Storage, netuid: u16) -> Addr {
    SUBNET_OWNER.load(store, netuid).unwrap()
}

pub fn get_subnet_owner_cut(store: &dyn Storage) -> u16 {
    SUBNET_OWNER_CUT.load(store).unwrap()
}

pub fn do_sudo_set_subnet_owner_cut(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    subnet_owner_cut: u16,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, info.sender)?;

    SUBNET_OWNER_CUT.save(deps.storage, &subnet_owner_cut)?;

    deps.api.debug(&format!(
        "SubnetOwnerCut( subnet_owner_cut: {:?} ) ",
        subnet_owner_cut
    ));

    Ok(Response::default()
        .add_attribute("active", "subnet_owner_cut_set")
        .add_attribute("subnet_owner_cut", format!("{}", subnet_owner_cut))
    )
}

pub fn do_sudo_set_network_rate_limit(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    rate_limit: u64,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, info.sender)?;

    NETWORK_RATE_LIMIT.save(deps.storage, &rate_limit)?;

    deps.api.debug(&format!(
        "NetworkRateLimit( rate_limit: {:?} ) ",
        rate_limit
    ));

    Ok(Response::default()
        .add_attribute("active", "network_rate_limit_set")
        .add_attribute("rate_limit", format!("{}", rate_limit))
    )
}

pub fn do_sudo_set_tempo(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    netuid: u16,
    tempo: u16,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, info.sender)?;

    ensure!(if_subnet_exist(deps.storage, netuid), ContractError::NetworkDoesNotExist {});

    TEMPO.save(deps.storage, netuid, &tempo)?;

    deps.api.debug(&format!("TempoSet( netuid: {:?} tempo: {:?} ) ", netuid, tempo));

    Ok(Response::default()
        .add_attribute("action", "tempo_set")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute("tempo", format!("{}", tempo))
    )
}

pub fn do_set_total_issuance(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    total_issuance: u64,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, info.sender)?;

    TOTAL_ISSUANCE.save(deps.storage, &total_issuance)?;

    Ok(Response::default()
        .add_attribute("action", "set_total_issuance")
        .add_attribute("total_issuance", format!("{}", total_issuance))
    )
}

pub fn get_rao_recycled(store: &dyn Storage, netuid: u16) -> u64 {
    RAO_RECYCLED_FOR_REGISTRATION.load(store, netuid).unwrap()
}

pub fn do_set_rao_recycled(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    netuid: u16,
    rao_recycled: u64,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, info.sender)?;

    ensure!(if_subnet_exist(deps.storage, netuid), ContractError::NetworkDoesNotExist {});

    RAO_RECYCLED_FOR_REGISTRATION.save(deps.storage, netuid, &rao_recycled)?;

    Ok(Response::default()
        .add_attribute("aciton", "rao_recycled_for_registration_set")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute("rao_recycled", format!("{}", rao_recycled))
    )
}

pub fn do_sudo_set_network_immunity_period(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    immunity_period: u64,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, info.sender)?;

    NETWORK_IMMUNITY_PERIOD.save(deps.storage, &immunity_period)?;

    deps.api.debug(&format!(
        "NetworkImmunityPeriod( period: {:?} ) ",
        immunity_period
    ));

    Ok(Response::default()
        .add_attribute("action", "network_immunity_period_set")
        .add_attribute("immunity_period", format!("{}", immunity_period))
    )
}

pub fn do_sudo_set_network_min_lock_cost(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    lock_cost: u64,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, info.sender)?;

    NETWORK_MIN_LOCK_COST.save(deps.storage, &lock_cost)?;

    deps.api.debug(&format!(
        "NetworkMinLockCost( lock_cost: {:?} ) ",
        lock_cost
    ));

    Ok(Response::default()
        .add_attribute("action", "netowrk_min_lock_cost_set")
        .add_attribute("lock_cost", format!("{}", lock_cost))
    )
}

pub fn do_sudo_set_subnet_limit(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    max_subnets: u16,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, info.sender)?;

    SUBNET_LIMIT.save(deps.storage, &max_subnets)?;

    deps.api.debug(&format!(
        "SubnetLimit( max_subnets: {:?} ) ",
        max_subnets
    ));

    Ok(Response::default()
        .add_attribute("action", "subnet_limit_set")
        .add_attribute("max_subnets", format!("{}", max_subnets))
    )
}

pub fn do_sudo_set_lock_reduction_interval(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    interval: u64,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, info.sender)?;

    NETWORK_LOCK_REDUCTION_INTERVAL.save(deps.storage, &interval)?;

    deps.api.debug(&format!(
        "NetworkLockReductionInterval( interval: {:?} ) ",
        interval
    ));

    Ok(Response::default()
        .add_attribute("action", "network_lock_cost_reduction_set")
        .add_attribute("interval", format!("{}", interval))
    )
}

// TODO added
pub fn do_sudo_set_validator_permit_for_uid(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    netuid: u16,
    uid: u16,
    validator_permit: bool,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, info.sender)?;

    set_validator_permit_for_uid(deps.storage, netuid, uid, validator_permit);

    deps.api.debug(&format!(
        "VALIDATOR_PERMIT( netuid: {:?} uid: {:?} validator_permit: {:?} ) ",
        netuid,
        uid,
        validator_permit,
    ));

    Ok(Response::default()
        .add_attribute("action", "network_lock_cost_reduction_set")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute("uid", format!("{}", uid))
        .add_attribute("validator_permit", format!("{}", validator_permit))
    )
}
