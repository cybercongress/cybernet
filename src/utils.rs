use std::ops::Deref;

use cosmwasm_std::{ensure, Addr, Api, DepsMut, Env, MessageInfo, Storage};

use crate::root::if_subnet_exist;
use crate::state::{
    ACTIVE, ACTIVITY_CUTOFF, ADJUSTMENTS_ALPHA, ADJUSTMENT_INTERVAL, BLOCKS_SINCE_LAST_STEP,
    BLOCK_AT_REGISTRATION, BLOCK_EMISSION, BONDS_MOVING_AVERAGE, BURN,
    BURN_REGISTRATIONS_THIS_INTERVAL, CONSENSUS, DEFAULT_TAKE, DIFFICULTY, DIVIDENDS, EMISSION,
    EMISSION_VALUES, IMMUNITY_PERIOD, INCENTIVE, KAPPA, LAST_ADJUSTMENT_BLOCK,
    LAST_MECHANISM_STEP_BLOCK, LAST_TX_BLOCK, LAST_UPDATE, MAX_ALLOWED_UIDS,
    MAX_ALLOWED_VALIDATORS, MAX_BURN, MAX_DIFFICULTY, MAX_REGISTRATION_PER_BLOCK,
    MAX_WEIGHTS_LIMIT, METADATA, MIN_ALLOWED_WEIGHTS, MIN_BURN, MIN_DIFFICULTY,
    NETWORK_IMMUNITY_PERIOD, NETWORK_LOCK_REDUCTION_INTERVAL, NETWORK_MIN_LOCK_COST,
    NETWORK_RATE_LIMIT, NETWORK_REGISTRATION_ALLOWED, PENDING_EMISSION,
    POW_REGISTRATIONS_THIS_INTERVAL, PRUNING_SCORES, RANK, RAO_RECYCLED_FOR_REGISTRATION,
    REGISTRATIONS_THIS_BLOCK, REGISTRATIONS_THIS_INTERVAL, RHO, ROOT, SERVING_RATE_LIMIT,
    SUBNET_LIMIT, SUBNET_LOCKED, SUBNET_OWNER, SUBNET_OWNER_CUT, TARGET_REGISTRATIONS_PER_INTERVAL,
    TEMPO, TOTAL_ISSUANCE, TRUST, TX_RATE_LIMIT, VALIDATOR_PERMIT, VALIDATOR_PRUNE_LEN,
    VALIDATOR_TRUST, WEIGHTS_SET_RATE_LIMIT, WEIGHTS_VERSION_KEY,
};
use crate::uids::get_subnetwork_n;
use crate::ContractError;
use cyber_std::Response;

pub fn ensure_subnet_owner_or_root(
    store: &dyn Storage,
    coldkey: &Addr,
    netuid: u16,
) -> Result<(), ContractError> {
    ensure!(
        if_subnet_exist(store, netuid),
        ContractError::NetworkDoesNotExist {}
    );

    let subnet_owner = SUBNET_OWNER.load(store, netuid);
    let root = ROOT.load(store)?;
    if subnet_owner.is_ok() {
        ensure!(
            subnet_owner.unwrap() == coldkey || root == coldkey,
            ContractError::Unauthorized {}
        );
    } else {
        ensure!(root == coldkey, ContractError::Unauthorized {});
    }
    Ok(())
}

pub fn ensure_root(store: &dyn Storage, address: &Addr) -> Result<(), ContractError> {
    let root = ROOT.load(store)?;
    ensure!(root == address, ContractError::Unauthorized {});
    Ok(())
}

// ========================
// ==== Global Setters ====
// ========================
#[cfg(test)]
pub fn set_tempo(store: &mut dyn Storage, netuid: u16, tempo: u16) {
    TEMPO.save(store, netuid, &tempo).unwrap();
}

#[cfg(test)]
pub fn set_last_adjustment_block(store: &mut dyn Storage, netuid: u16, last_adjustment_block: u64) {
    LAST_ADJUSTMENT_BLOCK
        .save(store, netuid, &last_adjustment_block)
        .unwrap();
}

#[cfg(test)]
pub fn set_blocks_since_last_step(
    store: &mut dyn Storage,
    netuid: u16,
    blocks_since_last_step: u64,
) {
    BLOCKS_SINCE_LAST_STEP
        .save(store, netuid, &blocks_since_last_step)
        .unwrap();
}

#[cfg(test)]
pub fn set_registrations_this_block(
    store: &mut dyn Storage,
    netuid: u16,
    registrations_this_block: u16,
) {
    REGISTRATIONS_THIS_BLOCK
        .save(store, netuid, &registrations_this_block)
        .unwrap();
}

#[cfg(test)]
pub fn set_last_mechanism_step_block(
    store: &mut dyn Storage,
    netuid: u16,
    last_mechanism_step_block: u64,
) {
    LAST_MECHANISM_STEP_BLOCK
        .save(store, netuid, &last_mechanism_step_block)
        .unwrap();
}

#[cfg(test)]
pub fn set_registrations_this_interval(
    store: &mut dyn Storage,
    netuid: u16,
    registrations_this_interval: u16,
) {
    REGISTRATIONS_THIS_INTERVAL
        .save(store, netuid, &registrations_this_interval)
        .unwrap();
}

#[cfg(test)]
pub fn set_pow_registrations_this_interval(
    store: &mut dyn Storage,
    netuid: u16,
    pow_registrations_this_interval: u16,
) {
    POW_REGISTRATIONS_THIS_INTERVAL
        .save(store, netuid, &pow_registrations_this_interval)
        .unwrap();
}

#[cfg(test)]
pub fn set_burn_registrations_this_interval(
    store: &mut dyn Storage,
    netuid: u16,
    burn_registrations_this_interval: u16,
) {
    BURN_REGISTRATIONS_THIS_INTERVAL
        .save(store, netuid, &burn_registrations_this_interval)
        .unwrap();
}

