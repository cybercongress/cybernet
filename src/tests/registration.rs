use cosmwasm_std::testing::mock_info;
use cosmwasm_std::{Addr, Uint128};

use crate::contract::execute;
use crate::msg::ExecuteMsg;
use crate::registration::{create_work_for_block_number, get_neuron_to_prune};
use crate::serving::get_axon_info;
use crate::staking::get_owning_coldkey_for_hotkey;
use crate::state::AxonInfoOf;
use crate::test_helpers::{
    add_balance_to_coldkey_account, add_network, burned_register_ok_neuron, instantiate_contract,
    pow_register_ok_neuron, register_ok_neuron, run_step_to_block, step_block,
};
use crate::uids::{
    get_hotkey_for_net_and_uid, get_stake_for_uid_and_subnetwork, get_subnetwork_n,
    get_uid_for_net_and_hotkey, is_uid_exist_on_network,
};
use crate::utils::{
    get_burn_as_u64, get_difficulty_as_u64, get_emission_value, get_immunity_period,
    get_max_allowed_uids, get_max_registrations_per_block, get_neuron_block_at_registration,
    get_pruning_score_for_uid, get_rao_recycled, get_registrations_this_block,
    get_registrations_this_interval, get_target_registrations_per_interval, get_tempo,
    set_adjustment_interval, set_burn, set_difficulty, set_immunity_period, set_max_allowed_uids,
    set_max_registrations_per_block, set_network_registration_allowed, set_pruning_score_for_uid,
    set_target_registrations_per_interval,
};
use crate::ContractError;

/********************************************
    subscribing::subscribe() tests
*********************************************/

#[test]
fn test_registration_difficulty() {
    let (deps, _) = instantiate_contract();

    assert_eq!(get_difficulty_as_u64(&deps.storage, 1), 10000000)
}

// #[test]
// fn test_registration_invalid_seal_hotkey() {
//     let (mut deps, env) = instantiate_contract();
//
//         let block_number: u64 = 0;
//         let netuid: u16 = 2;
//         let tempo: u16 = 13;
//         let hotkey_account_id_1 = "addr1";
//         let hotkey_account_id_2 = "addr2";
//         let coldkey_account_id = "addr667"; // Neighbour of the beast, har har
//         let (nonce, work): (u64, Vec<u8>) = create_work_for_block_number(
//             &deps.storage,
//             netuid,
//             block_number,
//             0,
//             &hotkey_account_id_1,
//         );
//         let (nonce2, work2): (u64, Vec<u8>) = create_work_for_block_number(
//             &deps.storage,
//             netuid,
//             block_number,
//             0,
//             &hotkey_account_id_1,
//         );
//
//         //add network
//         add_network(&mut deps.storage, netuid, tempo, 0);
//
//         let result = pow_register_ok_neuron(
//             deps.as_mut(),
//             env.clone(),
//             netuid,
//             env.block.height,
//             nonce,
//             work.clone(),
//             &hotkey_account_id_1,
//             &coldkey_account_id
//         );
//      assert_eq!(result.is_ok);
//
//         let result = pow_register_ok_neuron(
//             deps.as_mut(),
//             env.clone(),
//             netuid,
//             block_number,
//             nonce2,
//             work2.clone(),
//             &hotkey_account_id_2,
//             &coldkey_account_id,
//         );
//         assert_eq!(result.unwrap_err(), ContractError::InvalidSeal{})
// }

#[test]
fn test_registration_ok() {
    let (mut deps, env) = instantiate_contract();

    let block_number: u64 = 0;
    let netuid: u16 = 2;
    let tempo: u16 = 13;
    let hotkey_account_id = "addr1";
    let coldkey_account_id = "addr667"; // Neighbour of the beast, har har

    add_network(&mut deps.storage, netuid, tempo, 0);

    let (nonce, work): (u64, Vec<u8>) = create_work_for_block_number(
        &deps.storage,
        netuid,
        block_number,
        129123813,
        hotkey_account_id,
    );

    let result = pow_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        env.block.height,
        nonce,
        work,
        hotkey_account_id,
        coldkey_account_id,
    );
    assert!(result.is_ok());

    // Check if neuron has added to the specified network(netuid)
    assert_eq!(get_subnetwork_n(&deps.storage, netuid), 1);

    //check if hotkey is added to the Hotkeys
    assert_eq!(
        get_owning_coldkey_for_hotkey(&deps.storage, &Addr::unchecked(hotkey_account_id)),
        coldkey_account_id
    );

    // Check if the neuron has added to the Keys
    let neuron_uid =
        get_uid_for_net_and_hotkey(&deps.storage, netuid, &Addr::unchecked(hotkey_account_id))
            .unwrap();

    assert!(
        get_uid_for_net_and_hotkey(&deps.storage, netuid, &Addr::unchecked(hotkey_account_id))
            .is_ok()
    );
    // Check if neuron has added to Uids
    let neuro_uid =
        get_uid_for_net_and_hotkey(&deps.storage, netuid, &Addr::unchecked(hotkey_account_id))
            .unwrap();
    assert_eq!(neuro_uid, neuron_uid);

    // Check if the balance of this hotkey account for this subnetwork == 0
    assert_eq!(
        get_stake_for_uid_and_subnetwork(&deps.storage, netuid, neuron_uid),
        0
    )
}

/********************************************
    registration::do_burned_registration tests
*********************************************/

