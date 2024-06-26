use std::time::Instant;

use cosmwasm_std::testing::mock_info;
use cosmwasm_std::{Addr, Api, DepsMut, Env, Storage};
use rand::{distributions::Uniform, rngs::StdRng, seq::SliceRandom, thread_rng, Rng, SeedableRng};
use substrate_fixed::transcendental::{cos, ln, sqrt, PI};
use substrate_fixed::types::{I32F32, I64F64};

use crate::contract::{execute, get_economy};
use crate::epoch::{epoch, get_bonds};
use crate::msg::ExecuteMsg;
use crate::registration::create_work_for_block_number;
use crate::root::{get_subnet_emission_value, set_emission_values};
use crate::staking::{get_total_stake_for_hotkey, increase_stake_on_coldkey_hotkey_account};
use crate::test_helpers::{
    add_balance_to_coldkey_account, add_network, instantiate_contract, pow_register_ok_neuron,
    run_step_to_block, set_weights, step_block,
};
use crate::tests::block_step::epoch_dense;
use crate::uids::{append_neuron, get_hotkey_for_net_and_uid, get_subnetwork_n};
use crate::utils::{
    get_activity_cutoff, get_consensus_for_uid, get_dividends_for_uid, get_emission_for_uid,
    get_incentive_for_uid, get_max_allowed_uids, get_max_allowed_validators, get_rank_for_uid,
    get_trust_for_uid, get_validator_permit_for_uid, set_activity_cutoff, set_difficulty,
    set_max_allowed_uids, set_max_allowed_validators, set_max_registrations_per_block,
    set_max_weight_limit, set_min_allowed_weights, set_min_difficulty,
    set_target_registrations_per_interval, set_weights_set_rate_limit,
};

pub fn fixed(val: f32) -> I32F32 {
    I32F32::from_num(val)
}

pub fn fixed_to_u16(x: I32F32) -> u16 {
    x.to_num::<u16>()
}

pub fn fixed_proportion_to_u16(x: I32F32) -> u16 {
    fixed_to_u16(x * I32F32::from_num(u16::MAX))
}

// Normalizes (sum to 1 except 0) the input vector directly in-place.
#[allow(dead_code)]
pub fn inplace_normalize(x: &mut Vec<I32F32>) {
    let x_sum: I32F32 = x.iter().sum();
    if x_sum == I32F32::from_num(0.0 as f32) {
        return;
    }
    for i in 0..x.len() {
        x[i] = x[i] / x_sum;
    }
}

// Inplace normalize the passed positive integer weights so that they sum to u16 max value.
fn normalize_weights(mut weights: Vec<u16>) -> Vec<u16> {
    let sum: u64 = weights.iter().map(|x| *x as u64).sum();
    if sum == 0 {
        return weights;
    }
    weights.iter_mut().for_each(|x| {
        *x = (*x as u64 * u16::max_value() as u64 / sum) as u16;
    });
    return weights;
}

// Return as usize an I32F32 ratio of a usize input, avoiding the 0% and 100% extremes.
fn non_extreme_fixed_ratio(ratio: I32F32, total: usize) -> usize {
    if total == 0 {
        return total;
    }
    let mut subset: usize = (ratio * I32F32::from_num(total)).to_num::<usize>();
    if subset == 0 {
        subset = 1;
    } else if subset == total {
        subset = total - 1;
    }
    return subset;
}

// Box-Muller Transform converting two uniform random samples to a normal random sample.
fn normal(size: usize, rng: &mut StdRng, dist: &Uniform<u16>) -> Vec<I32F32> {
    let max = I32F32::from_num(u16::MAX);
    let two = I32F32::from_num(2);
    let eps = I32F32::from_num(0.000001);
    let pi = I32F32::from_num(PI);

    let uniform_u16: Vec<u16> = (0..(2 * size)).map(|_| rng.sample(&dist)).collect();
    let uniform: Vec<I32F32> = uniform_u16
        .iter()
        .map(|&x| I32F32::from_num(x) / max)
        .collect();
    let mut normal: Vec<I32F32> = vec![I32F32::from_num(0); size as usize];

    for i in 0..size {
        let u1: I32F32 = uniform[i] + eps;
        let u2: I32F32 = uniform[i + size] + eps;
        normal[i] = sqrt::<I32F32, I32F32>(-two * ln::<I32F32, I32F32>(u1).expect("")).expect("")
            * cos(two * pi * u2);
    }
    normal
}

// Returns validators and servers uids with either blockwise, regular, or random interleaving.
fn distribute_nodes(
    validators_n: usize,
    network_n: usize,
    interleave: usize,
) -> (Vec<u16>, Vec<u16>) {
    let mut validators: Vec<u16> = vec![];
    let mut servers: Vec<u16> = vec![];

    if interleave == 0 {
        // blockwise [validator_block, server_block]
        validators = (0..validators_n as u16).collect();
        servers = (validators_n as u16..network_n as u16).collect();
    } else if interleave == 1 {
        // regular interleaving [val, srv, srv, ..., srv, val, srv, srv, ..., srv, val, srv, ..., srv]
        (validators, servers) = (0..network_n as u16)
            .collect::<Vec<u16>>()
            .iter()
            .partition(|&i| *i as usize % (network_n / validators_n) == 0);
    } else if interleave == 2 {
        // random interleaving
        let mut permuted_uids: Vec<u16> = (0..network_n as u16).collect();
        permuted_uids.shuffle(&mut thread_rng());
        validators = permuted_uids[0..validators_n as usize].into();
        servers = permuted_uids[validators_n as usize..network_n as usize].into();
    }

    return (validators, servers);
}

#[allow(dead_code)]
fn uid_stats(store: &dyn Storage, netuid: u16, uid: u16) {
    log::info!(
        "stake: {:?}",
        get_total_stake_for_hotkey(store, &Addr::unchecked((1000 + uid).to_string()))
    );
    log::info!("rank: {:?}", get_rank_for_uid(store, netuid, uid));
    log::info!("trust: {:?}", get_trust_for_uid(store, netuid, uid));
    log::info!("consensus: {:?}", get_consensus_for_uid(store, netuid, uid));
    log::info!("incentive: {:?}", get_incentive_for_uid(store, netuid, uid));
    log::info!("dividend: {:?}", get_dividends_for_uid(store, netuid, uid));
    log::info!("emission: {:?}", get_emission_for_uid(store, netuid, uid));
}

fn init_run_epochs(
    mut deps: DepsMut,
    mut env: &mut Env,
    netuid: u16,
    n: u16,
    validators: &Vec<u16>,
    servers: &Vec<u16>,
    epochs: u16,
    stake_per_validator: u64,
    server_self: bool,
    input_stake: &Vec<u64>,
    use_input_stake: bool,
    input_weights: &Vec<Vec<(u16, u16)>>,
    use_input_weights: bool,
    random_weights: bool,
    random_seed: u64,
    sparse: bool,
) {
    // let (mut deps, mut env) = instantiate_contract();
    // === Create the network
    add_network(deps.storage, netuid, u16::MAX - 1, 0); // set higher tempo to avoid built-in epoch, then manual epoch instead

    // === Register uids
    set_max_allowed_uids(deps.storage, netuid, n);
    for key in 0..n {
        let stake: u64;
        if use_input_stake {
            stake = input_stake[key as usize];
        } else {
            stake = if validators.contains(&key) {
                stake_per_validator
            } else {
                0
            }; // only validators receive stake
        }
        // let stake: u64 = 1; // alternative test: all nodes receive stake, should be same outcome, except stake
        add_balance_to_coldkey_account(&Addr::unchecked((1000 + key).to_string()), stake);
        append_neuron(
            deps.storage,
            deps.api,
            netuid,
            &(Addr::unchecked((1000 + key).to_string())),
            0,
        )
        .unwrap();
        increase_stake_on_coldkey_hotkey_account(
            deps.storage,
            &Addr::unchecked((1000 + key).to_string()),
            &Addr::unchecked((1000 + key).to_string()),
            stake as u64,
        );
    }
    assert_eq!(get_subnetwork_n(deps.storage, netuid), n);

    // === Issue validator permits
    set_max_allowed_validators(deps.storage, netuid, validators.len() as u16);

    assert_eq!(
        get_max_allowed_validators(deps.storage, netuid),
        validators.len() as u16
    );

    epoch(
        deps.storage,
        deps.api,
        netuid,
        1_000_000_000,
        env.block.height,
    )
    .unwrap(); // run first epoch to set allowed validators
    step_block(deps.branch(), &mut env).unwrap(); // run to next block to ensure weights are set on nodes after their registration block

    // === Set weights
    let mut rng = StdRng::seed_from_u64(random_seed); // constant seed so weights over multiple runs are equal
    let range = Uniform::new(0, u16::MAX);
    let mut weights: Vec<u16> = vec![u16::MAX / n; servers.len() as usize];
    for uid in validators {
        if random_weights {
            weights = (0..servers.len()).map(|_| rng.sample(&range)).collect();
            weights = normalize_weights(weights);
            // assert_eq!(weights.iter().map(|x| *x as u64).sum::<u64>(), u16::MAX as u64); // normalized weight sum not always u16::MAX
        }
        if use_input_weights {
            let sparse_weights = input_weights[*uid as usize].clone();
            weights = sparse_weights.iter().map(|(_, w)| *w).collect();
            let srvs: Vec<u16> = sparse_weights.iter().map(|(s, _)| *s).collect();

            let msg = ExecuteMsg::SetWeights {
                netuid,
                dests: srvs.clone(),
                weights: weights.clone(),
                version_key: 0,
            };
            let info = mock_info((1000 + uid).to_string().as_str(), &[]);
            let res = execute(deps.branch(), env.clone(), info, msg);
            assert_eq!(res.is_ok(), true);
        } else {
            let msg = ExecuteMsg::SetWeights {
                netuid,
                dests: servers.clone(),
                weights: weights.clone(),
                version_key: 0,
            };
            let info = mock_info((1000 + uid).to_string().as_str(), &[]);
            let res = execute(deps.branch(), env.clone(), info, msg);
            assert_eq!(res.is_ok(), true);
        }
    }
    for uid in servers {
        if server_self {
            let msg = ExecuteMsg::SetWeights {
                netuid,
                dests: vec![*uid as u16],
                weights: vec![u16::MAX],
                version_key: 0,
            }; // server self-weight
            let info = mock_info((1000 + uid).to_string().as_str(), &[]);
            let res = execute(deps.branch(), env.clone(), info, msg);
            assert_eq!(res.is_ok(), true);
        }
    }

    // === Run the epochs.
    log::info!("Start {epochs} epoch(s)");
    let start = Instant::now();
    for _ in 0..epochs {
        if sparse {
            epoch(
                deps.storage,
                deps.api,
                netuid,
                1_000_000_000,
                env.block.height,
            )
            .unwrap();
        } else {
            epoch_dense(deps.storage, netuid, 1_000_000_000, env.block.height);
        }
    }
    let duration = start.elapsed();
    log::info!(
        "Time elapsed in (sparse={sparse}) epoch() is: {:?}",
        duration
    );

    // let bonds = get_bonds(&deps.storage, netuid );
    // for (uid, node) in vec![ (validators[0], "validator"), (servers[0], "server") ] {
    // 	log::info!("\n{node}" );
    // 	uid_stats(netuid, uid);
    // 	log::info!("bonds: {:?} (on validator), {:?} (on server)", bonds[uid as usize][0], bonds[uid as usize][servers[0] as usize]);
    // }
}

// Generate a random graph that is split into a major and minor set, each setting specific weight on itself and the complement on the other.
fn split_graph(
    major_stake: I32F32,
    major_weight: I32F32,
    minor_weight: I32F32,
    weight_stddev: I32F32,
    validators_n: usize,
    network_n: usize,
    interleave: usize,
) -> (
    Vec<u16>,
    Vec<u16>,
    Vec<u16>,
    Vec<u16>,
    Vec<u16>,
    Vec<u16>,
    Vec<u64>,
    Vec<Vec<(u16, u16)>>,
    I32F32,
) {
    let servers_n: usize = network_n - validators_n;
    let major_servers_n: usize = non_extreme_fixed_ratio(major_stake, servers_n);
    let major_validators_n: usize = non_extreme_fixed_ratio(major_stake, validators_n);

    let (validators, servers) = distribute_nodes(validators_n, network_n, interleave as usize);
    let major_validators: Vec<u16> = (0..major_validators_n).map(|i| validators[i]).collect();
    let minor_validators: Vec<u16> = (major_validators_n..validators_n)
        .map(|i| validators[i])
        .collect();
    let major_servers: Vec<u16> = (0..major_servers_n).map(|i| servers[i]).collect();
    let minor_servers: Vec<u16> = (major_servers_n..servers_n).map(|i| servers[i]).collect();

    let zero: I32F32 = I32F32::from_num(0);
    let one: I32F32 = I32F32::from_num(1);
    let stddev: I32F32 = I32F32::from_num(0.3);
    let total_stake: I64F64 = I64F64::from_num(21_000_000_000_000_000 as u64);
    let mut rng = StdRng::seed_from_u64(0); // constant seed so weights over multiple runs are equal
    let dist = Uniform::new(0, u16::MAX);

    let mut stake: Vec<u64> = vec![0; network_n];
    let mut stake_fixed: Vec<I32F32> = vec![zero; network_n];
    for (ratio, vals) in vec![
        (major_stake, &major_validators),
        (one - major_stake, &minor_validators),
    ] {
        let mut sample = normal(vals.len(), &mut rng, &dist)
            .iter()
            .map(|x: &I32F32| {
                let v: I32F32 = (stddev * x) + one;
                if v < zero {
                    zero
                } else {
                    v
                }
            })
            .collect();
        inplace_normalize(&mut sample);
        for (i, &val) in vals.iter().enumerate() {
            stake[val as usize] =
                (I64F64::from_num(ratio) * I64F64::from_num(sample[i]) * total_stake)
                    .to_num::<u64>();
            stake_fixed[val as usize] =
                I32F32::from_num(I64F64::from_num(ratio) * I64F64::from_num(sample[i]));
        }
    }

    let mut weights: Vec<Vec<(u16, u16)>> = vec![vec![]; network_n as usize];
    let mut weights_fixed: Vec<Vec<I32F32>> = vec![vec![zero; network_n]; network_n];
    for (first, second, vals) in vec![
        (major_weight, one - major_weight, &major_validators),
        (one - minor_weight, minor_weight, &minor_validators),
    ] {
        for &val in vals {
            for (weight, srvs) in vec![(first, &major_servers), (second, &minor_servers)] {
                let mut sample: Vec<I32F32> = normal(srvs.len(), &mut rng, &dist)
                    .iter()
                    .map(|x: &I32F32| {
                        let v: I32F32 = (weight_stddev * x) + one;
                        if v < zero {
                            zero
                        } else {
                            v
                        }
                    })
                    .collect();
                inplace_normalize(&mut sample);

                for (i, &srv) in srvs.iter().enumerate() {
                    weights[val as usize].push((srv, fixed_proportion_to_u16(weight * sample[i])));
                    weights_fixed[val as usize][srv as usize] = weight * sample[i];
                }
            }
            inplace_normalize(&mut weights_fixed[val as usize]);
        }
    }

    inplace_normalize(&mut stake_fixed);

    // Calculate stake-weighted mean per server
    let mut weight_mean: Vec<I32F32> = vec![zero; network_n];
    for val in 0..network_n {
        if stake_fixed[val] > zero {
            for srv in 0..network_n {
                weight_mean[srv] += stake_fixed[val] * weights_fixed[val][srv];
            }
        }
    }

    // Calculate stake-weighted absolute standard deviation
    let mut weight_dev: Vec<I32F32> = vec![zero; network_n];
    for val in 0..network_n {
        if stake_fixed[val] > zero {
            for srv in 0..network_n {
                weight_dev[srv] +=
                    stake_fixed[val] * (weight_mean[srv] - weights_fixed[val][srv]).abs();
            }
        }
    }

    // Calculate rank-weighted mean of weight_dev
    let avg_weight_dev: I32F32 =
        weight_dev.iter().sum::<I32F32>() / weight_mean.iter().sum::<I32F32>();

    (
        validators,
        servers,
        major_validators,
        minor_validators,
        major_servers,
        minor_servers,
        stake,
        weights,
        avg_weight_dev,
    )
}

// Test consensus guarantees with an epoch on a graph with 4096 nodes, of which the first 128 are validators, the graph is split into a major and minor set, each setting specific weight on itself and the complement on the other. Asserts that the major emission ratio >= major stake ratio.
// #[test]
// fn test_consensus_guarantees() {
//     let netuid: u16 = 0;
//     let network_n: u16 = 512;
//     let validators_n: u16 = 64;
//     let epochs: u16 = 1;
//     let interleave = 2;
//     log::info!("test_consensus_guarantees ({network_n:?}, {validators_n:?} validators)");
//     for (major_stake, major_weight, minor_weight, weight_stddev) in vec![
//         (0.51, 1., 1., 0.001),
//         (0.51, 0.03, 0., 0.001),
//         (0.51, 0.51, 0.49, 0.001),
//         (0.51, 0.51, 1., 0.001),
//         (0.51, 0.61, 0.8, 0.1),
//         (0.6, 0.67, 0.65, 0.2),
//         (0.6, 0.74, 0.77, 0.4),
//         (0.6, 0.76, 0.8, 0.4),
//         (0.6, 0.76, 1., 0.4),
//         (0.6, 0.92, 1., 0.4),
//         (0.6, 0.94, 1., 0.4),
//         (0.65, 0.78, 0.85, 0.6),
//         (0.7, 0.81, 0.85, 0.8),
//         (0.7, 0.83, 0.85, 1.),
//     ] {
//         let (
//             validators,
//             servers,
//             major_validators,
//             minor_validators,
//             major_servers,
//             minor_servers,
//             stake,
//             weights,
//             _avg_weight_dev,
//         ) = split_graph(
//             fixed(major_stake),
//             fixed(major_weight),
//             fixed(minor_weight),
//             fixed(weight_stddev),
//             validators_n as usize,
//             network_n as usize,
//             interleave as usize,
//         );

