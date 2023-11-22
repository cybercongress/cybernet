use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const ROOT: Item<Addr> = Item::new("root");

// --- ITEM ( percentage ) // TODO change to decimal
// pub const SENATE_REQUIRED_STAKE_PERCENTAGE: Item<u64> = Item::new("senate_required_stake_percentage");

// ============================
// ==== Staking + Accounts ====
// ============================

// --- ITEM ( total_stake )
pub const TOTAL_STAKE: Item<u64> = Item::new("total_stake");
// --- ITEM ( default_take )
pub const DEFAULT_TAKE: Item<u16> = Item::new("default_take");
// --- ITEM ( global_block_emission )
pub const BLOCK_EMISSION: Item<u64> = Item::new("global_block_emission");
// --- ITEM ( total_issuance )
pub const TOTAL_ISSUANCE: Item<u64> = Item::new("total_issuance");
// --- MAP ( hot ) --> stake | Returns the total amount of stake under a hotkey.
pub const TOTAL_HOTKEY_STAKE: Map<&Addr, u64> = Map::new("total_hotkey_stake");
// --- MAP ( cold ) --> stake | Returns the total amount of stake under a coldkey.
pub const TOTAL_COLDKEY_STAKE: Map<&Addr, u64> = Map::new("total_coldkey_stake");
// --- MAP ( hot ) --> cold | Returns the controlling coldkey for a hotkey.
pub const OWNER: Map<&Addr, Addr> = Map::new("hotkey_coldkey");
// --- MAP ( hot ) --> stake | Returns the hotkey delegation stake. And signals that this key is open for delegation.
pub const DELEGATES: Map<&Addr, u16> = Map::new("hotkey_stake");
// --- DMAP ( hot, cold ) --> stake | Returns the stake under a coldkey prefixed by hotkey.
pub const STAKE: Map<(&Addr, &Addr), u64> = Map::new("staked_hotkey_coldkey");

// =====================================
// ==== Difficulty / Registrations =====
// =====================================

// ---- StorageItem Global Used Work.
pub const USED_WORK: Map<Vec<u8>, u64> = Map::new("global_used_work");
// --- MAP ( netuid ) --> Burn
pub const BURN: Map<u16, u64> = Map::new("burn");
// --- MAP ( netuid ) --> Difficulty
pub const DIFFICULTY: Map<u16, u64> = Map::new("difficulty");
// --- MAP ( netuid ) --> MinBurn
pub const MIN_BURN: Map<u16, u64> = Map::new("min_burn");
// --- MAP ( netuid ) --> MaxBurn
pub const MAX_BURN: Map<u16, u64> = Map::new("max_burn");
// --- MAP ( netuid ) --> MinDifficulty
pub const MIN_DIFFICULTY: Map<u16, u64> = Map::new("min_difficulty");
// --- MAP ( netuid ) --> MaxDifficulty
pub const MAX_DIFFICULTY: Map<u16, u64> = Map::new("max_difficulty");
// --- MAP ( netuid ) -->  Block at last adjustment.
pub const LAST_ADJUSTMENT_BLOCK: Map<u16, u64> = Map::new("last_adjustment_block");
// --- MAP ( netuid ) --> Registrations of this Block.
pub const REGISTRATIONS_THIS_BLOCK: Map<u16, u16> = Map::new("registrations_this_block");
// --- MAP ( netuid ) --> global_max_registrations_per_block
pub const MAX_REGISTRATION_PER_BLOCK: Map<u16, u16> = Map::new("max_registration_per_block");
// --- MAP ( netuid ) --> global_RAO_recycled_for_registration )
pub const RAO_RECYCLED_FOR_REGISTRATION: Map<u16, u64> = Map::new("rao_recycled_for_registration");

// ==============================
// ==== Subnetworks Storage =====
// ==============================