// ========================
// ==== Global Getters ====
// ========================
#[cfg(test)]
pub fn get_total_issuance(store: &dyn Storage) -> u64 {
    TOTAL_ISSUANCE.load(store).unwrap()
}

pub fn get_block_emission(store: &dyn Storage) -> u64 {
    BLOCK_EMISSION.load(store).unwrap()
}

// ==============================
// ==== YumaConsensus params ====
// ==============================
#[cfg(test)]
pub fn get_rank(store: &dyn Storage, netuid: u16) -> Vec<u16> {
    RANK.load(store, netuid).unwrap()
}

#[cfg(test)]
pub fn get_trust(store: &dyn Storage, netuid: u16) -> Vec<u16> {
    TRUST.load(store, netuid).unwrap()
}

pub fn get_active(store: &dyn Storage, netuid: u16) -> Vec<bool> {
    ACTIVE.load(store, netuid).unwrap()
}

#[cfg(test)]
pub fn get_emission(store: &dyn Storage, netuid: u16) -> Vec<u64> {
    EMISSION.load(store, netuid).unwrap()
}

#[cfg(test)]
pub fn get_consensus(store: &dyn Storage, netuid: u16) -> Vec<u16> {
    CONSENSUS.load(store, netuid).unwrap()
}

#[cfg(test)]
pub fn get_incentive(store: &dyn Storage, netuid: u16) -> Vec<u16> {
    INCENTIVE.load(store, netuid).unwrap()
}

#[cfg(test)]
pub fn get_dividends(store: &dyn Storage, netuid: u16) -> Vec<u16> {
    DIVIDENDS.load(store, netuid).unwrap()
}

pub fn get_last_update(store: &dyn Storage, netuid: u16) -> Vec<u64> {
    LAST_UPDATE.load(store, netuid).unwrap()
}

#[cfg(test)]
pub fn get_pruning_score(store: &dyn Storage, netuid: u16) -> Vec<u16> {
    PRUNING_SCORES.load(store, netuid).unwrap()
}

#[cfg(test)]
pub fn get_validator_trust(store: &dyn Storage, netuid: u16) -> Vec<u16> {
    VALIDATOR_TRUST.load(store, netuid).unwrap()
}

pub fn get_validator_permit(store: &dyn Storage, netuid: u16) -> Vec<bool> {
    VALIDATOR_PERMIT.load(store, netuid).unwrap()
}

// ==================================
// ==== YumaConsensus UID params ====
// ==================================
pub fn set_last_update_for_uid(store: &mut dyn Storage, netuid: u16, uid: u16, last_update: u64) {
    let mut updated_last_update_vec = get_last_update(store, netuid);
    if (uid as usize) < updated_last_update_vec.len() {
        updated_last_update_vec[uid as usize] = last_update;
        LAST_UPDATE
            .save(store, netuid, &updated_last_update_vec)
            .unwrap();
    }
}

pub fn set_active_for_uid(store: &mut dyn Storage, netuid: u16, uid: u16, active: bool) {
    let mut updated_active_vec = get_active(store.deref(), netuid);
    if (uid as usize) < updated_active_vec.len() {
        updated_active_vec[uid as usize] = active;
        ACTIVE.save(store, netuid, &updated_active_vec).unwrap();
    }
}

pub fn set_pruning_score_for_uid(
    store: &mut dyn Storage,
    _api: &dyn Api,
    netuid: u16,
    uid: u16,
    pruning_score: u16,
) {
    PRUNING_SCORES
        .update::<_, ContractError>(store, netuid, |v| {
            let mut v = v.unwrap();
            v[uid as usize] = pruning_score;
            Ok(v)
        })
        .unwrap();
}