#[test]
fn test_burned_registration_ok() {
    let (mut deps, env) = instantiate_contract();

    let netuid: u16 = 2;
    let tempo: u16 = 13;
    let hotkey_account_id = "addr1";
    let burn_cost = 1000;
    let coldkey_account_id = "addr667"; // Neighbour of the beast, har har

    set_burn(&mut deps.storage, netuid, burn_cost);
    add_network(&mut deps.storage, netuid, tempo, 0);
    // Give it some $$$ in his coldkey balance
    add_balance_to_coldkey_account(&Addr::unchecked(coldkey_account_id), 10000);

    assert_eq!(
        burned_register_ok_neuron(
            deps.as_mut(),
            env.clone(),
            netuid,
            hotkey_account_id,
            coldkey_account_id,
        )
        .is_ok(),
        true
    );

    // Check if balance has  decreased to pay for the burn.
    // assert_eq!(
    //     get_coldkey_balance(&coldkey_account_id) as u64,
    //     10000 - burn_cost
    // ); // funds drained on reg.
    // Check if neuron has added to the specified network(netuid)
    assert_eq!(get_subnetwork_n(&deps.storage, netuid), 1);
    //check if hotkey is added to the Hotkeys
    assert_eq!(
        get_owning_coldkey_for_hotkey(&deps.storage, &Addr::unchecked(hotkey_account_id)),
        coldkey_account_id
    );
    // Check if the neuron has added to the Keys
    let neuron_uid =
        get_uid_for_net_and_hotkey(&deps.storage, netuid, &Addr::unchecked(hotkey_account_id))
            .unwrap();
    assert!(
        get_uid_for_net_and_hotkey(&deps.storage, netuid, &Addr::unchecked(hotkey_account_id))
            .is_ok()
    );
    // Check if neuron has added to Uids
    let neuro_uid =
        get_uid_for_net_and_hotkey(&deps.storage, netuid, &Addr::unchecked(hotkey_account_id))
            .unwrap();
    assert_eq!(neuro_uid, neuron_uid);
    // Check if the balance of this hotkey account for this subnetwork == 0
    assert_eq!(
        get_stake_for_uid_and_subnetwork(&deps.storage, netuid, neuron_uid),
        0
    )
}

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
    assert_eq!(get_burn_as_u64(&deps.storage, netuid), 1500000000)
}

#[test]
#[cfg(not(tarpaulin))]
fn test_registration_too_many_registrations_per_block() {
    let (mut deps, env) = instantiate_contract();

    let netuid: u16 = 2;
    let tempo: u16 = 13;
    add_network(&mut deps.storage, netuid, tempo, 0);
    set_max_registrations_per_block(&mut deps.storage, netuid, 10);
    set_target_registrations_per_interval(&mut deps.storage, netuid, 10);

    assert_eq!(get_max_registrations_per_block(&deps.storage, netuid), 10);

    let block_number: u64 = 0;
    let (nonce0, work0): (u64, Vec<u8>) =
        create_work_for_block_number(&deps.storage, netuid, block_number, 3942084, "addr0");
    let (nonce1, work1): (u64, Vec<u8>) =
        create_work_for_block_number(&deps.storage, netuid, block_number, 11231312312, "addr1");
    let (nonce2, work2): (u64, Vec<u8>) =
        create_work_for_block_number(&deps.storage, netuid, block_number, 212312414, "addr2");
    let (nonce3, work3): (u64, Vec<u8>) =
        create_work_for_block_number(&deps.storage, netuid, block_number, 21813123, "addr3");
    let (nonce4, work4): (u64, Vec<u8>) =
        create_work_for_block_number(&deps.storage, netuid, block_number, 148141209, "addr4");
    let (nonce5, work5): (u64, Vec<u8>) =
        create_work_for_block_number(&deps.storage, netuid, block_number, 1245235534, "addr5");
    let (nonce6, work6): (u64, Vec<u8>) =
        create_work_for_block_number(&deps.storage, netuid, block_number, 256234, "addr6");
    let (nonce7, work7): (u64, Vec<u8>) =
        create_work_for_block_number(&deps.storage, netuid, block_number, 6923424, "addr7");
    let (nonce8, work8): (u64, Vec<u8>) =
        create_work_for_block_number(&deps.storage, netuid, block_number, 124242, "addr8");
    let (nonce9, work9): (u64, Vec<u8>) =
        create_work_for_block_number(&deps.storage, netuid, block_number, 153453, "addr9");
    let (nonce10, work10): (u64, Vec<u8>) =
        create_work_for_block_number(&deps.storage, netuid, block_number, 345923888, "addr10");
    assert_eq!(get_difficulty_as_u64(&deps.storage, netuid), 1);

    let result = pow_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        block_number,
        nonce0,
        work0,
        "addr0",
        "addr0",
    );
    assert!(result.is_ok());
    assert_eq!(get_registrations_this_block(&deps.storage, netuid), 1);

    let result = pow_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        block_number,
        nonce1,
        work1,
        "addr1",
        "addr1",
    );
    assert!(result.is_ok());
    assert_eq!(get_registrations_this_block(&deps.storage, netuid), 2);

    let result = pow_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        block_number,
        nonce2,
        work2,
        "addr2",
        "addr2",
    );
    assert!(result.is_ok());
    assert_eq!(get_registrations_this_block(&deps.storage, netuid), 3);

    let result = pow_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        block_number,
        nonce3,
        work3,
        "addr3",
        "addr3",
    );
    assert!(result.is_ok());
    assert_eq!(get_registrations_this_block(&deps.storage, netuid), 4);

    let result = pow_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        block_number,
        nonce4,
        work4,
        "addr4",
        "addr4",
    );
    assert!(result.is_ok());
    assert_eq!(get_registrations_this_block(&deps.storage, netuid), 5);

    let result = pow_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        block_number,
        nonce5,
        work5,
        "addr5",
        "addr5",
    );
    assert!(result.is_ok());
    assert_eq!(get_registrations_this_block(&deps.storage, netuid), 6);

    let result = pow_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        block_number,
        nonce6,
        work6,
        "addr6",
        "addr6",
    );
    assert!(result.is_ok());
    assert_eq!(get_registrations_this_block(&deps.storage, netuid), 7);

    let result = pow_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        block_number,
        nonce7,
        work7,
        "addr7",
        "addr7",
    );
    assert!(result.is_ok());
    assert_eq!(get_registrations_this_block(&deps.storage, netuid), 8);

    let result = pow_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        block_number,
        nonce8,
        work8,
        "addr8",
        "addr8",
    );
    assert!(result.is_ok());
    assert_eq!(get_registrations_this_block(&deps.storage, netuid), 9);

    let result = pow_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        block_number,
        nonce9,
        work9,
        "addr9",
        "addr9",
    );
    assert!(result.is_ok());
    assert_eq!(get_registrations_this_block(&deps.storage, netuid), 10);

    let result = pow_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        block_number,
        nonce10,
        work10,
        "addr10",
        "addr10",
    );
    assert_eq!(
        result.unwrap_err(),
        ContractError::TooManyRegistrationsThisBlock {}
    )
}

