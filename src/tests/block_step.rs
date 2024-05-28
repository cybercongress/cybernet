use cosmwasm_std::{Addr, Order, Storage};
use substrate_fixed::types::{I32F32, I64F64, I96F32};

use crate::block_step::{
    blocks_until_next_epoch, drain_emission, generate_emission, get_loaded_emission_tuples,
    has_loaded_emission_tuples, tuples_to_drain_this_block,
};
use crate::epoch::{get_block_at_registration, get_bonds, get_float_kappa, get_weights};
use crate::math::{
    fixed_proportion_to_u16, inplace_col_clip, inplace_col_max_upscale, inplace_col_normalize,
    inplace_mask_diag, inplace_mask_matrix, inplace_mask_rows, inplace_mask_vector,
    inplace_normalize, inplace_normalize_64, inplace_normalize_using_sum, inplace_row_normalize,
    is_topk, is_zero, mat_ema, matmul, matmul_transpose, row_hadamard, row_sum,
    vec_fixed64_to_fixed32, vec_fixed_proportions_to_u16, vec_max_upscale_to_u16, vecdiv,
    weighted_median_col,
};
use crate::registration::create_work_for_block_number;
use crate::root::set_emission_values;
use crate::staking::get_total_stake_for_hotkey;
use crate::state::{
    ACTIVE, BONDS, CONSENSUS, DIVIDENDS, EMISSION, INCENTIVE, KEYS, PRUNING_SCORES, RANK, TRUST,
    VALIDATOR_PERMIT, VALIDATOR_TRUST,
};
use crate::test_helpers::{
    add_balance_to_coldkey_account, add_network, burned_register_ok_neuron, instantiate_contract,
    pow_register_ok_neuron, step_block, sudo_register_ok_neuron,
};
use crate::uids::get_subnetwork_n;
use crate::utils::{
    get_activity_cutoff, get_adjustment_interval, get_bonds_moving_average, get_burn_as_u64,
    get_difficulty_as_u64, get_last_update, get_max_allowed_validators, get_validator_permit,
    set_adjustment_alpha, set_adjustment_interval, set_burn, set_difficulty, set_max_allowed_uids,
    set_max_registrations_per_block, set_min_difficulty, set_target_registrations_per_interval,
};

