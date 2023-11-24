use cosmwasm_std::Addr;

use crate::block_step::blocks_until_next_epoch;
use crate::registration::create_work_for_block_number;
use crate::root::{
    get_all_subnet_netuids, get_max_subnets, get_network_lock_cost, get_num_subnets,
    get_subnet_emission_value, if_subnet_exist, remove_network, root_epoch,
    set_lock_reduction_interval, set_network_last_lock,
};
use crate::staking::{add_balance_to_coldkey_account, hotkey_is_delegate};
use crate::test_helpers::{
    add_network, add_stake, burned_register_ok_neuron, instantiate_contract,
    pow_register_ok_neuron, register_network, root_register, set_weights, step_block,
};
use crate::uids::{get_subnetwork_n, get_uid_for_net_and_hotkey, is_hotkey_registered_on_network};
use crate::utils::{
    get_pending_emission, get_total_issuance, set_burn, set_difficulty, set_max_allowed_uids,
    set_max_registrations_per_block, set_target_registrations_per_interval, set_tempo,
    set_weights_set_rate_limit,
};
use crate::ContractError;

#[test]
fn test_root_register_network_exist() {
    let (mut deps, mut env) = instantiate_contract();

    let hotkey_account_id = "addr1";
    let coldkey_account_id = "addr667";

    assert_eq!(
        root_register(
            deps.as_mut(),
            env.clone(),
            hotkey_account_id,
            coldkey_account_id
        )
        .is_ok(),
        true
    );
}

#[test]
fn test_root_register_normal_on_root_fails() {
    let (mut deps, mut env) = instantiate_contract();

    // Test fails because normal registrations are not allowed
    // on the root network.
    let root_netuid: u16 = 0;
    let hotkey_account_id = "addr1";
    let coldkey_account_id = "addr667";

    // Burn registration fails.
    set_burn(&mut deps.storage, root_netuid, 1000);
    set_difficulty(&mut deps.storage, root_netuid, 0);
    add_balance_to_coldkey_account(&Addr::unchecked(coldkey_account_id), 1);
    assert_eq!(
        burned_register_ok_neuron(
            deps.as_mut(),
            env.clone(),
            root_netuid,
            hotkey_account_id,
            coldkey_account_id,
        ),
        Err(ContractError::OperationNotPermittedOnRootSubnet {})
    );
    // Pow registration fails.
    let (nonce, work): (u64, Vec<u8>) = create_work_for_block_number(
        &mut deps.storage,
        root_netuid,
        env.block.height,
        0,
        hotkey_account_id,
    );
    assert_eq!(
        pow_register_ok_neuron(
            deps.as_mut(),
            env.clone(),
            root_netuid,
            env.block.height,
            nonce,
            work,
            hotkey_account_id,
            coldkey_account_id,
        ),
        Err(ContractError::OperationNotPermittedOnRootSubnet {})
    );
}