#[test]
fn test_registration_too_many_registrations_per_interval() {
    let (mut deps, env) = instantiate_contract();

    let netuid: u16 = 2;
    let tempo: u16 = 13;
    add_network(&mut deps.storage, netuid, tempo, 0);
    set_max_registrations_per_block(&mut deps.storage, netuid, 11);
    assert_eq!(get_max_registrations_per_block(&deps.storage, netuid), 11);
    set_target_registrations_per_interval(&mut deps.storage, netuid, 3);

    assert_eq!(
        get_target_registrations_per_interval(&deps.storage, netuid),
        3
    );
    // Then the max is 3 * 3 = 9

    let block_number: u64 = 0;
    let (nonce0, work0): (u64, Vec<u8>) =
        create_work_for_block_number(&deps.storage, netuid, block_number, 3942084, "addr0");
    let (nonce1, work1): (u64, Vec<u8>) =
        create_work_for_block_number(&deps.storage, netuid, block_number, 11231312312, "addr1");
    let (nonce2, work2): (u64, Vec<u8>) =
        create_work_for_block_number(&deps.storage, netuid, block_number, 212312414, "addr2");
    let (nonce3, work3): (u64, Vec<u8>) =
        create_work_for_block_number(&deps.storage, netuid, block_number, 21813123, "addr3");
    let (nonce4, work4): (u64, Vec<u8>) =
        create_work_for_block_number(&deps.storage, netuid, block_number, 148141209, "addr4");
    let (nonce5, work5): (u64, Vec<u8>) =
        create_work_for_block_number(&deps.storage, netuid, block_number, 1245235534, "addr5");
    let (nonce6, work6): (u64, Vec<u8>) =
        create_work_for_block_number(&deps.storage, netuid, block_number, 256234, "addr6");
    let (nonce7, work7): (u64, Vec<u8>) =
        create_work_for_block_number(&deps.storage, netuid, block_number, 6923424, "addr7");
    let (nonce8, work8): (u64, Vec<u8>) =
        create_work_for_block_number(&deps.storage, netuid, block_number, 124242, "addr8");
    let (nonce9, work9): (u64, Vec<u8>) =
        create_work_for_block_number(&deps.storage, netuid, block_number, 153453, "addr9");
    assert_eq!(get_difficulty_as_u64(&deps.storage, netuid), 1);

    // Try 10 registrations, this is less than the max per block, but more than the max per interval
    let result = pow_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        block_number,
        nonce0,
        work0,
        "addr0",
        "addr0",
    );
    assert!(result.is_ok());
    assert_eq!(get_registrations_this_interval(&deps.storage, netuid), 1);

    let result = pow_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        block_number,
        nonce1,
        work1,
        "addr1",
        "addr1",
    );
    assert!(result.is_ok());
    assert_eq!(get_registrations_this_interval(&deps.storage, netuid), 2);

    let result = pow_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        block_number,
        nonce2,
        work2,
        "addr2",
        "addr2",
    );
    assert!(result.is_ok());
    assert_eq!(get_registrations_this_interval(&deps.storage, netuid), 3);

    let result = pow_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        block_number,
        nonce3,
        work3,
        "addr3",
        "addr3",
    );
    assert!(result.is_ok());
    assert_eq!(get_registrations_this_interval(&deps.storage, netuid), 4);

    let result = pow_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        block_number,
        nonce4,
        work4,
        "addr4",
        "addr4",
    );
    assert!(result.is_ok());
    assert_eq!(get_registrations_this_interval(&deps.storage, netuid), 5);

    let result = pow_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        block_number,
        nonce5,
        work5,
        "addr5",
        "addr5",
    );
    assert!(result.is_ok());
    assert_eq!(get_registrations_this_interval(&deps.storage, netuid), 6);

    let result = pow_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        block_number,
        nonce6,
        work6,
        "addr6",
        "addr6",
    );
    assert!(result.is_ok());
    assert_eq!(get_registrations_this_interval(&deps.storage, netuid), 7);

    let result = pow_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        block_number,
        nonce7,
        work7,
        "addr7",
        "addr7",
    );
    assert!(result.is_ok());
    assert_eq!(get_registrations_this_interval(&deps.storage, netuid), 8);

    let result = pow_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        block_number,
        nonce8,
        work8,
        "addr8",
        "addr8",
    );
    assert!(result.is_ok());
    assert_eq!(get_registrations_this_interval(&deps.storage, netuid), 9);

    let result = pow_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        block_number,
        nonce9,
        work9,
        "addr9",
        "addr9",
    );
    assert_eq!(
        result.unwrap_err(),
        ContractError::TooManyRegistrationsThisInterval {}
    )
}