// Calculates reward consensus and returns the emissions for uids/hotkeys in a given `netuid`.
// (Dense version used only for testing purposes.)
pub fn epoch_dense(
    store: &mut dyn Storage,
    netuid: u16,
    token_emission: u64,
    current_block: u64,
) -> Vec<(Addr, u64, u64)> {
    // Get subnetwork size.
    let n: u16 = get_subnetwork_n(store, netuid);
    // println!("n:\n{:?}\n", n);

    // ======================
    // == Active & updated ==
    // ======================

    // Get current block.
    // let current_block: u64 = env.block.height;
    // println!("current_block:\n{:?}\n", current_block);

    // Get activity cutoff.
    let activity_cutoff: u64 = get_activity_cutoff(store, netuid) as u64;
    // println!("activity_cutoff:\n{:?}\n", activity_cutoff);

    // Last update vector.
    let last_update: Vec<u64> = get_last_update(store, netuid);
    // println!("Last update:\n{:?}\n", &last_update);

    // Inactive mask.
    let inactive: Vec<bool> = last_update
        .iter()
        .map(|updated| *updated + activity_cutoff < current_block)
        .collect();
    // println!("Inactive:\n{:?}\n", inactive.clone());

    // Logical negation of inactive.
    let active: Vec<bool> = inactive.iter().map(|&b| !b).collect();

    // Block at registration vector (block when each neuron was most recently registered).
    let block_at_registration: Vec<u64> = get_block_at_registration(store, netuid);
    // println!("Block at registration:\n{:?}\n", &block_at_registration);

    // Outdated matrix, updated_ij=True if i has last updated (weights) after j has last registered.
    let outdated: Vec<Vec<bool>> = last_update
        .iter()
        .map(|updated| {
            block_at_registration
                .iter()
                .map(|registered| updated <= registered)
                .collect()
        })
        .collect();
    // println!("Outdated:\n{:?}\n", &outdated);

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
    // println!("hotkeys: {:?}", &hotkeys);

    // Access network stake as normalized vector.
    let mut stake_64: Vec<I64F64> = vec![I64F64::from_num(0.0); n as usize];
    for (uid_i, hotkey) in hotkeys.iter() {
        stake_64[*uid_i as usize] = I64F64::from_num(get_total_stake_for_hotkey(store, hotkey));
    }
    inplace_normalize_64(&mut stake_64);
    let stake: Vec<I32F32> = vec_fixed64_to_fixed32(stake_64);
    // println!("S:\n{:?}\n", &stake);

    // =======================
    // == Validator permits ==
    // =======================

    // Get validator permits.
    let validator_permits: Vec<bool> = get_validator_permit(store, netuid);
    // println!("validator_permits: {:?}", validator_permits);

    // Logical negation of validator_permits.
    let validator_forbids: Vec<bool> = validator_permits.iter().map(|&b| !b).collect();

    // Get max allowed validators.
    let max_allowed_validators: u16 = get_max_allowed_validators(store, netuid);
    // println!("max_allowed_validators: {:?}", max_allowed_validators);

    // Get new validator permits.
    let new_validator_permits: Vec<bool> = is_topk(&stake, max_allowed_validators as usize);
    // println!("new_validator_permits: {:?}", new_validator_permits);

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
    // println!("S:\n{:?}\n", &active_stake);

    // =============
    // == Weights ==
    // =============

    // Access network weights row unnormalized.
    let mut weights: Vec<Vec<I32F32>> = get_weights(store, netuid);
    // println!("W:\n{:?}\n", &weights);

    // Mask weights that are not from permitted validators.
    inplace_mask_rows(&validator_forbids, &mut weights);
    // println!("W (permit): {:?}", &weights);

    // Remove self-weight by masking diagonal.
    inplace_mask_diag(&mut weights);
    // println!("W (permit+diag):\n{:?}\n", &weights);

    // Mask outdated weights: remove weights referring to deregistered neurons.
    inplace_mask_matrix(&outdated, &mut weights);
    // println!("W (permit+diag+outdate):\n{:?}\n", &weights);

    // Normalize remaining weights.
    inplace_row_normalize(&mut weights);
    // println!("W (mask+norm):\n{:?}\n", &weights);

    // ================================
    // == Consensus, Validator Trust ==
    // ================================

    // Compute preranks: r_j = SUM(i) w_ij * s_i
    let preranks: Vec<I32F32> = matmul(&weights, &active_stake);

    // Clip weights at majority consensus
    let kappa: I32F32 = get_float_kappa(store, netuid); // consensus majority ratio, e.g. 51%.
    let consensus: Vec<I32F32> = weighted_median_col(&active_stake, &weights, kappa);
    inplace_col_clip(&mut weights, &consensus);
    let validator_trust: Vec<I32F32> = row_sum(&weights);

    // ====================================
    // == Ranks, Server Trust, Incentive ==
    // ====================================

    // Compute ranks: r_j = SUM(i) w_ij * s_i
    let mut ranks: Vec<I32F32> = matmul(&weights, &active_stake);

    // Compute server trust: ratio of rank after vs. rank before.
    let trust: Vec<I32F32> = vecdiv(&ranks, &preranks);

    inplace_normalize(&mut ranks);
    let incentive: Vec<I32F32> = ranks.clone();
    // println!("I:\n{:?}\n", &incentive);

    // =========================
    // == Bonds and Dividends ==
    // =========================

    // Access network bonds.
    let mut bonds: Vec<Vec<I32F32>> = get_bonds(store, netuid);
    inplace_mask_matrix(&outdated, &mut bonds); // mask outdated bonds
    inplace_col_normalize(&mut bonds); // sum_i b_ij = 1
    // println!("B:\n{:?}\n", &bonds);

    // Compute bonds delta column normalized.
    let mut bonds_delta: Vec<Vec<I32F32>> = row_hadamard(&weights, &active_stake); // ΔB = W◦S
    inplace_col_normalize(&mut bonds_delta); // sum_i b_ij = 1
    // println!("ΔB:\n{:?}\n", &bonds_delta);

    // Compute bonds moving average.
    let bonds_moving_average: I64F64 =
        I64F64::from_num(get_bonds_moving_average(store, netuid)) / I64F64::from_num(1_000_000);
    let alpha: I32F32 = I32F32::from_num(1) - I32F32::from_num(bonds_moving_average);
    let mut ema_bonds: Vec<Vec<I32F32>> = mat_ema(&bonds_delta, &bonds, alpha);
    inplace_col_normalize(&mut ema_bonds); // sum_i b_ij = 1
    // println!("emaB:\n{:?}\n", &ema_bonds);

    // Compute dividends: d_i = SUM(j) b_ij * inc_j
    let mut dividends: Vec<I32F32> = matmul_transpose(&ema_bonds, &incentive);
    inplace_normalize(&mut dividends);
    // println!("D:\n{:?}\n", &dividends);

    // =================================
    // == Emission and Pruning scores ==
    // =================================

    // Compute emission scores.

    // Compute normalized emission scores. range: I32F32(0, 1)
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

    // Used only to track combined emission in the storage.
    let combined_emission: Vec<I96F32> = normalized_combined_emission
        .iter()
        .map(|ce: &I32F32| I96F32::from_num(*ce) * float_token_emission)
        .collect();
    let combined_emission: Vec<u64> = combined_emission
        .iter()
        .map(|e: &I96F32| e.to_num::<u64>())
        .collect();

    // api.debug(&format!( "nSE: {:?}", &normalized_server_emission ));
    // println!("SE: {:?}", &server_emission);
    // api.debug(&format!( "nVE: {:?}", &normalized_validator_emission ));
    // println!("VE: {:?}", &validator_emission);
    // api.debug(&format!( "nCE: {:?}", &normalized_combined_emission ));
    // println!("CE: {:?}", &combined_emission);

    // Set pruning scores using combined emission scores.
    let pruning_scores: Vec<I32F32> = normalized_combined_emission.clone();
    // println!("P: {:?}", &pruning_scores);

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
    inplace_col_max_upscale(&mut ema_bonds);
    for i in 0..n {
        // Set bonds only if uid retains validator permit, otherwise clear bonds.
        if new_validator_permits[i as usize] {
            let new_bonds_row: Vec<(u16, u16)> = (0..n)
                .zip(vec_fixed_proportions_to_u16(ema_bonds[i as usize].clone()))
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

    let mut result: Vec<(Addr, u64, u64)> = vec![];
    for (uid_i, hotkey) in hotkeys.iter() {
        result.push((
            hotkey.clone(),
            server_emission[*uid_i as usize],
            validator_emission[*uid_i as usize],
        ));
    }
    result
}
#[test]
fn test_loaded_emission() {
    let (mut deps, mut env) = instantiate_contract();

    let n: u16 = 100;
    let netuid: u16 = 2; // I changed netuid here and block until next epoch not equal
    let tempo: u16 = 10;
    let netuids: Vec<u16> = vec![1];
    let emission: Vec<u64> = vec![1000000000];
    add_network(&mut deps.storage, netuid, tempo, 0);
    set_max_allowed_uids(&mut deps.storage, netuid, n);
    set_emission_values(&mut deps.storage, &deps.api, &netuids, emission).unwrap();
    for i in 0..n {
        // append_neuron(&mut deps.storage, &deps.api, netuid, &Addr::unchecked(i.to_string()), 0).unwrap();
        sudo_register_ok_neuron(
            deps.as_mut(),
            env.clone(),
            netuid,
            (1000 + i).to_string().as_str(),
            (1000 + i).to_string().as_str(),
        );
    }
    assert!(!has_loaded_emission_tuples(&deps.storage, netuid));

    // Try loading at block 0
    //
    let block: u64 = env.block.height;
    assert_eq!(blocks_until_next_epoch(netuid, tempo, block), 7);
    generate_emission(&mut deps.storage, &deps.api, block).expect("TODO: panic message");
    assert!(!has_loaded_emission_tuples(&deps.storage, netuid));

    // Try loading at block = 9;
    env.block.height = 8;
    let block: u64 = 8;
    assert_eq!(blocks_until_next_epoch(netuid, tempo, block), 0);
    generate_emission(&mut deps.storage, &deps.api, block).unwrap();
    assert!(has_loaded_emission_tuples(&deps.storage, netuid));
    assert_eq!(
        get_loaded_emission_tuples(&deps.storage, netuid).len(),
        n as usize
    );

    // Try draining the emission tuples
    // None remaining because we are at epoch.
    let block: u64 = 8;
    drain_emission(&mut deps.storage, &deps.api, block).unwrap();
    assert!(!has_loaded_emission_tuples(&deps.storage, netuid));

    // Generate more emission.
    generate_emission(&mut deps.storage, &deps.api, 8).unwrap();
    assert_eq!(
        get_loaded_emission_tuples(&deps.storage, netuid).len(),
        n as usize
    );

    for block in 9..19 {
        let mut n_remaining: usize = 0;
        let mut n_to_drain: usize = 0;
        if has_loaded_emission_tuples(&deps.storage, netuid) {
            n_remaining = get_loaded_emission_tuples(&deps.storage, netuid).len();
            n_to_drain = tuples_to_drain_this_block(
                netuid,
                tempo,
                block,
                get_loaded_emission_tuples(&deps.storage, netuid).len(),
            );
        }
        drain_emission(&mut deps.storage, &deps.api, block).unwrap(); // drain it with 9 more blocks to go
        if has_loaded_emission_tuples(&deps.storage, netuid) {
            assert_eq!(
                get_loaded_emission_tuples(&deps.storage, netuid).len(),
                n_remaining - n_to_drain
            );
        }
        log::info!("n_to_drain:{:?}", n_to_drain.clone());
        log::info!(
            "get_loaded_emission_tuples( netuid ).len():{:?}",
            n_remaining - n_to_drain
        );
    }
}

#[test]
fn test_tuples_to_drain_this_block() {
    // TODO revisit logic here.
    // pub fn tuples_to_drain_this_block( netuid: u16, tempo: u16, block_number: u64, n_remaining: usize ) -> usize {
    assert_eq!(tuples_to_drain_this_block(0, 1, 1, 10), 10); // drain all epoch block.
    assert_eq!(tuples_to_drain_this_block(0, 0, 1, 10), 10); // drain all no tempo.
    assert_eq!(tuples_to_drain_this_block(0, 10, 1, 10), 2); // drain 10 / ( 10 / 2 ) = 2
    assert_eq!(tuples_to_drain_this_block(0, 20, 1, 10), 1); // drain 10 / ( 20 / 2 ) = 1
    assert_eq!(tuples_to_drain_this_block(0, 10, 1, 20), 5); // drain 20 / ( 9 / 2 ) = 5
    assert_eq!(tuples_to_drain_this_block(0, 20, 1, 0), 0); // nothing to drain.
    assert_eq!(tuples_to_drain_this_block(0, 10, 2, 20), 5); // drain 19 / ( 10 / 2 ) = 4
    assert_eq!(tuples_to_drain_this_block(0, 10, 10, 20), 20); // drain 19 / ( 10 / 2 ) = 4
    assert_eq!(tuples_to_drain_this_block(0, 10, 15, 20), 6); // drain 19 / ( 10 / 2 ) = 4
    assert_eq!(tuples_to_drain_this_block(0, 10, 19, 20), 20); // drain 19 / ( 10 / 2 ) = 4
    assert_eq!(tuples_to_drain_this_block(0, 10, 20, 20), 20); // drain 19 / ( 10 / 2 ) = 4
    for i in 0..10 {
        for j in 0..10 {
            for k in 0..10 {
                for l in 0..10 {
                    assert!(tuples_to_drain_this_block(i, j, k, l) <= 10);
                }
            }
        }
    }
}

#[test]
fn test_blocks_until_epoch() {
    // Check tempo = 0 block = * netuid = *
    assert_eq!(blocks_until_next_epoch(0, 0, 0), 1000);

    // Check tempo = 1 block = * netuid = *
    assert_eq!(blocks_until_next_epoch(0, 1, 1), 0);
    assert_eq!(blocks_until_next_epoch(1, 1, 1), 1);
    assert_eq!(blocks_until_next_epoch(0, 1, 2), 1);
    assert_eq!(blocks_until_next_epoch(1, 1, 2), 0);
    assert_eq!(blocks_until_next_epoch(0, 1, 3), 0);
    assert_eq!(blocks_until_next_epoch(1, 1, 3), 1);
    for i in 0..100 {
        if i % 2 == 0 {
            assert_eq!(blocks_until_next_epoch(0, 1, i), 1);
            assert_eq!(blocks_until_next_epoch(1, 1, i), 0);
        } else {
            assert_eq!(blocks_until_next_epoch(0, 1, i), 0);
            assert_eq!(blocks_until_next_epoch(1, 1, i), 1);
        }
    }

    // Check general case.
    for netuid in 0..30 as u16 {
        for block in 1..30 as u64 {
            for tempo in 1..30 as u16 {
                assert_eq!(
                    blocks_until_next_epoch(netuid, tempo, block),
                    tempo as u64 - (block + netuid as u64) % (tempo as u64 + 1)
                );
                // println!(
                //     "netuid: {:?} | tempo: {:?} | block: {:?} | until {:?} | run {:?} |",
                //     netuid,
                //     tempo,
                //     block,
                //     tempo as u64 - (block + netuid as u64) % (tempo as u64 + 1),
                //     (tempo as u64 - (block + netuid as u64) % (tempo as u64 + 1)) == 0
                // );
            }
        }
    }
}

// /********************************************
//     block_step::adjust_registration_terms_for_networks tests
// *********************************************/
#[test]
fn test_burn_adjustment() {
    let (mut deps, mut env) = instantiate_contract();

    let netuid: u16 = 2;
    let tempo: u16 = 13;
    let burn_cost: u64 = 1000000000;
    let adjustment_interval = 1;
    let target_registrations_per_interval = 1;
    add_network(&mut deps.storage, netuid, tempo, 0);
    set_burn(&mut deps.storage, netuid, burn_cost);
    set_adjustment_interval(&mut deps.storage, netuid, adjustment_interval);
    set_target_registrations_per_interval(
        &mut deps.storage,
        netuid,
        target_registrations_per_interval,
    );
    assert_eq!(
        get_adjustment_interval(&deps.storage, netuid),
        adjustment_interval
    ); // Sanity check the adjustment interval.

    // Register key 1.
    let hotkey_account_id_1 = "addr1";
    let coldkey_account_id_1 = "addr1";
    add_balance_to_coldkey_account(&Addr::unchecked(coldkey_account_id_1), 10000);
    assert_eq!(
        burned_register_ok_neuron(
            deps.as_mut(),
            env.clone(),
            netuid,
            hotkey_account_id_1,
            coldkey_account_id_1,
        )
        .is_ok(),
        true
    );

    // Register key 2.
    let hotkey_account_id_2 = "1002";
    let coldkey_account_id_2 = "1002";
    add_balance_to_coldkey_account(&Addr::unchecked(coldkey_account_id_2), 10000);
    assert_eq!(
        burned_register_ok_neuron(
            deps.as_mut(),
            env.clone(),
            netuid,
            hotkey_account_id_2,
            coldkey_account_id_2,
        )
        .is_ok(),
        true
    );

    // We are over the number of regs allowed this interval.
    // Step the block and trigger the adjustment.
    step_block(deps.as_mut(), &mut env).unwrap();

    // Check the adjusted burn.
    assert_eq!(get_burn_as_u64(&deps.storage, netuid), 1500000000);
}

#[test]
fn test_burn_adjustment_with_moving_average() {
    let (mut deps, mut env) = instantiate_contract();

    let netuid: u16 = 2;
    let tempo: u16 = 13;
    let burn_cost: u64 = 1000000000;
    let adjustment_interval = 1;
    let target_registrations_per_interval = 1;
    add_network(&mut deps.storage, netuid, tempo, 0);
    set_burn(&mut deps.storage, netuid, burn_cost);
    set_adjustment_interval(&mut deps.storage, netuid, adjustment_interval);
    set_target_registrations_per_interval(
        &mut deps.storage,
        netuid,
        target_registrations_per_interval,
    );
    // Set alpha here.
    set_adjustment_alpha(&mut deps.storage, netuid, u64::MAX / 2);

    // Register key 1.
    let hotkey_account_id_1 = "addr1";
    let coldkey_account_id_1 = "addr1";
    add_balance_to_coldkey_account(&Addr::unchecked(coldkey_account_id_1), 10000);
    assert_eq!(
        burned_register_ok_neuron(
            deps.as_mut(),
            env.clone(),
            netuid,
            hotkey_account_id_1,
            coldkey_account_id_1,
        )
        .is_ok(),
        true
    );

    // Register key 2.
    let hotkey_account_id_2 = "addr2";
    let coldkey_account_id_2 = "addr2";
    add_balance_to_coldkey_account(&Addr::unchecked(coldkey_account_id_2), 10000);
    assert_eq!(
        burned_register_ok_neuron(
            deps.as_mut(),
            env.clone(),
            netuid,
            hotkey_account_id_2,
            coldkey_account_id_2,
        )
        .is_ok(),
        true
    );

    // We are over the number of regs allowed this interval.
    // Step the block and trigger the adjustment.
    step_block(deps.as_mut(), &mut env).unwrap();

    // Check the adjusted burn.
    // 0.5 * 1000 + 0.5 * 1500 = 1250
    // assert_eq!(get_burn_as_u64(&deps.storage, netuid), 1250000000);
    assert_eq!(get_burn_as_u64(&deps.storage, netuid), 1250001907);
}

#[test]
#[allow(unused_assignments)]
fn test_burn_adjustment_case_a() {
    // Test case A of the difficulty and burn adjustment algorithm.
    // ====================
    // There are too many registrations this interval and most of them are pow registrations
    // this triggers an increase in the pow difficulty.
    let (mut deps, mut env) = instantiate_contract();

    let netuid: u16 = 2;
    let tempo: u16 = 13;
    let burn_cost: u64 = 1000000000;
    let adjustment_interval = 1;
    let target_registrations_per_interval = 1;
    let start_diff: u64 = 10_000;
    let mut curr_block_num = env.block.height;
    add_network(&mut deps.storage, netuid, tempo, 0);
    set_burn(&mut deps.storage, netuid, burn_cost);
    set_min_difficulty(&mut deps.storage, netuid, 1);
    set_difficulty(&mut deps.storage, netuid, start_diff);
    set_adjustment_interval(&mut deps.storage, netuid, adjustment_interval);
    set_target_registrations_per_interval(
        &mut deps.storage,
        netuid,
        target_registrations_per_interval,
    );

    // Register key 1. This is a burn registration.
    let hotkey_account_id_1 = "addr1";
    let coldkey_account_id_1 = "addr1";
    add_balance_to_coldkey_account(&Addr::unchecked(coldkey_account_id_1), 10000);
    assert_eq!(
        burned_register_ok_neuron(
            deps.as_mut(),
            env.clone(),
            netuid,
            hotkey_account_id_1,
            coldkey_account_id_1,
        )
        .is_ok(),
        true
    );

    // Register key 2. This is a POW registration
    let hotkey_account_id_2 = "addr2";
    let coldkey_account_id_2 = "addr2";
    let (nonce0, work0): (u64, Vec<u8>) = create_work_for_block_number(
        &deps.storage,
        netuid,
        curr_block_num,
        0,
        &hotkey_account_id_2,
    );
    let _ = pow_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        curr_block_num,
        nonce0,
        work0,
        &hotkey_account_id_2,
        &coldkey_account_id_2,
    );

    // Register key 3. This is a POW registration
    let hotkey_account_id_3 = "addr3";
    let coldkey_account_id_3 = "addr3";
    let (nonce1, work1): (u64, Vec<u8>) = create_work_for_block_number(
        &deps.storage,
        netuid,
        curr_block_num,
        11231312312,
        &hotkey_account_id_3,
    );
    let _ = pow_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        curr_block_num,
        nonce1,
        work1,
        &hotkey_account_id_3,
        &coldkey_account_id_3,
    );

    // We are over the number of regs allowed this interval.
    // Most of them are POW registrations (2 out of 3)
    // Step the block and trigger the adjustment.
    step_block(deps.as_mut(), &mut env).unwrap();
    curr_block_num += 1;

    // Check the adjusted POW difficulty has INCREASED.
    //   and the burn has not changed.
    let adjusted_burn = get_burn_as_u64(&deps.storage, netuid);
    assert_eq!(adjusted_burn, burn_cost);

    let adjusted_diff = get_difficulty_as_u64(&deps.storage, netuid);
    assert!(adjusted_diff > start_diff);
    assert_eq!(adjusted_diff, 20_000);
}

