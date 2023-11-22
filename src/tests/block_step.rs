use cosmwasm_std::Addr;

use crate::block_step::{
    blocks_until_next_epoch, drain_emission, generate_emission, get_loaded_emission_tuples,
    has_loaded_emission_tuples, tuples_to_drain_this_block,
};
use crate::registration::create_work_for_block_number;
use crate::root::set_emission_values;
use crate::staking::add_balance_to_coldkey_account;
use crate::test_helpers::{
    add_network, burned_register_ok_neuron, instantiate_contract, pow_register_ok_neuron,
    step_block, sudo_register_ok_neuron,
};
use crate::utils::{
    get_adjustment_interval, get_burn_as_u64, get_difficulty_as_u64, set_adjustment_alpha,
    set_adjustment_interval, set_burn, set_difficulty, set_max_allowed_uids,
    set_max_registrations_per_block, set_min_difficulty, set_target_registrations_per_interval,
};

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
        for block in 0..30 as u64 {
            for tempo in 1..30 as u16 {
                assert_eq!(
                    blocks_until_next_epoch(netuid, tempo, block),
                    tempo as u64 - (block + netuid as u64) % (tempo as u64 + 1)
                );
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
    let burn_cost: u64 = 1000;
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
    burned_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        hotkey_account_id_1,
        coldkey_account_id_1,
    );

    // Register key 2.
    let hotkey_account_id_2 = "1002";
    let coldkey_account_id_2 = "1002";
    add_balance_to_coldkey_account(&Addr::unchecked(coldkey_account_id_2), 10000);
    burned_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        hotkey_account_id_2,
        coldkey_account_id_2,
    );

    // We are over the number of regs allowed this interval.
    // Step the block and trigger the adjustment.
    step_block(deps.as_mut(), &mut env).unwrap();

    // Check the adjusted burn.
    assert_eq!(get_burn_as_u64(&deps.storage, netuid), 1500);
}

#[test]
fn test_burn_adjustment_with_moving_average() {
    let (mut deps, mut env) = instantiate_contract();

    let netuid: u16 = 2;
    let tempo: u16 = 13;
    let burn_cost: u64 = 1000;
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
    burned_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        hotkey_account_id_1,
        coldkey_account_id_1,
    );

    // Register key 2.
    let hotkey_account_id_2 = "addr2";
    let coldkey_account_id_2 = "addr2";
    add_balance_to_coldkey_account(&Addr::unchecked(coldkey_account_id_2), 10000);
    burned_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        hotkey_account_id_2,
        coldkey_account_id_2,
    );

    // We are over the number of regs allowed this interval.
    // Step the block and trigger the adjustment.
    step_block(deps.as_mut(), &mut env).unwrap();

    // Check the adjusted burn.
    // 0.5 * 1000 + 0.5 * 1500 = 1250
    assert_eq!(get_burn_as_u64(&deps.storage, netuid), 1250);
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
    let burn_cost: u64 = 1000;
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
    burned_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        hotkey_account_id_1,
        coldkey_account_id_1,
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
    let burn_cost: u64 = 1000;
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
    burned_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        hotkey_account_id_1,
        coldkey_account_id_1,
    );

    // Register key 2.
    let hotkey_account_id_2 = "addr2";
    let coldkey_account_id_2 = "addr2";
    add_balance_to_coldkey_account(&Addr::unchecked(coldkey_account_id_2), 10000);
    burned_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        hotkey_account_id_2,
        coldkey_account_id_2,
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
    assert_eq!(adjusted_burn, 2_000);

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
    burned_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        &hotkey_account_id_1,
        coldkey_account_id_1,
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
    assert_eq!(adjusted_burn, 875);

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
    burned_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        hotkey_account_id_1,
        coldkey_account_id_1,
    );

    // Register key 2. This is a BURN registration
    let hotkey_account_id_2 = "addr2";
    let coldkey_account_id_2 = "addr2";
    add_balance_to_coldkey_account(&Addr::unchecked(coldkey_account_id_2), 10000);
    burned_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        hotkey_account_id_2,
        coldkey_account_id_2,
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
    let burn_cost: u64 = 1000;
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
    burned_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        &hotkey_account_id_2,
        coldkey_account_id_2,
    );
    step_block(deps.as_mut(), &mut env).unwrap();
    curr_block_num += 1;

    // We are UNDER the number of regs allowed this interval.
    // And the number of regs of each type is equal

    // Check the adjusted BURN has DECREASED.
    let adjusted_burn = get_burn_as_u64(&deps.storage, netuid);
    assert!(adjusted_burn < burn_cost);
    assert_eq!(adjusted_burn, 833);

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
    let burn_cost: u64 = 1000;
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
    burned_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        hotkey_account_id_2,
        coldkey_account_id_2,
    );

    step_block(deps.as_mut(), &mut env).unwrap();
    curr_block_num += 1;
    // We are OVER the number of regs allowed this interval.
    // And the number of regs of each type is equal

    // Check the adjusted BURN has INCREASED.
    let adjusted_burn = get_burn_as_u64(&deps.storage, netuid);
    assert!(adjusted_burn > burn_cost);
    assert_eq!(adjusted_burn, 1_500);

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
    let burn_cost: u64 = 1000;
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
    assert_eq!(adjusted_burn, 500);

    // Check the adjusted POW difficulty has DECREASED.
    let adjusted_diff = get_difficulty_as_u64(&deps.storage, netuid);
    assert!(adjusted_diff < start_diff);
    assert_eq!(adjusted_diff, 10_000);
}