//         new_test_ext().execute_with(|| {
//             init_run_epochs(
//                 netuid,
//                 network_n,
//                 &validators,
//                 &servers,
//                 epochs,
//                 1,
//                 true,
//                 &stake,
//                 true,
//                 &weights,
//                 true,
//                 false,
//                 0,
//                 false,
//             );

//             let mut major_emission: I64F64 = I64F64::from_num(0);
//             let mut minor_emission: I64F64 = I64F64::from_num(0);
//             for set in vec![major_validators, major_servers] {
//                 for uid in set {
//                     major_emission +=
//                         I64F64::from_num(get_emission_for_uid(netuid, uid));
//                 }
//             }
//             for set in vec![minor_validators, minor_servers] {
//                 for uid in set {
//                     minor_emission +=
//                         I64F64::from_num(get_emission_for_uid(netuid, uid));
//                 }
//             }
//             let major_ratio: I32F32 =
//                 I32F32::from_num(major_emission / (major_emission + minor_emission));
//             assert!(major_stake <= major_ratio);
//         });
//     }
// }

// Test an epoch on an empty graph.
// #[test]
// fn test_overflow() {
//     new_test_ext().execute_with(|| {
//         log::info!("test_overflow:");
//         let netuid: u16 = 1;
//         add_network(netuid, 0, 0);
//         set_max_allowed_uids(netuid, 3);
//         increase_stake_on_coldkey_hotkey_account(
//             Addr::unchecked(1000.to_string()),
//             Addr::unchecked(1000.to_string()),
//             10,
//         );
//         increase_stake_on_coldkey_hotkey_account(
//             Addr::unchecked(1001.to_string()),
//             Addr::unchecked(1001.to_string()),
//             10,
//         );
//         increase_stake_on_coldkey_hotkey_account(
//             Addr::unchecked(1002.to_string()),
//             Addr::unchecked(1002.to_string()),
//             10,
//         );
//         append_neuron(netuid, Addr::unchecked(1000.to_string()), 0);
//         append_neuron(netuid, Addr::unchecked(1001.to_string()), 0);
//         append_neuron(netuid, Addr::unchecked(1002.to_string()), 0);
//         set_validator_permit_for_uid(0, 0, true);
//         set_validator_permit_for_uid(0, 1, true);
//         set_validator_permit_for_uid(0, 2, true);
//         assert_ok!(set_weights(
//             Addr::unchecked(1000.to_string())),
//             netuid,
//             vec![0, 1, 2],
//             vec![u16::MAX / 3, u16::MAX / 3, u16::MAX],
//             0
//         ));
//         assert_ok!(set_weights(
//             Addr::unchecked(1001.to_string())),
//             netuid,
//             vec![1, 2],
//             vec![u16::MAX / 2, u16::MAX / 2],
//             0
//         ));
//         assert_ok!(set_weights(
//             Addr::unchecked(1002.to_string())),
//             netuid,
//             vec![2],
//             vec![u16::MAX],
//             0
//         ));
//         epoch(0, u64::MAX);
//     });
// }

// Test an epoch on an empty graph.
// #[test]
// fn test_nill_epoch_subtensor() {
//     new_test_ext().execute_with(|| {
//         log::info!("test_nill_epoch:");
//         epoch(0, 0);
//     });
// }

// Test an epoch on a graph with a single item.
#[test]
fn test_1_graph() {
    let (mut deps, mut env) = instantiate_contract();

    log::info!("test_1_graph:");
    let netuid: u16 = 2;
    let coldkey = "addr0";
    let hotkey = "addr0";
    let uid: u16 = 0;
    let stake_amount: u64 = 1;
    add_network(&mut deps.storage, netuid, u16::MAX - 1, 0); // set higher tempo to avoid built-in epoch, then manual epoch instead
    set_max_allowed_uids(&mut deps.storage, netuid, 1);
    add_balance_to_coldkey_account(&Addr::unchecked(hotkey), stake_amount);
    append_neuron(
        &mut deps.storage,
        &deps.api,
        netuid,
        &Addr::unchecked(hotkey),
        0,
    )
    .unwrap();
    increase_stake_on_coldkey_hotkey_account(
        &mut deps.storage,
        &Addr::unchecked(coldkey),
        &Addr::unchecked(hotkey),
        stake_amount,
    );
    assert_eq!(get_subnetwork_n(&deps.storage, netuid), 1);
    // run_to_block(1);
    step_block(deps.as_mut(), &mut env).unwrap(); // run to next block to ensure weights are set on nodes after their registration block

    set_weights(
        deps.as_mut(),
        env.clone(),
        hotkey,
        netuid,
        vec![uid as u16],
        vec![u16::MAX],
        0,
    )
    .unwrap();

    // set_weights_for_testing( netuid, i as u16, vec![ ( 0, u16::MAX )]); // doesn't set update status
    // set_bonds_for_testing( netuid, uid, vec![ ( 0, u16::MAX )]); // rather, bonds are calculated in epoch
    set_emission_values(
        &mut deps.storage,
        &deps.api,
        &vec![netuid],
        vec![1_000_000_000],
    )
    .unwrap();
    assert_eq!(
        get_subnet_emission_value(&deps.storage, netuid),
        1_000_000_000
    );
    epoch(
        &mut deps.storage,
        &deps.api,
        netuid,
        1_000_000_000,
        env.block.height,
    )
    .unwrap();
    assert_eq!(
        get_total_stake_for_hotkey(&deps.storage, &Addr::unchecked(hotkey)),
        stake_amount
    );
    assert_eq!(get_rank_for_uid(&deps.storage, netuid, uid), 0);
    assert_eq!(get_trust_for_uid(&deps.storage, netuid, uid), 0);
    assert_eq!(get_consensus_for_uid(&deps.storage, netuid, uid), 0);
    assert_eq!(get_incentive_for_uid(&deps.storage, netuid, uid), 0);
    assert_eq!(get_dividends_for_uid(&deps.storage, netuid, uid), 0);
    assert_eq!(
        get_emission_for_uid(&deps.storage, netuid, uid),
        1_000_000_000
    );
}

// Test an epoch on a graph with two items.
#[test]
fn test_10_graph() {
    let (mut deps, mut env) = instantiate_contract();

    log::info!("test_10_graph");
    // Function for adding a nodes to the graph.
    pub fn add_node(
        store: &mut dyn Storage,
        api: &dyn Api,
        netuid: u16,
        coldkey: Addr,
        hotkey: Addr,
        uid: u16,
        stake_amount: u64,
    ) {
        log::info!(
            "+Add net:{:?} coldkey:{:?} hotkey:{:?} uid:{:?} stake_amount: {:?} subn: {:?}",
            netuid,
            coldkey,
            hotkey,
            uid,
            stake_amount,
            get_subnetwork_n(store, netuid),
        );
        append_neuron(store, api, netuid, &hotkey, 1).unwrap();
        increase_stake_on_coldkey_hotkey_account(store, &coldkey, &hotkey, stake_amount);
        assert_eq!(get_subnetwork_n(store, netuid) - 1, uid);
    }
    // Build the graph with 10 items
    // each with 1 stake and self weights.
    let n: usize = 10;
    let netuid: u16 = 2;
    add_network(&mut deps.storage, netuid, u16::MAX - 1, 0); // set higher tempo to avoid built-in epoch, then manual epoch instead
    set_max_allowed_uids(&mut deps.storage, netuid, n as u16);
    set_weights_set_rate_limit(&mut deps.storage, netuid, 0);
    for i in 0..10 {
        add_node(
            &mut deps.storage,
            &deps.api,
            netuid,
            Addr::unchecked(i.to_string()),
            Addr::unchecked(i.to_string()),
            i as u16,
            1,
        );

        let gas = deps.storage.gas_used.borrow();
        println!(
            "total {:?} gas {:?} write {:?} read {:?}",
            gas.total, gas.last, gas.write_cnt, gas.read_cnt
        );
        drop(gas);
    }
    assert_eq!(get_subnetwork_n(&deps.storage, netuid), 10);
    env.block.height += 1; // run to next block to ensure weights are set on nodes after their registration block
    for i in 0..10 {
        let msg = ExecuteMsg::SetWeights {
            netuid,
            dests: vec![i as u16],
            weights: vec![u16::MAX],
            version_key: 0,
        }; // server self-weight
        let info = mock_info(i.to_string().as_str(), &[]);
        let res = execute(deps.as_mut(), env.clone(), info, msg);
        // println!("{:?} {:?}",i, res);
        assert_eq!(res.is_ok(), true);

        let gas = deps.storage.gas_used.borrow();
        println!(
            "total {:?} gas {:?} write {:?} read {:?}",
            gas.total, gas.last, gas.write_cnt, gas.read_cnt
        );
        drop(gas);
    }
    // Run the epoch.
    epoch(
        &mut deps.storage,
        &deps.api,
        netuid,
        1_000_000_000,
        env.block.height,
    )
    .unwrap();
    let gas = deps.storage.gas_used.borrow();
    println!(
        "total {:?} gas {:?} write {:?} read {:?}",
        gas.total, gas.last, gas.write_cnt, gas.read_cnt
    );
    drop(gas);
    // Check return values.
    for i in 0..n {
        assert_eq!(
            get_total_stake_for_hotkey(&deps.storage, &Addr::unchecked(i.to_string())),
            1
        );
        assert_eq!(get_rank_for_uid(&deps.storage, netuid, i as u16), 0);
        assert_eq!(get_trust_for_uid(&deps.storage, netuid, i as u16), 0);
        assert_eq!(get_consensus_for_uid(&deps.storage, netuid, i as u16), 0);
        assert_eq!(get_incentive_for_uid(&deps.storage, netuid, i as u16), 0);
        assert_eq!(get_dividends_for_uid(&deps.storage, netuid, i as u16), 0);
        assert_eq!(
            get_emission_for_uid(&deps.storage, netuid, i as u16),
            99999999
        );
    }
}

// Test an epoch on a graph with 512 nodes, of which the first 64 are validators setting non-self weights, and the rest servers setting only self-weights.
#[test]
fn test_512_graph() {
    let netuid: u16 = 2;
    let network_n: u16 = 512;
    let validators_n: u16 = 64;
    let max_stake_per_validator: u64 = 328_125_000_000_000; // 21_000_000_000_000_000 / 64
    let epochs: u16 = 3;
    log::info!("test_{network_n:?}_graph ({validators_n:?} validators)");
    for interleave in 0..3 {
        for server_self in vec![false, true] {
            // server-self weight off/on
            let (validators, servers) = distribute_nodes(
                validators_n as usize,
                network_n as usize,
                interleave as usize,
            );
            let server: usize = servers[0] as usize;
            let validator: usize = validators[0] as usize;
            let (mut deps, mut env) = instantiate_contract();
            init_run_epochs(
                deps.as_mut(),
                &mut env,
                netuid,
                network_n,
                &validators,
                &servers,
                epochs,
                max_stake_per_validator,
                server_self,
                &vec![],
                false,
                &vec![],
                false,
                false,
                0,
                false,
            );
            let bonds = get_bonds(&deps.storage, netuid);
            for uid in validators {
                assert_eq!(
                    get_total_stake_for_hotkey(
                        &deps.storage,
                        &Addr::unchecked((1000 + uid).to_string())
                    ),
                    max_stake_per_validator
                );
                assert_eq!(get_rank_for_uid(&deps.storage, netuid, uid), 0);
                assert_eq!(get_trust_for_uid(&deps.storage, netuid, uid), 0);
                assert_eq!(get_consensus_for_uid(&deps.storage, netuid, uid), 0);
                assert_eq!(get_incentive_for_uid(&deps.storage, netuid, uid), 0);
                assert_eq!(get_dividends_for_uid(&deps.storage, netuid, uid), 1023); // Note D = floor(1 / 64 * 65_535) = 1023
                assert_eq!(get_emission_for_uid(&deps.storage, netuid, uid), 7812500); // Note E = 0.5 / 200 * 1_000_000_000 = 7_812_500
                assert_eq!(bonds[uid as usize][validator], 0.0);
                assert_eq!(bonds[uid as usize][server], I32F32::from_num(65_535));
                // Note B_ij = floor(1 / 64 * 65_535) / 65_535 = 1023 / 65_535, then max-upscaled to 65_535
            }
            for uid in servers {
                assert_eq!(
                    get_total_stake_for_hotkey(
                        &deps.storage,
                        &Addr::unchecked((1000 + uid).to_string())
                    ),
                    0
                );
                assert_eq!(get_rank_for_uid(&deps.storage, netuid, uid), 146); // Note R = floor(1 / (512 - 64) * 65_535) = 146
                assert_eq!(get_trust_for_uid(&deps.storage, netuid, uid), 65535);
                assert_eq!(get_consensus_for_uid(&deps.storage, netuid, uid), 146); // Note C = floor(1 / (512 - 64) * 65_535) = 146
                assert_eq!(get_incentive_for_uid(&deps.storage, netuid, uid), 146); // Note I = floor(1 / (512 - 64) * 65_535) = 146
                assert_eq!(get_dividends_for_uid(&deps.storage, netuid, uid), 0);
                assert_eq!(get_emission_for_uid(&deps.storage, netuid, uid), 1116071); // Note E = floor(0.5 / (512 - 64) * 1_000_000_000) = 1_116_071
                assert_eq!(bonds[uid as usize][validator], 0.0);
                assert_eq!(bonds[uid as usize][server], 0.0);
            }
            drop(deps);
            drop(env);
        }
    }
}

// Test an epoch on a graph with 512 nodes, of which the first 64 are validators setting random non-self weights, and the rest servers setting only self-weights.
#[test]
fn test_512_graph_random_weights() {
    let netuid: u16 = 2;
    let network_n: u16 = 512;
    let validators_n: u16 = 64;
    let epochs: u16 = 1;
    log::info!("test_{network_n:?}_graph_random_weights ({validators_n:?} validators)");
    for interleave in 0..3 {
        for server_self in vec![false, true] {
            // server-self weight off/on
            let (validators, servers) = distribute_nodes(
                validators_n as usize,
                network_n as usize,
                interleave as usize,
            );
            let server: usize = servers[0] as usize;
            let validator: usize = validators[0] as usize;
            let (mut rank, mut incentive, mut dividend, mut emission, mut bondv, mut bonds): (
                Vec<u16>,
                Vec<u16>,
                Vec<u16>,
                Vec<u64>,
                Vec<I32F32>,
                Vec<I32F32>,
            ) = (vec![], vec![], vec![], vec![], vec![], vec![]);

            // Dense epoch
            let (mut deps, mut env) = instantiate_contract();
            init_run_epochs(
                deps.as_mut(),
                &mut env,
                netuid,
                network_n,
                &validators,
                &servers,
                epochs,
                1,
                server_self,
                &vec![],
                false,
                &vec![],
                false,
                true,
                interleave as u64,
                false,
            );

            let bond = get_bonds(&deps.storage, netuid);
            for uid in 0..network_n {
                rank.push(get_rank_for_uid(&deps.storage, netuid, uid));
                incentive.push(get_incentive_for_uid(&deps.storage, netuid, uid));
                dividend.push(get_dividends_for_uid(&deps.storage, netuid, uid));
                emission.push(get_emission_for_uid(&deps.storage, netuid, uid));
                bondv.push(bond[uid as usize][validator]);
                bonds.push(bond[uid as usize][server]);
            }
            drop(deps);
            drop(env);

            // Sparse epoch (same random seed as dense)
            let (mut deps, mut env) = instantiate_contract();
            init_run_epochs(
                deps.as_mut(),
                &mut env,
                netuid,
                network_n,
                &validators,
                &servers,
                epochs,
                1,
                server_self,
                &vec![],
                false,
                &vec![],
                false,
                true,
                interleave as u64,
                true,
            );
            // Assert that dense and sparse epoch results are equal
            let bond = get_bonds(&deps.storage, netuid);
            for uid in 0..network_n {
                assert_eq!(
                    get_rank_for_uid(&deps.storage, netuid, uid),
                    rank[uid as usize]
                );
                assert_eq!(
                    get_incentive_for_uid(&deps.storage, netuid, uid),
                    incentive[uid as usize]
                );
                assert_eq!(
                    get_dividends_for_uid(&deps.storage, netuid, uid),
                    dividend[uid as usize]
                );
                assert_eq!(
                    get_emission_for_uid(&deps.storage, netuid, uid),
                    emission[uid as usize]
                );
                assert_eq!(bond[uid as usize][validator], bondv[uid as usize]);
                assert_eq!(bond[uid as usize][server], bonds[uid as usize]);
            }
            drop(deps);
            drop(env);
        }
    }
}