pub fn set_validator_permit_for_uid(
    store: &mut dyn Storage,
    netuid: u16,
    uid: u16,
    validator_permit: bool,
) {
    let mut updated_validator_permit = get_validator_permit(store, netuid);
    if (uid as usize) < updated_validator_permit.len() {
        updated_validator_permit[uid as usize] = validator_permit;
        VALIDATOR_PERMIT
            .save(store, netuid, &updated_validator_permit)
            .unwrap();
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

#[cfg(test)]
pub fn get_pending_emission(store: &dyn Storage, netuid: u16) -> u64 {
    PENDING_EMISSION.load(store, netuid).unwrap()
}

#[cfg(test)]
pub fn get_last_adjustment_block(store: &dyn Storage, netuid: u16) -> u64 {
    LAST_ADJUSTMENT_BLOCK.load(store, netuid).unwrap()
}

pub fn get_blocks_since_last_step(store: &dyn Storage, netuid: u16) -> u64 {
    BLOCKS_SINCE_LAST_STEP.load(store, netuid).unwrap()
}

// pub fn get_difficulty(store: &dyn Storage, netuid: u16) -> U256 {
//     U256::from(get_difficulty_as_u64(store, netuid))
// }

pub fn get_registrations_this_block(store: &dyn Storage, netuid: u16) -> u16 {
    REGISTRATIONS_THIS_BLOCK
        .load(store, netuid)
        .unwrap_or_default()
}

#[cfg(test)]
pub fn get_last_mechanism_step_block(store: &dyn Storage, netuid: u16) -> u64 {
    LAST_MECHANISM_STEP_BLOCK.load(store, netuid).unwrap()
}

pub fn get_registrations_this_interval(store: &dyn Storage, netuid: u16) -> u16 {
    REGISTRATIONS_THIS_INTERVAL.load(store, netuid).unwrap()
}

#[cfg(test)]
pub fn get_pow_registrations_this_interval(store: &dyn Storage, netuid: u16) -> u16 {
    POW_REGISTRATIONS_THIS_INTERVAL.load(store, netuid).unwrap()
}

#[cfg(test)]
pub fn get_burn_registrations_this_interval(store: &dyn Storage, netuid: u16) -> u16 {
    BURN_REGISTRATIONS_THIS_INTERVAL
        .load(store, netuid)
        .unwrap()
}

pub fn get_neuron_block_at_registration(store: &dyn Storage, netuid: u16, neuron_uid: u16) -> u64 {
    BLOCK_AT_REGISTRATION
        .load(store, (netuid, neuron_uid))
        .unwrap()
}

// ========================
// ==== Rate Limiting =====
// ========================
pub fn set_last_tx_block(store: &mut dyn Storage, key: &Addr, block: u64) {
    LAST_TX_BLOCK.save(store, key, &block).unwrap();
}

pub fn get_last_tx_block(store: &dyn Storage, key: &Addr) -> u64 {
    LAST_TX_BLOCK.load(store, key).unwrap_or_default()
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
    TOTAL_ISSUANCE.update(store, |v| Ok(v.saturating_sub(amount)))
}

pub fn set_subnet_locked_balance(store: &mut dyn Storage, netuid: u16, amount: u64) {
    SUBNET_LOCKED.save(store, netuid, &amount).unwrap();
}

#[cfg(test)]
pub fn get_subnet_locked_balance(store: &mut dyn Storage, netuid: u16) -> u64 {
    SUBNET_LOCKED.load(store, netuid).unwrap()
}

pub fn get_default_take(store: &dyn Storage) -> u16 {
    DEFAULT_TAKE.load(store).unwrap()
}

pub fn do_sudo_set_default_take(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    default_take: u16,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, &info.sender)?;

    DEFAULT_TAKE.save(deps.storage, &default_take)?;
    deps.api.debug(&format!(
        "🛸 DefaultTakeSet ( default_take: {:?} ) ",
        default_take
    ));

    Ok(Response::default()
        .add_attribute("action", "default_take_set")
        .add_attribute("default_take", format!("{}", default_take)))
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
    _env: Env,
    info: MessageInfo,
    tx_rate_limit: u64,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, &info.sender)?;

    TX_RATE_LIMIT.save(deps.storage, &tx_rate_limit)?;

    deps.api.debug(&format!(
        "🛸 TxRateLimitSet ( tx_rate_limit: {:?} ) ",
        tx_rate_limit
    ));

    Ok(Response::default()
        .add_attribute("action", "tx_rate_limit_set")
        .add_attribute("tx_rate_limit", format!("{}", tx_rate_limit)))
}

pub fn get_serving_rate_limit(store: &dyn Storage, netuid: u16) -> u64 {
    SERVING_RATE_LIMIT.load(store, netuid).unwrap()
}

#[cfg(test)]
pub fn set_serving_rate_limit(store: &mut dyn Storage, netuid: u16, rate_limit: u64) {
    SERVING_RATE_LIMIT.save(store, netuid, &rate_limit).unwrap()
}

pub fn do_sudo_set_serving_rate_limit(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    netuid: u16,
    serving_rate_limit: u64,
) -> Result<Response, ContractError> {
    ensure_subnet_owner_or_root(deps.storage, &info.sender, netuid)?;

    SERVING_RATE_LIMIT.save(deps.storage, netuid, &serving_rate_limit)?;

    deps.api.debug(&format!(
        "🛸 ServingRateLimitSet ( serving_rate_limit: {:?} ) ",
        serving_rate_limit
    ));

    Ok(Response::default()
        .add_attribute("active", "serving_rate_limit_set")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute("serving_rate_limit", format!("{}", serving_rate_limit)))
}

#[cfg(test)]
pub fn get_min_difficulty(store: &dyn Storage, netuid: u16) -> u64 {
    MIN_DIFFICULTY.load(store, netuid).unwrap()
}

#[cfg(test)]
pub fn set_min_difficulty(store: &mut dyn Storage, netuid: u16, min_difficulty: u64) {
    MIN_DIFFICULTY.save(store, netuid, &min_difficulty).unwrap()
}

pub fn do_sudo_set_min_difficulty(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    netuid: u16,
    min_difficulty: u64,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, &info.sender)?;

    ensure!(
        if_subnet_exist(deps.storage, netuid),
        ContractError::NetworkDoesNotExist {}
    );

    MIN_DIFFICULTY.save(deps.storage, netuid, &min_difficulty)?;

    deps.api.debug(&format!(
        "🛸 MinDifficultySet ( netuid: {:?} min_difficulty: {:?} ) ",
        netuid, min_difficulty
    ));

    Ok(Response::default()
        .add_attribute("action", "min_difficulty_set")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute("min_diffuculty", format!("{}", min_difficulty)))
}

#[cfg(test)]
pub fn get_max_difficulty(store: &dyn Storage, netuid: u16) -> u64 {
    MAX_DIFFICULTY.load(store, netuid).unwrap()
}

#[cfg(test)]
pub fn set_max_difficulty(store: &mut dyn Storage, netuid: u16, max_difficulty: u64) {
    MAX_DIFFICULTY.save(store, netuid, &max_difficulty).unwrap()
}

pub fn do_sudo_set_max_difficulty(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    netuid: u16,
    max_difficulty: u64,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, &info.sender)?;

    ensure!(
        if_subnet_exist(deps.storage, netuid),
        ContractError::NetworkDoesNotExist {}
    );

    MAX_DIFFICULTY.save(deps.storage, netuid, &max_difficulty)?;

    deps.api.debug(&format!(
        "🛸 MaxDifficultySet ( netuid: {:?} max_difficulty: {:?} ) ",
        netuid, max_difficulty
    ));

    Ok(Response::default()
        .add_attribute("active", "max_difficulty_set")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute("max_difficulty", format!("{}", max_difficulty)))
}