#[test]
fn test_root_register_stake_based_pruning_works() {
    let (mut deps, mut env) = instantiate_contract();

    // Add two networks.
    let root_netuid: u16 = 0;
    let other_netuid: u16 = 1;
    remove_network(&mut deps.storage, 1).unwrap(); // delete after contract creation network
    add_network(&mut deps.storage, other_netuid, 0, 0);

    // Set params to allow all registrations to subnet.
    set_burn(&mut deps.storage, other_netuid, 0);
    set_max_registrations_per_block(&mut deps.storage, other_netuid, 256);
    set_target_registrations_per_interval(&mut deps.storage, other_netuid, 256);

    set_max_registrations_per_block(&mut deps.storage, root_netuid, 1000);
    set_target_registrations_per_interval(&mut deps.storage, root_netuid, 1000);

    // Register 128 accounts with stake to the other network.
    for i in 0..128 {
        let hot = (1000 + i).to_string();
        let cold = (1000 + i).to_string();
        // Add balance
        add_balance_to_coldkey_account(&Addr::unchecked(cold.clone()), 1000 + (i as u64));
        // Register
        assert_eq!(
            burned_register_ok_neuron(
                deps.as_mut(),
                env.clone(),
                other_netuid,
                hot.as_str(),
                cold.as_str(),
            )
            .is_ok(),
            true
        );
        // Add stake on other network
        assert_eq!(
            add_stake(
                deps.as_mut(),
                env.clone(),
                hot.as_str(),
                cold.as_str(),
                1000 + (i as u64)
            )
            .is_ok(),
            true
        );

        // Check succesfull registration.
        assert!(get_uid_for_net_and_hotkey(
            &deps.storage,
            other_netuid,
            &Addr::unchecked(hot.clone())
        )
        .is_ok());
        // Check that they are NOT all delegates
        assert!(!hotkey_is_delegate(&deps.storage, &Addr::unchecked(hot)));
    }

    // Register the first 64 accounts with stake to the root network.
    for i in 0..64 {
        let hot = (1000 + i).to_string();
        let cold = (1000 + i).to_string();
        assert_eq!(
            root_register(deps.as_mut(), env.clone(), hot.as_str(), cold.as_str()).is_ok(),
            true
        );
        // Check succesfull registration.
        assert!(get_uid_for_net_and_hotkey(
            &deps.storage,
            root_netuid,
            &Addr::unchecked(hot.clone())
        )
        .is_ok());
        // Check that they are all delegates
        assert!(hotkey_is_delegate(
            &deps.storage,
            &Addr::unchecked(hot.clone())
        ));
    }

    // Register the second 64 accounts with stake to the root network.
    // Replaces the first 64
    for i in 64..128 {
        let hot = (1000 + i).to_string();
        let cold = (1000 + i).to_string();
        assert_eq!(
            root_register(deps.as_mut(), env.clone(), hot.as_str(), cold.as_str()).is_ok(),
            true
        );
        // Check succesfull registration.
        assert!(get_uid_for_net_and_hotkey(
            &deps.storage,
            root_netuid,
            &Addr::unchecked(hot.clone())
        )
        .is_ok());
    }

    // Register the first 64 accounts again, this time failing because they
    // dont have enough stake.
    for i in 0..64 {
        let hot = (1000 + i).to_string();
        let cold = (1000 + i).to_string();
        assert_eq!(
            root_register(deps.as_mut(), env.clone(), hot.as_str(), cold.as_str()),
            Err(ContractError::StakeTooLowForRoot {})
        );
        // Check for unsuccesfull registration.
        assert!(!get_uid_for_net_and_hotkey(
            &deps.storage,
            root_netuid,
            &Addr::unchecked(hot.clone())
        )
        .is_ok());
        // Check that they are NOT senate members
        // assert!(!is_senate_member(&hot));
    }
}

