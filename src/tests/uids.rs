/********************************************
    tests for uids.rs file
*********************************************/

/********************************************
    tests uids::replace_neuron()
*********************************************/

use cosmwasm_std::Addr;

use crate::registration::create_work_for_block_number;
use crate::staking::{
    get_stake_for_coldkey_and_hotkey, get_total_stake_for_hotkey,
    increase_stake_on_coldkey_hotkey_account,
};
use crate::test_helpers::{add_network, instantiate_contract, pow_register_ok_neuron};
use crate::uids::{
    get_hotkey_for_net_and_uid, get_uid_for_net_and_hotkey, is_hotkey_registered_on_any_network,
    is_hotkey_registered_on_network, replace_neuron,
};

#[test]
fn test_replace_neuron() {
    let (mut deps, mut env) = instantiate_contract();

    let netuid: u16 = 1;
    let tempo: u16 = 13;
    let hotkey_account_id = Addr::unchecked("1".to_string());
    let (nonce, work): (u64, Vec<u8>) = create_work_for_block_number(
        &mut deps.storage,
        netuid,
        env.block.height,
        111111,
        &hotkey_account_id,
    );
    let coldkey_account_id = Addr::unchecked("1234".to_string());

    let new_hotkey_account_id = Addr::unchecked("2");
    let _new_colkey_account_id = Addr::unchecked("12345");

    //add network
    add_network(&mut deps.storage, netuid, tempo, 0);

    // Register a neuron.
    pow_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        env.block.height,
        nonce,
        work,
        &hotkey_account_id,
        &coldkey_account_id,
    );

    // Get UID
    let neuron_uid = get_uid_for_net_and_hotkey(&deps.storage, netuid, &hotkey_account_id).unwrap();

    // Replace the neuron.
    replace_neuron(
        &mut deps.storage,
        &deps.api,
        netuid,
        neuron_uid,
        &new_hotkey_account_id,
        env.block.height,
    )
    .unwrap();

    // Check old hotkey is not registered on any network.
    assert!(get_uid_for_net_and_hotkey(&deps.storage, netuid, &hotkey_account_id).is_err());
    assert!(!is_hotkey_registered_on_any_network(
        &deps.storage,
        &hotkey_account_id,
    ));

    let curr_hotkey = get_hotkey_for_net_and_uid(&deps.storage, netuid, neuron_uid).unwrap();
    assert_ne!(curr_hotkey, hotkey_account_id);

    // Check new hotkey is registered on the network.
    assert!(get_uid_for_net_and_hotkey(&deps.storage, netuid, &new_hotkey_account_id).is_ok());
    assert!(is_hotkey_registered_on_any_network(
        &deps.storage,
        &new_hotkey_account_id,
    ));
    assert_eq!(curr_hotkey, new_hotkey_account_id.clone());
}

#[test]
fn test_replace_neuron_multiple_subnets() {
    let (mut deps, mut env) = instantiate_contract();

    let block_number: u64 = 0;
    let netuid: u16 = 1;
    let netuid1: u16 = 2;
    let tempo: u16 = 13;
    let hotkey_account_id = Addr::unchecked("1".to_string());
    let new_hotkey_account_id = Addr::unchecked("2".to_string());

    let (nonce, work): (u64, Vec<u8>) = create_work_for_block_number(
        &mut deps.storage,
        netuid,
        block_number,
        111111,
        &hotkey_account_id,
    );
    let (nonce1, work1): (u64, Vec<u8>) = create_work_for_block_number(
        &mut deps.storage,
        netuid1,
        block_number,
        111111 * 5,
        &hotkey_account_id.clone(),
    );

    let coldkey_account_id = Addr::unchecked("1234".to_string());

    let _new_colkey_account_id = Addr::unchecked("12345".to_string());

    //add network
    add_network(&mut deps.storage, netuid, tempo, 0);
    add_network(&mut deps.storage, netuid1, tempo, 0);

    // Register a neuron on both networks.
    pow_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        env.block.height,
        nonce,
        work,
        &hotkey_account_id,
        &coldkey_account_id,
    );

    pow_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid1,
        env.block.height,
        nonce1,
        work1,
        &hotkey_account_id,
        &coldkey_account_id,
    );

    // Get UID
    let neuron_uid = get_uid_for_net_and_hotkey(&deps.storage, netuid, &hotkey_account_id);
    assert_eq!(neuron_uid.is_ok(), true);

    // Verify neuron is registered on both networks.
    assert!(is_hotkey_registered_on_network(
        &deps.storage,
        netuid,
        &hotkey_account_id,
    ));
    assert!(is_hotkey_registered_on_network(
        &deps.storage,
        netuid1,
        &hotkey_account_id,
    ));
    assert!(is_hotkey_registered_on_any_network(
        &deps.storage,
        &hotkey_account_id,
    ));

    // Replace the neuron.
    // Only replace on ONE network.
    replace_neuron(
        &mut deps.storage,
        &deps.api,
        netuid,
        neuron_uid.unwrap(),
        &new_hotkey_account_id,
        block_number,
    )
    .unwrap();

    // Check old hotkey is not registered on netuid network.
    assert!(get_uid_for_net_and_hotkey(&deps.storage, netuid, &hotkey_account_id).is_err());

    // Verify still registered on netuid1 network.
    assert!(is_hotkey_registered_on_any_network(
        &deps.storage,
        &hotkey_account_id,
    ));
    assert!(is_hotkey_registered_on_network(
        &deps.storage,
        netuid1,
        &hotkey_account_id.clone(),
    ));
}