#[cfg(test)]
pub fn get_weights_version_key(store: &dyn Storage, netuid: u16) -> u64 {
    WEIGHTS_VERSION_KEY.load(store, netuid).unwrap()
}

#[cfg(test)]
pub fn set_weights_version_key(store: &mut dyn Storage, netuid: u16, version_key: u64) {
    WEIGHTS_VERSION_KEY
        .save(store, netuid, &version_key)
        .unwrap()
}

pub fn do_sudo_set_weights_version_key(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    netuid: u16,
    weights_version_key: u64,
) -> Result<Response, ContractError> {
    ensure_subnet_owner_or_root(deps.storage, &info.sender, netuid)?;

    WEIGHTS_VERSION_KEY.save(deps.storage, netuid, &weights_version_key)?;
    deps.api.debug(&format!(
        "🛸 WeightsVersionKeySet ( netuid: {:?} weights_version_key: {:?} ) ",
        netuid, weights_version_key
    ));

    Ok(Response::default()
        .add_attribute("action", "weights_version_key_set")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute("weights_version_key", format!("{}", weights_version_key)))
}

pub fn get_weights_set_rate_limit(store: &dyn Storage, netuid: u16) -> u64 {
    WEIGHTS_SET_RATE_LIMIT.load(store, netuid).unwrap()
}

#[cfg(test)]
pub fn set_weights_set_rate_limit(store: &mut dyn Storage, netuid: u16, rate_limit: u64) {
    WEIGHTS_SET_RATE_LIMIT
        .save(store, netuid, &rate_limit)
        .unwrap()
}

pub fn do_sudo_set_weights_set_rate_limit(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    netuid: u16,
    weights_set_rate_limit: u64,
) -> Result<Response, ContractError> {
    ensure_subnet_owner_or_root(deps.storage, &info.sender, netuid)?;

    WEIGHTS_SET_RATE_LIMIT.save(deps.storage, netuid, &weights_set_rate_limit)?;

    deps.api.debug(&format!(
        "🛸 WeightsSetRateLimitSet ( netuid: {:?} weights_set_rate_limit: {:?} ) ",
        netuid, weights_set_rate_limit
    ));

    Ok(Response::default()
        .add_attribute("active", "weights_set_rate_limit_set")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute(
            "weights_set_rate_limit",
            format!("{}", weights_set_rate_limit),
        ))
}

#[cfg(test)]
pub fn get_adjustment_interval(store: &dyn Storage, netuid: u16) -> u16 {
    ADJUSTMENT_INTERVAL.load(store, netuid).unwrap()
}

#[cfg(test)]
pub fn set_adjustment_interval(store: &mut dyn Storage, netuid: u16, adjustment_interval: u16) {
    ADJUSTMENT_INTERVAL
        .save(store, netuid, &adjustment_interval)
        .unwrap();
}

pub fn do_sudo_set_adjustment_interval(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    netuid: u16,
    adjustment_interval: u16,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, &info.sender)?;

    ensure!(
        if_subnet_exist(deps.storage, netuid),
        ContractError::NetworkDoesNotExist {}
    );

    ADJUSTMENT_INTERVAL.save(deps.storage, netuid, &adjustment_interval)?;

    deps.api.debug(&format!(
        "🛸 AdjustmentIntervalSet ( netuid: {:?} adjustment_interval: {:?} ) ",
        netuid, adjustment_interval
    ));

    Ok(Response::default()
        .add_attribute("active", "adjustments_interval_set")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute("adjustment_interval", format!("{}", adjustment_interval)))
}

#[cfg(test)]
pub fn get_adjustment_alpha(store: &dyn Storage, netuid: u16) -> u64 {
    ADJUSTMENTS_ALPHA.load(store, netuid).unwrap()
}

#[cfg(test)]
pub fn set_adjustment_alpha(store: &mut dyn Storage, netuid: u16, adjustments_alpha: u64) {
    ADJUSTMENTS_ALPHA
        .save(store, netuid, &adjustments_alpha)
        .unwrap();
}

pub fn do_sudo_set_adjustment_alpha(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    netuid: u16,
    adjustment_alpha: u64,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, &info.sender)?;

    ensure!(
        if_subnet_exist(deps.storage, netuid),
        ContractError::NetworkDoesNotExist {}
    );

    ADJUSTMENTS_ALPHA.save(deps.storage, netuid, &adjustment_alpha)?;

    deps.api.debug(&format!(
        "🛸 AdjustmentAlphaSet ( adjustment_alpha: {:?} ) ",
        adjustment_alpha
    ));

    Ok(Response::default()
        .add_attribute("active", "adjustment_alpha_set")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute("adjustment_alpha", format!("{}", adjustment_alpha)))
}

#[cfg(test)]
pub fn get_validator_prune_len(store: &dyn Storage, netuid: u16) -> u64 {
    VALIDATOR_PRUNE_LEN.load(store, netuid).unwrap()
}

pub fn do_sudo_set_validator_prune_len(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    netuid: u16,
    validator_prune_len: u64,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, &info.sender)?;

    ensure!(
        if_subnet_exist(deps.storage, netuid),
        ContractError::NetworkDoesNotExist {}
    );

    VALIDATOR_PRUNE_LEN.save(deps.storage, netuid, &validator_prune_len)?;

    deps.api.debug(&format!(
        "🛸 ValidatorPruneLenSet ( netuid: {:?} validator_prune_len: {:?} ) ",
        netuid, validator_prune_len
    ));

    Ok(Response::default()
        .add_attribute("active", "validator_prune_len_set")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute("validator_prune_len", format!("{}", validator_prune_len)))
}

pub fn get_max_weight_limit(store: &dyn Storage, netuid: u16) -> u16 {
    MAX_WEIGHTS_LIMIT.load(store, netuid).unwrap()
}

