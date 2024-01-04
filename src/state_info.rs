use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Order, StdResult, Storage};

use crate::state::{
    AxonInfo, PrometheusInfo, ACTIVE, ACTIVITY_CUTOFF, ADJUSTMENTS_ALPHA, ADJUSTMENT_INTERVAL,
    ALLOW_FAUCET, AXONS, BLOCKS_SINCE_LAST_STEP, BLOCK_AT_REGISTRATION, BLOCK_EMISSION, BONDS,
    BONDS_MOVING_AVERAGE, BURN, BURN_REGISTRATIONS_THIS_INTERVAL, CONSENSUS, DEFAULT_TAKE,
    DELEGATES, DIFFICULTY, DIVIDENDS, EMISSION, EMISSION_VALUES, IMMUNITY_PERIOD, INCENTIVE,
    IS_NETWORK_MEMBER, KAPPA, KEYS, LAST_ADJUSTMENT_BLOCK, LAST_MECHANISM_STEP_BLOCK,
    LAST_TX_BLOCK, LAST_UPDATE, LOADED_EMISSION, MAX_ALLOWED_UIDS, MAX_ALLOWED_VALIDATORS,
    MAX_BURN, MAX_DIFFICULTY, MAX_REGISTRATION_PER_BLOCK, MAX_WEIGHTS_LIMIT, MIN_ALLOWED_WEIGHTS,
    MIN_BURN, MIN_DIFFICULTY, NETWORKS_ADDED, NETWORK_IMMUNITY_PERIOD, NETWORK_LAST_LOCK_COST,
    NETWORK_LAST_REGISTERED, NETWORK_LOCK_REDUCTION_INTERVAL, NETWORK_MIN_LOCK_COST,
    NETWORK_MODALITY, NETWORK_RATE_LIMIT, NETWORK_REGISTERED_AT, NETWORK_REGISTRATION_ALLOWED,
    NEURONS_TO_PRUNE_AT_NEXT_EPOCH, OWNER, PENDING_EMISSION, POW_REGISTRATIONS_THIS_INTERVAL,
    PROMETHEUS, PRUNING_SCORES, RANK, RAO_RECYCLED_FOR_REGISTRATION, REGISTRATIONS_THIS_BLOCK,
    REGISTRATIONS_THIS_INTERVAL, RHO, ROOT, SCALING_LAW_POWER, SERVING_RATE_LIMIT, STAKE,
    SUBNETWORK_N, SUBNET_LIMIT, SUBNET_LOCKED, SUBNET_OWNER, SUBNET_OWNER_CUT,
    TARGET_REGISTRATIONS_PER_INTERVAL, TEMPO, TOTAL_COLDKEY_STAKE, TOTAL_HOTKEY_STAKE,
    TOTAL_ISSUANCE, TOTAL_NETWORKS, TOTAL_STAKE, TRUST, TX_RATE_LIMIT, UIDS, USED_WORK,
    VALIDATOR_PERMIT, VALIDATOR_PRUNE_LEN, VALIDATOR_TRUST, WEIGHTS, WEIGHTS_SET_RATE_LIMIT,
    WEIGHTS_VERSION_KEY,
};