// // Test an epoch on a graph with 4096 nodes, of which the first 256 are validators setting non-self weights, and the rest servers setting only self-weights.
// TODO revisit because commented also in subtensor repo and tests fails
// #[test]
#[allow(dead_code)]
fn test_4096_graph() {
    let netuid: u16 = 2;
    let network_n: u16 = 4096;
    let validators_n: u16 = 256;
    let epochs: u16 = 2;
    let max_stake_per_validator: u64 = 82_031_250_000_000;
    // 21_000_000_000_000_000 / 256
    log::info!("test_{network_n:?}_graph ({validators_n:?} validators)");
    for interleave in 0..3 {
        let (validators, servers) = distribute_nodes(
            validators_n as usize,
            network_n as usize,
            interleave as usize,
        );
        let server: usize = servers[0] as usize;
        let validator: usize = validators[0] as usize;
        for server_self in vec![false, true] {
            // server-self weight off/on
            let (mut deps, mut env) = instantiate_contract();
            init_run_epochs(
                deps.as_mut(),
                &mut env,
                netuid,
                network_n,
                &validators,
                &servers,
                epochs,
                max_stake_per_validator,
                server_self,
                &vec![],
                false,
                &vec![],
                false,
                false,
                0,
                true,
            );
            // Because of genesis init
            // assert_eq!(get_total_stake(&deps.storage), 21_000_000_000_000_300);
            // assert_eq!(get_total_stake(&deps.storage), 21_000_000_000_000_000);
            let bonds = get_bonds(&deps.storage, netuid);
            for uid in &validators {
                assert_eq!(
                    get_total_stake_for_hotkey(
                        &deps.storage,
                        &Addr::unchecked((*uid as u64).to_string())
                    ),
                    max_stake_per_validator
                );
                assert_eq!(get_rank_for_uid(&deps.storage, netuid, *uid), 0);
                assert_eq!(get_trust_for_uid(&deps.storage, netuid, *uid), 0);
                assert_eq!(get_consensus_for_uid(&deps.storage, netuid, *uid), 0);
                assert_eq!(get_incentive_for_uid(&deps.storage, netuid, *uid), 0);
                assert_eq!(get_dividends_for_uid(&deps.storage, netuid, *uid), 255); // Note D = floor(1 / 256 * 65_535)
                assert_eq!(get_emission_for_uid(&deps.storage, netuid, *uid), 1953125); // Note E = 0.5 / 256 * 1_000_000_000 = 1953125
                assert_eq!(bonds[*uid as usize][validator], 0.0);
                assert_eq!(
                    bonds[*uid as usize][server],
                    I32F32::from_num(255) / I32F32::from_num(65_535)
                ); // Note B_ij = floor(1 / 256 * 65_535) / 65_535
            }
            for uid in &servers {
                assert_eq!(
                    get_total_stake_for_hotkey(
                        &deps.storage,
                        &Addr::unchecked((*uid as u64).to_string())
                    ),
                    0
                );
                assert_eq!(get_rank_for_uid(&deps.storage, netuid, *uid), 17); // Note R = floor(1 / (4096 - 256) * 65_535) = 17
                assert_eq!(get_trust_for_uid(&deps.storage, netuid, *uid), 65535);
                assert_eq!(get_consensus_for_uid(&deps.storage, netuid, *uid), 17); // Note C = floor(1 / (4096 - 256) * 65_535) = 17
                assert_eq!(get_incentive_for_uid(&deps.storage, netuid, *uid), 17); // Note I = floor(1 / (4096 - 256) * 65_535) = 17
                assert_eq!(get_dividends_for_uid(&deps.storage, netuid, *uid), 0);
                assert_eq!(get_emission_for_uid(&deps.storage, netuid, *uid), 130208); // Note E = floor(0.5 / (4096 - 256) * 1_000_000_000) = 130208
                assert_eq!(bonds[*uid as usize][validator], 0.0);
                assert_eq!(bonds[*uid as usize][server], 0.0);
            }
            drop(deps);
            drop(env);
        }
    }
}

// // Test an epoch_sparse on a graph with 16384 nodes, of which the first 512 are validators setting non-self weights, and the rest servers setting only self-weights.
// TODO revisit because commented also in subtensor repo and tests fails
// #[test]
#[allow(dead_code)]
fn test_16384_graph_sparse() {
    let (mut deps, mut env) = instantiate_contract();
    let netuid: u16 = 2;
    let n: u16 = 16384;
    let validators_n: u16 = 512;
    let validators: Vec<u16> = (0..validators_n).collect();
    let servers: Vec<u16> = (validators_n..n).collect();
    let server: u16 = servers[0];
    let epochs: u16 = 1;
    log::info!("test_{n:?}_graph ({validators_n:?} validators)");

    init_run_epochs(
        deps.as_mut(),
        &mut env,
        netuid,
        n,
        &validators,
        &servers,
        epochs,
        1,
        false,
        &vec![],
        false,
        &vec![],
        false,
        false,
        0,
        true,
    );
    let bonds = get_bonds(&deps.storage, netuid);
    for uid in validators {
        assert_eq!(
            get_total_stake_for_hotkey(&deps.storage, &Addr::unchecked((1000 + uid).to_string())),
            1
        );
        assert_eq!(get_rank_for_uid(&deps.storage, netuid, uid), 0);
        assert_eq!(get_trust_for_uid(&deps.storage, netuid, uid), 0);
        assert_eq!(get_consensus_for_uid(&deps.storage, netuid, uid), 438); // Note C = 0.0066928507 = (0.0066928507*65_535) = floor( 438.6159706245 )
        assert_eq!(get_incentive_for_uid(&deps.storage, netuid, uid), 0);
        assert_eq!(get_dividends_for_uid(&deps.storage, netuid, uid), 127); // Note D = floor(1 / 512 * 65_535) = 127
        assert_eq!(get_emission_for_uid(&deps.storage, netuid, uid), 976085); // Note E = 0.5 / 512 * 1_000_000_000 = 976_562 (discrepancy)
        assert_eq!(bonds[uid as usize][0], 0.0);
        assert_eq!(
            bonds[uid as usize][server as usize],
            I32F32::from_num(127) / I32F32::from_num(65_535)
        ); // Note B_ij = floor(1 / 512 * 65_535) / 65_535 = 127 / 65_535
    }
    for uid in servers {
        assert_eq!(
            get_total_stake_for_hotkey(&deps.storage, &Addr::unchecked((1000 + uid).to_string())),
            0
        );
        assert_eq!(get_rank_for_uid(&deps.storage, netuid, uid), 4); // Note R = floor(1 / (16384 - 512) * 65_535) = 4
        assert_eq!(get_trust_for_uid(&deps.storage, netuid, uid), 65535);
        assert_eq!(get_consensus_for_uid(&deps.storage, netuid, uid), 4); // Note C = floor(1 / (16384 - 512) * 65_535) = 4
        assert_eq!(get_incentive_for_uid(&deps.storage, netuid, uid), 4); // Note I = floor(1 / (16384 - 512) * 65_535) = 4
        assert_eq!(get_dividends_for_uid(&deps.storage, netuid, uid), 0);
        assert_eq!(get_emission_for_uid(&deps.storage, netuid, uid), 31517); // Note E = floor(0.5 / (16384 - 512) * 1_000_000_000) = 31502 (discrepancy)
        assert_eq!(bonds[uid as usize][0], 0.0);
        assert_eq!(bonds[uid as usize][server as usize], 0.0);
    }
}