#[cfg(test)]
pub fn set_max_weight_limit(store: &mut dyn Storage, netuid: u16, max_weights: u16) {
    MAX_WEIGHTS_LIMIT.save(store, netuid, &max_weights).unwrap()
}

pub fn do_sudo_set_max_weight_limit(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    netuid: u16,
    max_weight_limit: u16,
) -> Result<Response, ContractError> {
    ensure_subnet_owner_or_root(deps.storage, &info.sender, netuid)?;

    MAX_WEIGHTS_LIMIT.save(deps.storage, netuid, &max_weight_limit)?;

    deps.api.debug(&format!(
        "🛸 MaxWeightLimitSet ( netuid: {:?} max_weight_limit: {:?} ) ",
        netuid, max_weight_limit
    ));

    Ok(Response::default()
        .add_attribute("active", "max_weight_limit_set")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute("max_weight_limit", format!("{}", max_weight_limit)))
}

pub fn get_immunity_period(store: &dyn Storage, netuid: u16) -> u16 {
    IMMUNITY_PERIOD.load(store, netuid).unwrap()
}

#[cfg(test)]
pub fn set_immunity_period(store: &mut dyn Storage, netuid: u16, immunity_period: u16) {
    IMMUNITY_PERIOD
        .save(store, netuid, &immunity_period)
        .unwrap()
}

pub fn do_sudo_set_immunity_period(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    netuid: u16,
    immunity_period: u16,
) -> Result<Response, ContractError> {
    ensure_subnet_owner_or_root(deps.storage, &info.sender, netuid)?;

    ensure!(
        immunity_period <= 7200,
        ContractError::StorageValueOutOfRange {}
    );

    IMMUNITY_PERIOD.save(deps.storage, netuid, &immunity_period)?;

    deps.api.debug(&format!(
        "🛸 ImmunityPeriodSet ( netuid: {:?} immunity_period: {:?} ) ",
        netuid, immunity_period
    ));

    Ok(Response::default()
        .add_attribute("active", "immunity_period_set")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute("immunity_period", format!("{}", immunity_period)))
}

#[cfg(test)]
pub fn get_min_allowed_weights(store: &dyn Storage, netuid: u16) -> u16 {
    MIN_ALLOWED_WEIGHTS.load(store, netuid).unwrap()
}

#[cfg(test)]
pub fn set_min_allowed_weights(store: &mut dyn Storage, netuid: u16, min_weights: u16) {
    MIN_ALLOWED_WEIGHTS
        .save(store, netuid, &min_weights)
        .unwrap()
}

pub fn do_sudo_set_min_allowed_weights(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    netuid: u16,
    min_allowed_weights: u16,
) -> Result<Response, ContractError> {
    ensure_subnet_owner_or_root(deps.storage, &info.sender, netuid)?;

    MIN_ALLOWED_WEIGHTS.save(deps.storage, netuid, &min_allowed_weights)?;

    deps.api.debug(&format!(
        "🛸 MinAllowedWeightSet ( netuid: {:?} min_allowed_weights: {:?} ) ",
        netuid, min_allowed_weights
    ));

    Ok(Response::default()
        .add_attribute("active", "min_allowed_weights_set")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute("min_allowed_weights", format!("{}", min_allowed_weights)))
}

pub fn get_max_allowed_uids(store: &dyn Storage, netuid: u16) -> u16 {
    MAX_ALLOWED_UIDS.load(store, netuid).unwrap()
}

#[cfg(test)]
pub fn set_max_allowed_uids(store: &mut dyn Storage, netuid: u16, max_allowed_uids: u16) {
    MAX_ALLOWED_UIDS
        .save(store, netuid, &max_allowed_uids)
        .unwrap()
}

pub fn do_sudo_set_max_allowed_uids(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    netuid: u16,
    max_allowed_uids: u16,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, &info.sender)?;

    ensure!(
        if_subnet_exist(deps.storage, netuid),
        ContractError::NetworkDoesNotExist {}
    );

    ensure!(
        get_subnetwork_n(deps.storage, netuid) < max_allowed_uids,
        ContractError::MaxAllowedUIdsNotAllowed {}
    );

    MAX_ALLOWED_UIDS.save(deps.storage, netuid, &max_allowed_uids)?;

    deps.api.debug(&format!(
        "🛸 MaxAllowedUidsSet ( netuid: {:?} max_allowed_uids: {:?} ) ",
        netuid, max_allowed_uids
    ));

    Ok(Response::default()
        .add_attribute("active", "max_allowed_uids_set")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute("max_allowed_uids", format!("{}", max_allowed_uids)))
}

pub fn get_kappa(store: &dyn Storage, netuid: u16) -> u16 {
    KAPPA.load(store, netuid).unwrap()
}

pub fn do_sudo_set_kappa(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    netuid: u16,
    kappa: u16,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, &info.sender)?;

    ensure!(
        if_subnet_exist(deps.storage, netuid),
        ContractError::NetworkDoesNotExist {}
    );

    KAPPA.save(deps.storage, netuid, &kappa)?;

    deps.api.debug(&format!(
        "🛸 KappaSet ( netuid: {:?} kappa: {:?} ) ",
        netuid, kappa
    ));

    Ok(Response::default()
        .add_attribute("active", "kappa_set")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute("kappa", format!("{}", kappa)))
}

pub fn get_rho(store: &dyn Storage, netuid: u16) -> u16 {
    RHO.load(store, netuid).unwrap()
}