#[test]
fn test_registration_immunity_period() { //impl this test when epoch impl and calculating pruning score is done
                                         /* TO DO */
}

#[test]
fn test_registration_already_active_hotkey() {
    let (mut deps, env) = instantiate_contract();

    let block_number: u64 = 0;
    let netuid: u16 = 2;
    let tempo: u16 = 13;
    let hotkey_account_id = "addr1";
    let coldkey_account_id = "addr667";

    //add network
    add_network(&mut deps.storage, netuid, tempo, 0);

    let (nonce, work): (u64, Vec<u8>) =
        create_work_for_block_number(&deps.storage, netuid, block_number, 0, hotkey_account_id);

    let result = pow_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        env.block.height,
        nonce,
        work,
        hotkey_account_id,
        coldkey_account_id,
    );
    assert!(result.is_ok());

    let block_number: u64 = 0;
    let hotkey_account_id = "addr1";
    let coldkey_account_id = "addr667";
    let (nonce, work): (u64, Vec<u8>) =
        create_work_for_block_number(&deps.storage, netuid, block_number, 0, hotkey_account_id);
    let result = pow_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        block_number,
        nonce,
        work,
        hotkey_account_id,
        coldkey_account_id,
    );
    assert_eq!(result.unwrap_err(), ContractError::AlreadyRegistered {})
}

#[test]
fn test_registration_invalid_seal() {
    let (mut deps, env) = instantiate_contract();

    let block_number: u64 = 1;
    let netuid: u16 = 2;
    let tempo: u16 = 13;
    let hotkey_account_id = "addr1";
    let coldkey_account_id = "addr667";

    //add network
    add_network(&mut deps.storage, netuid, tempo, 0);

    let (nonce, work): (u64, Vec<u8>) =
            // invalid seal because should be because of different block, but we use same block number
            // invalid seal created using addr2 instead of addr1
            create_work_for_block_number(
                &deps.storage,netuid, 42, 0, "addr2");

    let result = pow_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        block_number,
        nonce,
        work,
        hotkey_account_id,
        coldkey_account_id,
    );
    assert_eq!(result.unwrap_err(), ContractError::InvalidSeal {})
}

#[test]
fn test_registration_invalid_block_number() {
    let (mut deps, env) = instantiate_contract();

    let block_number: u64 = 2; // higher than current block 1
    let netuid: u16 = 2;
    let tempo: u16 = 13;
    let hotkey_account_id = "addr1";
    let coldkey_account_id = "addr667";

    //add network
    add_network(&mut deps.storage, netuid, tempo, 0);

    let (nonce, work): (u64, Vec<u8>) =
        create_work_for_block_number(&deps.storage, netuid, block_number, 0, &hotkey_account_id);

    let result = pow_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        block_number,
        nonce,
        work,
        &hotkey_account_id,
        &coldkey_account_id,
    );
    assert_eq!(result.unwrap_err(), ContractError::InvalidWorkBlock {})
}

#[test]
fn test_registration_invalid_difficulty() {
    let (mut deps, env) = instantiate_contract();

    let block_number: u64 = 0;
    let netuid: u16 = 2;
    let tempo: u16 = 13;
    let hotkey_account_id = "addr1";
    let coldkey_account_id = "addr667";

    //add network
    add_network(&mut deps.storage, netuid, tempo, 0);

    let (nonce, work): (u64, Vec<u8>) =
        create_work_for_block_number(&deps.storage, netuid, block_number, 0, hotkey_account_id);

    set_difficulty(&mut deps.storage, netuid, 18_446_744_073_709_551_615u64);

    let result = pow_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        block_number,
        nonce,
        work,
        &hotkey_account_id,
        &coldkey_account_id,
    );
    assert_eq!(result.unwrap_err(), ContractError::InvalidDifficulty {})
}

#[test]
fn test_registration_get_uid_to_prune_all_in_immunity_period() {
    let (mut deps, env) = instantiate_contract();

    let netuid: u16 = 2;
    add_network(&mut deps.storage, netuid, 0, 0);

    assert_eq!(
        register_ok_neuron(
            deps.as_mut(),
            env.clone(),
            netuid,
            "addr0",
            "addr0",
            39420842,
        )
        .is_ok(),
        true
    );
    assert_eq!(
        register_ok_neuron(
            deps.as_mut(),
            env.clone(),
            netuid,
            "addr1",
            "addr1",
            12412392,
        )
        .is_ok(),
        true
    );

    set_pruning_score_for_uid(&mut deps.storage, &deps.api, netuid, 0, 100);
    set_pruning_score_for_uid(&mut deps.storage, &deps.api, netuid, 1, 110);
    set_immunity_period(&mut deps.storage, netuid, 2);

    assert_eq!(get_pruning_score_for_uid(&deps.storage, netuid, 0), 100);
    assert_eq!(get_pruning_score_for_uid(&deps.storage, netuid, 1), 110);
    assert_eq!(get_immunity_period(&deps.storage, netuid), 2);
    assert_eq!(env.block.height, 1);
    assert_eq!(
        get_neuron_block_at_registration(&deps.storage, netuid, 0),
        1
    );
    assert_eq!(
        get_neuron_to_prune(&mut deps.storage, &deps.api, 0, env.block.height),
        0
    )
}