#[cw_serde]
pub struct StateInfo {
    root: Addr,
    total_stake: u64,
    default_take: u16,
    global_block_emission: u64,
    total_issuance: u64,
    total_hotkey_stake: Vec<(Addr, u64)>,
    total_coldkey_stake: Vec<(Addr, u64)>,
    hotkey_coldkey: Vec<(Addr, Addr)>,
    hotkey_stake: Vec<(Addr, u16)>,
    staked_hotkey_coldkey: Vec<((Addr, Addr), u64)>,
    global_used_work: Vec<(Vec<u8>, u64)>,
    burn: Vec<(u16, u64)>,
    difficulty: Vec<(u16, u64)>,
    min_burn: Vec<(u16, u64)>,
    max_burn: Vec<(u16, u64)>,
    min_difficulty: Vec<(u16, u64)>,
    max_difficulty: Vec<(u16, u64)>,
    last_adjustment_block: Vec<(u16, u64)>,
    registrations_this_block: Vec<(u16, u16)>,
    max_registration_per_block: Vec<(u16, u16)>,
    rao_recycled_for_registration: Vec<(u16, u64)>,
    subnet_limit: u16,
    total_networks: u16,
    subnetwork_n: Vec<(u16, u16)>,
    network_modality: Vec<(u16, u16)>,
    networks_added: Vec<(u16, bool)>,
    is_network_member: Vec<((Addr, u16), bool)>,
    network_registration_allowed: Vec<(u16, bool)>,
    network_registered_at: Vec<(u16, u64)>,
    network_immunity_period: u64,
    network_last_registered: u64,
    network_min_lock_cost: u64,
    network_last_lock_cost: u64,
    network_lock_reduction_interval: u64,
    subnet_owner_cut: u16,
    network_rate_limit: u64,
    tempo: Vec<(u16, u16)>,
    emission_values: Vec<(u16, u64)>,
    pending_emission: Vec<(u16, u64)>,
    blocks_since_last_step: Vec<(u16, u64)>,
    last_mechanism_step_block: Vec<(u16, u64)>,
    // TODO will fail if subnet don't have owner, need to return Option<Addr>
    subnet_owner: Vec<(u16, Addr)>,
    subnet_locked: Vec<(u16, u64)>,
    tx_rate_limit: u64,
    last_tx_block: Vec<(Addr, u64)>,
    serving_rate_limit: Vec<(u16, u64)>,
    axon_info: Vec<((u16, Addr), AxonInfo)>,
    prometheus_info: Vec<((u16, Addr), PrometheusInfo)>,
    rho: Vec<(u16, u16)>,
    kappa: Vec<(u16, u16)>,
    neurons_to_prunet_at_next_epoch: Vec<(u16, u16)>,
    registrations_this_interval: Vec<(u16, u16)>,
    pow_registrations_this_interval: Vec<(u16, u16)>,
    burn_registrations_this_interval: Vec<(u16, u16)>,
    max_allowed_uids: Vec<(u16, u16)>,
    immunity_period: Vec<(u16, u16)>,
    activity_cutoff: Vec<(u16, u16)>,
    max_weights_limit: Vec<(u16, u16)>,
    weights_version_key: Vec<(u16, u64)>,
    min_allowed_weights: Vec<(u16, u16)>,
    max_allowed_validators: Vec<(u16, u16)>,
    adjustment_interval: Vec<(u16, u16)>,
    bonds_moving_average: Vec<(u16, u64)>,
    weights_set_rate_limit: Vec<(u16, u64)>,
    validator_prune_len: Vec<(u16, u64)>,
    scaling_law_power: Vec<(u16, u16)>,
    target_registrations_per_interval: Vec<(u16, u16)>,
    block_at_registration: Vec<((u16, u16), u64)>,
    adjustments_alpha: Vec<(u16, u64)>,
    uids: Vec<((u16, Addr), u16)>,
    keys: Vec<((u16, u16), Addr)>,
    loaded_emission: Vec<(u16, Vec<(Addr, u64, u64)>)>,
    active: Vec<(u16, Vec<bool>)>,
    rank: Vec<(u16, Vec<u16>)>,
    trust: Vec<(u16, Vec<u16>)>,
    consensus: Vec<(u16, Vec<u16>)>,
    incentive: Vec<(u16, Vec<u16>)>,
    dividends: Vec<(u16, Vec<u16>)>,
    emission: Vec<(u16, Vec<u64>)>,
    last_update: Vec<(u16, Vec<u64>)>,
    validator_trust: Vec<(u16, Vec<u16>)>,
    pruning_scores: Vec<(u16, Vec<u16>)>,
    validator_permit: Vec<(u16, Vec<bool>)>,
    weights: Vec<((u16, u16), Vec<(u16, u16)>)>,
    bonds: Vec<((u16, u16), Vec<(u16, u16)>)>,
    allow_faucet: bool,
}