// // Test bonds exponential moving average over a sequence of epochs.
#[test]
fn test_bonds() {
    let (mut deps, mut env) = instantiate_contract();
    let sparse: bool = true;
    let n: u16 = 8;
    let netuid: u16 = 2;
    let tempo: u16 = u16::MAX - 1; // high tempo to skip automatic epochs in on_initialize, use manual epochs instead
    let block_number: u64 = 0;
    let max_stake: u64 = 4;
    let stakes: Vec<u64> = vec![1, 2, 3, 4, 0, 0, 0, 0];
    add_network(&mut deps.storage, netuid, tempo, 0);
    set_max_allowed_uids(&mut deps.storage, netuid, n);
    assert_eq!(get_max_allowed_uids(&deps.storage, netuid), n);
    set_max_registrations_per_block(&mut deps.storage, netuid, n);
    set_target_registrations_per_interval(&mut deps.storage, netuid, n);
    set_weights_set_rate_limit(&mut deps.storage, netuid, 0);
    set_min_allowed_weights(&mut deps.storage, netuid, 1);
    set_max_weight_limit(&mut deps.storage, netuid, u16::MAX);
    set_min_difficulty(&mut deps.storage, netuid, 1_000);
    set_difficulty(&mut deps.storage, netuid, 1_000);

    // === Register [validator1, validator2, validator3, validator4, server1, server2, server3, server4]
    for key in 0..n as u64 {
        add_balance_to_coldkey_account(&Addr::unchecked((1000 + key).to_string()), max_stake);
        let (nonce, work): (u64, Vec<u8>) = create_work_for_block_number(
            &deps.storage,
            netuid,
            block_number,
            key * 1_000_000,
            Addr::unchecked((1000 + key).to_string()).as_str(),
        );
        pow_register_ok_neuron(
            deps.as_mut(),
            env.clone(),
            netuid,
            block_number,
            nonce,
            work,
            Addr::unchecked((1000 + key).to_string()).as_str(),
            Addr::unchecked((1000 + key).to_string()).as_str(),
        )
        .unwrap();
        increase_stake_on_coldkey_hotkey_account(
            &mut deps.storage,
            &Addr::unchecked((1000 + key).to_string()),
            &Addr::unchecked((1000 + key).to_string()),
            stakes[key as usize],
        );
    }
    assert_eq!(get_max_allowed_uids(&deps.storage, netuid), n);
    assert_eq!(get_subnetwork_n(&deps.storage, netuid), n);

    // === Issue validator permits
    set_max_allowed_validators(&mut deps.storage, netuid, n);
    assert_eq!(get_max_allowed_validators(&deps.storage, netuid), n);
    epoch(
        &mut deps.storage,
        &deps.api,
        netuid,
        1_000_000_000,
        env.block.height,
    )
    .unwrap(); // run first epoch to set allowed validators
    step_block(deps.as_mut(), &mut env).unwrap(); // run to next block to ensure weights are set on nodes after their registration block

    // === Set weights [val->srv1: 0.1, val->srv2: 0.2, val->srv3: 0.3, val->srv4: 0.4]
    for uid in 0..(n / 2) as u64 {
        set_weights(
            deps.as_mut(),
            env.clone(),
            Addr::unchecked((1000 + uid).to_string()).as_str(),
            netuid,
            ((n / 2)..n).collect(),
            vec![u16::MAX / 4, u16::MAX / 2, (u16::MAX / 4) * 3, u16::MAX],
            0,
        )
        .unwrap();
    }
    if sparse {
        epoch(
            &mut deps.storage,
            &deps.api,
            netuid,
            1_000_000_000,
            env.block.height,
        )
        .unwrap();
    } else {
        epoch_dense(&mut deps.storage, netuid, 1_000_000_000, env.block.height);
    }
    /*  n: 8
    current_block: 1; activity_cutoff: 5000; Last update: [1, 1, 1, 1, 0, 0, 0, 0]
    Inactive: [false, false, false, false, false, false, false, false]
    Block at registration: [0, 0, 0, 0, 0, 0, 0, 0]
    hotkeys: [(0, 0), (1, 1), (2, 2), (3, 3), (4, 4), (5, 5), (6, 6), (7, 7)]
    S: [0.0999999999, 0.2, 0.2999999998, 0.4, 0, 0, 0, 0]
    validator_permits: [true, true, true, true, true, true, true, true]
    max_allowed_validators: 8
    new_validator_permits: [true, true, true, true, true, true, true, true]
    S: [0.0999999999, 0.2, 0.2999999998, 0.4, 0, 0, 0, 0]
    W: [[(4, 16383), (5, 32767), (6, 49149), (7, 65535)], [(4, 16383), (5, 32767), (6, 49149), (7, 65535)], [(4, 16383), (5, 32767), (6, 49149), (7, 65535)], [(4, 16383), (5, 32767), (6, 49149), (7, 65535)], [], [], [], []]
    W (permit): [[(4, 16383), (5, 32767), (6, 49149), (7, 65535)], [(4, 16383), (5, 32767), (6, 49149), (7, 65535)], [(4, 16383), (5, 32767), (6, 49149), (7, 65535)], [(4, 16383), (5, 32767), (6, 49149), (7, 65535)], [], [], [], []]
    W (permit+diag): [[(4, 16383), (5, 32767), (6, 49149), (7, 65535)], [(4, 16383), (5, 32767), (6, 49149), (7, 65535)], [(4, 16383), (5, 32767), (6, 49149), (7, 65535)], [(4, 16383), (5, 32767), (6, 49149), (7, 65535)], [], [], [], []]
    W (permit+diag+outdate): [[(4, 16383), (5, 32767), (6, 49149), (7, 65535)], [(4, 16383), (5, 32767), (6, 49149), (7, 65535)], [(4, 16383), (5, 32767), (6, 49149), (7, 65535)], [(4, 16383), (5, 32767), (6, 49149), (7, 65535)], [], [], [], []]
    W (mask+norm): [[(4, 0.0999975584), (5, 0.2000012207), (6, 0.2999926754), (7, 0.400008545)], [(4, 0.0999975584), (5, 0.2000012207), (6, 0.2999926754), (7, 0.400008545)], [(4, 0.0999975584), (5, 0.2000012207), (6, 0.2999926754), (7, 0.400008545)], [(4, 0.0999975584), (5, 0.2000012207), (6, 0.2999926754), (7, 0.400008545)], [], [], [], []]
    R (before): [0, 0, 0, 0, 0.099997558, 0.2000012202, 0.2999926745, 0.4000085443]
    C: [0, 0, 0, 0, 0.0999975584, 0.2000012207, 0.2999926754, 0.400008545]
    W: [[(4, 0.0999975584), (5, 0.2000012207), (6, 0.2999926754), (7, 0.400008545)], [(4, 0.0999975584), (5, 0.2000012207), (6, 0.2999926754), (7, 0.400008545)], [(4, 0.0999975584), (5, 0.2000012207), (6, 0.2999926754), (7, 0.400008545)], [(4, 0.0999975584), (5, 0.2000012207), (6, 0.2999926754), (7, 0.400008545)], [], [], [], []]
    Tv: [0.9999999995, 0.9999999995, 0.9999999995, 0.9999999995, 0, 0, 0, 0]
    R (after): [0, 0, 0, 0, 0.099997558, 0.2000012202, 0.2999926745, 0.4000085443]
    T: [0, 0, 0, 0, 1, 1, 1, 1]
    I (=R): [0, 0, 0, 0, 0.0999975582, 0.2000012207, 0.2999926752, 0.4000085455]
    B: [[], [], [], [], [], [], [], []]
    B (outdatedmask): [[], [], [], [], [], [], [], []]
    B (mask+norm): [[], [], [], [], [], [], [], []]
    ΔB: [[(4, 0.0099997558), (5, 0.020000122), (6, 0.0299992673), (7, 0.0400008543)], [(4, 0.0199995115), (5, 0.040000244), (6, 0.0599985349), (7, 0.0800017088)], [(4, 0.0299992673), (5, 0.060000366), (6, 0.0899978024), (7, 0.1200025633)], [(4, 0.0399990233), (5, 0.080000488), (6, 0.11999707), (7, 0.1600034179)], [], [], [], []]
    ΔB (norm): [[(4, 0.0999999996), (5, 0.0999999999), (6, 0.0999999994), (7, 0.0999999996)], [(4, 0.1999999995), (5, 0.2), (6, 0.1999999997), (7, 0.1999999997)], [(4, 0.299999999), (5, 0.2999999998), (6, 0.3), (7, 0.3)], [(4, 0.4000000013), (5, 0.4), (6, 0.4000000004), (7, 0.4000000001)], [], [], [], []]
    emaB: [[(4, 0.0999999982), (5, 0.0999999985), (6, 0.099999998), (7, 0.099999998)], [(4, 0.199999999), (5, 0.1999999995), (6, 0.1999999986), (7, 0.1999999986)], [(4, 0.2999999996), (5, 0.3000000003), (6, 0.3000000012), (7, 0.3000000012)], [(4, 0.4000000027), (5, 0.4000000013), (6, 0.4000000018), (7, 0.4000000018)], [], [], [], []]
    D: [0.0999999978, 0.1999999983, 0.3000000012, 0.4000000022, 0, 0, 0, 0]
    nE: [0.0499999989, 0.0999999992, 0.1500000006, 0.2000000011, 0.049998779, 0.1000006103, 0.1499963375, 0.2000042726]
    E: [49999998, 99999999, 150000000, 200000001, 49998779, 100000610, 149996337, 200004272]
    P: [0.0499999989, 0.0999999992, 0.1500000006, 0.2000000011, 0.049998779, 0.1000006103, 0.1499963375, 0.2000042726]
    emaB: [[(4, 0.2499999937), (5, 0.2499999953), (6, 0.2499999937), (7, 0.2499999937)], [(4, 0.4999999942), (5, 0.499999997), (6, 0.4999999942), (7, 0.4999999942)], [(4, 0.7499999937), (5, 0.7499999981), (6, 0.7499999995), (7, 0.7499999995)], [(4, 1), (5, 1), (6, 1), (7, 1)], [], [], [], []] */
    let bonds = get_bonds(&deps.storage, netuid);
    assert_eq!(bonds[0][4], 16383);
    assert_eq!(bonds[1][4], 32767);
    assert_eq!(bonds[2][4], 49151);
    assert_eq!(bonds[3][4], 65535);

    // === Set self-weight only on val1
    let uid = 0;
    set_weights(
        deps.as_mut(),
        env.clone(),
        Addr::unchecked((1000 + uid).to_string()).as_str(),
        netuid,
        vec![uid],
        vec![u16::MAX],
        0,
    )
    .unwrap();

    step_block(deps.as_mut(), &mut env).unwrap();
    if sparse {
        epoch(
            &mut deps.storage,
            &deps.api,
            netuid,
            1_000_000_000,
            env.block.height,
        )
        .unwrap();
    } else {
        epoch_dense(&mut deps.storage, netuid, 1_000_000_000, env.block.height);
    }
    /*  n: 8
    current_block: 2
    activity_cutoff: 5000
    Last update: [1, 1, 1, 1, 0, 0, 0, 0]
    Inactive: [false, false, false, false, false, false, false, false]
    Block at registration: [0, 0, 0, 0, 0, 0, 0, 0]
    hotkeys: [(0, 0), (1, 1), (2, 2), (3, 3), (4, 4), (5, 5), (6, 6), (7, 7)]
    S: [0.0999999999, 0.2, 0.2999999998, 0.4, 0, 0, 0, 0]
    validator_permits: [true, true, true, true, true, true, true, true]
    max_allowed_validators: 8
    new_validator_permits: [true, true, true, true, true, true, true, true]
    S: [0.0999999999, 0.2, 0.2999999998, 0.4, 0, 0, 0, 0]
    W: [[(0, 65535)], [(4, 16383), (5, 32767), (6, 49149), (7, 65535)], [(4, 16383), (5, 32767), (6, 49149), (7, 65535)], [(4, 16383), (5, 32767), (6, 49149), (7, 65535)], [], [], [], []]
    W (permit): [[(0, 65535)], [(4, 16383), (5, 32767), (6, 49149), (7, 65535)], [(4, 16383), (5, 32767), (6, 49149), (7, 65535)], [(4, 16383), (5, 32767), (6, 49149), (7, 65535)], [], [], [], []]
    W (permit+diag): [[], [(4, 16383), (5, 32767), (6, 49149), (7, 65535)], [(4, 16383), (5, 32767), (6, 49149), (7, 65535)], [(4, 16383), (5, 32767), (6, 49149), (7, 65535)], [], [], [], []]
    W (permit+diag+outdate): [[], [(4, 16383), (5, 32767), (6, 49149), (7, 65535)], [(4, 16383), (5, 32767), (6, 49149), (7, 65535)], [(4, 16383), (5, 32767), (6, 49149), (7, 65535)], [], [], [], []]
    W (mask+norm): [[], [(4, 0.0999975584), (5, 0.2000012207), (6, 0.2999926754), (7, 0.400008545)], [(4, 0.0999975584), (5, 0.2000012207), (6, 0.2999926754), (7, 0.400008545)], [(4, 0.0999975584), (5, 0.2000012207), (6, 0.2999926754), (7, 0.400008545)], [], [], [], []]
    R (before): [0, 0, 0, 0, 0.0899978022, 0.1800010982, 0.2699934072, 0.36000769]
    C: [0, 0, 0, 0, 0.0999975584, 0.2000012207, 0.2999926754, 0.400008545]
    W: [[], [(4, 0.0999975584), (5, 0.2000012207), (6, 0.2999926754), (7, 0.400008545)], [(4, 0.0999975584), (5, 0.2000012207), (6, 0.2999926754), (7, 0.400008545)], [(4, 0.0999975584), (5, 0.2000012207), (6, 0.2999926754), (7, 0.400008545)], [], [], [], []]
    Tv: [0, 0.9999999995, 0.9999999995, 0.9999999995, 0, 0, 0, 0]
    R (after): [0, 0, 0, 0, 0.0899978022, 0.1800010982, 0.2699934072, 0.36000769]
    T: [0, 0, 0, 0, 1, 1, 1, 1]
    I (=R): [0, 0, 0, 0, 0.0999975582, 0.2000012207, 0.2999926754, 0.4000085455]
    B: [[(4, 16383), (5, 16383), (6, 16383), (7, 16383)], [(4, 32767), (5, 32767), (6, 32767), (7, 32767)], [(4, 49151), (5, 49151), (6, 49151), (7, 49151)], [(4, 65535), (5, 65535), (6, 65535), (7, 65535)], [], [], [], []]
    B (outdatedmask): [[(4, 16383), (5, 16383), (6, 16383), (7, 16383)], [(4, 32767), (5, 32767), (6, 32767), (7, 32767)], [(4, 49151), (5, 49151), (6, 49151), (7, 49151)], [(4, 65535), (5, 65535), (6, 65535), (7, 65535)], [], [], [], []]
    B (mask+norm): [[(4, 0.0999963377), (5, 0.0999963377), (6, 0.0999963377), (7, 0.0999963377)], [(4, 0.1999987792), (5, 0.1999987792), (6, 0.1999987792), (7, 0.1999987792)], [(4, 0.3000012205), (5, 0.3000012205), (6, 0.3000012205), (7, 0.3000012205)], [(4, 0.400003662), (5, 0.400003662), (6, 0.400003662), (7, 0.400003662)], [], [], [], []]
    ΔB: [[], [(4, 0.0199995115), (5, 0.040000244), (6, 0.0599985349), (7, 0.0800017088)], [(4, 0.0299992673), (5, 0.060000366), (6, 0.0899978024), (7, 0.1200025633)], [(4, 0.0399990233), (5, 0.080000488), (6, 0.11999707), (7, 0.1600034179)], [], [], [], []]
    ΔB (norm): [[], [(4, 0.2222222215), (5, 0.222222222), (6, 0.2222222218), (7, 0.2222222218)], [(4, 0.3333333323), (5, 0.3333333333), (6, 0.3333333333), (7, 0.3333333333)], [(4, 0.4444444457), (5, 0.4444444443), (6, 0.4444444447), (7, 0.4444444445)], [], [], [], []]
    emaB: [[(4, 0.0899967037), (5, 0.0899967037), (6, 0.0899967037), (7, 0.0899967037)], [(4, 0.2022211235), (5, 0.2022211235), (6, 0.2022211235), (7, 0.2022211235)], [(4, 0.3033344317), (5, 0.3033344317), (6, 0.3033344317), (7, 0.3033344317)], [(4, 0.4044477409), (5, 0.4044477406), (6, 0.4044477406), (7, 0.4044477406)], [], [], [], []]
    D: [0.0899967032, 0.2022211233, 0.303334432, 0.404447741, 0, 0, 0, 0]
    nE: [0.0449983515, 0.1011105615, 0.1516672159, 0.2022238704, 0.049998779, 0.1000006103, 0.1499963377, 0.2000042726]
    E: [44998351, 101110561, 151667215, 202223870, 49998779, 100000610, 149996337, 200004272]
    P: [0.0449983515, 0.1011105615, 0.1516672159, 0.2022238704, 0.049998779, 0.1000006103, 0.1499963377, 0.2000042726]
    emaB: [[(4, 0.2225175085), (5, 0.2225175085), (6, 0.2225175085), (7, 0.2225175085)], [(4, 0.499993208), (5, 0.4999932083), (6, 0.4999932083), (7, 0.4999932083)], [(4, 0.7499966028), (5, 0.7499966032), (6, 0.7499966032), (7, 0.7499966032)], [(4, 1), (5, 1), (6, 1), (7, 1)], [], [], [], []] */
    let bonds = get_bonds(&deps.storage, netuid);
    assert_eq!(bonds[0][4], 14582);
    assert_eq!(bonds[1][4], 32767);
    assert_eq!(bonds[2][4], 49151);
    assert_eq!(bonds[3][4], 65535);

    // === Set self-weight only on val2
    let uid = 1;
    set_weights(
        deps.as_mut(),
        env.clone(),
        Addr::unchecked((1000 + uid).to_string()).as_str(),
        netuid,
        vec![uid],
        vec![u16::MAX],
        0,
    )
    .unwrap();

    step_block(deps.as_mut(), &mut env).unwrap();
    if sparse {
        epoch(
            &mut deps.storage,
            &deps.api,
            netuid,
            1_000_000_000,
            env.block.height,
        )
        .unwrap();
    } else {
        epoch_dense(&mut deps.storage, netuid, 1_000_000_000, env.block.height);
    }
    /*  current_block: 3
    W: [[(0, 65535)], [(1, 65535)], [(4, 16383), (5, 32767), (6, 49149), (7, 65535)], [(4, 16383), (5, 32767), (6, 49149), (7, 65535)], [], [], [], []]
    W (permit): [[(0, 65535)], [(1, 65535)], [(4, 16383), (5, 32767), (6, 49149), (7, 65535)], [(4, 16383), (5, 32767), (6, 49149), (7, 65535)], [], [], [], []]
    W (permit+diag): [[], [], [(4, 16383), (5, 32767), (6, 49149), (7, 65535)], [(4, 16383), (5, 32767), (6, 49149), (7, 65535)], [], [], [], []]
    W (permit+diag+outdate): [[], [], [(4, 16383), (5, 32767), (6, 49149), (7, 65535)], [(4, 16383), (5, 32767), (6, 49149), (7, 65535)], [], [], [], []]
    W (mask+norm): [[], [], [(4, 0.0999975584), (5, 0.2000012207), (6, 0.2999926754), (7, 0.400008545)], [(4, 0.0999975584), (5, 0.2000012207), (6, 0.2999926754), (7, 0.400008545)], [], [], [], []]
    R (before): [0, 0, 0, 0, 0.0699982906, 0.1400008542, 0.2099948723, 0.2800059812]
    C: [0, 0, 0, 0, 0.0999975584, 0.2000012207, 0.2999926754, 0.400008545]
    W: [[], [], [(4, 0.0999975584), (5, 0.2000012207), (6, 0.2999926754), (7, 0.400008545)], [(4, 0.0999975584), (5, 0.2000012207), (6, 0.2999926754), (7, 0.400008545)], [], [], [], []]
    Tv: [0, 0, 0.9999999995, 0.9999999995, 0, 0, 0, 0]
    R (after): [0, 0, 0, 0, 0.0699982906, 0.1400008542, 0.2099948723, 0.2800059812]
    T: [0, 0, 0, 0, 1, 1, 1, 1]
    I (=R): [0, 0, 0, 0, 0.0999975582, 0.2000012207, 0.2999926754, 0.4000085455]
    B: [[(4, 14582), (5, 14582), (6, 14582), (7, 14582)], [(4, 32767), (5, 32767), (6, 32767), (7, 32767)], [(4, 49151), (5, 49151), (6, 49151), (7, 49151)], [(4, 65535), (5, 65535), (6, 65535), (7, 65535)], [], [], [], []]
    B (outdatedmask): [[(4, 14582), (5, 14582), (6, 14582), (7, 14582)], [(4, 32767), (5, 32767), (6, 32767), (7, 32767)], [(4, 49151), (5, 49151), (6, 49151), (7, 49151)], [(4, 65535), (5, 65535), (6, 65535), (7, 65535)], [], [], [], []]
    B (mask+norm): [[(4, 0.0899929027), (5, 0.0899929027), (6, 0.0899929027), (7, 0.0899929027)], [(4, 0.2022217421), (5, 0.2022217421), (6, 0.2022217421), (7, 0.2022217421)], [(4, 0.303335699), (5, 0.303335699), (6, 0.303335699), (7, 0.303335699)], [(4, 0.404449656), (5, 0.404449656), (6, 0.404449656), (7, 0.404449656)], [], [], [], []]
    ΔB: [[], [], [(4, 0.0299992673), (5, 0.060000366), (6, 0.0899978024), (7, 0.1200025633)], [(4, 0.0399990233), (5, 0.080000488), (6, 0.11999707), (7, 0.1600034179)], [], [], [], []]
    ΔB (norm): [[], [], [(4, 0.428571427), (5, 0.4285714284), (6, 0.4285714284), (7, 0.4285714284)], [(4, 0.5714285728), (5, 0.5714285714), (6, 0.5714285714), (7, 0.5714285714)], [], [], [], []]
    emaB: [[(4, 0.0809936123), (5, 0.0809936123), (6, 0.0809936123), (7, 0.0809936123)], [(4, 0.181999568), (5, 0.181999568), (6, 0.181999568), (7, 0.181999568)], [(4, 0.3158592717), (5, 0.315859272), (6, 0.315859272), (7, 0.315859272)], [(4, 0.4211475477), (5, 0.4211475474), (6, 0.4211475474), (7, 0.4211475474)], [], [], [], []]
    D: [0.0809936118, 0.1819995677, 0.3158592721, 0.421147548, 0, 0, 0, 0]
    nE: [0.040496806, 0.0909997837, 0.157929636, 0.2105737738, 0.049998779, 0.1000006103, 0.1499963377, 0.2000042726]
    E: [40496805, 90999783, 157929636, 210573773, 49998779, 100000610, 149996337, 200004272]
    P: [0.040496806, 0.0909997837, 0.157929636, 0.2105737738, 0.049998779, 0.1000006103, 0.1499963377, 0.2000042726]
    emaB: [[(4, 0.192316476), (5, 0.192316476), (6, 0.192316476), (7, 0.192316476)], [(4, 0.4321515555), (5, 0.4321515558), (6, 0.4321515558), (7, 0.4321515558)], [(4, 0.7499967015), (5, 0.7499967027), (6, 0.7499967027), (7, 0.7499967027)], [(4, 1), (5, 1), (6, 1), (7, 1)], [], [], [], []] */
    let bonds = get_bonds(&deps.storage, netuid);
    assert_eq!(bonds[0][4], 12603);
    assert_eq!(bonds[1][4], 28321);
    assert_eq!(bonds[2][4], 49151);
    assert_eq!(bonds[3][4], 65535);

    // === Set self-weight only on val3
    let uid = 2;
    set_weights(
        deps.as_mut(),
        env.clone(),
        Addr::unchecked((1000 + uid).to_string()).as_str(),
        netuid,
        vec![uid],
        vec![u16::MAX],
        0,
    )
    .unwrap();

    step_block(deps.as_mut(), &mut env).unwrap();
    if sparse {
        epoch(
            &mut deps.storage,
            &deps.api,
            netuid,
            1_000_000_000,
            env.block.height,
        )
        .unwrap();
    } else {
        epoch_dense(&mut deps.storage, netuid, 1_000_000_000, env.block.height);
    }
    /*  current_block: 4
    W: [[(0, 65535)], [(1, 65535)], [(2, 65535)], [(4, 16383), (5, 32767), (6, 49149), (7, 65535)], [], [], [], []]
    W (permit): [[(0, 65535)], [(1, 65535)], [(2, 65535)], [(4, 16383), (5, 32767), (6, 49149), (7, 65535)], [], [], [], []]
    W (permit+diag): [[], [], [], [(4, 16383), (5, 32767), (6, 49149), (7, 65535)], [], [], [], []]
    W (permit+diag+outdate): [[], [], [], [(4, 16383), (5, 32767), (6, 49149), (7, 65535)], [], [], [], []]
    W (mask+norm): [[], [], [], [(4, 0.0999975584), (5, 0.2000012207), (6, 0.2999926754), (7, 0.400008545)], [], [], [], []]
    R (before): [0, 0, 0, 0, 0.0399990233, 0.080000488, 0.11999707, 0.1600034179]
    C: [0, 0, 0, 0, 0, 0, 0, 0]
    W: [[], [], [], [], [], [], [], []]
    Tv: [0, 0, 0, 0, 0, 0, 0, 0]
    R (after): [0, 0, 0, 0, 0, 0, 0, 0]
    T: [0, 0, 0, 0, 0, 0, 0, 0]
    I (=R): [0, 0, 0, 0, 0, 0, 0, 0]
    B: [[(4, 12603), (5, 12603), (6, 12603), (7, 12603)], [(4, 28321), (5, 28321), (6, 28321), (7, 28321)], [(4, 49151), (5, 49151), (6, 49151), (7, 49151)], [(4, 65535), (5, 65535), (6, 65535), (7, 65535)], [], [], [], []]
    B (outdatedmask): [[(4, 12603), (5, 12603), (6, 12603), (7, 12603)], [(4, 28321), (5, 28321), (6, 28321), (7, 28321)], [(4, 49151), (5, 49151), (6, 49151), (7, 49151)], [(4, 65535), (5, 65535), (6, 65535), (7, 65535)], [], [], [], []]
    B (mask+norm): [[(4, 0.0809909387), (5, 0.0809909387), (6, 0.0809909387), (7, 0.0809909387)], [(4, 0.1819998713), (5, 0.1819998713), (6, 0.1819998713), (7, 0.1819998713)], [(4, 0.3158601632), (5, 0.3158601632), (6, 0.3158601632), (7, 0.3158601632)], [(4, 0.4211490264), (5, 0.4211490264), (6, 0.4211490264), (7, 0.4211490264)], [], [], [], []]
    ΔB: [[], [], [], [], [], [], [], []]
    ΔB (norm): [[], [], [], [], [], [], [], []]
    emaB: [[(4, 0.0809909385), (5, 0.0809909385), (6, 0.0809909385), (7, 0.0809909385)], [(4, 0.1819998713), (5, 0.1819998713), (6, 0.1819998713), (7, 0.1819998713)], [(4, 0.3158601632), (5, 0.3158601632), (6, 0.3158601632), (7, 0.3158601632)], [(4, 0.4211490266), (5, 0.4211490266), (6, 0.4211490266), (7, 0.4211490266)], [], [], [], []]
    D: [0, 0, 0, 0, 0, 0, 0, 0]
    nE: [0.0999999999, 0.2, 0.2999999998, 0.4, 0, 0, 0, 0]
    E: [99999999, 199999999, 299999999, 399999999, 0, 0, 0, 0]
    P: [0.0999999999, 0.2, 0.2999999998, 0.4, 0, 0, 0, 0]
    emaB: [[(4, 0.1923094518), (5, 0.1923094518), (6, 0.1923094518), (7, 0.1923094518)], [(4, 0.4321507583), (5, 0.4321507583), (6, 0.4321507583), (7, 0.4321507583)], [(4, 0.7499961846), (5, 0.7499961846), (6, 0.7499961846), (7, 0.7499961846)], [(4, 1), (5, 1), (6, 1), (7, 1)], [], [], [], []] */
    let bonds = get_bonds(&deps.storage, netuid);
    assert_eq!(bonds[0][7], 12602);
    assert_eq!(bonds[1][7], 28320);
    assert_eq!(bonds[2][7], 49150);
    assert_eq!(bonds[3][7], 65535);

    // === Set val3->srv4: 1
    set_weights(
        deps.as_mut(),
        env.clone(),
        Addr::unchecked(1002.to_string()).as_str(),
        netuid,
        vec![7],
        vec![u16::MAX],
        0,
    )
    .unwrap();

    step_block(deps.as_mut(), &mut env).unwrap();
    if sparse {
        epoch(
            &mut deps.storage,
            &deps.api,
            netuid,
            1_000_000_000,
            env.block.height,
        )
        .unwrap();
    } else {
        epoch_dense(&mut deps.storage, netuid, 1_000_000_000, env.block.height);
    }
    /*  current_block: 5
    W: [[(0, 65535)], [(1, 65535)], [(7, 65535)], [(4, 16383), (5, 32767), (6, 49149), (7, 65535)], [], [], [], []]
    W (permit): [[(0, 65535)], [(1, 65535)], [(7, 65535)], [(4, 16383), (5, 32767), (6, 49149), (7, 65535)], [], [], [], []]
    W (permit+diag): [[], [], [(7, 65535)], [(4, 16383), (5, 32767), (6, 49149), (7, 65535)], [], [], [], []]
    W (permit+diag+outdate): [[], [], [(7, 65535)], [(4, 16383), (5, 32767), (6, 49149), (7, 65535)], [], [], [], []]
    W (mask+norm): [[], [], [(7, 1)], [(4, 0.0999975584), (5, 0.2000012207), (6, 0.2999926754), (7, 0.400008545)], [], [], [], []]
    R (before): [0, 0, 0, 0, 0.0399990233, 0.080000488, 0.11999707, 0.4600034177]
    C: [0, 0, 0, 0, 0, 0, 0, 0.400008545]
    W: [[], [], [(7, 0.400008545)], [(7, 0.400008545)], [], [], [], []]
    Tv: [0, 0, 0.400008545, 0.400008545, 0, 0, 0, 0]
    R (after): [0, 0, 0, 0, 0, 0, 0, 0.2800059812]
    T: [0, 0, 0, 0, 0, 0, 0, 0.6087041323]
    I (=R): [0, 0, 0, 0, 0, 0, 0, 1]
    B: [[(4, 12602), (5, 12602), (6, 12602), (7, 12602)], [(4, 28320), (5, 28320), (6, 28320), (7, 28320)], [(4, 49150), (5, 49150), (6, 49150), (7, 49150)], [(4, 65535), (5, 65535), (6, 65535), (7, 65535)], [], [], [], []]
    B (outdatedmask): [[(4, 12602), (5, 12602), (6, 12602), (7, 12602)], [(4, 28320), (5, 28320), (6, 28320), (7, 28320)], [(4, 49150), (5, 49150), (6, 49150), (7, 49150)], [(4, 65535), (5, 65535), (6, 65535), (7, 65535)], [], [], [], []]
    B (mask+norm): [[(4, 0.0809860737), (5, 0.0809860737), (6, 0.0809860737), (7, 0.0809860737)], [(4, 0.1819969537), (5, 0.1819969537), (6, 0.1819969537), (7, 0.1819969537)], [(4, 0.3158598263), (5, 0.3158598263), (6, 0.3158598263), (7, 0.3158598263)], [(4, 0.4211571459), (5, 0.4211571459), (6, 0.4211571459), (7, 0.4211571459)], [], [], [], []]
    ΔB: [[], [], [(7, 0.1200025633)], [(7, 0.1600034179)], [], [], [], []]
    ΔB (norm): [[], [], [(7, 0.4285714284)], [(7, 0.5714285714)], [], [], [], []]
    emaB: [[(4, 0.0809860737), (5, 0.0809860737), (6, 0.0809860737), (7, 0.0728874663)], [(4, 0.1819969537), (5, 0.1819969537), (6, 0.1819969537), (7, 0.1637972582)], [(4, 0.3158598263), (5, 0.3158598263), (6, 0.3158598263), (7, 0.3271309866)], [(4, 0.421157146), (5, 0.421157146), (6, 0.421157146), (7, 0.4361842885)], [], [], [], []]
    D: [0.0728874663, 0.1637972582, 0.3271309866, 0.4361842885, 0, 0, 0, 0]
    nE: [0.0364437331, 0.081898629, 0.1635654932, 0.2180921442, 0, 0, 0, 0.5]
    E: [36443733, 81898628, 163565493, 218092144, 0, 0, 0, 500000000]
    P: [0.0364437331, 0.081898629, 0.1635654932, 0.2180921442, 0, 0, 0, 0.5]
    emaB: [[(4, 0.1922941932), (5, 0.1922941932), (6, 0.1922941932), (7, 0.1671024568)], [(4, 0.4321354993), (5, 0.4321354993), (6, 0.4321354993), (7, 0.3755230587)], [(4, 0.7499809256), (5, 0.7499809256), (6, 0.7499809256), (7, 0.749983425)], [(4, 1), (5, 1), (6, 1), (7, 1)], [], [], [], []] */
    let bonds = get_bonds(&deps.storage, netuid);
    assert_eq!(bonds[0][7], 10951);
    assert_eq!(bonds[1][7], 24609);
    assert_eq!(bonds[2][7], 49150);
    assert_eq!(bonds[3][7], 65535);

    step_block(deps.as_mut(), &mut env).unwrap();
    if sparse {
        epoch(
            &mut deps.storage,
            &deps.api,
            netuid,
            1_000_000_000,
            env.block.height,
        )
        .unwrap();
    } else {
        epoch_dense(&mut deps.storage, netuid, 1_000_000_000, env.block.height);
    }
    /*  current_block: 6
    B: [[(4, 12601), (5, 12601), (6, 12601), (7, 10951)], [(4, 28319), (5, 28319), (6, 28319), (7, 24609)], [(4, 49149), (5, 49149), (6, 49149), (7, 49150)], [(4, 65535), (5, 65535), (6, 65535), (7, 65535)], [], [], [], []]
    B (outdatedmask): [[(4, 12601), (5, 12601), (6, 12601), (7, 10951)], [(4, 28319), (5, 28319), (6, 28319), (7, 24609)], [(4, 49149), (5, 49149), (6, 49149), (7, 49150)], [(4, 65535), (5, 65535), (6, 65535), (7, 65535)], [], [], [], []]
    B (mask+norm): [[(4, 0.0809812085), (5, 0.0809812085), (6, 0.0809812085), (7, 0.0728876167)], [(4, 0.181994036), (5, 0.181994036), (6, 0.181994036), (7, 0.163792472)], [(4, 0.3158594894), (5, 0.3158594894), (6, 0.3158594894), (7, 0.3271323503)], [(4, 0.4211652656), (5, 0.4211652656), (6, 0.4211652656), (7, 0.4361875602)], [], [], [], []]
    ΔB: [[], [], [(7, 0.1200025633)], [(7, 0.1600034179)], [], [], [], []]
    ΔB (norm): [[], [], [(7, 0.4285714284)], [(7, 0.5714285714)], [], [], [], []]
    emaB: [[(4, 0.0809812082), (5, 0.0809812082), (6, 0.0809812082), (7, 0.0655988548)], [(4, 0.181994036), (5, 0.181994036), (6, 0.181994036), (7, 0.1474132247)], [(4, 0.3158594896), (5, 0.3158594896), (6, 0.3158594896), (7, 0.3372762585)], [(4, 0.4211652658), (5, 0.4211652658), (6, 0.4211652658), (7, 0.4497116616)], [], [], [], []]
    D: [0.0655988548, 0.1474132247, 0.3372762585, 0.4497116616, 0, 0, 0, 0]
    nE: [0.0327994274, 0.0737066122, 0.1686381293, 0.2248558307, 0, 0, 0, 0.5]
    E: [32799427, 73706612, 168638129, 224855830, 0, 0, 0, 500000000]
    P: [0.0327994274, 0.0737066122, 0.1686381293, 0.2248558307, 0, 0, 0, 0.5]
    emaB: [[(4, 0.1922789337), (5, 0.1922789337), (6, 0.1922789337), (7, 0.1458686984)], [(4, 0.4321202405), (5, 0.4321202405), (6, 0.4321202405), (7, 0.3277949789)], [(4, 0.749965667), (5, 0.749965667), (6, 0.749965667), (7, 0.74998335)], [(4, 1), (5, 1), (6, 1), (7, 1)], [], [], [], []] */
    let bonds = get_bonds(&deps.storage, netuid);
    assert_eq!(bonds[0][7], 9559);
    assert_eq!(bonds[1][7], 21482);
    assert_eq!(bonds[2][7], 49150);
    assert_eq!(bonds[3][7], 65535);

    step_block(deps.as_mut(), &mut env).unwrap();
    if sparse {
        epoch(
            &mut deps.storage,
            &deps.api,
            netuid,
            1_000_000_000,
            env.block.height,
        )
        .unwrap();
    } else {
        epoch_dense(&mut deps.storage, netuid, 1_000_000_000, env.block.height);
    }
    /*  current_block: 7
    B: [[(4, 12600), (5, 12600), (6, 12600), (7, 9559)], [(4, 28318), (5, 28318), (6, 28318), (7, 21482)], [(4, 49148), (5, 49148), (6, 49148), (7, 49150)], [(4, 65535), (5, 65535), (6, 65535), (7, 65535)], [], [], [], []]
    B (outdatedmask): [[(4, 12600), (5, 12600), (6, 12600), (7, 9559)], [(4, 28318), (5, 28318), (6, 28318), (7, 21482)], [(4, 49148), (5, 49148), (6, 49148), (7, 49150)], [(4, 65535), (5, 65535), (6, 65535), (7, 65535)], [], [], [], []]
    B (mask+norm): [[(4, 0.0809763432), (5, 0.0809763432), (6, 0.0809763432), (7, 0.065595707)], [(4, 0.1819911182), (5, 0.1819911182), (6, 0.1819911182), (7, 0.1474136391)], [(4, 0.3158591525), (5, 0.3158591525), (6, 0.3158591525), (7, 0.337276807)], [(4, 0.4211733856), (5, 0.4211733856), (6, 0.4211733856), (7, 0.4497138464)], [], [], [], []]
    ΔB: [[], [], [(7, 0.1200025633)], [(7, 0.1600034179)], [], [], [], []]
    ΔB (norm): [[], [], [(7, 0.4285714284)], [(7, 0.5714285714)], [], [], [], []]
    emaB: [[(4, 0.080976343), (5, 0.080976343), (6, 0.080976343), (7, 0.0590361361)], [(4, 0.181991118), (5, 0.181991118), (6, 0.181991118), (7, 0.1326722752)], [(4, 0.3158591525), (5, 0.3158591525), (6, 0.3158591525), (7, 0.3464062694)], [(4, 0.4211733858), (5, 0.4211733858), (6, 0.4211733858), (7, 0.4618853189)], [], [], [], []]
    D: [0.0590361361, 0.1326722752, 0.3464062694, 0.4618853189, 0, 0, 0, 0]
    nE: [0.029518068, 0.0663361375, 0.1732031347, 0.2309426593, 0, 0, 0, 0.5]
    E: [29518068, 66336137, 173203134, 230942659, 0, 0, 0, 500000000]
    P: [0.029518068, 0.0663361375, 0.1732031347, 0.2309426593, 0, 0, 0, 0.5]
    emaB: [[(4, 0.192263675), (5, 0.192263675), (6, 0.192263675), (7, 0.1278155716)], [(4, 0.4321049813), (5, 0.4321049813), (6, 0.4321049813), (7, 0.2872407278)], [(4, 0.7499504078), (5, 0.7499504078), (6, 0.7499504078), (7, 0.7499832863)], [(4, 1), (5, 1), (6, 1), (7, 1)], [], [], [], []] */
    let bonds = get_bonds(&deps.storage, netuid);
    assert_eq!(bonds[0][7], 8376);
    assert_eq!(bonds[1][7], 18824);
    assert_eq!(bonds[2][7], 49150);
    assert_eq!(bonds[3][7], 65535);

    // run_to_block(8);
    step_block(deps.as_mut(), &mut env).unwrap();
    if sparse {
        epoch(
            &mut deps.storage,
            &deps.api,
            netuid,
            1_000_000_000,
            env.block.height,
        )
        .unwrap();
    } else {
        epoch_dense(&mut deps.storage, netuid, 1_000_000_000, env.block.height);
    }
    /*  current_block: 8
    B: [[(4, 12599), (5, 12599), (6, 12599), (7, 8376)], [(4, 28317), (5, 28317), (6, 28317), (7, 18824)], [(4, 49147), (5, 49147), (6, 49147), (7, 49150)], [(4, 65535), (5, 65535), (6, 65535), (7, 65535)], [], [], [], []]
    B (outdatedmask): [[(4, 12599), (5, 12599), (6, 12599), (7, 8376)], [(4, 28317), (5, 28317), (6, 28317), (7, 18824)], [(4, 49147), (5, 49147), (6, 49147), (7, 49150)], [(4, 65535), (5, 65535), (6, 65535), (7, 65535)], [], [], [], []]
    B (mask+norm): [[(4, 0.0809714776), (5, 0.0809714776), (6, 0.0809714776), (7, 0.0590337245)], [(4, 0.1819882002), (5, 0.1819882002), (6, 0.1819882002), (7, 0.1326708249)], [(4, 0.3158588156), (5, 0.3158588156), (6, 0.3158588156), (7, 0.3464073015)], [(4, 0.421181506), (5, 0.421181506), (6, 0.421181506), (7, 0.4618881487)], [], [], [], []]
    ΔB: [[], [], [(7, 0.1200025633)], [(7, 0.1600034179)], [], [], [], []]
    ΔB (norm): [[], [], [(7, 0.4285714284)], [(7, 0.5714285714)], [], [], [], []]
    emaB: [[(4, 0.0809714776), (5, 0.0809714776), (6, 0.0809714776), (7, 0.053130352)], [(4, 0.1819882002), (5, 0.1819882002), (6, 0.1819882002), (7, 0.1194037423)], [(4, 0.3158588156), (5, 0.3158588156), (6, 0.3158588156), (7, 0.3546237142)], [(4, 0.4211815062), (5, 0.4211815062), (6, 0.4211815062), (7, 0.472842191)], [], [], [], []]
    D: [0.053130352, 0.1194037423, 0.3546237142, 0.472842191, 0, 0, 0, 0]
    nE: [0.026565176, 0.0597018711, 0.177311857, 0.2364210954, 0, 0, 0, 0.5]
    E: [26565175, 59701871, 177311856, 236421095, 0, 0, 0, 500000000]
    P: [0.026565176, 0.0597018711, 0.177311857, 0.2364210954, 0, 0, 0, 0.5]
    emaB: [[(4, 0.1922484161), (5, 0.1922484161), (6, 0.1922484161), (7, 0.1123638137)], [(4, 0.4320897225), (5, 0.4320897225), (6, 0.4320897225), (7, 0.2525234516)], [(4, 0.7499351487), (5, 0.7499351487), (6, 0.7499351487), (7, 0.7499832308)], [(4, 1), (5, 1), (6, 1), (7, 1)], [], [], [], []] */
}