pub fn do_sudo_set_rho(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    netuid: u16,
    rho: u16,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, &info.sender)?;

    ensure!(
        if_subnet_exist(deps.storage, netuid),
        ContractError::NetworkDoesNotExist {}
    );

    RHO.save(deps.storage, netuid, &rho)?;

    deps.api.debug(&format!(
        "🛸 RhoSet ( netuid: {:?} rho: {:?} ) ",
        netuid, rho
    ));

    Ok(Response::default()
        .add_attribute("active", "rho_set")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute("rho", format!("{}", rho)))
}

pub fn get_activity_cutoff(store: &dyn Storage, netuid: u16) -> u16 {
    ACTIVITY_CUTOFF.load(store, netuid).unwrap()
}

#[cfg(test)]
pub fn set_activity_cutoff(store: &mut dyn Storage, netuid: u16, activity_cutoff: u16) {
    ACTIVITY_CUTOFF
        .save(store, netuid, &activity_cutoff)
        .unwrap()
}

pub fn do_sudo_set_activity_cutoff(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    netuid: u16,
    activity_cutoff: u16,
) -> Result<Response, ContractError> {
    ensure_subnet_owner_or_root(deps.storage, &info.sender, netuid)?;

    ACTIVITY_CUTOFF.save(deps.storage, netuid, &activity_cutoff)?;

    deps.api.debug(&format!(
        "🛸 ActivityCutoffSet ( netuid: {:?} activity_cutoff: {:?} ) ",
        netuid, activity_cutoff
    ));

    Ok(Response::default()
        .add_attribute("active", "activity_cutoff_set")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute("activity_cutoff", format!("{}", activity_cutoff)))
}

#[cfg(test)]
pub fn get_network_registration_allowed(store: &dyn Storage, netuid: u16) -> bool {
    NETWORK_REGISTRATION_ALLOWED.load(store, netuid).unwrap()
}

#[cfg(test)]
pub fn set_network_registration_allowed(
    store: &mut dyn Storage,
    netuid: u16,
    registration_allowed: bool,
) {
    NETWORK_REGISTRATION_ALLOWED
        .save(store, netuid, &registration_allowed)
        .unwrap()
}

pub fn do_sudo_set_network_registration_allowed(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    netuid: u16,
    registration_allowed: bool,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, &info.sender)?;

    NETWORK_REGISTRATION_ALLOWED.save(deps.storage, netuid, &registration_allowed)?;

    deps.api.debug(&format!(
        "🛸 NetworkRegistrationAllowed ( registration_allowed: {:?} ) ",
        registration_allowed
    ));

    Ok(Response::default()
        .add_attribute("active", "registration_allowed")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute("registration_allowed", format!("{}", registration_allowed)))
}

pub fn get_target_registrations_per_interval(store: &dyn Storage, netuid: u16) -> u16 {
    TARGET_REGISTRATIONS_PER_INTERVAL
        .load(store, netuid)
        .unwrap()
}

#[cfg(test)]
pub fn set_target_registrations_per_interval(
    store: &mut dyn Storage,
    netuid: u16,
    target_registrations: u16,
) {
    TARGET_REGISTRATIONS_PER_INTERVAL
        .save(store, netuid, &target_registrations)
        .unwrap()
}

pub fn do_sudo_set_target_registrations_per_interval(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    netuid: u16,
    target_registrations_per_interval: u16,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, &info.sender)?;

    ensure!(
        if_subnet_exist(deps.storage, netuid),
        ContractError::NetworkDoesNotExist {}
    );

    TARGET_REGISTRATIONS_PER_INTERVAL.save(
        deps.storage,
        netuid,
        &target_registrations_per_interval,
    )?;

    deps.api.debug(&format!(
        "🛸 RegistrationPerIntervalSet ( netuid: {:?} target_registrations_per_interval: {:?} ) ",
        netuid, target_registrations_per_interval
    ));

    Ok(Response::default()
        .add_attribute("active", "registratoin_per_interval_set")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute(
            "target_registrations_per_interval",
            format!("{}", target_registrations_per_interval),
        ))
}

pub fn get_burn_as_u64(store: &dyn Storage, netuid: u16) -> u64 {
    BURN.load(store, netuid).unwrap()
}

#[cfg(test)]
pub fn set_burn(store: &mut dyn Storage, netuid: u16, burn: u64) {
    BURN.save(store, netuid, &burn).unwrap();
}

#[cfg(test)]
pub fn get_min_burn_as_u64(store: &dyn Storage, netuid: u16) -> u64 {
    MIN_BURN.load(store, netuid).unwrap()
}

pub fn do_sudo_set_min_burn(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    netuid: u16,
    min_burn: u64,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, &info.sender)?;

    ensure!(
        if_subnet_exist(deps.storage, netuid),
        ContractError::NetworkDoesNotExist {}
    );

    MIN_BURN.save(deps.storage, netuid, &min_burn)?;

    deps.api.debug(&format!(
        "🛸 MinBurnSet ( netuid: {:?} min_burn: {:?} ) ",
        netuid, min_burn
    ));

    Ok(Response::default()
        .add_attribute("active", "min_burn_set")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute("min_burn", format!("{}", min_burn)))
}

#[cfg(test)]
pub fn get_max_burn_as_u64(store: &dyn Storage, netuid: u16) -> u64 {
    MAX_BURN.load(store, netuid).unwrap()
}

pub fn do_sudo_set_max_burn(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    netuid: u16,
    max_burn: u64,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, &info.sender)?;

    ensure!(
        if_subnet_exist(deps.storage, netuid),
        ContractError::NetworkDoesNotExist {}
    );

    MAX_BURN.save(deps.storage, netuid, &max_burn)?;

    deps.api.debug(&format!(
        "🛸 MaxBurnSet ( netuid: {:?} max_burn: {:?} ) ",
        netuid, max_burn
    ));

    Ok(Response::default()
        .add_attribute("acitve", "max_burn_set")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute("max_burn", format!("{}", max_burn)))
}