#[test]
fn test_root_set_weights() {
    let (mut deps, mut env) = instantiate_contract();

    let n: usize = 10;
    let root_netuid: u16 = 0;
    set_max_registrations_per_block(&mut deps.storage, root_netuid, n as u16);
    set_target_registrations_per_interval(&mut deps.storage, root_netuid, n as u16);
    set_max_allowed_uids(&mut deps.storage, root_netuid, n as u16);

    remove_network(&mut deps.storage, 1).unwrap();
    set_weights_set_rate_limit(&mut deps.storage, 0, 0);

    for i in 0..n {
        let hotkey_account_id = (1000 + i).to_string();
        let coldkey_account_id = (1000 + i).to_string();
        add_balance_to_coldkey_account(
            &Addr::unchecked(coldkey_account_id.clone()),
            1_000_000_000_000_000,
        );
        assert_eq!(
            root_register(
                deps.as_mut(),
                env.clone(),
                hotkey_account_id.as_str(),
                coldkey_account_id.as_str()
            )
            .is_ok(),
            true
        );
        assert_eq!(
            add_stake(
                deps.as_mut(),
                env.clone(),
                hotkey_account_id.as_str(),
                coldkey_account_id.as_str(),
                1000
            )
            .is_ok(),
            true
        );
    }

    println!("subnet limit: {:?}", get_max_subnets(&deps.storage));
    println!("current subnet count: {:?}", get_num_subnets(&deps.storage));

    // Lets create n networks
    for netuid in 1..n {
        println!("Adding network with netuid: {}", netuid);
        assert_eq!(
            register_network(
                deps.as_mut(),
                env.clone(),
                (1000 + netuid).to_string().as_str()
            )
            .is_ok(),
            true
        );
    }

    // Set weights into diagonal matrix.
    for i in 0..n {
        let uids: Vec<u16> = vec![i as u16];
        let values: Vec<u16> = vec![1];
        // assert_eq!(set_weights(
        //     deps.as_mut(),
        //     env.clone(),
        //     (1000 + i).to_string().as_str(),
        //     root_netuid,
        //     uids,
        //     values,
        //     0,
        // ).is_ok(), true);
        let res = set_weights(
            deps.as_mut(),
            env.clone(),
            (1000 + i).to_string().as_str(),
            root_netuid,
            uids,
            values,
            0,
        );
        println!("set_weights: {:?}", res);
    }
    // Run the root epoch
    println!("Running Root epoch");
    set_tempo(&mut deps.storage, root_netuid, 1);
    assert_eq!(
        root_epoch(&mut deps.storage, &deps.api, 1_000_000_001).is_ok(),
        true
    );
    // Check that the emission values have been set.
    for netuid in 1..n {
        println!("check emission for netuid: {}", netuid);
        assert_eq!(
            get_subnet_emission_value(&deps.storage, netuid as u16),
            99_999_999
        );
    }

    step_block(deps.as_mut(), &mut env).unwrap();
    step_block(deps.as_mut(), &mut env).unwrap();

    // Check that the pending emission values have been set.
    for netuid in 1..n {
        println!(
            "check pending emission for netuid {} has pending {}",
            netuid,
            get_pending_emission(&deps.storage, netuid as u16)
        );
        assert_eq!(
            get_pending_emission(&deps.storage, netuid as u16),
            199_999_998
        );
    }
    step_block(deps.as_mut(), &mut env).unwrap();
    for netuid in 1..n {
        println!(
            "check pending emission for netuid {} has pending {}",
            netuid,
            get_pending_emission(&deps.storage, netuid as u16)
        );
        assert_eq!(
            get_pending_emission(&deps.storage, netuid as u16),
            299_999_997
        );
    }
    let step = blocks_until_next_epoch(9, 1000, env.block.height);

    // println!("step: {}", step);
    // step_block(step as u16);
    // let block = env.block.height;
    // run_step_to_block(deps.as_mut(), &mut env, block + step).unwrap();
    // assert_eq!(run_step_to_block(
    //     deps.as_mut(),
    //     &mut env,
    //     block + step
    // ).is_ok(), true);

    // TODO back to this
    // assert_eq!(get_pending_emission(&deps.storage, 9), 0);
}