#[test]
fn test_replace_neuron_multiple_subnets_unstake_all() {
    let (mut deps, mut env) = instantiate_contract();

    let block_number: u64 = 0;
    let netuid: u16 = 1;
    let netuid1: u16 = 2;
    let tempo: u16 = 13;

    let hotkey_account_id = Addr::unchecked("1".to_string());
    let new_hotkey_account_id = Addr::unchecked("2".to_string());

    let (nonce, work): (u64, Vec<u8>) = create_work_for_block_number(
        &mut deps.storage,
        netuid,
        block_number,
        111111,
        &hotkey_account_id,
    );
    let (nonce1, work1): (u64, Vec<u8>) = create_work_for_block_number(
        &mut deps.storage,
        netuid1,
        block_number,
        111111 * 5,
        &hotkey_account_id.clone(),
    );

    let coldkey_account_id = Addr::unchecked("1234".to_string());
    let coldkey_account1_id = Addr::unchecked("1235".to_string());
    let coldkey_account2_id = Addr::unchecked("1236".to_string());

    let stake_amount = 1000;

    //add network
    add_network(&mut deps.storage, netuid, tempo, 0);
    add_network(&mut deps.storage, netuid1, tempo, 0);

    // Register a neuron on both networks.
    pow_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        env.block.height,
        nonce,
        work,
        &hotkey_account_id,
        &coldkey_account_id,
    );

    pow_register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid1,
        env.block.height,
        nonce1,
        work1,
        &hotkey_account_id,
        &coldkey_account_id,
    );

    // Get UID
    let neuron_uid = get_uid_for_net_and_hotkey(&deps.storage, netuid, &hotkey_account_id).unwrap();
    // assert_eq!(neuron_uid.is_ok(), true);

    // Stake on neuron with multiple coldkeys.
    increase_stake_on_coldkey_hotkey_account(
        &mut deps.storage,
        &coldkey_account_id,
        &hotkey_account_id,
        stake_amount,
    );
    increase_stake_on_coldkey_hotkey_account(
        &mut deps.storage,
        &coldkey_account1_id,
        &hotkey_account_id,
        stake_amount + 1,
    );
    increase_stake_on_coldkey_hotkey_account(
        &mut deps.storage,
        &coldkey_account2_id,
        &hotkey_account_id,
        stake_amount + 2,
    );

    // Check stake on neuron
    assert_eq!(
        get_stake_for_coldkey_and_hotkey(&deps.storage, &coldkey_account_id, &hotkey_account_id,),
        stake_amount
    );
    assert_eq!(
        get_stake_for_coldkey_and_hotkey(&deps.storage, &coldkey_account1_id, &hotkey_account_id,),
        stake_amount + 1
    );
    assert_eq!(
        get_stake_for_coldkey_and_hotkey(&deps.storage, &coldkey_account2_id, &hotkey_account_id,),
        stake_amount + 2
    );

    // Check total stake on neuron
    assert_eq!(
        get_total_stake_for_hotkey(&deps.storage, &hotkey_account_id),
        (stake_amount * 3) + (1 + 2)
    );

    // Replace the neuron.
    replace_neuron(
        &mut deps.storage,
        &deps.api,
        netuid,
        neuron_uid,
        &new_hotkey_account_id.clone(),
        block_number,
    )
    .unwrap();

    // The stakes should still be on the neuron. It is still registered on one network.
    assert!(is_hotkey_registered_on_any_network(
        &deps.storage,
        &hotkey_account_id,
    ));

    // Check the stake is still on the coldkey accounts.
    assert_eq!(
        get_stake_for_coldkey_and_hotkey(&deps.storage, &coldkey_account_id, &hotkey_account_id,),
        stake_amount
    );
    assert_eq!(
        get_stake_for_coldkey_and_hotkey(&deps.storage, &coldkey_account1_id, &hotkey_account_id,),
        stake_amount + 1
    );
    assert_eq!(
        get_stake_for_coldkey_and_hotkey(&deps.storage, &coldkey_account2_id, &hotkey_account_id,),
        stake_amount + 2
    );

    // Check total stake on neuron
    assert_eq!(
        get_total_stake_for_hotkey(&deps.storage, &hotkey_account_id),
        (stake_amount * 3) + (1 + 2)
    );

    // replace on second network
    replace_neuron(
        &mut deps.storage,
        &deps.api,
        netuid1,
        neuron_uid,
        &new_hotkey_account_id,
        block_number,
    )
    .unwrap();

    // The neuron should be unregistered now.
    assert!(!is_hotkey_registered_on_any_network(
        &deps.storage,
        &hotkey_account_id,
    ));

    // Check the stake is now on the free balance of the coldkey accounts.
    assert_eq!(
        get_stake_for_coldkey_and_hotkey(&deps.storage, &coldkey_account_id, &hotkey_account_id,),
        0
    );
    // assert_eq!(Balances::free_balance(&coldkey_account_id), stake_amount);

    assert_eq!(
        get_stake_for_coldkey_and_hotkey(&deps.storage, &coldkey_account1_id, &hotkey_account_id,),
        0
    );
    // assert_eq!(
    //     Balances::free_balance(&coldkey_account1_id),
    //     stake_amount + 1
    // );

    assert_eq!(
        get_stake_for_coldkey_and_hotkey(&deps.storage, &coldkey_account2_id, &hotkey_account_id,),
        0
    );
    // assert_eq!(
    //     Balances::free_balance(&coldkey_account2_id),
    //     stake_amount + 2
    // );

    // Check total stake on neuron
    assert_eq!(
        get_total_stake_for_hotkey(&deps.storage, &hotkey_account_id),
        0
    );
}
