use std::collections::HashMap;
use crate::ContractError;
use cosmwasm_std::{Addr, Api, Order, Storage};
use substrate_fixed::types::{I32F32, I64F64, I96F32};

use crate::math::{
    col_clip_sparse, fixed_proportion_to_u16, inplace_col_max_upscale_sparse,
    inplace_col_normalize_sparse, inplace_mask_vector, inplace_normalize, inplace_normalize_64,
    inplace_normalize_using_sum, inplace_row_normalize_sparse, is_topk, is_zero, mask_diag_sparse,
    mask_rows_sparse, mat_ema_sparse, matmul_sparse, matmul_transpose_sparse, row_hadamard_sparse,
    row_sum_sparse, vec_fixed64_to_fixed32, vec_mask_sparse_matrix, vec_max_upscale_to_u16, vecdiv,
    weighted_median_col_sparse,
};
use crate::staking::get_total_stake_for_hotkey;
use crate::state::{
    ACTIVE, BONDS, CONSENSUS, DIVIDENDS, EMISSION, INCENTIVE, KEYS, PRUNING_SCORES, RANK, TRUST,
    VALIDATOR_PERMIT, VALIDATOR_TRUST, WEIGHTS,
};
use crate::uids::{get_stake_for_uid_and_subnetwork, get_subnetwork_n};
use crate::utils::{
    get_activity_cutoff, get_bonds_moving_average, get_kappa, get_last_update,
    get_max_allowed_validators, get_neuron_block_at_registration, get_rho, get_validator_permit,
};