#[test]
fn test_registration_get_uid_to_prune_none_in_immunity_period() {
    let (mut deps, mut env) = instantiate_contract();

    let netuid: u16 = 2;
    add_network(&mut deps.storage, netuid, 0, 0);

    assert_eq!(
        register_ok_neuron(
            deps.as_mut(),
            env.clone(),
            netuid,
            "addr0",
            "addr0",
            39420842,
        )
        .is_ok(),
        true
    );
    assert_eq!(
        register_ok_neuron(
            deps.as_mut(),
            env.clone(),
            netuid,
            "addr1",
            "addr1",
            12412392,
        )
        .is_ok(),
        true
    );
    set_pruning_score_for_uid(&mut deps.storage, &deps.api, netuid, 0, 100);
    set_pruning_score_for_uid(&mut deps.storage, &deps.api, netuid, 1, 110);
    set_immunity_period(&mut deps.storage, netuid, 2);
    assert_eq!(get_pruning_score_for_uid(&deps.storage, netuid, 0), 100);
    assert_eq!(get_pruning_score_for_uid(&deps.storage, netuid, 1), 110);
    assert_eq!(get_immunity_period(&deps.storage, netuid), 2);
    assert_eq!(env.block.height, 1);
    assert_eq!(
        get_neuron_block_at_registration(&deps.storage, netuid, 0),
        1
    );
    run_step_to_block(deps.as_mut(), &mut env, 3).unwrap();

    assert_eq!(env.block.height, 3);
    assert_eq!(
        get_neuron_to_prune(&mut deps.storage, &deps.api, 0, env.block.height),
        0
    )
}

#[test]
fn test_registration_pruning() {
    let (mut deps, env) = instantiate_contract();

    let netuid: u16 = 2;
    let block_number: u64 = 0;
    let tempo: u16 = 13;
    let hotkey_account_id = "addr1";
    let coldkey_account_id = "addr667";

    //add network
    add_network(&mut deps.storage, netuid, tempo, 0);

    let (nonce0, work0): (u64, Vec<u8>) = create_work_for_block_number(
        &deps.storage,
        netuid,
        block_number,
        3942084,
        hotkey_account_id,
    );

    let result = pow_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        env.block.height,
        nonce0,
        work0,
        hotkey_account_id,
        coldkey_account_id,
    );
    assert!(result.is_ok());

    let neuron_uid =
        get_uid_for_net_and_hotkey(&deps.storage, netuid, &Addr::unchecked(hotkey_account_id))
            .unwrap();
    set_pruning_score_for_uid(&mut deps.storage, &deps.api, netuid, neuron_uid, 2);

    let hotkey_account_id1 = "addr2";
    let coldkey_account_id1 = "addr668";
    let (nonce1, work1): (u64, Vec<u8>) = create_work_for_block_number(
        &deps.storage,
        netuid,
        block_number,
        11231312312,
        hotkey_account_id1,
    );

    let result = pow_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        env.block.height,
        nonce1,
        work1,
        hotkey_account_id1,
        coldkey_account_id1,
    );
    assert!(result.is_ok());
    //
    let neuron_uid1 =
        get_uid_for_net_and_hotkey(&deps.storage, netuid, &Addr::unchecked(hotkey_account_id))
            .unwrap();
    set_pruning_score_for_uid(&mut deps.storage, &deps.api, netuid, neuron_uid1, 3);
    //
    let hotkey_account_id2 = "addr3";
    let coldkey_account_id2 = "addr669";
    let (nonce2, work2): (u64, Vec<u8>) = create_work_for_block_number(
        &deps.storage,
        netuid,
        block_number,
        212312414,
        hotkey_account_id2,
    );

    let result = pow_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        env.block.height,
        nonce2,
        work2,
        hotkey_account_id2,
        coldkey_account_id2,
    );
    assert!(result.is_ok());
}

#[test]
fn test_registration_get_neuron_metadata() {
    let (mut deps, env) = instantiate_contract();

    let netuid: u16 = 2;
    let block_number: u64 = 0;
    let tempo: u16 = 13;
    let hotkey_account_id = "addr1";
    let coldkey_account_id = "addr667";

    add_network(&mut deps.storage, netuid, tempo, 0);

    let (nonce0, work0): (u64, Vec<u8>) = create_work_for_block_number(
        &deps.storage,
        netuid,
        block_number,
        3942084,
        hotkey_account_id,
    );

    let result = pow_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        env.block.height,
        nonce0,
        work0,
        hotkey_account_id,
        coldkey_account_id,
    );
    assert!(result.is_ok());
    //
    //let neuron_id = get_uid_for_net_and_hotkey(&deps.storage, netuid, &hotkey_account_id);
    // let neuron_uid = get_uid_for_net_and_hotkey(&deps.storage,  netuid, &hotkey_account_id ).unwrap();
    let neuron: AxonInfoOf =
        get_axon_info(&deps.storage, netuid, &Addr::unchecked(hotkey_account_id));
    assert_eq!(neuron.ip, Uint128::zero());
    assert_eq!(neuron.version, 0);
    assert_eq!(neuron.port, 0)
}

#[test]
fn test_registration_add_network_size() {
    let (mut deps, env) = instantiate_contract();

    let netuid: u16 = 2;
    let netuid2: u16 = 3;
    let block_number: u64 = 0;
    let hotkey_account_id = "addr1";
    let hotkey_account_id1 = "addr2";
    let hotkey_account_id2 = "addr3";

    add_network(&mut deps.storage, netuid, 13, 0);
    assert_eq!(get_subnetwork_n(&deps.storage, netuid), 0);

    add_network(&mut deps.storage, netuid2, 13, 0);
    assert_eq!(get_subnetwork_n(&deps.storage, netuid2), 0);

    let (nonce0, work0): (u64, Vec<u8>) = create_work_for_block_number(
        &deps.storage,
        netuid,
        block_number,
        3942084,
        hotkey_account_id,
    );
    let (nonce1, work1): (u64, Vec<u8>) = create_work_for_block_number(
        &deps.storage,
        netuid2,
        block_number,
        11231312312,
        hotkey_account_id1,
    );
    let (nonce2, work2): (u64, Vec<u8>) = create_work_for_block_number(
        &deps.storage,
        netuid2,
        block_number,
        21813123,
        hotkey_account_id2,
    );
    let coldkey_account_id = "addr667";

    let result = pow_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        env.block.height,
        nonce0,
        work0,
        hotkey_account_id,
        coldkey_account_id,
    );
    assert!(result.is_ok());
    assert_eq!(get_subnetwork_n(&deps.storage, netuid), 1);
    assert_eq!(get_registrations_this_interval(&deps.storage, netuid), 1);

    let result = pow_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid2,
        env.block.height,
        nonce1,
        work1,
        hotkey_account_id1,
        coldkey_account_id,
    );
    assert!(result.is_ok());
    let result = pow_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid2,
        env.block.height,
        nonce2,
        work2,
        hotkey_account_id2,
        coldkey_account_id,
    );
    assert!(result.is_ok());
    assert_eq!(get_subnetwork_n(&deps.storage, netuid2), 2);
    assert_eq!(get_registrations_this_interval(&deps.storage, netuid2), 2)
}