#[test]
#[allow(unused_assignments)]
fn test_burn_adjustment_case_b() {
    // Test case B of the difficulty and burn adjustment algorithm.
    // ====================
    // There are too many registrations this interval and most of them are burn registrations
    // this triggers an increase in the burn cost.
    let (mut deps, mut env) = instantiate_contract();

    let netuid: u16 = 2;
    let tempo: u16 = 13;
    let burn_cost: u64 = 1000000000;
    let adjustment_interval = 1;
    let target_registrations_per_interval = 1;
    let start_diff: u64 = 20_000;
    let mut curr_block_num = 0;
    add_network(&mut deps.storage, netuid, tempo, 0);
    set_burn(&mut deps.storage, netuid, burn_cost);
    set_min_difficulty(&mut deps.storage, netuid, 1);
    set_difficulty(&mut deps.storage, netuid, start_diff);
    set_adjustment_interval(&mut deps.storage, netuid, adjustment_interval);
    set_target_registrations_per_interval(
        &mut deps.storage,
        netuid,
        target_registrations_per_interval,
    );

    // Register key 1.
    let hotkey_account_id_1 = "addr1";
    let coldkey_account_id_1 = "addr1";
    add_balance_to_coldkey_account(&Addr::unchecked(coldkey_account_id_1), 10000);
    assert_eq!(
        burned_register_ok_neuron(
            deps.as_mut(),
            env.clone(),
            netuid,
            hotkey_account_id_1,
            coldkey_account_id_1,
        )
        .is_ok(),
        true
    );

    // Register key 2.
    let hotkey_account_id_2 = "addr2";
    let coldkey_account_id_2 = "addr2";
    add_balance_to_coldkey_account(&Addr::unchecked(coldkey_account_id_2), 10000);
    assert_eq!(
        burned_register_ok_neuron(
            deps.as_mut(),
            env.clone(),
            netuid,
            hotkey_account_id_2,
            coldkey_account_id_2,
        )
        .is_ok(),
        true
    );

    // Register key 3. This one is a POW registration
    let hotkey_account_id_3 = "addr3";
    let coldkey_account_id_3 = "addr3";
    let (nonce, work): (u64, Vec<u8>) = create_work_for_block_number(
        &deps.storage,
        netuid,
        curr_block_num,
        0,
        hotkey_account_id_3,
    );
    let _ = pow_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        curr_block_num,
        nonce,
        work,
        hotkey_account_id_3,
        coldkey_account_id_3,
    );

    // We are over the number of regs allowed this interval.
    // Most of them are burn registrations (2 out of 3)
    // Step the block and trigger the adjustment.
    step_block(deps.as_mut(), &mut env).unwrap();
    curr_block_num += 1;

    // Check the adjusted burn has INCREASED.
    //   and the difficulty has not changed.
    let adjusted_burn = get_burn_as_u64(&deps.storage, netuid);
    assert!(adjusted_burn > burn_cost);
    assert_eq!(adjusted_burn, 2000000000);

    let adjusted_diff = get_difficulty_as_u64(&deps.storage, netuid);
    assert_eq!(adjusted_diff, start_diff);
}