// TODO FAILED
#[test]
fn test_root_set_weights_out_of_order_netuids() {
    let (mut deps, mut env) = instantiate_contract();

    let n: usize = 10;
    let root_netuid: u16 = 0;
    set_max_registrations_per_block(&mut deps.storage, root_netuid, n as u16);
    set_target_registrations_per_interval(&mut deps.storage, root_netuid, n as u16);
    set_max_allowed_uids(&mut deps.storage, root_netuid, n as u16);

    remove_network(&mut deps.storage, 1).unwrap();
    set_weights_set_rate_limit(&mut deps.storage, 0, 0);

    for i in 0..n {
        let hotkey_account_id = (1000 + i).to_string();
        let coldkey_account_id = (1000 + i).to_string();
        add_balance_to_coldkey_account(
            &Addr::unchecked(coldkey_account_id.clone()),
            1_000_000_000_000_000,
        );
        assert_eq!(
            root_register(
                deps.as_mut(),
                env.clone(),
                hotkey_account_id.as_str(),
                coldkey_account_id.as_str()
            )
            .is_ok(),
            true
        );
        assert_eq!(
            add_stake(
                deps.as_mut(),
                env.clone(),
                hotkey_account_id.as_str(),
                coldkey_account_id.as_str(),
                1000
            )
            .is_ok(),
            true
        );
    }

    println!("subnet limit: {:?}", get_max_subnets(&deps.storage));
    println!("current subnet count: {:?}", get_num_subnets(&deps.storage));

    // Lets create n networks
    for netuid in 1..n {
        println!("Adding network with netuid: {}", netuid);

        if netuid % 2 == 0 {
            assert_eq!(
                register_network(
                    deps.as_mut(),
                    env.clone(),
                    (1000 + netuid).to_string().as_str()
                )
                .is_ok(),
                true
            );
        } else {
            add_network(&mut deps.storage, netuid as u16 * 10, 1000, 0)
        }
    }

    println!("netuids: {:?}", get_all_subnet_netuids(&deps.storage));
    println!(
        "root network count: {:?}",
        get_subnetwork_n(&deps.storage, 0)
    );

    let subnets = get_all_subnet_netuids(&deps.storage);
    // Set weights into diagonal matrix.
    for (i, netuid) in subnets.iter().enumerate() {
        let uids: Vec<u16> = vec![*netuid];

        let values: Vec<u16> = vec![1];
        // assert_eq!(set_weights(
        //     deps.as_mut(),
        //     env.clone(),
        //     (1000 + i).to_string().as_str(),
        //     root_netuid,
        //     uids,
        //     values,
        //     0,
        // ).is_ok(), true);
        let res = set_weights(
            deps.as_mut(),
            env.clone(),
            (1000 + i).to_string().as_str(),
            root_netuid,
            uids,
            values,
            0,
        );
        println!("set_weights: {:?}", res);
    }
    // Run the root epoch
    println!("Running Root epoch");
    set_tempo(&mut deps.storage, root_netuid, 1);

    assert_eq!(
        root_epoch(&mut deps.storage, &deps.api, 1_000_000_001).is_ok(),
        true
    );
    // Check that the emission values have been set.
    for netuid in subnets.iter() {
        println!("check emission for netuid: {}", netuid);
        assert_eq!(
            get_subnet_emission_value(&deps.storage, *netuid),
            99_999_999
        );
    }
    step_block(deps.as_mut(), &mut env).unwrap();
    step_block(deps.as_mut(), &mut env).unwrap();

    // Check that the pending emission values have been set.
    for netuid in subnets.iter() {
        if *netuid == 0 {
            continue;
        }

        println!(
            "check pending emission for netuid {} has pending {}",
            netuid,
            get_pending_emission(&deps.storage, *netuid)
        );
        assert_eq!(get_pending_emission(&deps.storage, *netuid), 199_999_998);
    }
    step_block(deps.as_mut(), &mut env).unwrap();
    for netuid in subnets.iter() {
        if *netuid == 0 {
            continue;
        }

        println!(
            "check pending emission for netuid {} has pending {}",
            netuid,
            get_pending_emission(&deps.storage, *netuid)
        );
        assert_eq!(get_pending_emission(&deps.storage, *netuid), 299_999_997);
    }
    let step = blocks_until_next_epoch(9, 1000, env.block.height);
    // step_block(step as u16);
    // let block = env.block.height;
    // assert_eq!(run_step_to_block(
    //     deps.as_mut(),
    //     &mut env,
    //     block + step
    // ).is_ok(), true);

    // TODO back to this
    // assert_eq!(get_pending_emission(&deps.storage, 9), 0);
}