#[test]
fn test_burn_registration_increase_recycled_rao() {
    let (mut deps, mut env) = instantiate_contract();

    let netuid: u16 = 2;
    let netuid2: u16 = 3;

    let hotkey_account_id = "addr1";
    let _coldkey_account_id = "addr667";

    // Give funds for burn. 1000 TAO
    // let _ = Balances::deposit_creating(
    //     &coldkey_account_id,
    //     Balance::from(1_000_000_000_000 as u64),
    // );

    add_network(&mut deps.storage, netuid, 13, 0);
    assert_eq!(get_subnetwork_n(&deps.storage, netuid), 0);

    add_network(&mut deps.storage, netuid2, 13, 0);
    assert_eq!(get_subnetwork_n(&deps.storage, netuid2), 0);

    step_block(deps.as_mut(), &mut env).unwrap();

    let burn_amount = get_burn_as_u64(&deps.storage, netuid);
    assert_eq!(
        burned_register_ok_neuron(
            deps.as_mut(),
            env.clone(),
            netuid,
            hotkey_account_id,
            hotkey_account_id,
        )
        .is_ok(),
        true
    );
    assert_eq!(get_rao_recycled(&deps.storage, netuid), burn_amount);

    step_block(deps.as_mut(), &mut env).unwrap();

    let burn_amount2 = get_burn_as_u64(&deps.storage, netuid2);
    assert_eq!(
        burned_register_ok_neuron(
            deps.as_mut(),
            env.clone(),
            netuid2,
            hotkey_account_id,
            hotkey_account_id,
        )
        .is_ok(),
        true
    );
    assert_eq!(
        burned_register_ok_neuron(deps.as_mut(), env.clone(), netuid2, "addr2", "addr2").is_ok(),
        true
    );
    assert_eq!(get_rao_recycled(&deps.storage, netuid2), burn_amount2 * 2);
    // Validate netuid is not affected.
    assert_eq!(get_rao_recycled(&deps.storage, netuid), burn_amount)
}