// // Test that epoch masks out inactive stake of validators with outdated weights beyond activity cutoff.
#[test]
fn test_active_stake() {
    let (mut deps, mut env) = instantiate_contract();

    let sparse: bool = true;
    let n: u16 = 4;
    let netuid: u16 = 2;
    let tempo: u16 = u16::MAX - 1; // high tempo to skip automatic epochs in on_initialize, use manual epochs instead
    let block_number: u64 = 0;
    let stake: u64 = 1;
    add_network(&mut deps.storage, netuid, tempo, 0);
    set_max_allowed_uids(&mut deps.storage, netuid, n);
    assert_eq!(get_max_allowed_uids(&deps.storage, netuid), n);
    set_max_registrations_per_block(&mut deps.storage, netuid, n);
    set_target_registrations_per_interval(&mut deps.storage, netuid, n);
    set_min_allowed_weights(&mut deps.storage, netuid, 0);
    set_max_weight_limit(&mut deps.storage, netuid, u16::MAX);
    set_weights_set_rate_limit(&mut deps.storage, netuid, 0);
    set_activity_cutoff(&mut deps.storage, netuid, 50);
    set_min_difficulty(&mut deps.storage, netuid, 1_000);
    set_difficulty(&mut deps.storage, netuid, 1_000);

    // === Register [validator1, validator2, server1, server2]
    for key in 0..n as u64 {
        add_balance_to_coldkey_account(&Addr::unchecked((1000 + key).to_string()), stake);
        let (nonce, work): (u64, Vec<u8>) = create_work_for_block_number(
            &deps.storage,
            netuid,
            block_number,
            key * 1_000_000,
            Addr::unchecked((1000 + key).to_string()).as_str(),
        );
        pow_register_ok_neuron(
            deps.as_mut(),
            env.clone(),
            netuid,
            block_number,
            nonce,
            work,
            Addr::unchecked((1000 + key).to_string()).as_str(),
            Addr::unchecked((1000 + key).to_string()).as_str(),
        )
        .unwrap();
        increase_stake_on_coldkey_hotkey_account(
            &mut deps.storage,
            &Addr::unchecked((1000 + key).to_string()),
            &Addr::unchecked((1000 + key).to_string()),
            stake,
        );
    }
    assert_eq!(get_max_allowed_uids(&deps.storage, netuid), n);
    assert_eq!(get_subnetwork_n(&deps.storage, netuid), n);

    // === Issue validator permits
    set_max_allowed_validators(&mut deps.storage, netuid, n);
    assert_eq!(get_max_allowed_validators(&deps.storage, netuid), n);
    epoch(
        &mut deps.storage,
        &deps.api,
        netuid,
        1_000_000_000,
        env.block.height,
    )
    .unwrap(); // run first epoch to set allowed validators
    step_block(deps.as_mut(), &mut env).unwrap(); // run to next block to ensure weights are set on nodes after their registration block

    // === Set weights [val1->srv1: 0.5, val1->srv2: 0.5, val2->srv1: 0.5, val2->srv2: 0.5]
    for uid in 0..(n / 2) as u64 {
        set_weights(
            deps.as_mut(),
            env.clone(),
            Addr::unchecked((1000 + uid).to_string()).as_str(),
            netuid,
            ((n / 2)..n).collect(),
            vec![u16::MAX / (n / 2); (n / 2) as usize],
            0,
        )
        .unwrap();
    }

    if sparse {
        epoch(
            &mut deps.storage,
            &deps.api,
            netuid,
            1_000_000_000,
            env.block.height,
        )
        .unwrap();
    } else {
        epoch_dense(&mut deps.storage, netuid, 1_000_000_000, env.block.height);
    }

    let bonds = get_bonds(&deps.storage, netuid);
    for uid in 0..n as u16 {
        // log::info!("\n{uid}" );
        // uid_stats(netuid, uid);
        // log::info!("bonds: {:?}", bonds[uid as usize]);
        if uid < n / 2 {
            assert_eq!(get_dividends_for_uid(&deps.storage, netuid, uid), 32767);
            // Note D = floor(0.5 * 65_535)
        }
        assert_eq!(get_emission_for_uid(&deps.storage, netuid, uid), 250000000);
        // Note E = 0.5 / (n/2) * 1_000_000_000 = 250_000_000
    }
    for validator in 0..(n / 2) as usize {
        for on_validator in 0..(n / 2) as usize {
            assert_eq!(bonds[validator][on_validator], 0);
        }
        for server in ((n / 2) as usize)..n as usize {
            assert_eq!(bonds[validator][server], I32F32::from_num(65_535)); // floor(0.5*(2^16-1))/(2^16-1), then max-upscale to 65_535
        }
    }
    let activity_cutoff: u64 = get_activity_cutoff(&deps.storage, netuid) as u64;
    // run_to_block(activity_cutoff + 2);
    run_step_to_block(deps.as_mut(), &mut env, activity_cutoff + 3).unwrap(); // run to block where validator (uid 0, 1) weights become outdated

    // === Update uid 0 weights
    set_weights(
        deps.as_mut(),
        env.clone(),
        Addr::unchecked(1000.to_string()).as_str(),
        netuid,
        ((n / 2)..n).collect(),
        vec![u16::MAX / (n / 2); (n / 2) as usize],
        0,
    )
    .unwrap();
    if sparse {
        epoch(
            &mut deps.storage,
            &deps.api,
            netuid,
            1_000_000_000,
            env.block.height,
        )
        .unwrap();
    } else {
        epoch_dense(&mut deps.storage, netuid, 1_000_000_000, env.block.height);
    }
    /*  current_block: 5002; activity_cutoff: 5000
    Last update: [5002, 1, 0, 0]; Inactive: [false, true, true, true]; Block at registration: [0, 0, 0, 0]
    S: [0.25, 0.25, 0.25, 0.25]; S (mask): [0.25, 0, 0, 0]; S (mask+norm): [1, 0, 0, 0]
    validator_permits: [true, true, true, true]; max_allowed_validators: 4; new_validator_permits: [true, true, true, true]
    W: [[(2, 0.4999923704), (3, 0.4999923704)], [(2, 0.4999923704), (3, 0.4999923704)], [], []]
    W (permit): [[(2, 0.4999923704), (3, 0.4999923704)], [(2, 0.4999923704), (3, 0.4999923704)], [], []]
    W (permit+diag): [[(2, 0.4999923704), (3, 0.4999923704)], [(2, 0.4999923704), (3, 0.4999923704)], [], []]
    W (permit+diag+outdate): [[(2, 0.4999923704), (3, 0.4999923704)], [(2, 0.4999923704), (3, 0.4999923704)], [], []]
    W (mask+norm): [[(2, 0.5), (3, 0.5)], [(2, 0.5), (3, 0.5)], [], []]
    R: [0, 0, 0.5, 0.5]
    W (threshold): [[(2, 1), (3, 1)], [(2, 1), (3, 1)], [], []]
    T: [0, 0, 1, 1]
    C: [0.006693358, 0.006693358, 0.9933076561, 0.9933076561]
    I: [0, 0, 0.5, 0.5]
    B: [[(2, 0.4999923704), (3, 0.4999923704)], [(2, 0.4999923704), (3, 0.4999923704)], [], []]
    B (outdatedmask): [[(2, 0.4999923704), (3, 0.4999923704)], [(2, 0.4999923704), (3, 0.4999923704)], [], []]
    B (mask+norm): [[(2, 0.5), (3, 0.5)], [(2, 0.5), (3, 0.5)], [], []]
    ΔB: [[(2, 0.5), (3, 0.5)], [(2, 0), (3, 0)], [], []]
    ΔB (norm): [[(2, 1), (3, 1)], [(2, 0), (3, 0)], [], []]
    emaB: [[(2, 0.55), (3, 0.55)], [(2, 0.45), (3, 0.45)], [], []]
    emaB (max-upscale): [[(2, 1), (3, 1)], [(2, 1), (3, 1)], [], []]
    D: [0.55, 0.4499999997, 0, 0]
    nE: [0.275, 0.2249999999, 0.25, 0.25]
    E: [274999999, 224999999, 250000000, 250000000]
    P: [0.275, 0.2249999999, 0.25, 0.25]
    P (u16): [65535, 53619, 59577, 59577] */
    let bonds = get_bonds(&deps.storage, netuid);
    assert_eq!(get_dividends_for_uid(&deps.storage, netuid, 0), 36044); // Note D = floor((0.5 * 0.9 + 0.1) * 65_535)
    assert_eq!(get_emission_for_uid(&deps.storage, netuid, 0), 274999999); // Note E = 0.5 * 0.55 * 1_000_000_000 = 275_000_000 (discrepancy)
    for server in ((n / 2) as usize)..n as usize {
        assert_eq!(bonds[0][server], I32F32::from_num(65_535)); // floor(0.55*(2^16-1))/(2^16-1), then max-upscale
    }
    for validator in 1..(n / 2) as u16 {
        assert_eq!(
            get_dividends_for_uid(&deps.storage, netuid, validator),
            29490
        ); // Note D = floor((0.5 * 0.9) * 65_535)
        assert_eq!(
            get_emission_for_uid(&deps.storage, netuid, validator),
            224999999
        ); // Note E = 0.5 * 0.45 * 1_000_000_000 = 225_000_000 (discrepancy)
        for server in ((n / 2) as usize)..n as usize {
            assert_eq!(bonds[validator as usize][server], I32F32::from_num(53619));
            // floor(0.45*(2^16-1))/(2^16-1), then max-upscale
        }
    }

    // === Update uid 1 weights as well
    set_weights(
        deps.as_mut(),
        env.clone(),
        Addr::unchecked(1001.to_string()).as_str(),
        netuid,
        ((n / 2)..n).collect(),
        vec![u16::MAX / (n / 2); (n / 2) as usize],
        0,
    )
    .unwrap();

    // run_to_block(activity_cutoff + 3); // run to block where validator (uid 0, 1) weights become outdated
    // run_step_to_block(deps.as_mut(), &mut env, activity_cutoff + 4).unwrap();
    step_block(deps.as_mut(), &mut env).unwrap();

    if sparse {
        epoch(
            &mut deps.storage,
            &deps.api,
            netuid,
            1_000_000_000,
            env.block.height,
        )
        .unwrap();
    } else {
        epoch_dense(&mut deps.storage, netuid, 1_000_000_000, env.block.height);
    }
    /*  current_block: 5003; activity_cutoff: 5000
    Last update: [5002, 5002, 0, 0]; Inactive: [false, false, true, true]; Block at registration: [0, 0, 0, 0]
    S: [0.25, 0.25, 0.25, 0.25]; S (mask): [0.25, 0.25, 0, 0]; S (mask+norm): [0.5, 0.5, 0, 0]
    validator_permits: [true, true, true, true]; max_allowed_validators: 4; new_validator_permits: [true, true, true, true]
    W: [[(2, 0.4999923704), (3, 0.4999923704)], [(2, 0.4999923704), (3, 0.4999923704)], [], []]
    W (permit): [[(2, 0.4999923704), (3, 0.4999923704)], [(2, 0.4999923704), (3, 0.4999923704)], [], []]
    W (permit+diag): [[(2, 0.4999923704), (3, 0.4999923704)], [(2, 0.4999923704), (3, 0.4999923704)], [], []]
    W (permit+diag+outdate): [[(2, 0.4999923704), (3, 0.4999923704)], [(2, 0.4999923704), (3, 0.4999923704)], [], []]
    W (mask+norm): [[(2, 0.5), (3, 0.5)], [(2, 0.5), (3, 0.5)], [], []]
    R: [0, 0, 0.5, 0.5]
    W (threshold): [[(2, 1), (3, 1)], [(2, 1), (3, 1)], [], []]
    T: [0, 0, 1, 1]
    C: [0.006693358, 0.006693358, 0.9933076561, 0.9933076561]
    I: [0, 0, 0.5, 0.5]
    B: [[(2, 65535), (3, 65535)], [(2, 53619), (3, 53619)], [], []]
    B (outdatedmask): [[(2, 65535), (3, 65535)], [(2, 53619), (3, 53619)], [], []]
    B (mask+norm): [[(2, 0.5500025176), (3, 0.5500025176)], [(2, 0.4499974821), (3, 0.4499974821)], [], []]
    ΔB: [[(2, 0.25), (3, 0.25)], [(2, 0.25), (3, 0.25)], [], []]
    ΔB (norm): [[(2, 0.5), (3, 0.5)], [(2, 0.5), (3, 0.5)], [], []]
    emaB: [[(2, 0.545002266), (3, 0.545002266)], [(2, 0.4549977337), (3, 0.4549977337)], [], []]
    emaB (max-upscale): [[(2, 1), (3, 1)], [(2, 0.8348547556), (3, 0.8348547556)], [], []]
    D: [0.545002266, 0.4549977337, 0, 0]
    nE: [0.272501133, 0.2274988669, 0.25, 0.25]
    E: [272501132, 227498866, 250000000, 250000000]
    P: [0.272501133, 0.2274988669, 0.25, 0.25]
    P (u16): [65535, 54711, 60123, 60123] */
    let bonds = get_bonds(&deps.storage, netuid);
    assert_eq!(get_dividends_for_uid(&deps.storage, netuid, 0), 35716); // Note D = floor((0.55 * 0.9 + 0.5 * 0.1) * 65_535)
    assert_eq!(get_emission_for_uid(&deps.storage, netuid, 0), 272501132); // Note E = 0.5 * (0.55 * 0.9 + 0.5 * 0.1) * 1_000_000_000 = 272_500_000 (discrepancy)
    for server in ((n / 2) as usize)..n as usize {
        assert_eq!(bonds[0][server], I32F32::from_num(65_535)); // floor((0.55 * 0.9 + 0.5 * 0.1)*(2^16-1))/(2^16-1), then max-upscale
    }
    assert_eq!(get_dividends_for_uid(&deps.storage, netuid, 1), 29818); // Note D = floor((0.45 * 0.9 + 0.5 * 0.1) * 65_535)
    assert_eq!(get_emission_for_uid(&deps.storage, netuid, 1), 227498866); // Note E = 0.5 * (0.45 * 0.9 + 0.5 * 0.1) * 1_000_000_000 = 227_500_000 (discrepancy)
    for server in ((n / 2) as usize)..n as usize {
        assert_eq!(bonds[1][server], I32F32::from_num(54712)); // floor((0.45 * 0.9 + 0.5 * 0.1)/(0.55 * 0.9 + 0.5 * 0.1)*(2^16-1))
    }
}