// TODO FAILED, check logic of network lock cost and update original asserts
#[test]
fn test_root_subnet_creation_deletion() {
    let (mut deps, mut env) = instantiate_contract();

    // Owner of subnets.
    let owner = "addr0";
    remove_network(&mut deps.storage, 1).unwrap();
    // step_block(deps.as_mut(), &mut env).unwrap();
    set_lock_reduction_interval(&mut deps.storage, 2);

    // Add a subnet.
    add_balance_to_coldkey_account(&Addr::unchecked(owner), 1_000_000_000_000_000);
    // last_lock: 100000000000, min_lock: 100000000000, last_lock_block: 0, lock_reduction_interval: 2, current_block: 0, mult: 1 lock_cost: 100000000000
    assert_eq!(
        register_network(deps.as_mut(), env.clone(), owner).is_ok(),
        true
    );
    // // last_lock: 100000000000, min_lock: 100000000000, last_lock_block: 0, lock_reduction_interval: 2, current_block: 0, mult: 1 lock_cost: 100000000000
    // assert_eq!(
    //     get_network_lock_cost(&deps.storage, &deps.api, env.block.height).unwrap(),
    //     100_000_000_000
    // );
    //
    // step_block(deps.as_mut(), &mut env).unwrap();
    // // last_lock: 100000000000, min_lock: 100000000000, last_lock_block: 0, lock_reduction_interval: 2, current_block: 1, mult: 1 lock_cost: 100000000000
    // assert_eq!(register_network(deps.as_mut(), env.clone(), owner).is_ok(), true);
    // // last_lock: 100000000000, min_lock: 100000000000, last_lock_block: 1, lock_reduction_interval: 2, current_block: 1, mult: 2 lock_cost: 200000000000
    assert_eq!(
        get_network_lock_cost(&deps.storage, &deps.api, env.block.height).unwrap(),
        200_000_000_000
    ); // Doubles from previous subnet creation

    step_block(deps.as_mut(), &mut env).unwrap();
    // last_lock: 100000000000, min_lock: 100000000000, last_lock_block: 1, lock_reduction_interval: 2, current_block: 2, mult: 2 lock_cost: 150000000000
    assert_eq!(
        get_network_lock_cost(&deps.storage, &deps.api, env.block.height).unwrap(),
        150_000_000_000
    ); // Reduced by 50%

    step_block(deps.as_mut(), &mut env).unwrap();
    // last_lock: 100000000000, min_lock: 100000000000, last_lock_block: 1, lock_reduction_interval: 2, current_block: 3, mult: 2 lock_cost: 100000000000
    assert_eq!(
        get_network_lock_cost(&deps.storage, &deps.api, env.block.height).unwrap(),
        100_000_000_000
    ); // Reduced another 50%

    step_block(deps.as_mut(), &mut env).unwrap();
    // last_lock: 100000000000, min_lock: 100000000000, last_lock_block: 1, lock_reduction_interval: 2, current_block: 4, mult: 2 lock_cost: 100000000000
    assert_eq!(
        get_network_lock_cost(&deps.storage, &deps.api, env.block.height).unwrap(),
        100_000_000_000
    ); // Reaches min value
    assert_eq!(
        register_network(deps.as_mut(), env.clone(), owner).is_ok(),
        true
    );
    // last_lock: 100000000000, min_lock: 100000000000, last_lock_block: 4, lock_reduction_interval: 2, current_block: 4, mult: 2 lock_cost: 200000000000
    assert_eq!(
        get_network_lock_cost(&deps.storage, &deps.api, env.block.height).unwrap(),
        200_000_000_000
    ); // Doubles from previous subnet creation

    step_block(deps.as_mut(), &mut env).unwrap();
    // last_lock: 100000000000, min_lock: 100000000000, last_lock_block: 4, lock_reduction_interval: 2, current_block: 5, mult: 2 lock_cost: 150000000000
    assert_eq!(
        register_network(deps.as_mut(), env.clone(), owner).is_ok(),
        true
    );
    // last_lock: 150000000000, min_lock: 100000000000, last_lock_block: 5, lock_reduction_interval: 2, current_block: 5, mult: 2 lock_cost: 300000000000
    assert_eq!(
        get_network_lock_cost(&deps.storage, &deps.api, env.block.height).unwrap(),
        300_000_000_000
    ); // Doubles from previous subnet creation

    step_block(deps.as_mut(), &mut env).unwrap();
    // last_lock: 150000000000, min_lock: 100000000000, last_lock_block: 5, lock_reduction_interval: 2, current_block: 6, mult: 2 lock_cost: 225000000000
    assert_eq!(
        register_network(deps.as_mut(), env.clone(), owner).is_ok(),
        true
    );
    // last_lock: 225000000000, min_lock: 100000000000, last_lock_block: 6, lock_reduction_interval: 2, current_block: 6, mult: 2 lock_cost: 450000000000
    assert_eq!(
        get_network_lock_cost(&deps.storage, &deps.api, env.block.height).unwrap(),
        450_000_000_000
    ); // Increasing

    step_block(deps.as_mut(), &mut env).unwrap();
    // last_lock: 225000000000, min_lock: 100000000000, last_lock_block: 6, lock_reduction_interval: 2, current_block: 7, mult: 2 lock_cost: 337500000000
    assert_eq!(
        register_network(deps.as_mut(), env.clone(), owner).is_ok(),
        true
    );
    // last_lock: 337500000000, min_lock: 100000000000, last_lock_block: 7, lock_reduction_interval: 2, current_block: 7, mult: 2 lock_cost: 675000000000
    assert_eq!(
        get_network_lock_cost(&deps.storage, &deps.api, env.block.height).unwrap(),
        675_000_000_000
    ); // Increasing.
    assert_eq!(
        register_network(deps.as_mut(), env.clone(), owner).is_ok(),
        true
    );
    // last_lock: 337500000000, min_lock: 100000000000, last_lock_block: 7, lock_reduction_interval: 2, current_block: 7, mult: 2 lock_cost: 675000000000
    assert_eq!(
        get_network_lock_cost(&deps.storage, &deps.api, env.block.height).unwrap(),
        1_350_000_000_000
    ); // Double increasing.
    assert_eq!(
        register_network(deps.as_mut(), env.clone(), owner).is_ok(),
        true
    );
    assert_eq!(
        get_network_lock_cost(&deps.storage, &deps.api, env.block.height).unwrap(),
        2_700_000_000_000
    ); // Double increasing again.

    // Now drop it like its hot to min again.
    step_block(deps.as_mut(), &mut env).unwrap();
    assert_eq!(
        get_network_lock_cost(&deps.storage, &deps.api, env.block.height).unwrap(),
        2_025_000_000_000
    ); // 675_000_000_000 decreasing.

    step_block(deps.as_mut(), &mut env).unwrap();
    assert_eq!(
        get_network_lock_cost(&deps.storage, &deps.api, env.block.height).unwrap(),
        1_350_000_000_000
    ); // 675_000_000_000 decreasing.

    step_block(deps.as_mut(), &mut env).unwrap();
    assert_eq!(
        get_network_lock_cost(&deps.storage, &deps.api, env.block.height).unwrap(),
        675_000_000_000
    ); // 675_000_000_000 decreasing.

    step_block(deps.as_mut(), &mut env).unwrap();
    assert_eq!(
        get_network_lock_cost(&deps.storage, &deps.api, env.block.height).unwrap(),
        100_000_000_000
    ); // 675_000_000_000 decreasing with 100000000000 min
}

