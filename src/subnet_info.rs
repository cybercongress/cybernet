use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Deps, Order, StdResult};

use crate::root::if_subnet_exist;
use crate::state::{
    Metadata, ACTIVITY_CUTOFF, ADJUSTMENT_INTERVAL, BLOCKS_SINCE_LAST_STEP, BONDS_MOVING_AVERAGE, BURN,
    DIFFICULTY, EMISSION_VALUES, IMMUNITY_PERIOD, KAPPA, MAX_ALLOWED_UIDS, MAX_ALLOWED_VALIDATORS,
    MAX_BURN, MAX_DIFFICULTY, MAX_REGISTRATION_PER_BLOCK, MAX_WEIGHTS_LIMIT, METADATA,
    MIN_ALLOWED_WEIGHTS, MIN_BURN, MIN_DIFFICULTY, NETWORKS_ADDED, NETWORK_MODALITY,
    NETWORK_REGISTRATION_ALLOWED, RHO, SUBNET_OWNER, TARGET_REGISTRATIONS_PER_INTERVAL, TEMPO,
    WEIGHTS_SET_RATE_LIMIT, WEIGHTS_VERSION_KEY,
};
use crate::uids::get_subnetwork_n;

#[cw_serde]
pub struct SubnetInfo {
    pub netuid: u16,
    pub rho: u16,
    pub kappa: u16,
    pub difficulty: u64,
    pub immunity_period: u16,
    pub max_allowed_validators: u16,
    pub min_allowed_weights: u16,
    pub max_weights_limit: u16,
    pub subnetwork_n: u16,
    pub max_allowed_uids: u16,
    pub blocks_since_last_step: u64,
    pub tempo: u16,
    pub network_modality: u16,
    pub emission_values: u64,
    pub burn: u64,
    pub owner: Addr,
    pub metadata: Metadata,
}

#[cw_serde]
pub struct SubnetHyperparams {
    pub rho: u16,
    pub kappa: u16,
    pub immunity_period: u16,
    pub min_allowed_weights: u16,
    pub max_weights_limit: u16,
    pub tempo: u16,
    pub min_difficulty: u64,
    pub max_difficulty: u64,
    pub weights_version: u64,
    pub weights_rate_limit: u64,
    pub adjustment_interval: u16,
    pub activity_cutoff: u16,
    pub registration_allowed: bool,
    pub target_regs_per_interval: u16,
    pub min_burn: u64,
    pub max_burn: u64,
    pub bonds_moving_avg: u64,
    pub max_regs_per_block: u16,
}

pub fn get_subnet_info(deps: Deps, netuid: u16) -> StdResult<Option<SubnetInfo>> {
    if !if_subnet_exist(deps.storage, netuid) {
        return Ok(None);
    }

    let rho = RHO.load(deps.storage, netuid)?;
    let kappa = KAPPA.load(deps.storage, netuid)?;
    let difficulty = DIFFICULTY.load(deps.storage, netuid)?;
    let immunity_period = IMMUNITY_PERIOD.load(deps.storage, netuid)?;
    let max_allowed_validators = MAX_ALLOWED_VALIDATORS.load(deps.storage, netuid)?;
    let min_allowed_weights = MIN_ALLOWED_WEIGHTS.load(deps.storage, netuid)?;
    let max_weights_limit = MAX_WEIGHTS_LIMIT.load(deps.storage, netuid)?;
    let subnetwork_n = get_subnetwork_n(deps.storage, netuid);
    let max_allowed_uids = MAX_ALLOWED_UIDS.load(deps.storage, netuid)?;
    let blocks_since_last_step = BLOCKS_SINCE_LAST_STEP.load(deps.storage, netuid)?;
    let tempo = TEMPO.load(deps.storage, netuid)?;
    let network_modality = NETWORK_MODALITY.load(deps.storage, netuid)?;
    let emission_values = EMISSION_VALUES.load(deps.storage, netuid)?;
    let burn = BURN.load(deps.storage, netuid)?;
    let owner = SUBNET_OWNER.load(deps.storage, netuid)?;
    let metadata = METADATA.load(deps.storage, netuid)?;

    return Ok(Some(SubnetInfo {
        rho: rho.into(),
        kappa: kappa.into(),
        difficulty: difficulty.into(),
        immunity_period: immunity_period.into(),
        netuid: netuid.into(),
        max_allowed_validators: max_allowed_validators.into(),
        min_allowed_weights: min_allowed_weights.into(),
        max_weights_limit: max_weights_limit.into(),
        subnetwork_n: subnetwork_n.into(),
        max_allowed_uids: max_allowed_uids.into(),
        blocks_since_last_step: blocks_since_last_step.into(),
        tempo: tempo.into(),
        network_modality: network_modality.into(),
        emission_values: emission_values.into(),
        burn,
        owner: owner.into(),
        metadata: metadata.into(),
    }));
}

