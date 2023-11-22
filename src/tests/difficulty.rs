use cosmwasm_std::Addr;

use crate::test_helpers::{add_network, instantiate_contract, register_ok_neuron, step_block};
use crate::uids::{get_hotkey_for_net_and_uid, get_subnetwork_n};
use crate::utils::{
    get_adjustment_interval, get_difficulty_as_u64, get_last_adjustment_block,
    get_max_allowed_uids, get_max_difficulty, get_max_registrations_per_block, get_min_difficulty,
    get_network_registration_allowed, get_registrations_this_block,
    get_registrations_this_interval, get_target_registrations_per_interval,
    set_adjustment_interval, set_difficulty, set_immunity_period, set_max_allowed_uids,
    set_max_difficulty, set_max_registrations_per_block, set_min_difficulty,
    set_network_registration_allowed, set_target_registrations_per_interval,
};

// TODO adjust difficulty in tests to make it faster tests to pass.
#[test]
#[cfg(not(tarpaulin))]
fn test_registration_difficulty_adjustment() {
    let (mut deps, mut env) = instantiate_contract();

    // Create Net 1
    let netuid: u16 = 2;
    let tempo: u16 = 1;
    let modality: u16 = 1;
    add_network(&mut deps.storage, netuid, tempo, modality);
    set_immunity_period(&mut deps.storage, netuid, 2);

    set_min_difficulty(&mut deps.storage, netuid, 10000);
    set_difficulty(&mut deps.storage, netuid, 10000);
    assert_eq!(get_difficulty_as_u64(&deps.storage, netuid), 10000); // Check initial difficulty.
    assert_eq!(get_last_adjustment_block(&deps.storage, netuid), 0); // Last adjustment block starts at 0.
    assert_eq!(get_registrations_this_block(&deps.storage, netuid), 0); // No registrations this block.

    set_target_registrations_per_interval(&mut deps.storage, netuid, 2);
    set_adjustment_interval(&mut deps.storage, netuid, 100);
    assert_eq!(
        get_network_registration_allowed(&deps.storage, netuid),
        true
    ); // Default registration allowed.

    // Set values and check.
    set_difficulty(&mut deps.storage, netuid, 20000);
    set_adjustment_interval(&mut deps.storage, netuid, 1);
    set_target_registrations_per_interval(&mut deps.storage, netuid, 1);
    set_max_registrations_per_block(&mut deps.storage, netuid, 3);
    set_max_allowed_uids(&mut deps.storage, netuid, 3);
    set_network_registration_allowed(&mut deps.storage, netuid, true);
    assert_eq!(get_difficulty_as_u64(&deps.storage, netuid), 20000); // Check set difficutly.
    assert_eq!(get_adjustment_interval(&deps.storage, netuid), 1); // Check set adjustment interval.
    assert_eq!(
        get_target_registrations_per_interval(&deps.storage, netuid),
        1
    ); // Check set adjustment interval.
    assert_eq!(get_max_registrations_per_block(&deps.storage, netuid), 3); // Check set registrations per block.
    assert_eq!(get_max_allowed_uids(&deps.storage, netuid), 3); // Check set registrations per block.
    assert_eq!(
        get_network_registration_allowed(&deps.storage, netuid),
        true
    ); // Check set registration allowed

    // Lets register 3 neurons...
    let hotkey0 = "addr0";
    let hotkey1 = "addr100";
    let hotkey2 = "addr2000";
    let coldkey0 = "addr0";
    let coldkey1 = "addr1000";
    let coldkey2 = "addr20000";

    register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        hotkey0,
        coldkey0,
        39420842,
    );
    register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        hotkey1,
        coldkey1,
        12412392,
    );
    register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        hotkey2,
        coldkey2,
        21813123,
    );

    assert_eq!(
        get_hotkey_for_net_and_uid(&deps.storage, netuid, 0).unwrap(),
        Addr::unchecked(hotkey0.to_string())
    );
    assert_eq!(
        get_hotkey_for_net_and_uid(&deps.storage, netuid, 1).unwrap(),
        Addr::unchecked(hotkey1.to_string())
    );
    assert_eq!(
        get_hotkey_for_net_and_uid(&deps.storage, netuid, 2).unwrap(),
        Addr::unchecked(hotkey2.to_string())
    );

    assert_eq!(get_subnetwork_n(&deps.storage, netuid), 3); // All 3 are registered.
    assert_eq!(get_registrations_this_block(&deps.storage, netuid), 3); // 3 Registrations.
    assert_eq!(get_registrations_this_interval(&deps.storage, netuid), 3); // 3 Registrations this interval.

    // Fast forward 1 block.
    assert_eq!(get_difficulty_as_u64(&deps.storage, netuid), 20000); // Difficulty is unchanged.
    step_block(deps.as_mut(), &mut env).unwrap();
    assert_eq!(get_registrations_this_block(&deps.storage, netuid), 0); // Registrations have been erased.
    assert_eq!(get_last_adjustment_block(&deps.storage, netuid), 2); // We just adjusted on the first block.
    assert_eq!(get_difficulty_as_u64(&deps.storage, netuid), 40000); // Difficulty is increased ( 20000 * ( 3 + 1 ) / ( 1 + 1 ) ) = 80_000
    assert_eq!(get_registrations_this_interval(&deps.storage, netuid), 0); // Registrations this interval has been wiped.

    // Lets change the adjustment interval
    set_adjustment_interval(&mut deps.storage, netuid, 3);
    assert_eq!(get_adjustment_interval(&deps.storage, netuid), 3); // Check set adjustment interval.

    set_target_registrations_per_interval(&mut deps.storage, netuid, 3);
    assert_eq!(
        get_target_registrations_per_interval(&deps.storage, netuid),
        3
    ); // Target is default.

    // Register 3 more
    register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        "addr1",
        "addr1",
        3942084,
    );
    register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        "addr101",
        "addr1001",
        1241239,
    );
    register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        "addr2001",
        "addr20001",
        2181312,
    );
    assert_eq!(
        get_hotkey_for_net_and_uid(&deps.storage, netuid, 0).unwrap(),
        Addr::unchecked("addr1".to_string())
    ); // replace 0
    assert_eq!(
        get_hotkey_for_net_and_uid(&deps.storage, netuid, 1).unwrap(),
        Addr::unchecked("addr101".to_string())
    ); // replace 1
    assert_eq!(
        get_hotkey_for_net_and_uid(&deps.storage, netuid, 2).unwrap(),
        Addr::unchecked("addr2001".to_string())
    ); // replace 2
    assert_eq!(get_registrations_this_block(&deps.storage, netuid), 3); // Registrations have been erased.
    assert_eq!(get_registrations_this_interval(&deps.storage, netuid), 3); // Registrations this interval = 3

    step_block(deps.as_mut(), &mut env).unwrap();
    assert_eq!(get_last_adjustment_block(&deps.storage, netuid), 2); // Still previous adjustment block.
    assert_eq!(get_registrations_this_block(&deps.storage, netuid), 0); // Registrations have been erased.
    assert_eq!(get_registrations_this_interval(&deps.storage, netuid), 3); // Registrations this interval = 3

    // Register 3 more.
    register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        "addr2",
        "addr2",
        394208420,
    );
    register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        "addr102",
        "addr1002",
        124123920,
    );
    register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        "addr2002",
        "addr20002",
        218131230,
    );
    assert_eq!(get_registrations_this_block(&deps.storage, netuid), 3); // Registrations have been erased.

    // We have 6 registrations this adjustment interval.
    step_block(deps.as_mut(), &mut env).unwrap();
    assert_eq!(get_registrations_this_interval(&deps.storage, netuid), 6); // Registrations this interval = 6
    assert_eq!(get_difficulty_as_u64(&deps.storage, netuid), 40000); // Difficulty unchanged.
    step_block(deps.as_mut(), &mut env).unwrap();
    assert_eq!(get_difficulty_as_u64(&deps.storage, netuid), 60_000); // Difficulty changed ( 40000 ) * ( 6 + 3 / 3 + 3 ) = 40000 * 1.5 = 60_000
    assert_eq!(get_registrations_this_interval(&deps.storage, netuid), 0); // Registrations this interval drops to 0.

    // Test min value.
    set_min_difficulty(&mut deps.storage, netuid, 1);
    set_difficulty(&mut deps.storage, netuid, 4);
    assert_eq!(get_min_difficulty(&deps.storage, netuid), 1);
    assert_eq!(get_difficulty_as_u64(&deps.storage, netuid), 4);
    set_adjustment_interval(&mut deps.storage, netuid, 1);
    step_block(deps.as_mut(), &mut env).unwrap();
    assert_eq!(get_difficulty_as_u64(&deps.storage, netuid), 2); // Difficulty dropped 4 * ( 0 + 1 ) / (1 + 1) = 1/2 = 2
    step_block(deps.as_mut(), &mut env).unwrap();
    assert_eq!(get_difficulty_as_u64(&deps.storage, netuid), 1); // Difficulty dropped 2 * ( 0 + 1 ) / (1 + 1) = 1/2 = 1
    step_block(deps.as_mut(), &mut env).unwrap();
    assert_eq!(get_difficulty_as_u64(&deps.storage, netuid), 1); // Difficulty dropped 2 * ( 0 + 1 ) / (1 + 1) = 1/2 = max(0.5, 1)

    // Test max value.
    set_max_difficulty(&mut deps.storage, netuid, 10000);
    set_difficulty(&mut deps.storage, netuid, 5000);
    assert_eq!(get_max_difficulty(&deps.storage, netuid), 10000);
    assert_eq!(get_difficulty_as_u64(&deps.storage, netuid), 5000);
    set_max_registrations_per_block(&mut deps.storage, netuid, 4);
    register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        "addr3",
        "addr3",
        294208420,
    );
    register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        "addr103",
        "addr1003",
        824123920,
    );
    register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        "addr2003",
        "addr20003",
        324123920,
    );
    register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        "addr2004",
        "addr20004",
        524123920,
    );
    assert_eq!(get_registrations_this_interval(&deps.storage, netuid), 4);
    assert_eq!(
        get_target_registrations_per_interval(&deps.storage, netuid),
        3
    );
    step_block(deps.as_mut(), &mut env).unwrap();
    assert_eq!(get_difficulty_as_u64(&deps.storage, netuid), 5833); // Difficulty increased 5000 * ( 4 + 3 ) / (3 + 3) = 1.16 * 5000 = 5833

    register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        "addr4",
        "addr4",
        124208420,
    );
    register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        "addr104",
        "addr1004",
        314123920,
    );
    register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        "addr2004",
        "addr20004",
        834123920,
    );
    step_block(deps.as_mut(), &mut env).unwrap();
    assert_eq!(get_difficulty_as_u64(&deps.storage, netuid), 5833); // Difficulty unchanged
}