#[test]
fn test_network_pruning() {
    let (mut deps, mut env) = instantiate_contract();

    assert_eq!(get_total_issuance(&deps.storage), 0);

    let n: usize = 10;
    let root_netuid: u16 = 0;
    set_max_registrations_per_block(&mut deps.storage, root_netuid, n as u16);
    set_target_registrations_per_interval(&mut deps.storage, root_netuid, n as u16);
    set_max_allowed_uids(&mut deps.storage, root_netuid, n as u16 + 1);
    set_tempo(&mut deps.storage, root_netuid, 1);
    set_weights_set_rate_limit(&mut deps.storage, root_netuid, 0);
    // No validators yet.
    assert_eq!(get_subnetwork_n(&deps.storage, root_netuid), 0);
    remove_network(&mut deps.storage, 1).unwrap();

    for i in 0..n {
        let hot = (1000 + i).to_string();
        let cold = (1000 + i).to_string();
        let uids: Vec<u16> = (0..i as u16).collect();
        let values: Vec<u16> = vec![1; i];
        add_balance_to_coldkey_account(&Addr::unchecked(cold.clone()), 1_000_000_000_000_000);
        assert_eq!(
            root_register(deps.as_mut(), env.clone(), hot.as_str(), cold.as_str()).is_ok(),
            true
        );
        assert_eq!(
            add_stake(
                deps.as_mut(),
                env.clone(),
                hot.as_str(),
                cold.as_str(),
                1000
            )
            .is_ok(),
            true
        );
        assert_eq!(
            register_network(deps.as_mut(), env.clone(), cold.as_str()).is_ok(),
            true
        );
        println!("Adding network with netuid: {}", (i as u16) + 1);
        assert!(if_subnet_exist(&deps.storage, (i as u16) + 1));
        assert!(is_hotkey_registered_on_network(
            &deps.storage,
            root_netuid,
            &Addr::unchecked(hot.clone())
        ));
        assert!(get_uid_for_net_and_hotkey(
            &deps.storage,
            root_netuid,
            &Addr::unchecked(hot.clone())
        )
        .is_ok());
        // error on first iteration because of empty weights for i=0
        let _ = set_weights(
            deps.as_mut(),
            env.clone(),
            hot.as_str(),
            root_netuid,
            uids,
            values,
            0,
        );
        set_tempo(&mut deps.storage, (i as u16) + 1, 1);
        set_burn(&mut deps.storage, (i as u16) + 1, 0);
        assert_eq!(
            burned_register_ok_neuron(
                deps.as_mut(),
                env.clone(),
                (i as u16) + 1,
                hot.as_str(),
                cold.as_str(),
            )
            .is_ok(),
            true
        );
        assert_eq!(get_total_issuance(&deps.storage), 1_000 * ((i as u64) + 1));
        assert_eq!(get_subnetwork_n(&deps.storage, root_netuid), (i as u16) + 1);
    }

    // All stake values.
    assert_eq!(get_total_issuance(&deps.storage), 10000);

    step_block(deps.as_mut(), &mut env).unwrap();
    assert_eq!(
        root_epoch(&mut deps.storage, &deps.api, 1_000_000_001).is_ok(),
        true
    );
    assert_eq!(get_subnet_emission_value(&deps.storage, 0), 277_820_113);
    assert_eq!(get_subnet_emission_value(&deps.storage, 1), 246_922_263);
    assert_eq!(get_subnet_emission_value(&deps.storage, 2), 215_549_466);
    assert_eq!(get_subnet_emission_value(&deps.storage, 3), 176_432_500);
    assert_eq!(get_subnet_emission_value(&deps.storage, 4), 77_181_559);
    assert_eq!(get_subnet_emission_value(&deps.storage, 5), 5_857_251);
    assert_eq!(get_total_issuance(&deps.storage), 10000);
    step_block(deps.as_mut(), &mut env).unwrap();
    assert_eq!(get_pending_emission(&deps.storage, 0), 0); // root network gets no pending emission.
    assert_eq!(get_pending_emission(&deps.storage, 1), 246_922_263);
    assert_eq!(get_pending_emission(&deps.storage, 2), 0); // This has been drained.
    assert_eq!(get_pending_emission(&deps.storage, 3), 176_432_500);
    assert_eq!(get_pending_emission(&deps.storage, 4), 0); // This network has been drained.
    assert_eq!(get_pending_emission(&deps.storage, 5), 5_857_251);
    step_block(deps.as_mut(), &mut env).unwrap();
    assert_eq!(get_total_issuance(&deps.storage), 585_930_498);
}