pub fn get_subnets_info(deps: Deps) -> StdResult<Vec<Option<SubnetInfo>>> {
    let mut subnet_netuids = Vec::<u16>::new();
    let mut max_netuid: u16 = 0;
    for item in NETWORKS_ADDED.range(deps.storage, None, None, Order::Ascending) {
        let (netuid, added) = item?;
        if added {
            subnet_netuids.push(netuid);
            if netuid > max_netuid {
                max_netuid = netuid;
            }
        }
    }

    let mut subnets_info = Vec::<Option<SubnetInfo>>::new();
    for netuid_ in 0..(max_netuid + 1) {
        if subnet_netuids.contains(&netuid_) {
            subnets_info.push(get_subnet_info(deps, netuid_).unwrap());
        }
    }

    return Ok(subnets_info);
}

pub fn get_subnet_hyperparams(deps: Deps, netuid: u16) -> StdResult<Option<SubnetHyperparams>> {
    if !if_subnet_exist(deps.storage, netuid) {
        return Ok(None);
    }

    let rho = RHO.load(deps.storage, netuid)?;
    let kappa = KAPPA.load(deps.storage, netuid)?;
    // let difficulty = DIFFICULTY.load(deps.storage, netuid)?;
    let immunity_period = IMMUNITY_PERIOD.load(deps.storage, netuid)?;
    let min_allowed_weights = MIN_ALLOWED_WEIGHTS.load(deps.storage, netuid)?;
    let max_weights_limit = MAX_WEIGHTS_LIMIT.load(deps.storage, netuid)?;
    let tempo = TEMPO.load(deps.storage, netuid)?;
    let min_difficulty = MIN_DIFFICULTY.load(deps.storage, netuid)?;
    let max_difficulty = MAX_DIFFICULTY.load(deps.storage, netuid)?;
    let weights_version = WEIGHTS_VERSION_KEY.load(deps.storage, netuid)?;
    let weights_rate_limit = WEIGHTS_SET_RATE_LIMIT.load(deps.storage, netuid)?;
    let adjustment_interval = ADJUSTMENT_INTERVAL.load(deps.storage, netuid)?;
    let activity_cutoff = ACTIVITY_CUTOFF.load(deps.storage, netuid)?;
    let registration_allowed = NETWORK_REGISTRATION_ALLOWED.load(deps.storage, netuid)?;
    let target_regs_per_interval = TARGET_REGISTRATIONS_PER_INTERVAL.load(deps.storage, netuid)?;
    let min_burn = MIN_BURN.load(deps.storage, netuid)?;
    let max_burn = MAX_BURN.load(deps.storage, netuid)?;
    let bonds_moving_avg = BONDS_MOVING_AVERAGE.load(deps.storage, netuid)?;
    let max_regs_per_block = MAX_REGISTRATION_PER_BLOCK.load(deps.storage, netuid)?;

    return Ok(Some(SubnetHyperparams {
        rho: rho.into(),
        kappa: kappa.into(),
        immunity_period: immunity_period.into(),
        min_allowed_weights: min_allowed_weights.into(),
        max_weights_limit: max_weights_limit.into(),
        tempo: tempo.into(),
        min_difficulty: min_difficulty.into(),
        max_difficulty: max_difficulty.into(),
        weights_version: weights_version.into(),
        weights_rate_limit: weights_rate_limit.into(),
        adjustment_interval: adjustment_interval.into(),
        activity_cutoff: activity_cutoff.into(),
        registration_allowed,
        target_regs_per_interval: target_regs_per_interval.into(),
        min_burn: min_burn.into(),
        max_burn: max_burn.into(),
        bonds_moving_avg: bonds_moving_avg.into(),
        max_regs_per_block: max_regs_per_block.into(),
    }));
}