// --- ITEM( total_number_of_existing_networks )
pub const SUBNET_LIMIT: Item<u16> = Item::new("subnet_limit");
// --- ITEM( total_number_of_existing_networks )
pub const TOTAL_NETWORKS: Item<u16> = Item::new("total_networks");
// --- MAP ( netuid ) --> subnetwork_n (Number of UIDs in the network).
pub const SUBNETWORK_N: Map<u16, u16> = Map::new("subnetwork_n");
// --- MAP ( netuid ) --> modality   TEXT: 0, IMAGE: 1, TENSOR: 2
pub const NETWORK_MODALITY: Map<u16, u16> = Map::new("network_modality");
// --- MAP ( netuid ) --> network_is_added
pub const NETWORKS_ADDED: Map<u16, bool> = Map::new("networks_added");
// --- DMAP ( hotkey, netuid ) --> bool
pub const IS_NETWORK_MEMBER: Map<(&Addr, u16), bool> = Map::new("is_network_member");
// --- MAP ( netuid ) --> network_registration_allowed
pub const NETWORK_REGISTRATION_ALLOWED: Map<u16, bool> = Map::new("network_registration_allowed");
// --- MAP ( netuid ) --> block_created
pub const NETWORK_REGISTERED_AT: Map<u16, u64> = Map::new("network_registered_at");
// ITEM( network_immunity_period )
pub const NETWORK_IMMUNITY_PERIOD: Item<u64> = Item::new("network_immunity_period");
// ITEM( network_last_registered_block )
pub const NETWORK_LAST_REGISTERED: Item<u64> = Item::new("network_last_registered");
// ITEM( network_min_allowed_uids )
pub const NETWORK_MIN_ALLOWED_UIDS: Item<u16> = Item::new("network_min_allowed_uids");
// ITEM( min_network_lock_cost )
pub const NETWORK_MIN_LOCK_COST: Item<u64> = Item::new("network_min_lock_cost");
// ITEM( last_network_lock_cost )
pub const NETWORK_LAST_LOCK_COST: Item<u64> = Item::new("network_last_lock_cost");
// ITEM( network_lock_reduction_interval )
pub const NETWORK_LOCK_REDUCTION_INTERVAL: Item<u64> = Item::new("network_lock_reduction_interval");
// ITEM( subnet_owner_cut )
pub const SUBNET_OWNER_CUT: Item<u16> = Item::new("subnet_owner_cut");
// ITEM( network_rate_limit )
pub const NETWORK_RATE_LIMIT: Item<u64> = Item::new("network_rate_limit");

// ==============================
// ==== Subnetwork Features =====
// ==============================

// --- MAP ( netuid ) --> tempo
pub const TEMPO: Map<u16, u16> = Map::new("tempo");
// --- MAP ( netuid ) --> emission_values
pub const EMISSION_VALUES: Map<u16, u64> = Map::new("emission_values");
// --- MAP ( netuid ) --> pending_emission
pub const PENDING_EMISSION: Map<u16, u64> = Map::new("pending_emission");
// --- MAP ( netuid ) --> blocks_since_last_step.
pub const BLOCKS_SINCE_LAST_STEP: Map<u16, u64> = Map::new("blocks_since_last_step");
// --- MAP ( netuid ) --> last_mechanism_step_block
pub const LAST_MECHANISM_STEP_BLOCK: Map<u16, u64> = Map::new("last_mechanism_step_block");
// --- MAP (netuid ) --> subnet_owner
pub const SUBNET_OWNER: Map<u16, Addr> = Map::new("subnet_owner");
// --- MAP (netuid ) --> subnet_locked
pub const SUBNET_LOCKED: Map<u16, u64> = Map::new("subnet_locked");

// =================================
// ==== Axon / Promo Endpoints =====
// =================================

// --- Struct for Axon.
pub type AxonInfoOf = AxonInfo;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AxonInfo {
    pub block: u64,       // --- Axon serving block.
    pub version: u32,     // --- Axon version
    pub ip: u128,         // --- Axon u128 encoded ip address of type v6 or v4.
    pub port: u16,        // --- Axon u16 encoded port.
    pub ip_type: u8,      // --- Axon ip type, 4 for ipv4 and 6 for ipv6.
    pub protocol: u8,     // --- Axon protocol. TCP, UDP, other.
    pub placeholder1: u8, // --- Axon proto placeholder 1.
    pub placeholder2: u8, // --- Axon proto placeholder 1.
}