#[test]
fn test_full_pass_through() {
    let (mut deps, env) = instantiate_contract();

    // Create 3 networks.
    let netuid0: u16 = 2;
    let netuid1: u16 = 3;
    let netuid2: u16 = 4;

    // With 3 tempos
    let tempo0: u16 = 2;
    let tempo1: u16 = 2;
    let tempo2: u16 = 2;

    // Create 3 keys.
    let hotkey0 = "addr0";
    let hotkey1 = "addr1";
    let hotkey2 = "addr2";

    // With 3 different coldkeys.
    let coldkey0 = "addr0";
    let coldkey1 = "addr1";
    let coldkey2 = "addr2";

    // Add the 3 networks.
    add_network(&mut deps.storage, netuid0, tempo0, 0);

    add_network(&mut deps.storage, netuid1, tempo1, 0);

    add_network(&mut deps.storage, netuid2, tempo2, 0);

    // Check their tempo.
    assert_eq!(get_tempo(&deps.storage, netuid0), tempo0);
    assert_eq!(get_tempo(&deps.storage, netuid1), tempo1);
    assert_eq!(get_tempo(&deps.storage, netuid2), tempo2);

    // Check their emission value.
    assert_eq!(get_emission_value(&deps.storage, netuid0), 0);
    assert_eq!(get_emission_value(&deps.storage, netuid1), 0);
    assert_eq!(get_emission_value(&deps.storage, netuid2), 0);

    // Set their max allowed uids.
    set_max_allowed_uids(&mut deps.storage, netuid0, 2);
    set_max_allowed_uids(&mut deps.storage, netuid1, 2);
    set_max_allowed_uids(&mut deps.storage, netuid2, 2);

    // Check their max allowed.
    assert_eq!(get_max_allowed_uids(&mut deps.storage, netuid0), 2);
    assert_eq!(get_max_allowed_uids(&mut deps.storage, netuid0), 2);
    assert_eq!(get_max_allowed_uids(&mut deps.storage, netuid0), 2);

    // Set the max registration per block.
    set_max_registrations_per_block(&mut deps.storage, netuid0, 3);
    set_max_registrations_per_block(&mut deps.storage, netuid1, 3);
    set_max_registrations_per_block(&mut deps.storage, netuid2, 3);
    assert_eq!(get_max_registrations_per_block(&deps.storage, netuid0), 3);
    assert_eq!(get_max_registrations_per_block(&deps.storage, netuid1), 3);
    assert_eq!(get_max_registrations_per_block(&deps.storage, netuid2), 3);

    // Check that no one has registered yet.
    assert_eq!(get_subnetwork_n(&deps.storage, netuid0), 0);
    assert_eq!(get_subnetwork_n(&deps.storage, netuid1), 0);
    assert_eq!(get_subnetwork_n(&deps.storage, netuid2), 0);

    // Registered the keys to all networks.
    assert_eq!(
        register_ok_neuron(
            deps.as_mut(),
            env.clone(),
            netuid0,
            hotkey0,
            coldkey0,
            39420842,
        )
        .is_ok(),
        true
    );
    assert_eq!(
        register_ok_neuron(
            deps.as_mut(),
            env.clone(),
            netuid0,
            hotkey1,
            coldkey1,
            12412392,
        )
        .is_ok(),
        true
    );
    assert_eq!(
        register_ok_neuron(
            deps.as_mut(),
            env.clone(),
            netuid1,
            hotkey0,
            coldkey0,
            21813123,
        )
        .is_ok(),
        true
    );
    assert_eq!(
        register_ok_neuron(
            deps.as_mut(),
            env.clone(),
            netuid1,
            hotkey1,
            coldkey1,
            25755207,
        )
        .is_ok(),
        true
    );
    assert_eq!(
        register_ok_neuron(
            deps.as_mut(),
            env.clone(),
            netuid2,
            hotkey0,
            coldkey0,
            251232207,
        )
        .is_ok(),
        true
    );
    assert_eq!(
        register_ok_neuron(
            deps.as_mut(),
            env.clone(),
            netuid2,
            hotkey1,
            coldkey1,
            159184122,
        )
        .is_ok(),
        true
    );

    // Check uids.
    // n0 [ h0, h1 ]
    // n1 [ h0, h1 ]
    // n2 [ h0, h1 ]
    assert_eq!(
        get_hotkey_for_net_and_uid(&deps.storage, netuid0, 0).unwrap(),
        hotkey0
    );
    assert_eq!(
        get_hotkey_for_net_and_uid(&deps.storage, netuid1, 0).unwrap(),
        hotkey0
    );
    assert_eq!(
        get_hotkey_for_net_and_uid(&deps.storage, netuid2, 0).unwrap(),
        hotkey0
    );
    assert_eq!(
        get_hotkey_for_net_and_uid(&deps.storage, netuid0, 1).unwrap(),
        hotkey1
    );
    assert_eq!(
        get_hotkey_for_net_and_uid(&deps.storage, netuid1, 1).unwrap(),
        hotkey1
    );
    assert_eq!(
        get_hotkey_for_net_and_uid(&deps.storage, netuid2, 1).unwrap(),
        hotkey1
    );

    // Check registered networks.
    // assert!( get_registered_networks_for_hotkey( &hotkey0 ).contains( &netuid0 ) );
    // assert!( get_registered_networks_for_hotkey( &hotkey0 ).contains( &netuid1 ) );
    // assert!( get_registered_networks_for_hotkey( &hotkey0 ).contains( &netuid2 ) );
    // assert!( get_registered_networks_for_hotkey( &hotkey1 ).contains( &netuid0 ) );
    // assert!( get_registered_networks_for_hotkey( &hotkey1 ).contains( &netuid1 ) );
    // assert!( get_registered_networks_for_hotkey( &hotkey1 ).contains( &netuid2 ) );
    // assert!( !get_registered_networks_for_hotkey( &hotkey2 ).contains( &netuid0 ) );
    // assert!( !get_registered_networks_for_hotkey( &hotkey2 ).contains( &netuid1 ) );
    // assert!( !get_registered_networks_for_hotkey( &hotkey2 ).contains( &netuid2 ) );

    // Check the number of registrations.
    assert_eq!(get_registrations_this_interval(&deps.storage, netuid0), 2);
    assert_eq!(get_registrations_this_interval(&deps.storage, netuid1), 2);
    assert_eq!(get_registrations_this_interval(&deps.storage, netuid2), 2);

    // Get the number of uids in each network.
    assert_eq!(get_subnetwork_n(&deps.storage, netuid0), 2);
    assert_eq!(get_subnetwork_n(&deps.storage, netuid1), 2);
    assert_eq!(get_subnetwork_n(&deps.storage, netuid2), 2);

    // Check the uids exist.
    assert!(is_uid_exist_on_network(&deps.storage, netuid0, 0));
    assert!(is_uid_exist_on_network(&deps.storage, netuid1, 0));
    assert!(is_uid_exist_on_network(&deps.storage, netuid2, 0));

    // Check the other exists.
    assert!(is_uid_exist_on_network(&deps.storage, netuid0, 1));
    assert!(is_uid_exist_on_network(&deps.storage, netuid1, 1));
    assert!(is_uid_exist_on_network(&deps.storage, netuid2, 1));

    // Get the hotkey under each uid.
    assert_eq!(
        get_hotkey_for_net_and_uid(&deps.storage, netuid0, 0).unwrap(),
        hotkey0
    );
    assert_eq!(
        get_hotkey_for_net_and_uid(&deps.storage, netuid1, 0).unwrap(),
        hotkey0
    );
    assert_eq!(
        get_hotkey_for_net_and_uid(&deps.storage, netuid2, 0).unwrap(),
        hotkey0
    );

    // Get the hotkey under the other uid.
    assert_eq!(
        get_hotkey_for_net_and_uid(&deps.storage, netuid0, 1).unwrap(),
        hotkey1
    );
    assert_eq!(
        get_hotkey_for_net_and_uid(&deps.storage, netuid1, 1).unwrap(),
        hotkey1
    );
    assert_eq!(
        get_hotkey_for_net_and_uid(&deps.storage, netuid2, 1).unwrap(),
        hotkey1
    );

    // Check for replacement.
    assert_eq!(get_subnetwork_n(&deps.storage, netuid0), 2);
    assert_eq!(get_subnetwork_n(&deps.storage, netuid1), 2);
    assert_eq!(get_subnetwork_n(&deps.storage, netuid2), 2);

    // Register the 3rd hotkey.
    assert_eq!(
        register_ok_neuron(
            deps.as_mut(),
            env.clone(),
            netuid0,
            hotkey2,
            coldkey2,
            59420842,
        )
        .is_ok(),
        true
    );
    assert_eq!(
        register_ok_neuron(
            deps.as_mut(),
            env.clone(),
            netuid1,
            hotkey2,
            coldkey2,
            31813123,
        )
        .is_ok(),
        true
    );
    assert_eq!(
        register_ok_neuron(
            deps.as_mut(),
            env.clone(),
            netuid2,
            hotkey2,
            coldkey2,
            451232207,
        )
        .is_ok(),
        true
    );

    // Check for replacement.
    assert_eq!(get_subnetwork_n(&deps.storage, netuid0), 2);
    assert_eq!(get_subnetwork_n(&deps.storage, netuid1), 2);
    assert_eq!(get_subnetwork_n(&deps.storage, netuid2), 2);

    // Check uids.
    // n0 [ h0, h1 ]
    // n1 [ h0, h1 ]
    // n2 [ h0, h1 ]
    assert_eq!(
        get_hotkey_for_net_and_uid(&deps.storage, netuid0, 0).unwrap(),
        hotkey2
    );
    assert_eq!(
        get_hotkey_for_net_and_uid(&deps.storage, netuid1, 0).unwrap(),
        hotkey2
    );
    assert_eq!(
        get_hotkey_for_net_and_uid(&deps.storage, netuid2, 0).unwrap(),
        hotkey2
    );
    assert_eq!(
        get_hotkey_for_net_and_uid(&deps.storage, netuid0, 1).unwrap(),
        hotkey1
    );
    assert_eq!(
        get_hotkey_for_net_and_uid(&deps.storage, netuid1, 1).unwrap(),
        hotkey1
    );
    assert_eq!(
        get_hotkey_for_net_and_uid(&deps.storage, netuid2, 1).unwrap(),
        hotkey1
    );

    // Check registered networks.
    // hotkey0 has been deregistered.
    // assert!( !get_registered_networks_for_hotkey( &hotkey0 ).contains( &netuid0 ) );
    // assert!( !get_registered_networks_for_hotkey( &hotkey0 ).contains( &netuid1 ) );
    // assert!( !get_registered_networks_for_hotkey( &hotkey0 ).contains( &netuid2 ) );
    // assert!( get_registered_networks_for_hotkey( &hotkey1 ).contains( &netuid0 ) );
    // assert!( get_registered_networks_for_hotkey( &hotkey1 ).contains( &netuid1 ) );
    // assert!( get_registered_networks_for_hotkey( &hotkey1 ).contains( &netuid2 ) );
    // assert!( get_registered_networks_for_hotkey( &hotkey2 ).contains( &netuid0 ) );
    // assert!( get_registered_networks_for_hotkey( &hotkey2 ).contains( &netuid1 ) );
    // assert!( get_registered_networks_for_hotkey( &hotkey2 ).contains( &netuid2 ) );

    // Check the registration counters.
    assert_eq!(get_registrations_this_interval(&deps.storage, netuid0), 3);
    assert_eq!(get_registrations_this_interval(&deps.storage, netuid1), 3);
    assert_eq!(get_registrations_this_interval(&deps.storage, netuid2), 3);

    // Check the hotkeys are expected.
    assert_eq!(
        get_hotkey_for_net_and_uid(&deps.storage, netuid0, 0).unwrap(),
        hotkey2
    );
    assert_eq!(
        get_hotkey_for_net_and_uid(&deps.storage, netuid1, 0).unwrap(),
        hotkey2
    );
    assert_eq!(
        get_hotkey_for_net_and_uid(&deps.storage, netuid2, 0).unwrap(),
        hotkey2
    )
}