#[test]
#[allow(unused_assignments)]
fn test_burn_adjustment_case_c() {
    // Test case C of the difficulty and burn adjustment algorithm.
    // ====================
    // There are not enough registrations this interval and most of them are POW registrations
    // this triggers a decrease in the burn cost
    let (mut deps, mut env) = instantiate_contract();

    let netuid: u16 = 2;
    let tempo: u16 = 13;
    let burn_cost: u64 = 1000000000;
    let adjustment_interval = 1;
    let target_registrations_per_interval = 4; // Needs registrations < 4 to trigger
    let start_diff: u64 = 20_000;
    let mut curr_block_num = 0;
    add_network(&mut deps.storage, netuid, tempo, 0);
    set_burn(&mut deps.storage, netuid, burn_cost);
    set_min_difficulty(&mut deps.storage, netuid, 1);
    set_difficulty(&mut deps.storage, netuid, start_diff);
    set_adjustment_interval(&mut deps.storage, netuid, adjustment_interval);
    set_target_registrations_per_interval(
        &mut deps.storage,
        netuid,
        target_registrations_per_interval,
    );

    // Register key 1. This is a BURN registration
    let hotkey_account_id_1 = "addr1";
    let coldkey_account_id_1 = "addr1";
    add_balance_to_coldkey_account(&Addr::unchecked(coldkey_account_id_1), 10000);
    assert_eq!(
        burned_register_ok_neuron(
            deps.as_mut(),
            env.clone(),
            netuid,
            &hotkey_account_id_1,
            coldkey_account_id_1,
        )
        .is_ok(),
        true
    );

    // Register key 2. This is a POW registration
    let hotkey_account_id_2 = "addr2";
    let coldkey_account_id_2 = "addr2";
    let (nonce0, work0): (u64, Vec<u8>) = create_work_for_block_number(
        &deps.storage,
        netuid,
        curr_block_num,
        0,
        &hotkey_account_id_2,
    );
    let _ = pow_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        curr_block_num,
        nonce0,
        work0,
        &hotkey_account_id_2,
        &coldkey_account_id_2,
    );

    // Register key 3. This is a POW registration
    let hotkey_account_id_3 = "addr3";
    let coldkey_account_id_3 = "addr3";
    let (nonce1, work1): (u64, Vec<u8>) = create_work_for_block_number(
        &deps.storage,
        netuid,
        curr_block_num,
        11231312312,
        &hotkey_account_id_3,
    );
    let _ = pow_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        curr_block_num,
        nonce1,
        work1,
        &hotkey_account_id_3,
        &coldkey_account_id_3,
    );

    // We are UNDER the number of regs allowed this interval.
    // Most of them are POW registrations (2 out of 3)
    // Step the block and trigger the adjustment.
    step_block(deps.as_mut(), &mut env).unwrap();
    curr_block_num += 1;

    // Check the adjusted burn has DECREASED.
    //   and the difficulty has not changed.
    let adjusted_burn = get_burn_as_u64(&deps.storage, netuid);
    assert!(adjusted_burn < burn_cost);
    assert_eq!(adjusted_burn, 875000000);

    let adjusted_diff = get_difficulty_as_u64(&deps.storage, netuid);
    assert_eq!(adjusted_diff, start_diff);
}