// Calculates reward consensus values, then updates rank, trust, consensus, incentive, dividend, pruning_score, emission and bonds, and
// returns the emissions for uids/hotkeys in a given `netuid`.
//
// # Args:
// 	* 'netuid': ( u16 ):
//         - The network to distribute the emission onto.
//
// 	* 'rao_emission': ( u64 ):
//         - The total emission for the epoch.
//
// 	* 'debug' ( bool ):
// 		- Print debugging outputs.
//
pub fn epoch(
    store: &mut dyn Storage,
    api: &dyn Api,
    netuid: u16,
    token_emission: u64,
    current_block: u64,
) -> Result<Vec<(Addr, u64, u64)>, ContractError> {
    // Get subnetwork size.
    let n: u16 = get_subnetwork_n(store, netuid);
    api.debug(&format!("⚪️ subnet_n: {:?}", n));

    // ======================
    // == Active & updated ==
    // ======================

    // Get current block.
    // let current_block: u64 = get_current_block_as_u64();
    api.debug(&format!("⚪️ current_block: {:?}", current_block));

    // Get activity cutoff.
    let activity_cutoff: u64 = get_activity_cutoff(store, netuid) as u64;
    api.debug(&format!("⚪️ activity_cutoff: {:?}", activity_cutoff));

    // Last update vector.
    let last_update: Vec<u64> = get_last_update(store, netuid);
    api.debug(&format!("⚪️ last_update: {:?}", &last_update));

    // Inactive mask.
    let inactive: Vec<bool> = last_update
        .iter()
        .map(|updated| *updated + activity_cutoff < current_block)
        .collect();
    api.debug(&format!("⚪️ inactive: {:?}", inactive.clone()));

    // Logical negation of inactive.
    let active: Vec<bool> = inactive.iter().map(|&b| !b).collect();

    // Block at registration vector (block when each neuron was most recently registered).
    let block_at_registration: Vec<u64> = get_block_at_registration(store, netuid);
    api.debug(&format!(
        "⚪️ block_at_registration: {:?}",
        &block_at_registration
    ));

    // ===========
    // == Stake ==
    // ===========

    let mut hotkeys: Vec<(u16, Addr)> = vec![];
    for item in KEYS
        .prefix(netuid)
        .range(store, None, None, Order::Ascending)
    {
        let (uid_i, hotkey) = item.unwrap();
        hotkeys.push((uid_i, hotkey));
    }
    api.debug(&format!("⚪️ hotkeys: {:?}", &hotkeys));

    // Access network stake as normalized vector.
    let mut stake_64: Vec<I64F64> = vec![I64F64::from_num(0.0); n as usize];
    for (uid_i, hotkey) in hotkeys.iter() {
        stake_64[*uid_i as usize] = I64F64::from_num(get_total_stake_for_hotkey(store, hotkey));
    }
    inplace_normalize_64(&mut stake_64);
    let stake: Vec<I32F32> = vec_fixed64_to_fixed32(stake_64);
    // range: I32F32(0, 1)
    api.debug(&format!("⚪️ stake: {:?}", &stake));

    // =======================
    // == Validator permits ==
    // =======================

    // Get current validator permits.
    let validator_permits: Vec<bool> = get_validator_permit(store, netuid);
    api.debug(&format!("⚪️ validator_permits: {:?}", validator_permits));

    // Logical negation of validator_permits.
    let validator_forbids: Vec<bool> = validator_permits.iter().map(|&b| !b).collect();

    // Get max allowed validators.
    let max_allowed_validators: u16 = get_max_allowed_validators(store, netuid);
    api.debug(&format!(
        "⚪️ max_allowed_validators: {:?}",
        max_allowed_validators
    ));

    // Get new validator permits.
    let new_validator_permits: Vec<bool> = is_topk(&stake, max_allowed_validators as usize);
    api.debug(&format!(
        "⚪️ new_validator_permits: {:?}",
        new_validator_permits
    ));

    // ==================
    // == Active Stake ==
    // ==================

    let mut active_stake: Vec<I32F32> = stake.clone();

    // Remove inactive stake.
    inplace_mask_vector(&inactive, &mut active_stake);

    // Remove non-validator stake.
    inplace_mask_vector(&validator_forbids, &mut active_stake);

    // Normalize active stake.
    inplace_normalize(&mut active_stake);
    api.debug(&format!("⚪️ stake: {:?}", &active_stake));

    // =============
    // == Weights ==
    // =============

    // Access network weights row unnormalized.
    let mut weights: Vec<Vec<(u16, I32F32)>> = get_weights_sparse(store, netuid);
    // api.debug(&format!("W: {:?}", &weights ));

    // Mask weights that are not from permitted validators.
    weights = mask_rows_sparse(&validator_forbids, &weights);
    // api.debug(&format!("W (permit): {:?}", &weights ));

    // Remove self-weight by masking diagonal.
    weights = mask_diag_sparse(&weights);
    // api.debug(&format!("W (permit+diag): {:?}", &weights ));

    // Remove weights referring to deregistered neurons.
    weights = vec_mask_sparse_matrix(
        &weights,
        &last_update,
        &block_at_registration,
        &|updated, registered| updated <= registered,
    );
    // api.debug(&format!("W (permit+diag+outdate): {:?}", &weights ));

    // Normalize remaining weights.
    inplace_row_normalize_sparse(&mut weights);
    // api.debug(&format!("W (mask+norm): {:?}", &weights ));

    // ================================
    // == Consensus, Validator Trust ==
    // ================================

    // Compute preranks: r_j = SUM(i) w_ij * s_i
    let preranks: Vec<I32F32> = matmul_sparse(&weights, &active_stake, n);
    // api.debug(&format!("R (before): {:?}", &preranks));

    // Clip weights at majority consensus
    let kappa: I32F32 = get_float_kappa(store, netuid); // consensus majority ratio, e.g. 51%.
    let consensus: Vec<I32F32> = weighted_median_col_sparse(&active_stake, &weights, n, kappa);
    api.debug(&format!("⚪️ consensus: {:?}", &consensus));

    weights = col_clip_sparse(&weights, &consensus);
    // api.debug(&format!("W: {:?}", &weights));

    let validator_trust: Vec<I32F32> = row_sum_sparse(&weights);
    api.debug(&format!("⚪️ validator_trust: {:?}", &validator_trust));

    // =============================
    // == Ranks, Trust, Incentive ==
    // =============================

    // Compute ranks: r_j = SUM(i) w_ij * s_i.
    let mut ranks: Vec<I32F32> = matmul_sparse(&weights, &active_stake, n);
    // api.debug(&format!("R (after): {:?}", &ranks));

    // Compute server trust: ratio of rank after vs. rank before.
    let trust: Vec<I32F32> = vecdiv(&ranks, &preranks);
    // range: I32F32(0, 1)
    api.debug(&format!("⚪️ trust: {:?}", &trust));

    inplace_normalize(&mut ranks); // range: I32F32(0, 1)
    let incentive: Vec<I32F32> = ranks.clone();
    api.debug(&format!("⚪️ incentive: {:?}", &incentive));

    // =========================
    // == Bonds and Dividends ==
    // =========================

    // Access network bonds.
    let mut bonds: Vec<Vec<(u16, I32F32)>> = get_bonds_sparse(store, netuid);
    // api.debug(&format!("B: {:?}", &bonds));

    // Remove bonds referring to deregistered neurons.
    bonds = vec_mask_sparse_matrix(
        &bonds,
        &last_update,
        &block_at_registration,
        &|updated, registered| updated <= registered,
    );
    // api.debug(&format!("B (outdatedmask): {:?}", &bonds));

    // Normalize remaining bonds: sum_i b_ij = 1.
    inplace_col_normalize_sparse(&mut bonds, n);
    // api.debug(&format!("B (mask+norm): {:?}", &bonds));

    // Compute bonds delta column normalized.
    // ΔB = W◦S (outdated W masked)
    let mut bonds_delta: Vec<Vec<(u16, I32F32)>> = row_hadamard_sparse(&weights, &active_stake);
    // api.debug(&format!("ΔB: {:?}", &bonds_delta));

    // Normalize bonds delta.
    // sum_i b_ij = 1
    inplace_col_normalize_sparse(&mut bonds_delta, n);
    // api.debug(&format!("ΔB (norm): {:?}", &bonds_delta));

    // Compute bonds moving average.
    let bonds_moving_average: I64F64 =
        I64F64::from_num(get_bonds_moving_average(store, netuid)) / I64F64::from_num(1_000_000);
    let alpha: I32F32 = I32F32::from_num(1) - I32F32::from_num(bonds_moving_average);
    let mut ema_bonds: Vec<Vec<(u16, I32F32)>> = mat_ema_sparse(&bonds_delta, &bonds, alpha);

    // Normalize EMA bonds.
    // sum_i b_ij = 1
    inplace_col_normalize_sparse(&mut ema_bonds, n);
    // api.debug(&format!("emaB: {:?}", &ema_bonds));

    // Compute dividends: d_i = SUM(j) b_ij * inc_j.
    // range: I32F32(0, 1)
    let mut dividends: Vec<I32F32> = matmul_transpose_sparse(&ema_bonds, &incentive);
    inplace_normalize(&mut dividends);
    api.debug(&format!("⚪️ dividends: {:?}", &dividends));

    // =================================
    // == Emission and Pruning scores ==
    // =================================

    // Compute normalized emission scores. range: I32F32(0, 1)
    let combined_emission: Vec<I32F32> = incentive
        .iter()
        .zip(dividends.clone())
        .map(|(ii, di)| ii + di)
        .collect();
    let emission_sum: I32F32 = combined_emission.iter().sum();

    let mut normalized_server_emission: Vec<I32F32> = incentive.clone(); // Servers get incentive.
    let mut normalized_validator_emission: Vec<I32F32> = dividends.clone(); // Validators get dividends.
    let mut normalized_combined_emission: Vec<I32F32> = combined_emission.clone();
    // Normalize on the sum of incentive + dividends.
    inplace_normalize_using_sum(&mut normalized_server_emission, emission_sum);
    inplace_normalize_using_sum(&mut normalized_validator_emission, emission_sum);
    inplace_normalize(&mut normalized_combined_emission);

    // If emission is zero, replace emission with normalized stake.
    if emission_sum == I32F32::from(0) {
        // no weights set | outdated weights | self_weights
        if is_zero(&active_stake) {
            // no active stake
            normalized_validator_emission = stake.clone(); // do not mask inactive, assumes stake is normalized
            normalized_combined_emission = stake.clone();
        } else {
            normalized_validator_emission = active_stake.clone(); // emission proportional to inactive-masked normalized stake
            normalized_combined_emission = active_stake.clone();
        }
    }

    // Compute rao based emission scores. range: I96F32(0, rao_emission)
    let float_token_emission: I96F32 = I96F32::from_num(token_emission);

    let server_emission: Vec<I96F32> = normalized_server_emission
        .iter()
        .map(|se: &I32F32| I96F32::from_num(*se) * float_token_emission)
        .collect();
    let server_emission: Vec<u64> = server_emission
        .iter()
        .map(|e: &I96F32| e.to_num::<u64>())
        .collect();

    let validator_emission: Vec<I96F32> = normalized_validator_emission
        .iter()
        .map(|ve: &I32F32| I96F32::from_num(*ve) * float_token_emission)
        .collect();
    let validator_emission: Vec<u64> = validator_emission
        .iter()
        .map(|e: &I96F32| e.to_num::<u64>())
        .collect();

    // Only used to track emission in storage.
    let combined_emission: Vec<I96F32> = normalized_combined_emission
        .iter()
        .map(|ce: &I32F32| I96F32::from_num(*ce) * float_token_emission)
        .collect();
    let combined_emission: Vec<u64> = combined_emission
        .iter()
        .map(|e: &I96F32| e.to_num::<u64>())
        .collect();

    api.debug(&format!("⚪️ nSE: {:?}", &normalized_server_emission));
    api.debug(&format!("⚪️ SE: {:?}", &server_emission));
    api.debug(&format!("⚪️ nVE: {:?}", &normalized_validator_emission));
    api.debug(&format!("⚪️ VE: {:?}", &validator_emission));
    api.debug(&format!("⚪️ nCE: {:?}", &normalized_combined_emission));
    api.debug(&format!("⚪️ CE: {:?}", &combined_emission));

    // Set pruning scores using combined emission scores.
    let pruning_scores: Vec<I32F32> = normalized_combined_emission.clone();
    api.debug(&format!("⚪️ Psc: {:?}", &pruning_scores));

    // ===================
    // == Value storage ==
    // ===================
    let cloned_emission: Vec<u64> = combined_emission.clone();
    let cloned_ranks: Vec<u16> = ranks
        .iter()
        .map(|xi| fixed_proportion_to_u16(*xi))
        .collect::<Vec<u16>>();
    let cloned_trust: Vec<u16> = trust
        .iter()
        .map(|xi| fixed_proportion_to_u16(*xi))
        .collect::<Vec<u16>>();
    let cloned_consensus: Vec<u16> = consensus
        .iter()
        .map(|xi| fixed_proportion_to_u16(*xi))
        .collect::<Vec<u16>>();
    let cloned_incentive: Vec<u16> = incentive
        .iter()
        .map(|xi| fixed_proportion_to_u16(*xi))
        .collect::<Vec<u16>>();
    let cloned_dividends: Vec<u16> = dividends
        .iter()
        .map(|xi| fixed_proportion_to_u16(*xi))
        .collect::<Vec<u16>>();
    let cloned_pruning_scores: Vec<u16> = vec_max_upscale_to_u16(&pruning_scores);
    let cloned_validator_trust: Vec<u16> = validator_trust
        .iter()
        .map(|xi| fixed_proportion_to_u16(*xi))
        .collect::<Vec<u16>>();

    ACTIVE.save(store, netuid, &active).unwrap();
    EMISSION.save(store, netuid, &cloned_emission).unwrap();
    RANK.save(store, netuid, &cloned_ranks).unwrap();
    TRUST.save(store, netuid, &cloned_trust).unwrap();
    CONSENSUS.save(store, netuid, &cloned_consensus).unwrap();
    INCENTIVE.save(store, netuid, &cloned_incentive).unwrap();
    DIVIDENDS.save(store, netuid, &cloned_dividends).unwrap();
    PRUNING_SCORES
        .save(store, netuid, &cloned_pruning_scores)
        .unwrap();
    VALIDATOR_TRUST
        .save(store, netuid, &cloned_validator_trust)
        .unwrap();
    VALIDATOR_PERMIT
        .save(store, netuid, &new_validator_permits)
        .unwrap();

    // Column max-upscale EMA bonds for storage: max_i w_ij = 1.
    inplace_col_max_upscale_sparse(&mut ema_bonds, n);
    for i in 0..n {
        // Set bonds only if uid retains validator permit, otherwise clear bonds.
        if new_validator_permits[i as usize] {
            let new_bonds_row: Vec<(u16, u16)> = ema_bonds[i as usize]
                .iter()
                .map(|(j, value)| (*j, fixed_proportion_to_u16(*value)))
                .collect();
            BONDS.save(store, (netuid, i), &new_bonds_row).unwrap();
        } else if validator_permits[i as usize] {
            // Only overwrite the intersection.
            let new_empty_bonds_row: Vec<(u16, u16)> = vec![];
            BONDS
                .save(store, (netuid, i), &new_empty_bonds_row)
                .unwrap();
        }
    }

    // Emission tuples ( hotkeys, server_emission, validator_emission )
    let mut result: Vec<(Addr, u64, u64)> = vec![];
    for (uid_i, hotkey) in hotkeys.iter() {
        result.push((
            hotkey.clone(),
            server_emission[*uid_i as usize],
            validator_emission[*uid_i as usize],
        ));
    }
    Ok(result)
}