// // Test that epoch masks out outdated weights and bonds of validators on deregistered servers.
#[test]
fn test_outdated_weights() {
    let (mut deps, mut env) = instantiate_contract();

    let sparse: bool = true;
    let n: u16 = 4;
    let netuid: u16 = 2;
    let tempo: u16 = u16::MAX - 1; // high tempo to skip automatic epochs in on_initialize, use manual epochs instead
                                   // let mut block_number: u64 = 0;
    let stake: u64 = 1;
    add_network(&mut deps.storage, netuid, tempo, 0);
    set_max_allowed_uids(&mut deps.storage, netuid, n);
    set_weights_set_rate_limit(&mut deps.storage, netuid, 0);
    set_max_registrations_per_block(&mut deps.storage, netuid, n);
    set_target_registrations_per_interval(&mut deps.storage, netuid, n);
    set_min_allowed_weights(&mut deps.storage, netuid, 0);
    set_max_weight_limit(&mut deps.storage, netuid, u16::MAX);
    set_min_difficulty(&mut deps.storage, netuid, 1_000);
    set_difficulty(&mut deps.storage, netuid, 1_000);

    // === Register [validator1, validator2, server1, server2]
    for key in 0..n as u64 {
        add_balance_to_coldkey_account(&Addr::unchecked((1000 + key).to_string()), stake);
        let (nonce, work): (u64, Vec<u8>) = create_work_for_block_number(
            &deps.storage,
            netuid,
            env.block.height,
            key * 1_000_000,
            Addr::unchecked((1000 + key).to_string()).as_str(),
        );
        pow_register_ok_neuron(
            deps.as_mut(),
            env.clone(),
            netuid,
            env.block.height,
            nonce,
            work,
            Addr::unchecked((1000 + key).to_string()).as_str(),
            Addr::unchecked((1000 + key).to_string()).as_str(),
        )
        .unwrap();
        increase_stake_on_coldkey_hotkey_account(
            &mut deps.storage,
            &Addr::unchecked((1000 + key).to_string()),
            &Addr::unchecked((1000 + key).to_string()),
            stake,
        );
    }
    assert_eq!(get_subnetwork_n(&deps.storage, netuid), n);

    // === Issue validator permits
    set_max_allowed_validators(&mut deps.storage, netuid, n);
    assert_eq!(get_max_allowed_validators(&deps.storage, netuid), n);
    epoch(
        &mut deps.storage,
        &deps.api,
        netuid,
        1_000_000_000,
        env.block.height,
    )
    .unwrap(); // run first epoch to set allowed validators

    step_block(deps.as_mut(), &mut env).unwrap(); // run to next block to ensure weights are set on nodes after their registration block

    // === Set weights [val1->srv1: 2/3, val1->srv2: 1/3, val2->srv1: 2/3, val2->srv2: 1/3, srv1->srv1: 1, srv2->srv2: 1]
    for uid in 0..(n / 2) as u64 {
        set_weights(
            deps.as_mut(),
            env.clone(),
            Addr::unchecked((1000 + uid).to_string()).as_str(),
            netuid,
            ((n / 2)..n).collect(),
            vec![2 * (u16::MAX / 3), u16::MAX / 3],
            0,
        )
        .unwrap();
    }
    for uid in ((n / 2) as u64)..n as u64 {
        set_weights(
            deps.as_mut(),
            env.clone(),
            Addr::unchecked((1000 + uid).to_string()).as_str(),
            netuid,
            vec![uid as u16],
            vec![u16::MAX],
            0,
        )
        .unwrap();
    }
    if sparse {
        epoch(
            &mut deps.storage,
            &deps.api,
            netuid,
            1_000_000_000,
            env.block.height,
        )
        .unwrap();
    } else {
        epoch_dense(&mut deps.storage, netuid, 1_000_000_000, env.block.height);
    }
    /*  current_block: 1; activity_cutoff: 5000
    Last update: [1, 1, 1, 1]; Inactive: [false, false, false, false]; Block at registration: [0, 0, 0, 0]
    S: [0.25, 0.25, 0.25, 0.25]; S (mask): [0.25, 0.25, 0.25, 0.25]; S (mask+norm): [0.25, 0.25, 0.25, 0.25]
    validator_permits: [true, true, true, true]; max_allowed_validators: 4; new_validator_permits: [true, true, true, true]
    W: [[(2, 65535), (3, 32768)], [(2, 65535), (3, 32768)], [(2, 65535)], [(3, 65535)]]
    W (permit): [[(2, 65535), (3, 32768)], [(2, 65535), (3, 32768)], [(2, 65535)], [(3, 65535)]]
    W (permit+diag): [[(2, 65535), (3, 32768)], [(2, 65535), (3, 32768)], [], []]
    W (permit+diag+outdate): [[(2, 65535), (3, 32768)], [(2, 65535), (3, 32768)], [], []]
    W (mask+norm): [[(2, 0.6666632756), (3, 0.3333367242)], [(2, 0.6666632756), (3, 0.3333367242)], [], []]
    R (before): [0, 0, 0.3333316376, 0.166668362]
    C: [0, 0, 0.6666632756, 0.3333367242]
    W: [[(2, 0.6666632756), (3, 0.3333367242)], [(2, 0.6666632756), (3, 0.3333367242)], [], []]
    Tv: [0.9999999998, 0.9999999998, 0, 0]
    R (after): [0, 0, 0.3333316376, 0.166668362]
    T: [0, 0, 1, 1]
    I (=R): [0, 0, 0.6666632756, 0.3333367242]
    B: [[], [], [], []]
    B (outdatedmask): [[], [], [], []]
    B (mask+norm): [[], [], [], []]
    ΔB: [[(2, 0.1666658188), (3, 0.083334181)], [(2, 0.1666658188), (3, 0.083334181)], [], []]
    ΔB (norm): [[(2, 0.5), (3, 0.5)], [(2, 0.5), (3, 0.5)], [], []]
    emaB: [[(2, 0.5), (3, 0.5)], [(2, 0.5), (3, 0.5)], [], []]
    D: [0.5, 0.5, 0, 0]
    nE: [0.25, 0.25, 0.3333316378, 0.166668362]
    E: [250000000, 250000000, 333331637, 166668361]
    P: [0.25, 0.25, 0.3333316378, 0.166668362]
    P (u16): [49151, 49151, 65535, 32767] */

    // === Dereg server2 at uid3 (least emission) + register new key over uid3
    let new_key: u64 = n as u64; // register a new key while at max capacity, which means the least incentive uid will be deregistered
    let (nonce, work): (u64, Vec<u8>) = create_work_for_block_number(
        &deps.storage,
        netuid,
        env.block.height,
        0,
        Addr::unchecked((1000 + new_key).to_string()).as_str(),
    );
    pow_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        env.block.height,
        nonce,
        work,
        Addr::unchecked((1000 + new_key).to_string()).as_str(),
        Addr::unchecked((1000 + new_key).to_string()).as_str(),
    )
    .unwrap();
    let deregistered_uid: u16 = n - 1; // since uid=n-1 only recieved 1/3 of weight, it will get pruned first
    assert_eq!(
        Addr::unchecked((1000 + new_key).to_string()),
        get_hotkey_for_net_and_uid(&deps.storage, netuid, deregistered_uid)
            .expect("Not registered")
    );
    step_block(deps.as_mut(), &mut env).unwrap(); // run to next block to outdate weights and bonds set on deregistered uid

    // === Update weights from only uid=0
    set_weights(
        deps.as_mut(),
        env.clone(),
        Addr::unchecked(1000.to_string()).as_str(),
        netuid,
        ((n / 2)..n).collect(),
        vec![2 * (u16::MAX / 3), u16::MAX / 3],
        0,
    )
    .unwrap();
    if sparse {
        epoch(
            &mut deps.storage,
            &deps.api,
            netuid,
            1_000_000_000,
            env.block.height,
        )
        .unwrap();
    } else {
        epoch_dense(&mut deps.storage, netuid, 1_000_000_000, env.block.height);
    }
    /*  current_block: 2; activity_cutoff: 5000
    Last update: [2, 1, 1, 1]; Inactive: [false, false, false, false]; Block at registration: [0, 0, 0, 1]
    S: [0.3333333333, 0.3333333333, 0.3333333333, 0]
    S (mask): [0.3333333333, 0.3333333333, 0.3333333333, 0]
    S (mask+norm): [0.3333333333, 0.3333333333, 0.3333333333, 0]
    validator_permits: [true, true, true, false]; max_allowed_validators: 4; new_validator_permits: [true, true, true, true]
    W: [[(2, 65535), (3, 32768)], [(2, 65535), (3, 32768)], [(2, 65535)], [(3, 65535)]]
    W (permit): [[(2, 65535), (3, 32768)], [(2, 65535), (3, 32768)], [(2, 65535)], [(3, 65535)]]
    W (permit+diag): [[(2, 65535), (3, 32768)], [(2, 65535), (3, 32768)], [], []]
    W (permit+diag+outdate): [[(2, 65535), (3, 32768)], [(2, 65535)], [], []]
    W (mask+norm): [[(2, 0.6666632756), (3, 0.3333367242)], [(2, 1)], [], []]
    R (before): [0, 0, 0.5555544249, 0.1111122412]
    C: [0, 0, 0.6666632756, 0]
    W: [[(2, 0.6666632756)], [(2, 0.6666632756)], [], []]
    Tv: [0.6666632756, 0.6666632756, 0, 0]
    R (after): [0, 0, 0.4444421832, 0]
    T: [0, 0, 0.799997558, 0]
    I (=R): [0, 0, 1, 0]
    B: [[(2, 65535), (3, 65535)], [(2, 65535), (3, 65535)], [], []]
    B (outdatedmask): [[(2, 65535), (3, 65535)], [(2, 65535)], [], []]
    B (mask+norm): [[(2, 0.5), (3, 1)], [(2, 0.5)], [], []]
    ΔB: [[(2, 0.2222210916)], [(2, 0.2222210916)], [], []]
    ΔB (norm): [[(2, 0.5)], [(2, 0.5)], [], []]
    emaB: [[(2, 0.5), (3, 1)], [(2, 0.5)], [], []]
    emaB (max-upscale): [[(2, 1), (3, 1)], [(2, 1)], [], []]
    D: [0.5, 0.5, 0, 0]
    nE: [0.25, 0.25, 0.5, 0]
    E: [250000000, 250000000, 500000000, 0]
    P: [0.25, 0.25, 0.5, 0]
    P (u16): [32767, 32767, 65535, 0] */
    let bonds = get_bonds(&mut deps.storage, netuid);
    assert_eq!(get_dividends_for_uid(&mut deps.storage, netuid, 0), 32767); // Note D = floor(0.5 * 65_535)
    assert_eq!(
        get_emission_for_uid(&mut deps.storage, netuid, 0),
        250000000
    ); // Note E = 0.5 * 0.5 * 1_000_000_000 = 249311245
    assert_eq!(bonds[0][2], I32F32::from_num(65_535)); // floor(0.5*(2^16-1))/(2^16-1), then max-upscale
    assert_eq!(bonds[0][3], I32F32::from_num(65_535)); // only uid0 has updated weights for new reg
}

