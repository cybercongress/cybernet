use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Order, StdResult, Storage};

use crate::root::if_subnet_exist;
use crate::serving::{get_axon_info, get_prometheus_info};
use crate::state::{AxonInfo, PrometheusInfo, BONDS, OWNER, STAKE, WEIGHTS};
use crate::uids::{get_hotkey_for_net_and_uid, get_subnetwork_n};
use crate::utils::{
    get_active_for_uid, get_consensus_for_uid, get_dividends_for_uid, get_emission_for_uid,
    get_incentive_for_uid, get_last_update_for_uid, get_pruning_score_for_uid, get_rank_for_uid,
    get_trust_for_uid, get_validator_permit_for_uid, get_validator_trust_for_uid,
};

#[cw_serde]
pub struct NeuronInfo {
    hotkey: Addr,
    coldkey: Addr,
    uid: u16,
    netuid: u16,
    active: bool,
    axon_info: AxonInfo,
    prometheus_info: PrometheusInfo,
    stake: Vec<(Addr, u64)>,
    // map of coldkey to stake on this neuron/hotkey (includes delegations)
    rank: u16,
    emission: u64,
    incentive: u16,
    consensus: u16,
    trust: u16,
    validator_trust: u16,
    dividends: u16,
    last_update: u64,
    validator_permit: bool,
    weights: Vec<(u16, u16)>,
    // Vec of (uid, weight)
    bonds: Vec<(u16, u16)>,
    // Vec of (uid, bond)
    pruning_score: u16,
}

#[cw_serde]
pub struct NeuronInfoLite {
    hotkey: Addr,
    coldkey: Addr,
    uid: u16,
    netuid: u16,
    active: bool,
    axon_info: AxonInfo,
    prometheus_info: PrometheusInfo,
    stake: Vec<(Addr, u64)>,
    // map of coldkey to stake on this neuron/hotkey (includes delegations)
    rank: u16,
    emission: u64,
    incentive: u16,
    consensus: u16,
    trust: u16,
    validator_trust: u16,
    dividends: u16,
    last_update: u64,
    validator_permit: bool,
    // has no weights or bonds
    pruning_score: u16,
}

pub fn get_neurons(store: &dyn Storage, netuid: u16) -> StdResult<Vec<NeuronInfo>> {
    if !if_subnet_exist(store, netuid) {
        return Ok(Vec::new());
    }

    let mut neurons = Vec::new();
    let n = get_subnetwork_n(store, netuid);
    for uid in 0..n {
        let uid = uid;
        let netuid = netuid;

        let _neuron = get_neuron_subnet_exists(store, netuid, uid)?;
        let neuron;
        if _neuron.is_none() {
            break; // No more neurons
        } else {
            // No error, hotkey was registered
            neuron = _neuron.expect("Neuron should exist");
        }

        neurons.push(neuron);
    }
    Ok(neurons)
}

fn get_neuron_subnet_exists(
    store: &dyn Storage,
    netuid: u16,
    uid: u16,
) -> StdResult<Option<NeuronInfo>> {
    let _hotkey = get_hotkey_for_net_and_uid(store, netuid, uid);
    let hotkey;
    if _hotkey.is_err() {
        return Ok(None);
    } else {
        // No error, hotkey was registered
        hotkey = _hotkey.expect("Hotkey should exist");
    }

    let axon_info = get_axon_info(store, netuid, &hotkey.clone());

    let prometheus_info = get_prometheus_info(store, netuid, &hotkey.clone());

    let coldkey = OWNER.load(store, &hotkey)?;

    let active = get_active_for_uid(store, netuid, uid as u16);
    let rank = get_rank_for_uid(store, netuid, uid as u16);
    let emission = get_emission_for_uid(store, netuid, uid as u16);
    let incentive = get_incentive_for_uid(store, netuid, uid as u16);
    let consensus = get_consensus_for_uid(store, netuid, uid as u16);
    let trust = get_trust_for_uid(store, netuid, uid as u16);
    let validator_trust = get_validator_trust_for_uid(store, netuid, uid as u16);
    let dividends = get_dividends_for_uid(store, netuid, uid as u16);
    let pruning_score = get_pruning_score_for_uid(store, netuid, uid as u16);
    let last_update = get_last_update_for_uid(store, netuid, uid as u16);
    let validator_permit = get_validator_permit_for_uid(store, netuid, uid as u16);

    let weights = WEIGHTS
        .load(store, (netuid, uid))?
        .iter()
        .filter_map(|(i, w)| if *w > 0 { Some((*i, *w)) } else { None })
        .collect::<Vec<(u16, u16)>>();

    let bonds = BONDS
        .load(store, (netuid, uid))?
        .iter()
        .filter_map(|(i, b)| if *b > 0 { Some((*i, *b)) } else { None })
        .collect::<Vec<(u16, u16)>>();

    let stake = STAKE
        .prefix(&hotkey)
        .range(store, None, None, Order::Ascending)
        .map(|item| item.map(|(address, stake)| (address, stake.into())))
        .collect::<StdResult<_>>()?;

    let neuron = NeuronInfo {
        hotkey: hotkey.clone(),
        coldkey: coldkey.clone(),
        uid: uid.into(),
        netuid: netuid.into(),
        active,
        axon_info,
        prometheus_info,
        stake,
        rank: rank.into(),
        emission: emission.into(),
        incentive: incentive.into(),
        consensus: consensus.into(),
        trust: trust.into(),
        validator_trust: validator_trust.into(),
        dividends: dividends.into(),
        last_update: last_update.into(),
        validator_permit,
        weights,
        bonds,
        pruning_score: pruning_score.into(),
    };

    return Ok(Some(neuron));
}