pub fn get_float_rho(store: &dyn Storage, netuid: u16) -> I32F32 {
    I32F32::from_num(get_rho(store, netuid))
}

pub fn get_float_kappa(store: &dyn Storage, netuid: u16) -> I32F32 {
    I32F32::from_num(get_kappa(store, netuid)) / I32F32::from_num(u16::MAX)
}

pub fn get_normalized_stake(store: &dyn Storage, netuid: u16) -> Vec<I32F32> {
    let n: usize = get_subnetwork_n(store, netuid) as usize;
    let mut stake_64: Vec<I64F64> = vec![I64F64::from_num(0.0); n];
    for neuron_uid in 0..n {
        stake_64[neuron_uid] = I64F64::from_num(get_stake_for_uid_and_subnetwork(
            store,
            netuid,
            neuron_uid as u16,
        ));
    }
    inplace_normalize_64(&mut stake_64);
    let stake: Vec<I32F32> = vec_fixed64_to_fixed32(stake_64);
    stake
}

pub fn get_block_at_registration(store: &dyn Storage, netuid: u16) -> Vec<u64> {
    let n: usize = get_subnetwork_n(store, netuid) as usize;
    let mut block_at_registration: Vec<u64> = vec![0; n];
    for neuron_uid in 0..n {
        if KEYS.has(store, (netuid, neuron_uid as u16)) {
            block_at_registration[neuron_uid] =
                get_neuron_block_at_registration(store, netuid, neuron_uid as u16);
        }
    }
    block_at_registration
}