pub fn get_difficulty_as_u64(store: &dyn Storage, netuid: u16) -> u64 {
    DIFFICULTY.load(store, netuid).unwrap()
}

#[cfg(test)]
pub fn set_difficulty(store: &mut dyn Storage, netuid: u16, difficulty: u64) {
    DIFFICULTY.save(store, netuid, &difficulty).unwrap();
}

pub fn do_sudo_set_difficulty(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    netuid: u16,
    difficulty: u64,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, &info.sender)?;

    ensure!(
        if_subnet_exist(deps.storage, netuid),
        ContractError::NetworkDoesNotExist {}
    );

    DIFFICULTY.save(deps.storage, netuid, &difficulty)?;

    deps.api.debug(&format!(
        "🛸 DifficultySet ( netuid: {:?} difficulty: {:?} ) ",
        netuid, difficulty
    ));

    Ok(Response::default()
        .add_attribute("active", "diffuculty_set")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute("difficulty", format!("{}", difficulty)))
}

pub fn get_max_allowed_validators(store: &dyn Storage, netuid: u16) -> u16 {
    MAX_ALLOWED_VALIDATORS.load(store, netuid).unwrap()
}

#[cfg(test)]
pub fn set_max_allowed_validators(store: &mut dyn Storage, netuid: u16, max_allowed: u16) {
    MAX_ALLOWED_VALIDATORS
        .save(store, netuid, &max_allowed)
        .unwrap()
}

pub fn do_sudo_set_max_allowed_validators(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    netuid: u16,
    max_allowed_validators: u16,
) -> Result<Response, ContractError> {
    ensure_subnet_owner_or_root(deps.storage, &info.sender, netuid)?;

    ensure!(
        max_allowed_validators <= get_max_allowed_uids(deps.storage, netuid),
        ContractError::StorageValueOutOfRange {}
    );

    MAX_ALLOWED_VALIDATORS.save(deps.storage, netuid, &max_allowed_validators)?;

    deps.api.debug(&format!(
        "🛸 MaxAllowedValidatorsSet ( netuid: {:?} max_allowed_validators: {:?} ) ",
        netuid, max_allowed_validators
    ));

    Ok(Response::default()
        .add_attribute("active", "max_allowed_validator_set")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute(
            "max_allowed_validators",
            format!("{}", max_allowed_validators),
        ))
}

pub fn get_bonds_moving_average(store: &dyn Storage, netuid: u16) -> u64 {
    BONDS_MOVING_AVERAGE.load(store, netuid).unwrap()
}

pub fn do_sudo_set_bonds_moving_average(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    netuid: u16,
    bonds_moving_average: u64,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, &info.sender)?;

    ensure!(
        if_subnet_exist(deps.storage, netuid),
        ContractError::NetworkDoesNotExist {}
    );

    BONDS_MOVING_AVERAGE.save(deps.storage, netuid, &bonds_moving_average)?;

    deps.api.debug(&format!(
        "🛸 BondsMovingAverageSet ( netuid: {:?} bonds_moving_average: {:?} ) ",
        netuid, bonds_moving_average
    ));

    Ok(Response::default()
        .add_attribute("active", "bonds_moving_average_set")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute("bonds_moving_average", format!("{}", bonds_moving_average)))
}

pub fn get_max_registrations_per_block(store: &dyn Storage, netuid: u16) -> u16 {
    MAX_REGISTRATION_PER_BLOCK.load(store, netuid).unwrap()
}

#[cfg(test)]
pub fn set_max_registrations_per_block(
    store: &mut dyn Storage,
    netuid: u16,
    max_registrations: u16,
) {
    MAX_REGISTRATION_PER_BLOCK
        .save(store, netuid, &max_registrations)
        .unwrap();
}

pub fn do_sudo_set_max_registrations_per_block(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    netuid: u16,
    max_registrations_per_block: u16,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, &info.sender)?;

    ensure!(
        if_subnet_exist(deps.storage, netuid),
        ContractError::NetworkDoesNotExist {}
    );

    MAX_REGISTRATION_PER_BLOCK.save(deps.storage, netuid, &max_registrations_per_block)?;

    deps.api.debug(&format!(
        "🛸 MaxRegistrationsPerBlock ( netuid: {:?} max_registrations_per_block: {:?} ) ",
        netuid, max_registrations_per_block
    ));

    Ok(Response::default()
        .add_attribute("acitve", "max_registration_per_block_set")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute(
            "max_registration_per_block",
            format!("{}", max_registrations_per_block),
        ))
}

pub fn get_subnet_owner(store: &dyn Storage, netuid: u16) -> Addr {
    SUBNET_OWNER.load(store, netuid).unwrap()
}

#[cfg(test)]
pub fn get_subnet_owner_cut(store: &dyn Storage) -> u16 {
    SUBNET_OWNER_CUT.load(store).unwrap()
}

pub fn do_sudo_set_subnet_owner_cut(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    subnet_owner_cut: u16,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, &info.sender)?;

    SUBNET_OWNER_CUT.save(deps.storage, &subnet_owner_cut)?;

    deps.api.debug(&format!(
        "🛸 SubnetOwnerCut ( subnet_owner_cut: {:?} ) ",
        subnet_owner_cut
    ));

    Ok(Response::default()
        .add_attribute("active", "subnet_owner_cut_set")
        .add_attribute("subnet_owner_cut", format!("{}", subnet_owner_cut)))
}

pub fn do_sudo_set_network_rate_limit(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    rate_limit: u64,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, &info.sender)?;

    NETWORK_RATE_LIMIT.save(deps.storage, &rate_limit)?;

    deps.api.debug(&format!(
        "🛸 NetworkRateLimit ( rate_limit: {:?} ) ",
        rate_limit
    ));

    Ok(Response::default()
        .add_attribute("active", "network_rate_limit_set")
        .add_attribute("rate_limit", format!("{}", rate_limit)))
}