#[test]
#[allow(unused_assignments)]
fn test_burn_adjustment_case_d() {
    // Test case D of the difficulty and burn adjustment algorithm.
    // ====================
    // There are not enough registrations this interval and most of them are BURN registrations
    // this triggers a decrease in the POW difficulty
    let (mut deps, mut env) = instantiate_contract();

    let netuid: u16 = 2;
    let tempo: u16 = 13;
    let burn_cost: u64 = 1000;
    let adjustment_interval = 1;
    let target_registrations_per_interval = 4; // Needs registrations < 4 to trigger
    let start_diff: u64 = 20_000;
    let mut curr_block_num = 0;
    add_network(&mut deps.storage, netuid, tempo, 0);
    set_burn(&mut deps.storage, netuid, burn_cost);
    set_min_difficulty(&mut deps.storage, netuid, 1);
    set_difficulty(&mut deps.storage, netuid, start_diff);
    set_adjustment_interval(&mut deps.storage, netuid, adjustment_interval);
    set_target_registrations_per_interval(
        &mut deps.storage,
        netuid,
        target_registrations_per_interval,
    );

    // Register key 1. This is a BURN registration
    let hotkey_account_id_1 = "addr1";
    let coldkey_account_id_1 = "addr1";
    add_balance_to_coldkey_account(&Addr::unchecked(coldkey_account_id_1), 10000);
    assert_eq!(
        burned_register_ok_neuron(
            deps.as_mut(),
            env.clone(),
            netuid,
            hotkey_account_id_1,
            coldkey_account_id_1,
        )
        .is_ok(),
        true
    );

    // Register key 2. This is a BURN registration
    let hotkey_account_id_2 = "addr2";
    let coldkey_account_id_2 = "addr2";
    add_balance_to_coldkey_account(&Addr::unchecked(coldkey_account_id_2), 10000);
    assert_eq!(
        burned_register_ok_neuron(
            deps.as_mut(),
            env.clone(),
            netuid,
            hotkey_account_id_2,
            coldkey_account_id_2,
        )
        .is_ok(),
        true
    );

    // Register key 3. This is a POW registration
    let hotkey_account_id_3 = "addr3";
    let coldkey_account_id_3 = "addr3";
    let (nonce1, work1): (u64, Vec<u8>) = create_work_for_block_number(
        &deps.storage,
        netuid,
        curr_block_num,
        11231312312,
        &hotkey_account_id_3,
    );
    let _ = pow_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        curr_block_num,
        nonce1,
        work1,
        &hotkey_account_id_3,
        &coldkey_account_id_3,
    );

    // We are UNDER the number of regs allowed this interval.
    // Most of them are BURN registrations (2 out of 3)
    // Step the block and trigger the adjustment.
    step_block(deps.as_mut(), &mut env).unwrap();
    curr_block_num += 1;

    // Check the adjusted POW difficulty has DECREASED.
    //   and the burn has not changed.
    let adjusted_burn = get_burn_as_u64(&deps.storage, netuid);
    assert_eq!(adjusted_burn, burn_cost);

    let adjusted_diff = get_difficulty_as_u64(&deps.storage, netuid);
    assert!(adjusted_diff < start_diff);
    assert_eq!(adjusted_diff, 17_500);
}