// Output unnormalized sparse weights, input weights are assumed to be row max-upscaled in u16.
pub fn get_weights_sparse(store: &dyn Storage, netuid: u16) -> Vec<Vec<(u16, I32F32)>> {
    let n: usize = get_subnetwork_n(store, netuid) as usize;
    let mut weights: Vec<Vec<(u16, I32F32)>> = vec![vec![]; n];
    for item in WEIGHTS
        .prefix(netuid)
        .range(store, None, None, Order::Ascending)
    {
        let (uid_i, weights_i) = item.unwrap();
        for (uid_j, weight_ij) in weights_i.iter() {
            weights[uid_i as usize].push((*uid_j, I32F32::from_num(*weight_ij)));
        }
    }
    weights
}

// Output unnormalized weights in [n, n] matrix, input weights are assumed to be row max-upscaled in u16.
pub fn get_weights(store: &dyn Storage, netuid: u16) -> Vec<Vec<I32F32>> {
    let n: usize = get_subnetwork_n(store, netuid) as usize;
    let mut weights: Vec<Vec<I32F32>> = vec![vec![I32F32::from_num(0.0); n]; n];
    for item in WEIGHTS
        .prefix(netuid)
        .range(store, None, None, Order::Ascending)
    {
        let (uid_i, weights_i) = item.unwrap();
        for (uid_j, weight_ij) in weights_i.iter() {
            weights[uid_i as usize][*uid_j as usize] = I32F32::from_num(*weight_ij);
        }
    }
    weights
}