pub fn get_state_info(store: &dyn Storage) -> StdResult<StateInfo> {
    let root: Addr = ROOT.load(store)?;
    let total_stake: u64 = TOTAL_STAKE.load(store)?;
    let default_take: u16 = DEFAULT_TAKE.load(store)?;
    let global_block_emission: u64 = BLOCK_EMISSION.load(store)?;
    let total_issuance: u64 = TOTAL_ISSUANCE.load(store)?;
    let total_hotkey_stake: Vec<(Addr, u64)> = TOTAL_HOTKEY_STAKE
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let total_coldkey_stake: Vec<(Addr, u64)> = TOTAL_COLDKEY_STAKE
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let hotkey_coldkey: Vec<(Addr, Addr)> = OWNER
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let hotkey_stake: Vec<(Addr, u16)> = DELEGATES
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let staked_hotkey_coldkey: Vec<((Addr, Addr), u64)> = STAKE
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let global_used_work: Vec<(Vec<u8>, u64)> = USED_WORK
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let burn: Vec<(u16, u64)> = BURN
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let difficulty: Vec<(u16, u64)> = DIFFICULTY
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let min_burn: Vec<(u16, u64)> = MIN_BURN
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let max_burn: Vec<(u16, u64)> = MAX_BURN
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let min_difficulty: Vec<(u16, u64)> = MIN_DIFFICULTY
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let max_difficulty: Vec<(u16, u64)> = MAX_DIFFICULTY
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let last_adjustment_block: Vec<(u16, u64)> = LAST_ADJUSTMENT_BLOCK
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let registrations_this_block: Vec<(u16, u16)> = REGISTRATIONS_THIS_BLOCK
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let max_registration_per_block: Vec<(u16, u16)> = MAX_REGISTRATION_PER_BLOCK
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let rao_recycled_for_registration: Vec<(u16, u64)> = RAO_RECYCLED_FOR_REGISTRATION
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let subnet_limit: u16 = SUBNET_LIMIT.load(store)?;
    let total_networks: u16 = TOTAL_NETWORKS.load(store)?;
    let subnetwork_n: Vec<(u16, u16)> = SUBNETWORK_N
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let network_modality: Vec<(u16, u16)> = NETWORK_MODALITY
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let networks_added: Vec<(u16, bool)> = NETWORKS_ADDED
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let is_network_member: Vec<((Addr, u16), bool)> = IS_NETWORK_MEMBER
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let network_registration_allowed: Vec<(u16, bool)> = NETWORK_REGISTRATION_ALLOWED
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let network_registered_at: Vec<(u16, u64)> = NETWORK_REGISTERED_AT
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let network_immunity_period: u64 = NETWORK_IMMUNITY_PERIOD.load(store)?;
    let network_last_registered: u64 = NETWORK_LAST_REGISTERED.load(store)?;
    let network_min_lock_cost: u64 = NETWORK_MIN_LOCK_COST.load(store)?;
    let network_last_lock_cost: u64 = NETWORK_LAST_LOCK_COST.load(store)?;
    let network_lock_reduction_interval: u64 = NETWORK_LOCK_REDUCTION_INTERVAL.load(store)?;
    let subnet_owner_cut: u16 = SUBNET_OWNER_CUT.load(store)?;
    let network_rate_limit: u64 = NETWORK_RATE_LIMIT.load(store)?;
    let tempo: Vec<(u16, u16)> = TEMPO
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let emission_values: Vec<(u16, u64)> = EMISSION_VALUES
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let pending_emission: Vec<(u16, u64)> = PENDING_EMISSION
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let blocks_since_last_step: Vec<(u16, u64)> = BLOCKS_SINCE_LAST_STEP
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let last_mechanism_step_block: Vec<(u16, u64)> = LAST_MECHANISM_STEP_BLOCK
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let subnet_owner: Vec<(u16, Addr)> = SUBNET_OWNER
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let subnet_locked: Vec<(u16, u64)> = SUBNET_LOCKED
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let tx_rate_limit: u64 = TX_RATE_LIMIT.load(store)?;
    let last_tx_block: Vec<(Addr, u64)> = LAST_TX_BLOCK
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let serving_rate_limit: Vec<(u16, u64)> = SERVING_RATE_LIMIT
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let axon_info: Vec<((u16, Addr), AxonInfo)> = AXONS
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let prometheus_info: Vec<((u16, Addr), PrometheusInfo)> = PROMETHEUS
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let rho: Vec<(u16, u16)> = RHO
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let kappa: Vec<(u16, u16)> = KAPPA
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let neurons_to_prunet_at_next_epoch: Vec<(u16, u16)> = NEURONS_TO_PRUNE_AT_NEXT_EPOCH
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let registrations_this_interval: Vec<(u16, u16)> = REGISTRATIONS_THIS_INTERVAL
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let pow_registrations_this_interval: Vec<(u16, u16)> = POW_REGISTRATIONS_THIS_INTERVAL
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let burn_registrations_this_interval: Vec<(u16, u16)> = BURN_REGISTRATIONS_THIS_INTERVAL
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let max_allowed_uids: Vec<(u16, u16)> = MAX_ALLOWED_UIDS
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let immunity_period: Vec<(u16, u16)> = IMMUNITY_PERIOD
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let activity_cutoff: Vec<(u16, u16)> = ACTIVITY_CUTOFF
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let max_weights_limit: Vec<(u16, u16)> = MAX_WEIGHTS_LIMIT
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let weights_version_key: Vec<(u16, u64)> = WEIGHTS_VERSION_KEY
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let min_allowed_weights: Vec<(u16, u16)> = MIN_ALLOWED_WEIGHTS
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let max_allowed_validators: Vec<(u16, u16)> = MAX_ALLOWED_VALIDATORS
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let adjustment_interval: Vec<(u16, u16)> = ADJUSTMENT_INTERVAL
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let bonds_moving_average: Vec<(u16, u64)> = BONDS_MOVING_AVERAGE
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let weights_set_rate_limit: Vec<(u16, u64)> = WEIGHTS_SET_RATE_LIMIT
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let validator_prune_len: Vec<(u16, u64)> = VALIDATOR_PRUNE_LEN
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let scaling_law_power: Vec<(u16, u16)> = SCALING_LAW_POWER
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let target_registrations_per_interval: Vec<(u16, u16)> = TARGET_REGISTRATIONS_PER_INTERVAL
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let block_at_registration: Vec<((u16, u16), u64)> = BLOCK_AT_REGISTRATION
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let adjustments_alpha: Vec<(u16, u64)> = ADJUSTMENTS_ALPHA
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let uids: Vec<((u16, Addr), u16)> = UIDS
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let keys: Vec<((u16, u16), Addr)> = KEYS
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let loaded_emission: Vec<(u16, Vec<(Addr, u64, u64)>)> = LOADED_EMISSION
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let active: Vec<(u16, Vec<bool>)> = ACTIVE
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let rank: Vec<(u16, Vec<u16>)> = RANK
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let trust: Vec<(u16, Vec<u16>)> = TRUST
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let consensus: Vec<(u16, Vec<u16>)> = CONSENSUS
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let incentive: Vec<(u16, Vec<u16>)> = INCENTIVE
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let dividends: Vec<(u16, Vec<u16>)> = DIVIDENDS
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let emission: Vec<(u16, Vec<u64>)> = EMISSION
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let last_update: Vec<(u16, Vec<u64>)> = LAST_UPDATE
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let validator_trust: Vec<(u16, Vec<u16>)> = VALIDATOR_TRUST
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let pruning_scores: Vec<(u16, Vec<u16>)> = PRUNING_SCORES
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let validator_permit: Vec<(u16, Vec<bool>)> = VALIDATOR_PERMIT
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let weights: Vec<((u16, u16), Vec<(u16, u16)>)> = WEIGHTS
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let bonds: Vec<((u16, u16), Vec<(u16, u16)>)> = BONDS
        .range(store, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    let allow_faucet: bool = ALLOW_FAUCET.load(store)?;

    let state = StateInfo {
        root,
        total_stake,
        default_take,
        global_block_emission,
        total_issuance,
        total_hotkey_stake,
        total_coldkey_stake,
        hotkey_coldkey,
        hotkey_stake,
        staked_hotkey_coldkey,
        global_used_work,
        burn,
        difficulty,
        min_burn,
        max_burn,
        min_difficulty,
        max_difficulty,
        last_adjustment_block,
        registrations_this_block,
        max_registration_per_block,
        rao_recycled_for_registration,
        subnet_limit,
        total_networks,
        subnetwork_n,
        network_modality,
        networks_added,
        is_network_member,
        network_registration_allowed,
        network_registered_at,
        network_immunity_period,
        network_last_registered,
        network_min_lock_cost,
        network_last_lock_cost,
        network_lock_reduction_interval,
        subnet_owner_cut,
        network_rate_limit,
        tempo,
        emission_values,
        pending_emission,
        blocks_since_last_step,
        last_mechanism_step_block,
        subnet_owner,
        subnet_locked,
        tx_rate_limit,
        last_tx_block,
        serving_rate_limit,
        axon_info,
        prometheus_info,
        rho,
        kappa,
        neurons_to_prunet_at_next_epoch,
        registrations_this_interval,
        pow_registrations_this_interval,
        burn_registrations_this_interval,
        max_allowed_uids,
        immunity_period,
        activity_cutoff,
        max_weights_limit,
        weights_version_key,
        min_allowed_weights,
        max_allowed_validators,
        adjustment_interval,
        bonds_moving_average,
        weights_set_rate_limit,
        validator_prune_len,
        scaling_law_power,
        target_registrations_per_interval,
        block_at_registration,
        adjustments_alpha,
        uids,
        keys,
        loaded_emission,
        active,
        rank,
        trust,
        consensus,
        incentive,
        dividends,
        emission,
        last_update,
        validator_trust,
        pruning_scores,
        validator_permit,
        weights,
        bonds,
        allow_faucet,
    };
    return Ok(state);
}