pub fn get_neuron(store: &dyn Storage, netuid: u16, uid: u16) -> StdResult<Option<NeuronInfo>> {
    if !if_subnet_exist(store, netuid) {
        return Ok(None);
    }

    let neuron = get_neuron_subnet_exists(store, netuid, uid);
    neuron
}

fn get_neuron_lite_subnet_exists(
    store: &dyn Storage,
    netuid: u16,
    uid: u16,
) -> StdResult<Option<NeuronInfoLite>> {
    let _hotkey = get_hotkey_for_net_and_uid(store, netuid, uid);
    let hotkey;
    if _hotkey.is_err() {
        return Ok(None);
    } else {
        // No error, hotkey was registered
        hotkey = _hotkey.expect("Hotkey should exist");
    }

    let axon_info = get_axon_info(store, netuid, &hotkey.clone());

    let prometheus_info = get_prometheus_info(store, netuid, &hotkey.clone());

    let coldkey = OWNER.load(store, &hotkey)?;

    let active = get_active_for_uid(store, netuid, uid as u16);
    let rank = get_rank_for_uid(store, netuid, uid as u16);
    let emission = get_emission_for_uid(store, netuid, uid as u16);
    let incentive = get_incentive_for_uid(store, netuid, uid as u16);
    let consensus = get_consensus_for_uid(store, netuid, uid as u16);
    let trust = get_trust_for_uid(store, netuid, uid as u16);
    let validator_trust = get_validator_trust_for_uid(store, netuid, uid as u16);
    let dividends = get_dividends_for_uid(store, netuid, uid as u16);
    let pruning_score = get_pruning_score_for_uid(store, netuid, uid as u16);
    let last_update = get_last_update_for_uid(store, netuid, uid as u16);
    let validator_permit = get_validator_permit_for_uid(store, netuid, uid as u16);

    let stake = STAKE
        .prefix(&hotkey)
        .range(store, None, None, Order::Ascending)
        .collect::<Result<Vec<(Addr, u64)>, _>>()?;

    let neuron = NeuronInfoLite {
        hotkey: hotkey.clone(),
        coldkey: coldkey.clone(),
        uid: uid.into(),
        netuid: netuid.into(),
        active,
        axon_info,
        prometheus_info,
        stake,
        rank: rank.into(),
        emission: emission.into(),
        incentive: incentive.into(),
        consensus: consensus.into(),
        trust: trust.into(),
        validator_trust: validator_trust.into(),
        dividends: dividends.into(),
        last_update: last_update.into(),
        validator_permit,
        pruning_score: pruning_score.into(),
    };

    return Ok(Some(neuron));
}

pub fn get_neurons_lite(store: &dyn Storage, netuid: u16) -> StdResult<Vec<NeuronInfoLite>> {
    if !if_subnet_exist(store, netuid) {
        return Ok(Vec::new());
    }

    let mut neurons: Vec<NeuronInfoLite> = Vec::new();
    let n = get_subnetwork_n(store, netuid);
    for uid in 0..n {
        let uid = uid;

        let _neuron = get_neuron_lite_subnet_exists(store, netuid, uid)?;
        let neuron;
        if _neuron.is_none() {
            break; // No more neurons
        } else {
            // No error, hotkey was registered
            neuron = _neuron.expect("Neuron should exist");
        }

        neurons.push(neuron);
    }
    Ok(neurons)
}

pub fn get_neuron_lite(
    store: &dyn Storage,
    netuid: u16,
    uid: u16,
) -> StdResult<Option<NeuronInfoLite>> {
    if !if_subnet_exist(store, netuid) {
        return Ok(None);
    }

    let neuron = get_neuron_lite_subnet_exists(store, netuid, uid);
    neuron
}