// Test the zero emission handling and fallback under zero effective weight conditions, to ensure non-zero effective emission.
#[test]
fn test_zero_weights() {
    let (mut deps, mut env) = instantiate_contract();

    let sparse: bool = true;
    let n: u16 = 2;
    let netuid: u16 = 2;
    let tempo: u16 = u16::MAX - 1; // high tempo to skip automatic epochs in on_initialize, use manual epochs instead
                                   // let mut block_number: u64 = en;
    let stake: u64 = 1;
    add_network(&mut deps.storage, netuid, tempo, 0);
    set_max_allowed_uids(&mut deps.storage, netuid, n);
    set_weights_set_rate_limit(&mut deps.storage, netuid, 0);
    set_max_registrations_per_block(&mut deps.storage, netuid, n);
    set_target_registrations_per_interval(&mut deps.storage, netuid, n);
    set_min_allowed_weights(&mut deps.storage, netuid, 0);
    set_max_weight_limit(&mut deps.storage, netuid, u16::MAX);
    set_min_difficulty(&mut deps.storage, netuid, 1_000);
    set_difficulty(&mut deps.storage, netuid, 1_000);

    // === Register [validator, server]
    for key in 0..n as u64 {
        let (nonce, work): (u64, Vec<u8>) = create_work_for_block_number(
            &mut deps.storage,
            netuid,
            env.block.height,
            key * 1_000_000,
            Addr::unchecked((1000 + key).to_string()).as_str(),
        );
        pow_register_ok_neuron(
            deps.as_mut(),
            env.clone(),
            netuid,
            env.block.height,
            nonce,
            work,
            Addr::unchecked((1000 + key).to_string()).as_str(),
            Addr::unchecked((1000 + key).to_string()).as_str(),
        )
        .unwrap();
    }
    for validator in 0..(n / 2) as u64 {
        add_balance_to_coldkey_account(&Addr::unchecked((1000 + validator).to_string()), stake);
        increase_stake_on_coldkey_hotkey_account(
            &mut deps.storage,
            &Addr::unchecked((1000 + validator).to_string()),
            &Addr::unchecked((1000 + validator).to_string()),
            stake,
        );
    }
    assert_eq!(get_subnetwork_n(&deps.storage, netuid), n);

    // === No weights
    if sparse {
        epoch(
            &mut deps.storage,
            &deps.api,
            netuid,
            1_000_000_000,
            env.block.height,
        )
        .unwrap();
    } else {
        epoch_dense(&mut deps.storage, netuid, 1_000_000_000, env.block.height);
    }
    /*	current_block: 0; activity_cutoff: 5000; Last update: [0, 0]; Inactive: [false, false]
    S: [1, 0]; S (mask): [1, 0]; S (mask+norm): [1, 0]; Block at registration: [0, 0]
    W: [[], []]; W (diagmask): [[], []]; W (diag+outdatemask): [[], []]; W (mask+norm): [[], []]
    R: [0, 0]; W (threshold): [[], []]; T: [0, 0]; C: [0.006693358, 0.006693358]; I: [0, 0]
    B: [[], []]; B (outdatedmask): [[], []]; B (mask+norm): [[], []];
    ΔB: [[], []]; ΔB (norm): [[], []]; emaB: [[], []]; D: [0, 0]
    E: [1000000000, 0]; P: [1, 0] */
    for validator in 0..(n / 2) as u16 {
        assert_eq!(
            get_emission_for_uid(&deps.storage, netuid, validator),
            1000000000
        ); // Note E = 1 * 1_000_000_000
    }
    for server in (n / 2)..n as u16 {
        assert_eq!(get_emission_for_uid(&deps.storage, netuid, server), 0);
        // no stake
    }
    step_block(deps.as_mut(), &mut env).unwrap(); // run to next block to ensure weights are set on nodes after their registration block

    // === Self-weights only: set weights [srv->srv: 1]
    for uid in ((n / 2) as u64)..n as u64 {
        set_weights(
            deps.as_mut(),
            env.clone(),
            Addr::unchecked((1000 + uid).to_string()).as_str(),
            netuid,
            vec![uid as u16],
            vec![u16::MAX],
            0,
        )
        .unwrap();
    }
    if sparse {
        epoch(
            &mut deps.storage,
            &deps.api,
            netuid,
            1_000_000_000,
            env.block.height,
        )
        .unwrap();
    } else {
        epoch_dense(&mut deps.storage, netuid, 1_000_000_000, env.block.height);
    }
    /*	current_block: 1; activity_cutoff: 5000; Last update: [0, 1]; Inactive: [false, false]
    S: [1, 0]; S (mask): [1, 0]; S (mask+norm): [1, 0]; Block at registration: [0, 0]
    W: [[], [(1, 1)]]
    W (diagmask): [[], []]; W (diag+outdatemask): [[], []]; W (mask+norm): [[], []]
    R: [0, 0]; W (threshold): [[], []]; T: [0, 0]; C: [0.006693358, 0.006693358]; I: [0, 0]
    B: [[], []]: B (outdatedmask): [[], []]; B (mask+norm): [[], []]
    ΔB: [[], []]; ΔB (norm): [[], []]; emaB: [[], []]; D: [0, 0]
    E: [1000000000, 0]; P: [1, 0] */
    for validator in 0..(n / 2) as u16 {
        assert_eq!(
            get_emission_for_uid(&deps.storage, netuid, validator),
            1000000000
        ); // Note E = 1 * 1_000_000_000
    }
    for server in (n / 2)..n as u16 {
        assert_eq!(get_emission_for_uid(&deps.storage, netuid, server), 0);
        // no stake
    }
    step_block(deps.as_mut(), &mut env).unwrap();

    // === Set weights [val->srv: 1/(n/2)]
    for uid in 0..(n / 2) as u64 {
        set_weights(
            deps.as_mut(),
            env.clone(),
            Addr::unchecked((1000 + uid).to_string()).as_str(),
            netuid,
            ((n / 2)..n).collect(),
            vec![u16::MAX / (n / 2); (n / 2) as usize],
            0,
        )
        .unwrap();
    }

    // === Outdate weights by reregistering servers
    for new_key in n..n + (n / 2) {
        // register a new key while at max capacity, which means the least emission uid will be deregistered
        let (nonce, work): (u64, Vec<u8>) = create_work_for_block_number(
            &mut deps.storage,
            netuid,
            env.block.height,
            new_key as u64 * 1_000_000,
            Addr::unchecked((1000 + new_key).to_string()).as_str(),
        );
        pow_register_ok_neuron(
            deps.as_mut(),
            env.clone(),
            netuid,
            env.block.height,
            nonce,
            work,
            Addr::unchecked((1000 + new_key).to_string()).as_str(),
            Addr::unchecked((1000 + new_key).to_string()).as_str(),
        )
        .unwrap();
    }
    if sparse {
        epoch(
            &mut deps.storage,
            &deps.api,
            netuid,
            1_000_000_000,
            env.block.height,
        )
        .unwrap();
    } else {
        epoch_dense(&mut deps.storage, netuid, 1_000_000_000, env.block.height);
    }
    /*	current_block: 2; activity_cutoff: 5000; Last update: [2, 1]; Inactive: [false, false];
    S: [1, 0]; S (mask): [1, 0]; S (mask+norm): [1, 0]; Block at registration: [0, 2];
    W: [[(1, 1)], []]; W (diagmask): [[(1, 1)], []]; W (diag+outdatemask): [[], []]; W (mask+norm): [[], []];
    R: [0, 0]; W (threshold): [[], []]; T: [0, 0]; C: [0.006693358, 0.006693358]; I: [0, 0];
    B: [[], []]; B (outdatedmask): [[], []]; B (mask+norm): [[], []];
    ΔB: [[], []]; ΔB (norm): [[], []]; emaB: [[], []]; D: [0, 0];
    E: [1000000000, 0]; P: [1, 0] */
    for validator in 0..(n / 2) as u16 {
        assert_eq!(
            get_emission_for_uid(&deps.storage, netuid, validator),
            1000000000
        ); // Note E = 1 * 1_000_000_000
    }
    for server in (n / 2)..n as u16 {
        assert_eq!(get_emission_for_uid(&deps.storage, netuid, server), 0);
        // no stake
    }
    step_block(deps.as_mut(), &mut env).unwrap();

    // === Set new weights [val->srv: 1/(n/2)] to check that updated weights would produce non-zero incentive
    for uid in 0..(n / 2) as u64 {
        set_weights(
            deps.as_mut(),
            env.clone(),
            Addr::unchecked((1000 + uid).to_string()).as_str(),
            netuid,
            ((n / 2)..n).collect(),
            vec![u16::MAX / (n / 2); (n / 2) as usize],
            0,
        )
        .unwrap();
    }
    if sparse {
        epoch(
            &mut deps.storage,
            &deps.api,
            netuid,
            1_000_000_000,
            env.block.height,
        )
        .unwrap();
    } else {
        epoch_dense(&mut deps.storage, netuid, 1_000_000_000, env.block.height);
    }
    /*	current_block: 3; activity_cutoff: 5000; Last update: [3, 1]; Inactive: [false, false];
    S: [1, 0]; S (mask): [1, 0]; S (mask+norm): [1, 0]; Block at registration: [0, 2];
    W: [[(1, 1)], []]; W (diagmask): [[(1, 1)], []]; W (diag+outdatemask): [[(1, 1)], []]; W (mask+norm): [[(1, 1)], []];
    R: [0, 1]; W (threshold): [[(1, 1)], []]; T: [0, 1]; C: [0.006693358, 0.9933076561]; I: [0, 1];
    B: [[], []]; B (outdatedmask): [[], []]; B (mask+norm): [[], []];
    ΔB: [[(1, 1)], []]; ΔB (norm): [[(1, 1)], []]; emaB: [[(1, 1)], []]; D: [1, 0]; emaB (max-upscale): [[(1, 1)], []]
    E: [500000000, 500000000]; P: [0.5, 0.5] */
    for validator in 0..n as u16 {
        assert_eq!(
            get_emission_for_uid(&deps.storage, netuid, validator),
            1000000000 / (n as u64)
        ); // Note E = 1/2 * 1_000_000_000
    }
}

// Test that epoch assigns validator permits to highest stake uids, varies uid interleaving and stake values.
#[test]
#[cfg(not(tarpaulin))]
fn test_validator_permits() {
    let netuid: u16 = 2;
    let tempo: u16 = u16::MAX - 1; // high tempo to skip automatic epochs in on_initialize, use manual epochs instead
    for interleave in 0..3 {
        for (network_n, validators_n) in vec![(2, 1), (4, 2), (8, 4)] {
            for assignment in 0..=1 {
                let (validators, servers) = distribute_nodes(
                    validators_n as usize,
                    network_n as usize,
                    interleave as usize,
                );
                let correct: bool = true;
                let mut stake: Vec<u64> = vec![0; network_n];
                for validator in &validators {
                    stake[*validator as usize] = match assignment {
                        1 => *validator as u64 + network_n as u64,
                        _ => 1,
                    };
                }
                for server in &servers {
                    stake[*server as usize] = match assignment {
                        1 => *server as u64,
                        _ => 0,
                    };
                }
                let (mut deps, mut env) = instantiate_contract();

                let block_number: u64 = env.block.height;
                add_network(&mut deps.storage, netuid, tempo, 0);
                set_min_difficulty(&mut deps.storage, netuid, 1_000);
                set_difficulty(&mut deps.storage, netuid, 1_000);
                set_max_allowed_uids(&mut deps.storage, netuid, network_n as u16);
                assert_eq!(
                    get_max_allowed_uids(&deps.storage, netuid),
                    network_n as u16
                );
                set_max_registrations_per_block(&mut deps.storage, netuid, network_n as u16);
                set_target_registrations_per_interval(&mut deps.storage, netuid, network_n as u16);

                // === Register [validator1, validator2, server1, server2]
                for key in 0..network_n as u64 {
                    add_balance_to_coldkey_account(
                        &Addr::unchecked((1000 + key).to_string()),
                        stake[key as usize],
                    );
                    let (nonce, work): (u64, Vec<u8>) = create_work_for_block_number(
                        &mut deps.storage,
                        netuid,
                        block_number,
                        key * 1_000_000,
                        &Addr::unchecked((1000 + key).to_string()).as_str(),
                    );
                    pow_register_ok_neuron(
                        deps.as_mut(),
                        env.clone(),
                        netuid,
                        block_number,
                        nonce,
                        work,
                        Addr::unchecked((1000 + key).to_string()).as_str(),
                        Addr::unchecked((1000 + key).to_string()).as_str(),
                    )
                    .unwrap();
                    increase_stake_on_coldkey_hotkey_account(
                        &mut deps.storage,
                        &Addr::unchecked((1000 + key).to_string()),
                        &Addr::unchecked((1000 + key).to_string()),
                        stake[key as usize],
                    );
                }
                assert_eq!(get_subnetwork_n(&deps.storage, netuid), network_n as u16);

                // === Issue validator permits
                set_max_allowed_validators(&mut deps.storage, netuid, validators_n as u16);
                assert_eq!(
                    get_max_allowed_validators(&deps.storage, netuid),
                    validators_n as u16
                );
                epoch(
                    &mut deps.storage,
                    &deps.api,
                    netuid,
                    1_000_000_000,
                    env.block.height,
                )
                .unwrap(); // run first epoch to set allowed validators
                for validator in &validators {
                    assert_eq!(
                        correct,
                        get_validator_permit_for_uid(&deps.storage, netuid, *validator)
                    );
                }
                for server in &servers {
                    assert_eq!(
                        !correct,
                        get_validator_permit_for_uid(&deps.storage, netuid, *server)
                    );
                }

                // === Increase server stake above validators
                for server in &servers {
                    add_balance_to_coldkey_account(
                        &Addr::unchecked((1000 + (*server as u64)).to_string()),
                        2 * network_n as u64,
                    );
                    increase_stake_on_coldkey_hotkey_account(
                        &mut deps.storage,
                        &Addr::unchecked((1000 + (*server as u64)).to_string()),
                        &Addr::unchecked((1000 + (*server as u64)).to_string()),
                        2 * network_n as u64,
                    );
                }

                // === Update validator permits
                step_block(deps.as_mut(), &mut env).unwrap();
                epoch(
                    &mut deps.storage,
                    &deps.api,
                    netuid,
                    1_000_000_000,
                    env.block.height,
                )
                .unwrap();

                // === Check that servers now own permits instead of the validator uids
                for validator in &validators {
                    assert_eq!(
                        !correct,
                        get_validator_permit_for_uid(&deps.storage, netuid, *validator)
                    );
                }
                for server in &servers {
                    assert_eq!(
                        correct,
                        get_validator_permit_for_uid(&deps.storage, netuid, *server)
                    );
                }

                drop(deps);
                drop(env);
            }
        }
    }
}