// --- Struct for Prometheus.
pub type PrometheusInfoOf = PrometheusInfo;
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PrometheusInfo {
    pub block: u64,   // --- Prometheus serving block.
    pub version: u32, // --- Prometheus version.
    pub ip: u128,     // --- Prometheus u128 encoded ip address of type v6 or v4.
    pub port: u16,    // --- Prometheus u16 encoded port.
    pub ip_type: u8,  // --- Prometheus ip type, 4 for ipv4 and 6 for ipv6.
}

// --- ITEM ( tx_rate_limit )
pub const TX_RATE_LIMIT: Item<u64> = Item::new("tx_rate_limit");
// --- MAP ( key ) --> last_block
pub const LAST_TX_BLOCK: Map<&Addr, u64> = Map::new("last_tx_block");
// --- MAP ( netuid ) --> serving_rate_limit
pub const SERVING_RATE_LIMIT: Map<u16, u64> = Map::new("serving_rate_limit");
// --- MAP ( netuid, hotkey ) --> axon_info
pub const AXONS: Map<(u16, &Addr), AxonInfo> = Map::new("axon_info");
// --- MAP ( netuid, hotkey ) --> prometheus_info
pub const PROMETHEUS: Map<(u16, &Addr), PrometheusInfo> = Map::new("prometheus_info");

// =======================================
// ==== Subnetwork Hyperparam storage ====
// =======================================

// --- MAP ( netuid ) --> Rho
pub const RHO: Map<u16, u16> = Map::new("rho");
// --- MAP ( netuid ) --> Kappa
pub const KAPPA: Map<u16, u16> = Map::new("kappa");
// --- MAP ( netuid ) --> uid, we use to record uids to prune at next epoch.
pub const NEURONS_TO_PRUNE_AT_NEXT_EPOCH: Map<u16, u16> =
    Map::new("neurons_to_prunet_at_next_epoch");
// --- MAP ( netuid ) --> registrations_this_interval
pub const REGISTRATIONS_THIS_INTERVAL: Map<u16, u16> = Map::new("registrations_this_interval");
// --- MAP ( netuid ) --> pow_registrations_this_interval
pub const POW_REGISTRATIONS_THIS_INTERVAL: Map<u16, u16> =
    Map::new("pow_registrations_this_interval");
// --- MAP ( netuid ) --> burn_registrations_this_interval
pub const BURN_REGISTRATIONS_THIS_INTERVAL: Map<u16, u16> =
    Map::new("burn_registrations_this_interval");
// --- MAP ( netuid ) --> max_allowed_uids
pub const MAX_ALLOWED_UIDS: Map<u16, u16> = Map::new("max_allowed_uids");
// --- MAP ( netuid ) --> immunity_period
pub const IMMUNITY_PERIOD: Map<u16, u16> = Map::new("immunity_period");
// --- MAP ( netuid ) --> activity_cutoff
pub const ACTIVITY_CUTOFF: Map<u16, u16> = Map::new("activity_cutoff");
// --- MAP ( netuid ) --> max_weight_limit
pub const MAX_WEIGHTS_LIMIT: Map<u16, u16> = Map::new("max_weights_limit");
// --- MAP ( netuid ) --> weights_version_key
pub const WEIGHTS_VERSION_KEY: Map<u16, u64> = Map::new("weights_version_key");
// --- MAP ( netuid ) --> min_allowed_weights
pub const MIN_ALLOWED_WEIGHTS: Map<u16, u16> = Map::new("min_allowed_weights");
// --- MAP ( netuid ) --> max_allowed_validators
pub const MAX_ALLOWED_VALIDATORS: Map<u16, u16> = Map::new("max_allowed_validators");
// --- MAP ( netuid ) --> adjustment_interval
pub const ADJUSTMENT_INTERVAL: Map<u16, u16> = Map::new("adjustment_interval");
// --- MAP ( netuid ) --> bonds_moving_average
pub const BONDS_MOVING_AVERAGE: Map<u16, u64> = Map::new("bonds_moving_average");
// --- MAP ( netuid ) --> weights_set_rate_limit
pub const WEIGHTS_SET_RATE_LIMIT: Map<u16, u64> = Map::new("weights_set_rate_limit");
// --- MAP ( netuid ) --> validator_prune_len
pub const VALIDATOR_PRUNE_LEN: Map<u16, u64> = Map::new("validator_prune_len");
// --- MAP ( netuid ) --> scaling_law_power
pub const SCALING_LAW_POWER: Map<u16, u16> = Map::new("scaling_law_power");
// --- MAP ( netuid ) --> target_registrations_this_interval
pub const TARGET_REGISTRATIONS_PER_INTERVAL: Map<u16, u16> =
    Map::new("target_registrations_per_interval");