// Output unnormalized sparse bonds, input bonds are assumed to be column max-upscaled in u16.
pub fn get_bonds_sparse(store: &dyn Storage, netuid: u16) -> Vec<Vec<(u16, I32F32)>> {
    let n: usize = get_subnetwork_n(store, netuid) as usize;
    let mut bonds: Vec<Vec<(u16, I32F32)>> = vec![vec![]; n];
    for item in BONDS
        .prefix(netuid)
        .range(store, None, None, Order::Ascending)
    {
        let (uid_i, bonds_i) = item.unwrap();
        for (uid_j, bonds_ij) in bonds_i.iter() {
            bonds[uid_i as usize].push((*uid_j, I32F32::from_num(*bonds_ij)));
        }
    }
    bonds
}

// Output unnormalized bonds in [n, n] matrix, input bonds are assumed to be column max-upscaled in u16.
pub fn get_bonds(store: &dyn Storage, netuid: u16) -> Vec<Vec<I32F32>> {
    let n: usize = get_subnetwork_n(store, netuid) as usize;
    let mut bonds: Vec<Vec<I32F32>> = vec![vec![I32F32::from_num(0.0); n]; n];
    for item in BONDS
        .prefix(netuid)
        .range(store, None, None, Order::Ascending)
    {
        let (uid_i, bonds_i) = item.unwrap();
        for (uid_j, bonds_ij) in bonds_i.iter() {
            bonds[uid_i as usize][*uid_j as usize] = I32F32::from_num(*bonds_ij);
        }
    }
    bonds
}