#[test]
#[allow(unused_assignments)]
fn test_burn_adjustment_case_e() {
    // Test case E of the difficulty and burn adjustment algorithm.
    // ====================
    // There are not enough registrations this interval and nobody registered either POW or BURN
    // this triggers a decrease in the BURN cost and POW difficulty
    let (mut deps, mut env) = instantiate_contract();

    let netuid: u16 = 2;
    let tempo: u16 = 13;
    let burn_cost: u64 = 1000000000;
    let adjustment_interval = 1;
    let target_registrations_per_interval: u16 = 3;
    let start_diff: u64 = 20_000;
    let mut curr_block_num = 0;
    add_network(&mut deps.storage, netuid, tempo, 0);
    set_max_registrations_per_block(&mut deps.storage, netuid, 10);
    set_burn(&mut deps.storage, netuid, burn_cost);
    set_min_difficulty(&mut deps.storage, netuid, 1);
    set_difficulty(&mut deps.storage, netuid, start_diff);
    set_adjustment_interval(&mut deps.storage, netuid, adjustment_interval);
    set_target_registrations_per_interval(
        &mut deps.storage,
        netuid,
        target_registrations_per_interval,
    );

    // Register key 1. This is a POW registration
    let hotkey_account_id_1 = "addr1";
    let coldkey_account_id_1 = "addr1";
    let (nonce1, work1): (u64, Vec<u8>) = create_work_for_block_number(
        &deps.storage,
        netuid,
        curr_block_num,
        11231312312,
        &hotkey_account_id_1,
    );
    let _ = pow_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        curr_block_num,
        nonce1,
        work1,
        &hotkey_account_id_1,
        &coldkey_account_id_1,
    );

    // Register key 2. This is a BURN registration
    let hotkey_account_id_2 = "addr2";
    let coldkey_account_id_2 = "addr2";
    add_balance_to_coldkey_account(&Addr::unchecked(coldkey_account_id_2), 10000);
    assert_eq!(
        burned_register_ok_neuron(
            deps.as_mut(),
            env.clone(),
            netuid,
            &hotkey_account_id_2,
            coldkey_account_id_2,
        )
        .is_ok(),
        true
    );
    step_block(deps.as_mut(), &mut env).unwrap();
    curr_block_num += 1;

    // We are UNDER the number of regs allowed this interval.
    // And the number of regs of each type is equal

    // Check the adjusted BURN has DECREASED.
    let adjusted_burn = get_burn_as_u64(&deps.storage, netuid);
    assert!(adjusted_burn < burn_cost);
    assert_eq!(adjusted_burn, 833333333);

    // Check the adjusted POW difficulty has DECREASED.
    let adjusted_diff = get_difficulty_as_u64(&deps.storage, netuid);
    assert!(adjusted_diff < start_diff);
    assert_eq!(adjusted_diff, 16_666);
}