// --- DMAP ( netuid, uid ) --> block_at_registration
pub const BLOCK_AT_REGISTRATION: Map<(u16, u16), u64> = Map::new("block_at_registration");
// --- DMAP ( netuid ) --> adjustment_alpha
pub const ADJUSTMENTS_ALPHA: Map<u16, u64> = Map::new("adjustments_alpha");

// =======================================
// ==== Subnetwork Consensus Storage  ====
// =======================================

// --- DMAP ( netuid, hotkey ) --> uid
pub const UIDS: Map<(u16, &Addr), u16> = Map::new("uids");
// --- DMAP ( netuid, uid ) --> hotkey
pub const KEYS: Map<(u16, u16), Addr> = Map::new("keys");
// --- DMAP ( netuid ) --> (hotkey, se, ve)
pub const LOADED_EMISSION: Map<u16, Vec<(Addr, u64, u64)>> = Map::new("loaded_emission");
// --- MAP ( netuid ) --> active
pub const ACTIVE: Map<u16, Vec<bool>> = Map::new("active");
// --- MAP ( netuid ) --> rank
pub const RANK: Map<u16, Vec<u16>> = Map::new("rank");
// --- MAP ( netuid ) --> trust
pub const TRUST: Map<u16, Vec<u16>> = Map::new("trust");
// --- MAP ( netuid ) --> consensus
pub const CONSENSUS: Map<u16, Vec<u16>> = Map::new("consensus");
// --- MAP ( netuid ) --> incentive
pub const INCENTIVE: Map<u16, Vec<u16>> = Map::new("incentive");
// --- MAP ( netuid ) --> dividends
pub const DIVIDENDS: Map<u16, Vec<u16>> = Map::new("dividends");
// --- MAP ( netuid ) --> emission
pub const EMISSION: Map<u16, Vec<u64>> = Map::new("emission");
// --- MAP ( netuid ) --> last_update
pub const LAST_UPDATE: Map<u16, Vec<u64>> = Map::new("last_update");
// --- MAP ( netuid ) --> validator_trust
pub const VALIDATOR_TRUST: Map<u16, Vec<u16>> = Map::new("validator_trust");
// --- MAP ( netuid ) --> pruning_scores
pub const PRUNING_SCORES: Map<u16, Vec<u16>> = Map::new("pruning_scores");
// --- MAP ( netuid ) --> validator_permit
pub const VALIDATOR_PERMIT: Map<u16, Vec<bool>> = Map::new("validator_permit");
// --- DMAP ( netuid, uid ) --> weights
pub const WEIGHTS: Map<(u16, u16), Vec<(u16, u16)>> = Map::new("weights");
// --- DMAP ( netuid, uid ) --> bonds
pub const BONDS: Map<(u16, u16), Vec<(u16, u16)>> = Map::new("bonds");

pub const ALLOW_FAUCET: Item<bool> = Item::new("allow_faucet");
