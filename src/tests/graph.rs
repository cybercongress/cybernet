use std::collections::HashMap;
use substrate_fixed::types::{I32F32, I64F64, I96F32};
use crate::math::{col_clip_sparse, inplace_col_normalize_sparse, inplace_normalize, inplace_normalize_using_sum, mat_ema_sparse, matmul_sparse, matmul_transpose_sparse, row_hadamard_sparse, row_sum_sparse, vecdiv, weighted_median_col_sparse};

pub fn find_intersection(arr1: &[i32], arr2: &[i32]) -> Vec<i32> {
    let mut intersection = Vec::new();

    for &num1 in arr1 {
        for &num2 in arr2 {
            if num1 == num2 {
                intersection.push(num1);
                break;
            }
        }
    }

    intersection
}
pub fn cosine_similarity(a: &[i32], b: &[i32]) -> f64 {
    let intersection = find_intersection(a, b);

    intersection.len() as f64 / ((a.len() * b.len()) as f64).sqrt()
}

struct UserOutLink {
    user_id: u16,
    out_link: u64,
}

// struct Pair {
//     user_id: u16,
//     cosim: I32F32,
// }

#[test]
pub fn test() {

    let user_out_links = vec![
        UserOutLink { user_id: 1, out_link: 1 },
        UserOutLink { user_id: 1, out_link: 2 },
        UserOutLink { user_id: 1, out_link: 3 },
        UserOutLink { user_id: 1, out_link: 4 },
        UserOutLink { user_id: 2, out_link: 3 },
        UserOutLink { user_id: 2, out_link: 5 },
        UserOutLink { user_id: 2, out_link: 6 },
        UserOutLink { user_id: 3, out_link: 6 },
        UserOutLink { user_id: 3, out_link: 1 },
        UserOutLink { user_id: 3, out_link: 5 },
        UserOutLink { user_id: 3, out_link: 2 },
        UserOutLink { user_id: 3, out_link: 7 },
        UserOutLink { user_id: 3, out_link: 8 },
    ];

    // Create a user-outlink matrix
    let mut user_out_link_matrix: HashMap<u16, Vec<i32>> = HashMap::new();
    for uol in user_out_links {
        user_out_link_matrix.entry(uol.user_id).or_insert_with(Vec::new).push(uol.out_link as i32);
    }

    let mut sparseWeightsMatrix: Vec<Vec<(u16, I32F32)>> = vec![vec![]; 4];

    // Compute cosine similarity between each pair of users
    for (&user_id1, out_links1) in &user_out_link_matrix {
        for (&user_id2, out_links2) in &user_out_link_matrix {
            if user_id1 != user_id2 {
                let cosim = cosine_similarity(out_links1, out_links2);
                sparseWeightsMatrix[user_id1 as usize].push( (user_id2, I32F32::from_num(cosim)));

                println!("Cosine similarity between user {} and user {}: {:.2}", user_id1, user_id2, cosim);
            }
        }
    }

    // =============
    // == Weights ==
    // =============

    let n = 4u16;

    println!("Sparse Matrix: : {:?}", &sparseWeightsMatrix);

    let mut active_stake: Vec<I32F32> = vec![
        I32F32::from_num(0),
        I32F32::from_num(2000u16),
        I32F32::from_num(3000u16),
        I32F32::from_num(4000u16),
    ];

    let mut bonds: Vec<Vec<(u16, I32F32)>> = vec![vec![]; n as usize];

    println!("Stake: {:?}", &active_stake);

    // ================================
    // == Consensus, Validator Trust ==
    // ================================

    // Compute preranks: r_j = SUM(i) w_ij * s_i
    let preranks: Vec<I32F32> = matmul_sparse(&sparseWeightsMatrix, &active_stake, n);
    println!("Preranks: {:?}", &preranks);

    // Clip weights at majority consensus
    let kappa: I32F32 = I32F32::from_num(32_767) / I32F32::from_num(u16::MAX); // consensus majority ratio, e.g. 51%.
    println!("Kappa: {:?}", &kappa);

    let consensus: Vec<I32F32> =
        weighted_median_col_sparse(&active_stake, &sparseWeightsMatrix, n, kappa);
    println!("Consensus {:?}", &consensus);

    let weights: Vec<Vec<(u16, I32F32)>> = col_clip_sparse(&sparseWeightsMatrix, &consensus);
    println!("Weights: {:?}", &weights);

    let validator_trust: Vec<I32F32> = row_sum_sparse(&weights);
    println!("Validator trust: {:?}", &validator_trust);

    // =============================
    // == Ranks, Trust, Incentive ==
    // =============================

    // Compute ranks: r_j = SUM(i) w_ij * s_i.
    let mut ranks: Vec<I32F32> = matmul_sparse(&weights, &active_stake, n);
    println!("Ranks: {:?}", &ranks);

    // Compute server trust: ratio of rank after vs. rank before.
    let trust: Vec<I32F32> = vecdiv(&ranks, &preranks);
    // range: I32F32(0, 1)
    println!("Trust: {:?}", &trust);

    inplace_normalize(&mut ranks); // range: I32F32(0, 1)
    let incentive: Vec<I32F32> = ranks.clone();
    println!("Incentive: {:?}", &incentive);

    // =========================
    // == Bonds and Dividends ==
    // =========================

    // Access network bonds.
    // let mut bonds: Vec<Vec<(u16, I32F32)>> = get_bonds_sparse(store, netuid);
    println!("Bonds: {:?}", &bonds);

    // Compute bonds delta column normalized.
    // ΔB = W◦S (outdated W masked)
    let mut bonds_delta: Vec<Vec<(u16, I32F32)>> = row_hadamard_sparse(&weights, &active_stake);
    println!("ΔBonds: {:?}", &bonds_delta);

    // Normalize bonds delta.
    // sum_i b_ij = 1
    inplace_col_normalize_sparse(&mut bonds_delta, n);
    println!("ΔBonds (norm): {:?}", &bonds_delta);

    // Compute bonds moving average.
    let bonds_moving_average: I64F64 =
        I64F64::from_num(900_000) / I64F64::from_num(1_000_000);
    let alpha: I32F32 = I32F32::from_num(1) - I32F32::from_num(bonds_moving_average);
    let mut ema_bonds: Vec<Vec<(u16, I32F32)>> = mat_ema_sparse(&bonds_delta, &bonds, alpha);

    // Normalize EMA bonds.
    // sum_i b_ij = 1
    inplace_col_normalize_sparse(&mut ema_bonds, n);
    println!("EMA Bonds: {:?}", &ema_bonds);

    // Compute dividends: d_i = SUM(j) b_ij * inc_j.
    // range: I32F32(0, 1)
    let mut dividends: Vec<I32F32> = matmul_transpose_sparse(&ema_bonds, &incentive);
    inplace_normalize(&mut dividends);
    println!("Dividends: {:?}", &dividends);

    // =================================
    // == Emission and Pruning scores ==
    // =================================

    // TODO TODO

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

    let token_emission = 1000000000u64;
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

    println!("nSE: {:?}", &normalized_server_emission);
    println!("SE: {:?}", &server_emission);
    println!("nVE: {:?}", &normalized_validator_emission);
    println!("VE: {:?}", &validator_emission);
    println!("nCE: {:?}", &normalized_combined_emission);
    println!("CE: {:?}", &combined_emission);

    // Set pruning scores using combined emission scores.
    let pruning_scores: Vec<I32F32> = normalized_combined_emission.clone();
    println!("Psc: {:?}", &pruning_scores);
}