#[test]
#[allow(unused_assignments)]
fn test_burn_adjustment_case_f() {
    // Test case F of the difficulty and burn adjustment algorithm.
    // ====================
    // There are too many registrations this interval and the pow and burn registrations are equal
    // this triggers an increase in the burn cost and pow difficulty
    let (mut deps, mut env) = instantiate_contract();

    let netuid: u16 = 2;
    let tempo: u16 = 13;
    let burn_cost: u64 = 1000000000;
    let adjustment_interval = 1;
    let target_registrations_per_interval: u16 = 1;
    let start_diff: u64 = 20_000;
    let mut curr_block_num = 0;
    add_network(&mut deps.storage, netuid, tempo, 0);
    set_max_registrations_per_block(&mut deps.storage, netuid, 10);
    set_burn(&mut deps.storage, netuid, burn_cost);
    set_min_difficulty(&mut deps.storage, netuid, 1);
    set_difficulty(&mut deps.storage, netuid, start_diff);
    set_adjustment_interval(&mut deps.storage, netuid, adjustment_interval);
    set_target_registrations_per_interval(
        &mut deps.storage,
        netuid,
        target_registrations_per_interval,
    );

    // Register key 1. This is a POW registration
    let hotkey_account_id_1 = "addr1";
    let coldkey_account_id_1 = "addr1";
    let (nonce1, work1): (u64, Vec<u8>) = create_work_for_block_number(
        &deps.storage,
        netuid,
        curr_block_num,
        11231312312,
        &hotkey_account_id_1,
    );
    let _ = pow_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        curr_block_num,
        nonce1,
        work1,
        &hotkey_account_id_1,
        &coldkey_account_id_1,
    );

    // Register key 2. This is a BURN registration
    let hotkey_account_id_2 = "addr2";
    let coldkey_account_id_2 = "addr2";
    add_balance_to_coldkey_account(&Addr::unchecked(coldkey_account_id_2), 10000);
    assert_eq!(
        burned_register_ok_neuron(
            deps.as_mut(),
            env.clone(),
            netuid,
            hotkey_account_id_2,
            coldkey_account_id_2,
        )
        .is_ok(),
        true
    );

    step_block(deps.as_mut(), &mut env).unwrap();
    curr_block_num += 1;
    // We are OVER the number of regs allowed this interval.
    // And the number of regs of each type is equal

    // Check the adjusted BURN has INCREASED.
    let adjusted_burn = get_burn_as_u64(&deps.storage, netuid);
    assert!(adjusted_burn > burn_cost);
    assert_eq!(adjusted_burn, 1_500_000_000);

    // Check the adjusted POW difficulty has INCREASED.
    let adjusted_diff = get_difficulty_as_u64(&deps.storage, netuid);
    assert!(adjusted_diff > start_diff);
    assert_eq!(adjusted_diff, 30_000);
}