pub fn do_sudo_set_tempo(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    netuid: u16,
    tempo: u16,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, &info.sender)?;

    ensure!(
        if_subnet_exist(deps.storage, netuid),
        ContractError::NetworkDoesNotExist {}
    );

    TEMPO.save(deps.storage, netuid, &tempo)?;

    deps.api.debug(&format!(
        "🛸 TempoSet ( netuid: {:?} tempo: {:?} ) ",
        netuid, tempo
    ));

    Ok(Response::default()
        .add_attribute("action", "tempo_set")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute("tempo", format!("{}", tempo)))
}

pub fn do_sudo_set_total_issuance(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    total_issuance: u64,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, &info.sender)?;

    TOTAL_ISSUANCE.save(deps.storage, &total_issuance)?;

    Ok(Response::default()
        .add_attribute("action", "set_total_issuance")
        .add_attribute("total_issuance", format!("{}", total_issuance)))
}

pub fn get_rao_recycled(store: &dyn Storage, netuid: u16) -> u64 {
    RAO_RECYCLED_FOR_REGISTRATION.load(store, netuid).unwrap()
}

pub fn increase_rao_recycled(store: &mut dyn Storage, netuid: u16, inc_rao_recycled: u64) {
    let curr_rao_recycled = get_rao_recycled(store, netuid);
    let rao_recycled = curr_rao_recycled.saturating_add(inc_rao_recycled);
    RAO_RECYCLED_FOR_REGISTRATION
        .save(store, netuid, &rao_recycled)
        .unwrap();
}

pub fn do_sudo_set_rao_recycled(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    netuid: u16,
    rao_recycled: u64,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, &info.sender)?;

    ensure!(
        if_subnet_exist(deps.storage, netuid),
        ContractError::NetworkDoesNotExist {}
    );

    RAO_RECYCLED_FOR_REGISTRATION.save(deps.storage, netuid, &rao_recycled)?;

    Ok(Response::default()
        .add_attribute("aciton", "rao_recycled_for_registration_set")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute("rao_recycled", format!("{}", rao_recycled)))
}

pub fn do_sudo_set_network_immunity_period(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    immunity_period: u64,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, &info.sender)?;

    NETWORK_IMMUNITY_PERIOD.save(deps.storage, &immunity_period)?;

    deps.api.debug(&format!(
        "🛸 NetworkImmunityPeriod ( period: {:?} ) ",
        immunity_period
    ));

    Ok(Response::default()
        .add_attribute("action", "network_immunity_period_set")
        .add_attribute("immunity_period", format!("{}", immunity_period)))
}

pub fn do_sudo_set_network_min_lock_cost(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    lock_cost: u64,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, &info.sender)?;

    NETWORK_MIN_LOCK_COST.save(deps.storage, &lock_cost)?;

    deps.api.debug(&format!(
        "🛸 NetworkMinLockCost ( lock_cost: {:?} ) ",
        lock_cost
    ));

    Ok(Response::default()
        .add_attribute("action", "netowrk_min_lock_cost_set")
        .add_attribute("lock_cost", format!("{}", lock_cost)))
}

pub fn do_sudo_set_subnet_limit(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    max_subnets: u16,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, &info.sender)?;

    SUBNET_LIMIT.save(deps.storage, &max_subnets)?;

    deps.api.debug(&format!(
        "🛸 SubnetLimit ( max_subnets: {:?} ) ",
        max_subnets
    ));

    Ok(Response::default()
        .add_attribute("action", "subnet_limit_set")
        .add_attribute("max_subnets", format!("{}", max_subnets)))
}

pub fn do_sudo_set_lock_reduction_interval(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    interval: u64,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, &info.sender)?;

    NETWORK_LOCK_REDUCTION_INTERVAL.save(deps.storage, &interval)?;

    deps.api.debug(&format!(
        "🛸 NetworkLockReductionInterval ( interval: {:?} ) ",
        interval
    ));

    Ok(Response::default()
        .add_attribute("action", "network_lock_cost_reduction_set")
        .add_attribute("interval", format!("{}", interval)))
}

// TODO added
pub fn do_sudo_set_validator_permit_for_uid(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    netuid: u16,
    uid: u16,
    validator_permit: bool,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, &info.sender)?;

    set_validator_permit_for_uid(deps.storage, netuid, uid, validator_permit);

    deps.api.debug(&format!(
        "🛸 ValidatorPermit ( netuid: {:?} uid: {:?} validator_permit: {:?} ) ",
        netuid, uid, validator_permit,
    ));

    Ok(Response::default()
        .add_attribute("action", "validator_permit_for_uid_set")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute("uid", format!("{}", uid))
        .add_attribute("validator_permit", format!("{}", validator_permit)))
}

pub fn do_sudo_set_block_emission(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    emission: u64,
) -> Result<Response, ContractError> {
    ensure_root(deps.storage, &info.sender)?;

    BLOCK_EMISSION.save(deps.storage, &emission)?;

    deps.api
        .debug(&format!("🛸 BlockEmission ( emission: {:?} ) ", emission));

    Ok(Response::default()
        .add_attribute("action", "block_emission_set")
        .add_attribute("emission", format!("{}", emission)))
}

pub fn do_sudo_set_subnet_metadata(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    netuid: u16,
    particle: String,
) -> Result<Response, ContractError> {
    ensure_subnet_owner_or_root(deps.storage, &info.sender, netuid)?;
    ensure!(particle.len() == 46, ContractError::MetadataSizeError {});

    METADATA.save(deps.storage, netuid, &particle)?;

    Ok(Response::default()
        .add_attribute("action", "metadata_set")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute("metadata", format!("{}", particle)))
}