#[test]
fn test_registration_origin_hotkey_mismatch() {
    let (mut deps, env) = instantiate_contract();

    let block_number: u64 = 0;
    let netuid: u16 = 2;
    let tempo: u16 = 13;
    let hotkey_account_id_1 = "addr1";
    let hotkey_account_id_2 = "addr2";
    let coldkey_account_id = "addr668";

    //add network
    add_network(&mut deps.storage, netuid, tempo, 0);

    let (nonce, work): (u64, Vec<u8>) =
        create_work_for_block_number(&deps.storage, netuid, block_number, 0, hotkey_account_id_1);

    let msg = ExecuteMsg::Register {
        netuid,
        block_number,
        nonce,
        work,
        hotkey: hotkey_account_id_2.to_string(),
        coldkey: coldkey_account_id.to_string(),
    };

    let info = mock_info(hotkey_account_id_1, &[]);
    let result = execute(deps.as_mut(), env.clone(), info, msg);
    assert_eq!(result.unwrap_err(), ContractError::HotkeyOriginMismatch {})
}

#[test]
fn test_registration_disabled() {
    let (mut deps, env) = instantiate_contract();

    let block_number: u64 = 0;
    let netuid: u16 = 2;
    let tempo: u16 = 13;
    let hotkey_account_id = "addr1";
    let coldkey_account_id = "addr668";

    //add network
    add_network(&mut deps.storage, netuid, tempo, 0);
    set_network_registration_allowed(&mut deps.storage, netuid, false);

    let (nonce, work): (u64, Vec<u8>) =
        create_work_for_block_number(&deps.storage, netuid, block_number, 0, hotkey_account_id);

    let result = pow_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        block_number,
        nonce,
        work.clone(),
        hotkey_account_id,
        coldkey_account_id,
    );
    assert_eq!(result.unwrap_err(), ContractError::RegistrationDisabled {})
}