#[test]
fn test_burn_adjustment_case_e_zero_registrations() {
    // Test case E of the difficulty and burn adjustment algorithm.
    // ====================
    // There are not enough registrations this interval and nobody registered either POW or BURN
    // this triggers a decrease in the BURN cost and POW difficulty

    // BUT there are zero registrations this interval.

    let (mut deps, mut env) = instantiate_contract();

    let netuid: u16 = 2;
    let tempo: u16 = 13;
    let burn_cost: u64 = 1000000000;
    let adjustment_interval = 0;
    let target_registrations_per_interval: u16 = 1;
    let start_diff: u64 = 20_000;
    add_network(&mut deps.storage, netuid, tempo, 0);
    set_max_registrations_per_block(&mut deps.storage, netuid, 10);
    set_burn(&mut deps.storage, netuid, burn_cost);
    set_min_difficulty(&mut deps.storage, netuid, 1);
    set_difficulty(&mut deps.storage, netuid, start_diff);
    set_adjustment_interval(&mut deps.storage, netuid, adjustment_interval);
    set_target_registrations_per_interval(
        &mut deps.storage,
        netuid,
        target_registrations_per_interval,
    );

    // No registrations this interval of any kind.
    // env.block.height=13;
    step_block(deps.as_mut(), &mut env).unwrap();

    // We are UNDER the number of regs allowed this interval.
    // And the number of regs of each type is equal

    // Check the adjusted BURN has DECREASED.
    let adjusted_burn = get_burn_as_u64(&deps.storage, netuid);
    assert!(adjusted_burn < burn_cost);
    assert_eq!(adjusted_burn, 500000000);

    // Check the adjusted POW difficulty has DECREASED.
    let adjusted_diff = get_difficulty_as_u64(&deps.storage, netuid);
    assert!(adjusted_diff < start_diff);
    assert_eq!(adjusted_diff, 10_000);
}