#[test]
fn test_graph_with_gas_sim() {
    let netuid: u16 = 2;
    let network_n: u16 = 512;
    let validators_n: u16 = 64;
    // let max_stake_per_validator: u64 = 328_125_000_000_000; // 21_000_000_000_000_000 / 64
    let epochs: u16 = 3;
    log::info!("test_{network_n:?}_graph ({validators_n:?} validators)");
    for interleave in 0..3 {
        for server_self in vec![false, true] {
            // server-self weight off/on
            let (validators1, servers1) = distribute_nodes(
                validators_n as usize,
                network_n as usize,
                interleave as usize,
            );
            let validators = &validators1;
            let servers = &servers1;
            let server: usize = servers[0] as usize;
            let validator: usize = validators[0] as usize;
            let (mut deps, mut env) = instantiate_contract();
            let gas = deps.storage.gas_used.borrow();
            println!(
                "before total {:?} gas {:?} write {:?} read {:?}",
                gas.total, gas.last, gas.write_cnt, gas.read_cnt
            );
            drop(gas);

            // fn init_run_epochs(
            //     mut deps: DepsMut,
            //     mut env: &mut Env,
            //     netuid: u16,
            //     n: u16,
            //     validators: &Vec<u16>,
            //     servers: &Vec<u16>,
            //     epochs: u16,
            //     stake_per_validator: u64,
            //     server_self: bool,
            //     input_stake: &Vec<u64>,
            //     use_input_stake: bool,
            //     input_weights: &Vec<Vec<(u16, u16)>>,
            //     use_input_weights: bool,
            //     random_weights: bool,
            //     random_seed: u64,
            //     sparse: bool,
            // ) {

            //             init_run_epochs(
            //                 deps.as_mut(),
            //                 &mut env,
            //                 netuid,
            //                 network_n,
            //                 &validators,
            //                 &servers,
            //                 epochs,
            //                 1,
            //                 server_self,
            //                 &vec![],
            //                 false,
            //                 &vec![],
            //                 false,
            //                 true,
            //                 interleave as u64,
            //                 false,
            //             );

            // netuid: u16,
            let n = network_n;
            // validators: &Vec<u16>,
            // servers: &Vec<u16>,
            // epochs: u16,
            let stake_per_validator = 1;
            // server_self: bool,
            let input_stake: &Vec<u64> = &vec![];
            let use_input_stake = false;
            let input_weights: &Vec<Vec<(u16, u16)>> =  &vec![];
            let use_input_weights = false;
            let random_weights = true;
            let random_seed = interleave;
            let sparse = false;

            // let (mut deps, mut env) = instantiate_contract();
            // === Create the network
            add_network(&mut deps.storage, netuid, u16::MAX - 1, 0); // set higher tempo to avoid built-in epoch, then manual epoch instead

            // === Register uids
            set_max_allowed_uids(&mut deps.storage, netuid, n);
            for key in 0..n {
                let stake: u64;
                if use_input_stake {
                    stake = input_stake[key as usize];
                } else {
                    stake = if validators.contains(&key) {
                        stake_per_validator
                    } else {
                        0
                    }; // only validators receive stake
                }
                // let stake: u64 = 1; // alternative test: all nodes receive stake, should be same outcome, except stake
                add_balance_to_coldkey_account(&Addr::unchecked((1000 + key).to_string()), stake);
                append_neuron(
                    &mut deps.storage,
                    &mut deps.api,
                    netuid,
                    &(Addr::unchecked((1000 + key).to_string())),
                    0,
                )
                    .unwrap();
                increase_stake_on_coldkey_hotkey_account(
                    &mut deps.storage,
                    &Addr::unchecked((1000 + key).to_string()),
                    &Addr::unchecked((1000 + key).to_string()),
                    stake as u64,
                );
            }
            assert_eq!(get_subnetwork_n(&mut deps.storage, netuid), n);

            // === Issue validator permits
            set_max_allowed_validators(&mut deps.storage, netuid, validators.len() as u16);

            assert_eq!(
                get_max_allowed_validators(&mut deps.storage, netuid),
                validators.len() as u16
            );

            epoch(
                &mut deps.storage,
                &mut deps.api,
                netuid,
                1_000_000_000,
                env.block.height,
            )
                .unwrap(); // run first epoch to set allowed validators
            step_block(deps.as_mut(), &mut env).unwrap(); // run to next block to ensure weights are set on nodes after their registration block

            // === Set weights
            let mut rng = StdRng::seed_from_u64(random_seed); // constant seed so weights over multiple runs are equal
            let range = Uniform::new(0, u16::MAX);
            let mut weights: Vec<u16> = vec![u16::MAX / n; servers.len() as usize];
            for uid in validators {
                if random_weights {
                    weights = (0..servers.len()).map(|_| rng.sample(&range)).collect();
                    weights = normalize_weights(weights);
                    // assert_eq!(weights.iter().map(|x| *x as u64).sum::<u64>(), u16::MAX as u64); // normalized weight sum not always u16::MAX
                }
                if use_input_weights {
                    let sparse_weights = input_weights[*uid as usize].clone();
                    weights = sparse_weights.iter().map(|(_, w)| *w).collect();
                    let srvs: Vec<u16> = sparse_weights.iter().map(|(s, _)| *s).collect();

                    let result = set_weights(
                        deps.as_mut(),
                        env.clone(),
                        (1000 + uid).to_string().as_str(),
                        netuid,
                        srvs.clone(),
                        weights.clone(),
                        0,
                    );
                    assert_eq!(result.is_ok(), true);


                    // let msg = ExecuteMsg::SetWeights {
                    //     netuid,
                    //     dests: srvs.clone(),
                    //     weights: weights.clone(),
                    //     version_key: 0,
                    // };
                    // let info = mock_info((1000 + uid).to_string().as_str(), &[]);
                    // let res = execute(deps.branch(), env.clone(), info, msg);
                    // assert_eq!(res.is_ok(), true);

                } else {
                    let result = set_weights(
                        deps.as_mut(),
                        env.clone(),
                        (1000 + uid).to_string().as_str(),
                        netuid,
                        servers.clone(),
                        weights.clone(),
                        0,
                    );
                    assert_eq!(result.is_ok(), true);

                    // let msg = ExecuteMsg::SetWeights {
                    //     netuid,
                    //     dests: servers.clone(),
                    //     weights: weights.clone(),
                    //     version_key: 0,
                    // };
                    // let info = mock_info((1000 + uid).to_string().as_str(), &[]);
                    // let res = execute(deps.branch(), env.clone(), info, msg);
                    // assert_eq!(res.is_ok(), true);
                }
            }
            for uid in servers {
                if server_self {
                    let result = set_weights(
                        deps.as_mut(),
                        env.clone(),
                        (1000 + uid).to_string().as_str(),
                        netuid,
                        vec![*uid as u16],
                        vec![u16::MAX],
                        0,
                    );
                    assert_eq!(result.is_ok(), true);

                    // let msg = ExecuteMsg::SetWeights {
                    //     netuid,
                    //     dests: vec![*uid as u16],
                    //     weights: vec![u16::MAX],
                    //     version_key: 0,
                    // }; // server self-weight
                    // let info = mock_info((1000 + uid).to_string().as_str(), &[]);
                    // let res = execute(deps.branch(), env.clone(), info, msg);
                    // assert_eq!(res.is_ok(), true);
                }
            }

            // === Run the epochs.
            for n in 0..epochs {
                println!("Start {n} epoch");
                let start = Instant::now();
                let gas = deps.storage.gas_used.borrow();
                let gas_total = gas.total;
                let gas_last = gas.last;
                let gas_write_cnt = gas.write_cnt;
                let gas_read_cnt = gas.read_cnt;
                println!(
                    "before epoch {:?} total {:?} gas {:?} write {:?} read {:?}",
                    n, gas.total, gas.last, gas.write_cnt, gas.read_cnt
                );
                drop(gas);

                if sparse {
                    epoch(
                        &mut deps.storage,
                        &mut deps.api,
                        netuid,
                        1_000_000_000,
                        env.block.height,
                    )
                        .unwrap();
                } else {
                    epoch_dense(&mut deps.storage, netuid, 1_000_000_000, env.block.height);
                }

                let gas = deps.storage.gas_used.borrow();
                println!(
                    "after epoch {:?} total {:?} gas {:?} write {:?} read {:?}",
                    n, gas.total, gas.last, gas.write_cnt, gas.read_cnt
                );
                println!(
                    "after epoch {:?} total {:?} gas {:?} write {:?} read {:?}",
                    n, gas_total, gas_last, gas_write_cnt, gas_read_cnt
                );
                println!(
                    "diff epoch {:?} gas {:?} write {:?} read {:?}",
                    n, gas.total-gas_total, gas.write_cnt-gas_write_cnt, gas.read_cnt-gas_read_cnt
                );
                drop(gas);

                let duration = start.elapsed();
                println!(
                    "Time elapsed in (sparse={sparse}) epoch() is: {:?}",
                    duration
                );
            }

            // let bonds = get_bonds(&deps.storage, netuid );
            // for (uid, node) in vec![ (validators[0], "validator"), (servers[0], "server") ] {
            // 	log::info!("\n{node}" );
            // 	uid_stats(netuid, uid);
            // 	log::info!("bonds: {:?} (on validator), {:?} (on server)", bonds[uid as usize][0], bonds[uid as usize][servers[0] as usize]);
            // }


            let gas = deps.storage.gas_used.borrow();
            println!(
                "after total {:?} gas {:?} write {:?} read {:?}",
                gas.total, gas.last, gas.write_cnt, gas.read_cnt
            );
            drop(gas);

            let bonds = get_bonds(&deps.storage, netuid);
            for uid in validators {
                // assert_eq!(
                //     get_total_stake_for_hotkey(
                //         &deps.storage,
                //         &Addr::unchecked((1000 + uid).to_string()),
                //     ),
                //     max_stake_per_validator
                // );
                assert_eq!(get_rank_for_uid(&deps.storage, netuid, *uid), 0);
                assert_eq!(get_trust_for_uid(&deps.storage, netuid, *uid), 0);
                assert_eq!(get_consensus_for_uid(&deps.storage, netuid, *uid), 0);
                assert_eq!(get_incentive_for_uid(&deps.storage, netuid, *uid), 0);
                // assert_eq!(get_dividends_for_uid(&deps.storage, netuid, *uid), 1023); // Note D = floor(1 / 64 * 65_535) = 1023
                // assert_eq!(get_emission_for_uid(&deps.storage, netuid, *uid), 7812500); // Note E = 0.5 / 200 * 1_000_000_000 = 7_812_500
                assert_eq!(bonds[*uid as usize][validator], 0.0);
                // assert_eq!(bonds[*uid as usize][server], I32F32::from_num(65_535));
                // Note B_ij = floor(1 / 64 * 65_535) / 65_535 = 1023 / 65_535, then max-upscaled to 65_535
            }
            for uid in servers {
                assert_eq!(
                    get_total_stake_for_hotkey(
                        &deps.storage,
                        &Addr::unchecked((1000 + uid).to_string()),
                    ),
                    0
                );
                // assert_eq!(get_rank_for_uid(&deps.storage, netuid, *uid), 146); // Note R = floor(1 / (512 - 64) * 65_535) = 146
                // assert_eq!(get_trust_for_uid(&deps.storage, netuid, *uid), 65535);
                // assert_eq!(get_consensus_for_uid(&deps.storage, netuid, *uid), 146); // Note C = floor(1 / (512 - 64) * 65_535) = 146
                // assert_eq!(get_incentive_for_uid(&deps.storage, netuid, *uid), 146); // Note I = floor(1 / (512 - 64) * 65_535) = 146
                assert_eq!(get_dividends_for_uid(&deps.storage, netuid, *uid), 0);
                // assert_eq!(get_emission_for_uid(&deps.storage, netuid, *uid), 1116071); // Note E = floor(0.5 / (512 - 64) * 1_000_000_000) = 1_116_071
                assert_eq!(bonds[*uid as usize][validator], 0.0);
                assert_eq!(bonds[*uid as usize][server], 0.0);
            }
            drop(deps);
            drop(env);
        }
    }
}

// // Map the retention graph for consensus guarantees with an single epoch on a graph with 512 nodes, of which the first 64 are validators, the graph is split into a major and minor set, each setting specific weight on itself and the complement on the other.
// //
// // ```import torch
// // import matplotlib.pyplot as plt
// // from matplotlib.pyplot import cm
// // %matplotlib inline
// //
// // with open('finney_consensus_0.4.txt') as f:  # test output saved to finney_consensus.txt
// //     retention_map = eval(f.read())
// //
// // major_ratios = {}
// // avg_weight_devs = {}
// // for major_stake, major_weight, minor_weight, avg_weight_dev, major_ratio in retention_map:
// //     major_stake = f'{major_stake:.2f}'
// //     maj, min = int(round(50 * major_weight)), int(round(50 * minor_weight))
// //     avg_weight_devs.setdefault(major_stake, torch.zeros((51, 51)))
// //     avg_weight_devs[major_stake][maj][min] = avg_weight_dev
// //     major_ratios.setdefault(major_stake, torch.zeros((51, 51)))
// //     major_ratios[major_stake][maj][min] = major_ratio
// //
// // _x = torch.linspace(0, 1, 51); _y = torch.linspace(0, 1, 51)
// // x, y = torch.meshgrid(_x, _y, indexing='ij')
// //
// // fig = plt.figure(figsize=(6, 6), dpi=70); ax = fig.gca()
// // ax.set_xticks(torch.arange(0, 1, 0.05)); ax.set_yticks(torch.arange(0, 1., 0.05))
// // ax.set_xticklabels([f'{_:.2f}'[1:] for _ in torch.arange(0, 1., 0.05)])
// // plt.grid(); plt.rc('grid', linestyle="dotted", color=[0.85, 0.85, 0.85])
// //
// // isolate = ['0.60']; stakes = [0.51, 0.55, 0.6, 0.65, 0.7, 0.75, 0.8, 0.85, 0.9, 0.95, 0.99]
// // colors = cm.viridis(torch.linspace(0, 1, len(stakes) + 1))
// // for i, stake in enumerate(stakes):
// //     contours = plt.contour(x, y, major_ratios[f'{stake:.2f}'], levels=[0., stake], colors=[colors[i + 1]])
// //     if f'{stake:.2f}' in isolate:
// //         contours.collections[1].set_linewidth(3)
// //     plt.clabel(contours, inline=True, fontsize=10)
// //
// // plt.title(f'Major emission [$stake_{{maj}}=emission_{{maj}}$ retention lines]')
// // plt.ylabel('Minor self-weight'); plt.xlabel('Major self-weight'); plt.show()
// // ```
// // #[test]
// fn _map_consensus_guarantees() {
//     let netuid: u16 = 1;
//     let network_n: u16 = 512;
//     let validators_n: u16 = 64;
//     let epochs: u16 = 1;
//     let interleave = 0;
//     let weight_stddev: I32F32 = fixed(0.4);
//     println!("[");
//     for _major_stake in vec![0.51, 0.55, 0.6, 0.65, 0.7, 0.75, 0.8, 0.85, 0.9, 0.95, 0.99] {
//         let major_stake: I32F32 = I32F32::from_num(_major_stake);
//         for _major_weight in 0..51 {
//             let major_weight: I32F32 = I32F32::from_num(50 - _major_weight) / I32F32::from_num(50);
//             for _minor_weight in 0..51 {
//                 let minor_weight: I32F32 =
//                     I32F32::from_num(50 - _minor_weight) / I32F32::from_num(50);
//                 let (
//                     validators,
//                     servers,
//                     major_validators,
//                     minor_validators,
//                     major_servers,
//                     minor_servers,
//                     stake,
//                     weights,
//                     avg_weight_dev,
//                 ) = split_graph(
//                     major_stake,
//                     major_weight,
//                     minor_weight,
//                     weight_stddev,
//                     validators_n as usize,
//                     network_n as usize,
//                     interleave as usize,
//                 );

//                 new_test_ext().execute_with(|| {
// 					init_run_epochs(netuid, network_n, &validators, &servers, epochs, 1, true, &stake, true, &weights, true, false, 0, true);

// 					let mut major_emission: I64F64 = I64F64::from_num(0);
// 					let mut minor_emission: I64F64 = I64F64::from_num(0);
// 					for set in vec![major_validators, major_servers] {
// 						for uid in set {
// 							major_emission += I64F64::from_num(SubtensorModule::get_emission_for_uid( netuid, uid ));
// 						}
// 					}
// 					for set in vec![minor_validators, minor_servers] {
// 						for uid in set {
// 							minor_emission += I64F64::from_num(SubtensorModule::get_emission_for_uid( netuid, uid ));
// 						}
// 					}
// 					let major_ratio: I32F32 = I32F32::from_num(major_emission / (major_emission + minor_emission));
// 					println!("[{major_stake}, {major_weight:.2}, {minor_weight:.2}, {avg_weight_dev:.3}, {major_ratio:.3}], ");
// 				});
//             }
//         }
//     }
//     println!("]");
// }
