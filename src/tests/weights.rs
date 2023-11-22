use cosmwasm_std::Addr;
use cw_multi_test::Executor;
use substrate_fixed::types::I32F32;

use crate::epoch::get_weights;
use crate::test_helpers::{add_network, instantiate_contract, register_ok_neuron, set_weights};
use crate::uids::{get_subnetwork_n, get_uid_for_net_and_hotkey};
use crate::utils::{
    set_difficulty, set_max_allowed_uids, set_max_registrations_per_block, set_max_weight_limit,
    set_min_allowed_weights, set_target_registrations_per_interval, set_validator_permit_for_uid,
    set_weights_set_rate_limit, set_weights_version_key,
};
use crate::weights::{
    check_len_uids_within_allowed, check_length, is_self_weight, max_weight_limited,
    normalize_weights,
};
use crate::ContractError;

#[test]
fn test_weights_err_no_validator_permit() {
    let (mut deps, mut env) = instantiate_contract();

    let hotkey_account_id = "addr55";
    let netuid: u16 = 2;
    let tempo: u16 = 13;
    add_network(&mut deps.storage, netuid, tempo, 0);

    set_min_allowed_weights(&mut deps.storage, netuid, 0);
    set_max_allowed_uids(&mut deps.storage, netuid, 3);
    set_max_weight_limit(&mut deps.storage, netuid, u16::MAX);

    register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        hotkey_account_id,
        "addr55",
        0,
    );
    env.block.height += 1;
    register_ok_neuron(deps.as_mut(), env.clone(), netuid, "addr5", "addr5", 65555);
    env.block.height += 1;
    register_ok_neuron(deps.as_mut(), env.clone(), netuid, "addr6", "addr6", 75555);

    let weights_keys: Vec<u16> = vec![1, 2];
    let weight_values: Vec<u16> = vec![1, 2];

    let err = set_weights(
        deps.as_mut(),
        env.clone(),
        hotkey_account_id,
        netuid,
        weights_keys.clone(),
        weight_values.clone(),
        0,
    )
    .unwrap_err();
    assert_eq!(ContractError::NoValidatorPermit {}, err);

    let neuron_uid =
        get_uid_for_net_and_hotkey(&deps.storage, netuid, &Addr::unchecked(hotkey_account_id))
            .unwrap();

    set_validator_permit_for_uid(&mut deps.storage, netuid, neuron_uid, true);

    let weights_keys: Vec<u16> = vec![1, 2];
    let weight_values: Vec<u16> = vec![1, 2];
    let result = set_weights(
        deps.as_mut(),
        env.clone(),
        hotkey_account_id,
        netuid,
        weights_keys.clone(),
        weight_values.clone(),
        0,
    );
    assert_eq!(result.is_ok(), true)
}

#[test]
fn test_weights_version_key() {
    let (mut deps, mut env) = instantiate_contract();

    let hotkey = "addr55";
    let coldkey = "addr66";
    let netuid0: u16 = 2;
    let netuid1: u16 = 3;
    let tempo: u16 = 13;

    add_network(&mut deps.storage, netuid0, tempo, 0);

    add_network(&mut deps.storage, netuid1, tempo, 0);

    register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid0,
        hotkey,
        coldkey,
        2143124,
    );
    env.block.height += 1;
    register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid1,
        hotkey,
        coldkey,
        2143124,
    );
    env.block.height += 1;

    let weights_keys: Vec<u16> = vec![0];
    let weight_values: Vec<u16> = vec![1];
    let result = set_weights(
        deps.as_mut(),
        env.clone(),
        hotkey,
        netuid0,
        weights_keys.clone(),
        weight_values.clone(),
        0,
    );
    assert!(result.is_ok());
    env.block.height += 100;

    let result = set_weights(
        deps.as_mut(),
        env.clone(),
        hotkey,
        netuid1,
        weights_keys.clone(),
        weight_values.clone(),
        0,
    );
    assert!(result.is_ok());
    env.block.height += 100;

    // Set version keys.
    let key0: u64 = 12312;
    let key1: u64 = 20313;

    set_weights_version_key(&mut deps.storage, netuid0, key0);
    set_weights_version_key(&mut deps.storage, netuid1, key1);

    // Setting works with version key.
    let result = set_weights(
        deps.as_mut(),
        env.clone(),
        hotkey,
        netuid0,
        weights_keys.clone(),
        weight_values.clone(),
        key0,
    );
    assert!(result.is_ok());
    env.block.height += 100;

    let result = set_weights(
        deps.as_mut(),
        env.clone(),
        hotkey,
        netuid1,
        weights_keys.clone(),
        weight_values.clone(),
        key1,
    );
    assert!(result.is_ok());
    env.block.height += 100;

    // validator:20313 >= network:12312 (accepted: validator newer)
    let result = set_weights(
        deps.as_mut(),
        env.clone(),
        hotkey,
        netuid0,
        weights_keys.clone(),
        weight_values.clone(),
        key1,
    );
    assert!(result.is_ok());
    env.block.height += 100;

    // Setting fails with incorrect keys.
    // validator:12312 < network:20313 (rejected: validator not updated)
    let result = set_weights(
        deps.as_mut(),
        env.clone(),
        hotkey,
        netuid1,
        weights_keys.clone(),
        weight_values.clone(),
        key0,
    );
    assert_eq!(result.is_err(), true);
}

#[test]
fn test_weights_err_setting_weights_too_fast() {
    let (mut deps, mut env) = instantiate_contract();

    let hotkey = "addr55";

    let netuid = 2;
    let tempo: u16 = 13;
    add_network(&mut deps.storage, netuid, tempo, 0);

    set_min_allowed_weights(&mut deps.storage, netuid, 0);
    set_max_allowed_uids(&mut deps.storage, netuid, 3);
    set_max_weight_limit(&mut deps.storage, netuid, u16::MAX);

    register_ok_neuron(deps.as_mut(), env.clone(), netuid, hotkey, "addr66", 0);
    env.block.height += 1;
    register_ok_neuron(deps.as_mut(), env.clone(), netuid, "addr1", "addr1", 65555);
    env.block.height += 1;
    register_ok_neuron(deps.as_mut(), env.clone(), netuid, "addr2", "addr2", 75555);
    env.block.height += 1;

    let neuron_uid =
        get_uid_for_net_and_hotkey(&deps.storage, netuid, &Addr::unchecked(hotkey)).unwrap();

    set_validator_permit_for_uid(&mut deps.storage, netuid, neuron_uid, true);

    set_weights_set_rate_limit(&mut deps.storage, netuid, 10);

    let weights_keys: Vec<u16> = vec![1, 2];
    let weight_values: Vec<u16> = vec![1, 2];

    for i in 1..100 {
        let result = set_weights(
            deps.as_mut(),
            env.clone(),
            hotkey,
            netuid,
            weights_keys.clone(),
            weight_values.clone(),
            0,
        );
        if i % 10 == 1 {
            assert!(result.is_ok());
        } else {
            assert_eq!(ContractError::SettingWeightsTooFast {}, result.unwrap_err());
        }
        env.block.height += 1;
    }
}

#[test]
fn test_weights_err_weights_vec_not_equal_size() {
    let (mut deps, mut env) = instantiate_contract();

    let hotkey = "addr55";

    let netuid = 2;
    let tempo: u16 = 13;
    add_network(&mut deps.storage, netuid, tempo, 0);

    register_ok_neuron(deps.as_mut(), env.clone(), netuid, hotkey, "addr66", 0);
    env.block.height += 1;

    let neuron_uid =
        get_uid_for_net_and_hotkey(&deps.storage, netuid, &Addr::unchecked(hotkey)).unwrap();

    set_validator_permit_for_uid(&mut deps.storage, netuid, neuron_uid, true);

    let weights_keys: Vec<u16> = vec![1, 2, 3, 4, 5, 6];
    let weight_values: Vec<u16> = vec![1, 2, 3, 4, 5]; // Uneven sizes

    let result = set_weights(
        deps.as_mut(),
        env.clone(),
        hotkey,
        netuid,
        weights_keys.clone(),
        weight_values.clone(),
        0,
    );
    assert_eq!(ContractError::WeightVecNotEqualSize {}, result.unwrap_err());
}

#[test]
fn test_weights_err_has_duplicate_ids() {
    let (mut deps, env) = instantiate_contract();

    let hotkey = "addr666";

    let netuid = 2;
    let tempo: u16 = 13;
    add_network(&mut deps.storage, netuid, tempo, 0);

    // Allow many registrations per block.
    set_max_allowed_uids(&mut deps.storage, netuid, 100);
    set_max_registrations_per_block(&mut deps.storage, netuid, 100);
    set_target_registrations_per_interval(&mut deps.storage, netuid, 100);

    // uid 0
    register_ok_neuron(deps.as_mut(), env.clone(), netuid, hotkey, "addr77", 0);
    let neuron_uid =
        get_uid_for_net_and_hotkey(&deps.storage, netuid, &Addr::unchecked(hotkey)).unwrap();

    set_validator_permit_for_uid(&mut deps.storage, netuid, neuron_uid, true);

    // uid 1
    register_ok_neuron(deps.as_mut(), env.clone(), netuid, "addr1", "addr1", 100000);
    let neuron_uid =
        get_uid_for_net_and_hotkey(&deps.storage, netuid, &Addr::unchecked("addr1")).unwrap();

    // uid 2
    register_ok_neuron(deps.as_mut(), env.clone(), netuid, "addr2", "addr2", 100000);
    let neuron_uid =
        get_uid_for_net_and_hotkey(&deps.storage, netuid, &Addr::unchecked("addr2")).unwrap();

    // uid 3
    register_ok_neuron(deps.as_mut(), env.clone(), netuid, "addr3", "addr3", 100000);
    let neuron_uid =
        get_uid_for_net_and_hotkey(&deps.storage, netuid, &Addr::unchecked("addr3")).unwrap();

    let weights_keys: Vec<u16> = vec![1, 1, 1]; // Contains duplicates
    let weight_values: Vec<u16> = vec![1, 2, 3];

    let result = set_weights(
        deps.as_mut(),
        env.clone(),
        hotkey,
        netuid,
        weights_keys.clone(),
        weight_values.clone(),
        0,
    );
    assert_eq!(ContractError::DuplicateUids {}, result.unwrap_err());
}

#[test]
fn test_weights_err_max_weight_limit() {
    let (mut deps, env) = instantiate_contract();

    let netuid = 2;
    let tempo: u16 = 13;
    add_network(&mut deps.storage, netuid, tempo, 0);

    set_max_allowed_uids(&mut deps.storage, netuid, 5);
    set_target_registrations_per_interval(&mut deps.storage, netuid, 5);
    set_max_weight_limit(&mut deps.storage, netuid, u16::MAX / 5);
    set_min_allowed_weights(&mut deps.storage, netuid, 0);
    set_max_registrations_per_block(&mut deps.storage, netuid, 100);

    // uid 0
    register_ok_neuron(deps.as_mut(), env.clone(), netuid, "addr0", "addr0", 55555);
    let neuron_uid =
        get_uid_for_net_and_hotkey(&deps.storage, netuid, &Addr::unchecked("addr0")).unwrap();

    set_validator_permit_for_uid(&mut deps.storage, netuid, neuron_uid, true);

    register_ok_neuron(deps.as_mut(), env.clone(), netuid, "addr1", "addr1", 65555);
    register_ok_neuron(deps.as_mut(), env.clone(), netuid, "addr2", "addr2", 75555);
    register_ok_neuron(deps.as_mut(), env.clone(), netuid, "addr3", "addr3", 95555);
    register_ok_neuron(deps.as_mut(), env.clone(), netuid, "addr4", "addr4", 35555);

    // Non self-weight fails.
    let uids: Vec<u16> = vec![1, 2, 3, 4];
    let values: Vec<u16> = vec![u16::MAX / 4, u16::MAX / 4, u16::MAX / 54, u16::MAX / 4];
    let result = set_weights(
        deps.as_mut(),
        env.clone(),
        "addr0",
        netuid,
        uids.clone(),
        values.clone(),
        0,
    );
    assert_eq!(ContractError::MaxWeightExceeded {}, result.unwrap_err());

    // Self-weight is a success.
    let uids: Vec<u16> = vec![0]; // Self.
    let values: Vec<u16> = vec![u16::MAX]; // normalizes to u32::MAX
    let result = set_weights(
        deps.as_mut(),
        env.clone(),
        "addr0",
        netuid,
        uids.clone(),
        values.clone(),
        0,
    );
    assert!(result.is_ok());
}

#[test]
fn test_set_weights_err_not_active() {
    let (mut deps, env) = instantiate_contract();

    let netuid = 2;
    let tempo: u16 = 13;
    add_network(&mut deps.storage, netuid, tempo, 0);

    // uid 0
    register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        "addr666",
        "addr2",
        100000,
    );
    let neuron_uid =
        get_uid_for_net_and_hotkey(&deps.storage, netuid, &Addr::unchecked("addr666")).unwrap();

    let weights_keys: Vec<u16> = vec![0]; // Uid 0 is valid.
    let weight_values: Vec<u16> = vec![1];
    // This hotkey is NOT registered.
    let result = set_weights(
        deps.as_mut(),
        env.clone(),
        "1",
        netuid,
        weights_keys.clone(),
        weight_values.clone(),
        0,
    );
    assert_eq!(ContractError::NotRegistered {}, result.unwrap_err());
}

#[test]
fn test_set_weights_err_invalid_uid() {
    let (mut deps, env) = instantiate_contract();

    let netuid = 2;
    let tempo: u16 = 13;
    add_network(&mut deps.storage, netuid, tempo, 0);

    let hotkey_account = "addr55";

    register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        hotkey_account,
        "addr66",
        100000,
    );
    let neuron_uid =
        get_uid_for_net_and_hotkey(&deps.storage, netuid, &Addr::unchecked(hotkey_account))
            .unwrap();

    set_validator_permit_for_uid(&mut deps.storage, netuid, neuron_uid, true);

    let weight_keys: Vec<u16> = vec![9999]; // Does not exist
    let weight_values: Vec<u16> = vec![88]; // random value
    let result = set_weights(
        deps.as_mut(),
        env.clone(),
        hotkey_account,
        netuid,
        weight_keys.clone(),
        weight_values.clone(),
        0,
    );
    assert_eq!(ContractError::InvalidUid {}, result.unwrap_err());
}

#[test]
fn test_set_weight_not_enough_values() {
    let (mut deps, mut env) = instantiate_contract();

    let netuid = 2;
    let tempo: u16 = 13;
    add_network(&mut deps.storage, netuid, tempo, 0);

    let hotkey_account = "addr1";

    register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        hotkey_account,
        "addr2",
        100000,
    );
    let neuron_uid =
        get_uid_for_net_and_hotkey(&deps.storage, netuid, &Addr::unchecked(hotkey_account))
            .unwrap();

    set_validator_permit_for_uid(&mut deps.storage, netuid, neuron_uid, true);

    register_ok_neuron(deps.as_mut(), env.clone(), netuid, "addr3", "addr4", 300000);

    set_max_weight_limit(&mut deps.storage, netuid, u16::MAX);

    set_min_allowed_weights(&mut deps.storage, netuid, 2);

    // Should fail because we are only setting a single value and its not the self weight.
    let weight_keys: Vec<u16> = vec![1]; // not weight.
    let weight_values: Vec<u16> = vec![88]; // random value.
    let result = set_weights(
        deps.as_mut(),
        env.clone(),
        hotkey_account,
        netuid,
        weight_keys.clone(),
        weight_values.clone(),
        0,
    );
    assert_eq!(
        ContractError::NotSettingEnoughWeights {},
        result.unwrap_err()
    );

    // Shouldnt fail because we setting a single value but it is the self weight.
    let weight_keys: Vec<u16> = vec![0]; // self weight.
    let weight_values: Vec<u16> = vec![88]; // random value.
    let result = set_weights(
        deps.as_mut(),
        env.clone(),
        hotkey_account,
        netuid,
        weight_keys.clone(),
        weight_values.clone(),
        0,
    );
    assert!(result.is_ok());
    env.block.height += 100;

    // Should pass because we are setting enough values.
    let weight_keys: Vec<u16> = vec![0, 1]; // self weight.
    let weight_values: Vec<u16> = vec![10, 10]; // random value.

    set_min_allowed_weights(&mut deps.storage, netuid, 2);
    let result = set_weights(
        deps.as_mut(),
        env.clone(),
        hotkey_account,
        netuid,
        weight_keys.clone(),
        weight_values.clone(),
        0,
    );
    assert!(result.is_ok());
}

#[test]
fn test_set_weight_too_many_uids() {
    let (mut deps, env) = instantiate_contract();

    let netuid = 2;
    let tempo: u16 = 13;
    add_network(&mut deps.storage, netuid, tempo, 0);

    let hotkey_account = "addr1";

    register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        hotkey_account,
        "addr2",
        100000,
    );
    let neuron_uid =
        get_uid_for_net_and_hotkey(&deps.storage, netuid, &Addr::unchecked(hotkey_account))
            .unwrap();

    set_validator_permit_for_uid(&mut deps.storage, netuid, neuron_uid, true);

    register_ok_neuron(deps.as_mut(), env.clone(), netuid, "addr3", "addr4", 300000);

    set_min_allowed_weights(&mut deps.storage, netuid, 2);
    set_max_weight_limit(&mut deps.storage, netuid, u16::MAX);

    // Should fail because we are setting more weights than there are neurons.
    let weight_keys: Vec<u16> = vec![0, 1, 2, 3, 4]; // more uids than neurons in subnet.
    let weight_values: Vec<u16> = vec![88, 102, 303, 1212, 11]; // random value.
    let result = set_weights(
        deps.as_mut(),
        env.clone(),
        "addr1",
        netuid,
        weight_keys.clone(),
        weight_values.clone(),
        0,
    );
    assert_eq!(ContractError::TooManyUids {}, result.unwrap_err());

    // Shouldnt fail because we are setting less weights than there are neurons.
    let weight_keys: Vec<u16> = vec![0, 1]; // Only on neurons that exist.
    let weight_values: Vec<u16> = vec![10, 10]; // random value.
    let result = set_weights(
        deps.as_mut(),
        env.clone(),
        "addr1",
        netuid,
        weight_keys.clone(),
        weight_values.clone(),
        0,
    );
    assert!(result.is_ok());
}

#[test]
fn test_set_weights_sum_larger_than_u16_max() {
    let (mut deps, env) = instantiate_contract();

    let netuid = 2;
    let tempo: u16 = 13;
    add_network(&mut deps.storage, netuid, tempo, 0);

    let hotkey_account = "addr1";

    register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        hotkey_account,
        "addr2",
        100000,
    );
    let neuron_uid =
        get_uid_for_net_and_hotkey(&deps.storage, netuid, &Addr::unchecked(hotkey_account))
            .unwrap();

    set_validator_permit_for_uid(&mut deps.storage, netuid, neuron_uid, true);

    register_ok_neuron(deps.as_mut(), env.clone(), netuid, "addr3", "addr4", 300000);

    set_max_weight_limit(&mut deps.storage, netuid, u16::MAX);
    set_min_allowed_weights(&mut deps.storage, netuid, 2);

    // Shouldn't fail because we are setting the right number of weights.
    let weight_keys: Vec<u16> = vec![0, 1];
    let weight_values: Vec<u16> = vec![u16::MAX, u16::MAX];
    // sum of weights is larger than u16 max.
    assert!(weight_values.iter().map(|x| *x as u64).sum::<u64>() > (u16::MAX as u64));

    let result = set_weights(
        deps.as_mut(),
        env.clone(),
        hotkey_account,
        netuid,
        weight_keys.clone(),
        weight_values.clone(),
        0,
    );
    assert!(result.is_ok());

    // Get max-upscaled unnormalized weights.
    let all_weights: Vec<Vec<I32F32>> = get_weights(&deps.storage, netuid);

    let weights_set: &Vec<I32F32> = &all_weights[neuron_uid as usize];
    assert_eq!(weights_set[0], u16::MAX);
    assert_eq!(weights_set[1], u16::MAX);
}

#[test]
fn test_check_length_allows_singleton() {
    let (mut deps, env) = instantiate_contract();

    let netuid = 2;
    let tempo: u16 = 13;
    add_network(&mut deps.storage, netuid, tempo, 0);

    let hotkey_account = Addr::unchecked("addr1");

    let max_allowed: u16 = 1;
    let min_allowed_weights = max_allowed;

    set_min_allowed_weights(&mut deps.storage, netuid, min_allowed_weights);

    let uids: Vec<u16> = Vec::from_iter((0..max_allowed).map(|id| id + 1));
    let uid: u16 = uids[0].clone();
    let weights: Vec<u16> = Vec::from_iter((0..max_allowed).map(|id| id + 1));

    let expected = true;
    let result = check_length(&deps.storage, netuid, uid, &uids, &weights);

    assert_eq!(expected, result, "Failed get expected result");
}

#[test]
fn test_check_length_weights_length_exceeds_min_allowed() {
    let (mut deps, env) = instantiate_contract();

    let netuid = 2;
    let tempo: u16 = 13;
    add_network(&mut deps.storage, netuid, tempo, 0);

    let hotkey_account = Addr::unchecked("addr1");

    let max_allowed: u16 = 3;
    let min_allowed_weights = max_allowed;

    set_min_allowed_weights(&mut deps.storage, netuid, min_allowed_weights);

    let uids: Vec<u16> = Vec::from_iter((0..max_allowed).map(|id| id + 1));
    let uid: u16 = uids[0].clone();
    let weights: Vec<u16> = Vec::from_iter((0..max_allowed).map(|id| id + 1));

    let expected = true;

    let result = check_length(&deps.storage, netuid, uid, &uids, &weights);

    assert_eq!(expected, result, "Failed get expected result");
}

#[test]
fn test_check_length_to_few_weights() {
    let (mut deps, env) = instantiate_contract();

    let netuid = 2;
    let tempo: u16 = 13;
    add_network(&mut deps.storage, netuid, tempo, 0);

    let min_allowed_weights = 3;

    set_max_registrations_per_block(&mut deps.storage, netuid, 100);
    set_target_registrations_per_interval(&mut deps.storage, netuid, 100);

    // register morw than min allowed
    register_ok_neuron(deps.as_mut(), env.clone(), netuid, "addr1", "addr1", 300001);
    register_ok_neuron(deps.as_mut(), env.clone(), netuid, "addr2", "addr2", 300002);
    register_ok_neuron(deps.as_mut(), env.clone(), netuid, "addr3", "addr3", 300003);
    register_ok_neuron(deps.as_mut(), env.clone(), netuid, "addr4", "addr4", 300004);
    register_ok_neuron(deps.as_mut(), env.clone(), netuid, "addr5", "addr5", 300005);
    register_ok_neuron(deps.as_mut(), env.clone(), netuid, "addr6", "addr6", 300006);
    register_ok_neuron(deps.as_mut(), env.clone(), netuid, "addr7", "addr7", 300007);

    set_min_allowed_weights(&mut deps.storage, netuid, min_allowed_weights);

    let uids: Vec<u16> = Vec::from_iter((0..2).map(|id| id + 1));
    let weights: Vec<u16> = Vec::from_iter((0..2).map(|id| id + 1));
    let uid: u16 = uids[0].clone();

    let expected = false;
    let result = check_length(&deps.storage, netuid, uid, &uids, &weights);
    assert_eq!(expected, result, "Failed get expected result");
}

#[test]
fn test_normalize_weights_does_not_mutate_when_sum_is_zero() {
    let max_allowed: u16 = 3;

    let weights: Vec<u16> = Vec::from_iter((0..max_allowed).map(|_| 0));

    let expected = weights.clone();
    let result = normalize_weights(weights);

    assert_eq!(
        expected, result,
        "Failed get expected result when everything _should_ be fine"
    );
}

#[test]
fn test_normalize_weights_does_not_mutate_when_sum_not_zero() {
    let max_allowed: u16 = 3;

    let weights: Vec<u16> = Vec::from_iter((0..max_allowed).map(|weight| weight));

    let expected = weights.clone();
    let result = normalize_weights(weights);

    assert_eq!(expected.len(), result.len(), "Length of weights changed?!");
}

#[test]
fn test_max_weight_limited_allow_self_weights_to_exceed_max_weight_limit() {
    let (mut deps, env) = instantiate_contract();

    let netuid = 2;
    let tempo: u16 = 13;
    add_network(&mut deps.storage, netuid, tempo, 0);

    let max_allowed: u16 = 1;

    let uids: Vec<u16> = Vec::from_iter((0..max_allowed).map(|id| id + 1));
    let uid: u16 = uids[0].clone();
    let weights: Vec<u16> = vec![0];

    let expected = true;
    let result = max_weight_limited(&deps.storage, netuid, uid, &uids, &weights);

    assert_eq!(
        expected, result,
        "Failed get expected result when everything _should_ be fine"
    );
}

#[test]
fn test_max_weight_limited_when_weight_limit_is_u16_max() {
    let (mut deps, env) = instantiate_contract();

    let netuid = 2;
    let tempo: u16 = 13;
    add_network(&mut deps.storage, netuid, tempo, 0);

    let max_allowed: u16 = 3;

    let uids: Vec<u16> = Vec::from_iter((0..max_allowed).map(|id| id + 1));
    let uid: u16 = uids[0].clone();
    let weights: Vec<u16> = Vec::from_iter((0..max_allowed).map(|_id| u16::MAX));

    let expected = true;

    let result = max_weight_limited(&deps.storage, netuid, uid, &uids, &weights);

    assert_eq!(
        expected, result,
        "Failed get expected result when everything _should_ be fine"
    );
}

#[test]
fn test_max_weight_limited_when_max_weight_is_within_limit() {
    let (mut deps, env) = instantiate_contract();

    let netuid = 2;
    let tempo: u16 = 13;
    add_network(&mut deps.storage, netuid, tempo, 0);

    let max_allowed: u16 = 1;
    let max_weight_limit = u16::MAX / 5;

    let uids: Vec<u16> = Vec::from_iter((0..max_allowed).map(|id| id + 1));
    let uid: u16 = uids[0].clone();
    let weights: Vec<u16> = Vec::from_iter((0..max_allowed).map(|id| max_weight_limit - id));

    set_max_weight_limit(&mut deps.storage, netuid, max_weight_limit);

    let expected = true;
    let result = max_weight_limited(&deps.storage, netuid, uid, &uids, &weights);
    assert_eq!(
        expected, result,
        "Failed get expected result when everything _should_ be fine"
    );
}

#[test]
fn test_max_weight_limited_when_guard_checks_are_not_triggered() {
    let (mut deps, env) = instantiate_contract();

    let netuid = 2;
    let tempo: u16 = 13;
    add_network(&mut deps.storage, netuid, tempo, 0);

    let max_allowed: u16 = 3;
    let max_weight_limit = u16::MAX / 5;

    let netuid: u16 = 1;
    let uids: Vec<u16> = Vec::from_iter((0..max_allowed).map(|id| id + 1));
    let uid: u16 = uids[0].clone();
    let weights: Vec<u16> = Vec::from_iter((0..max_allowed).map(|id| max_weight_limit + id));

    set_max_weight_limit(&mut deps.storage, netuid, max_weight_limit);

    let expected = false;
    let result = max_weight_limited(&deps.storage, netuid, uid, &uids, &weights);

    assert_eq!(
        expected, result,
        "Failed get expected result when guard-checks were not triggered"
    );
}

#[test]
fn test_is_self_weight_weights_length_not_one() {
    let max_allowed: u16 = 3;

    let uids: Vec<u16> = Vec::from_iter((0..max_allowed).map(|id| id + 1));
    let uid: u16 = uids[0].clone();
    let weights: Vec<u16> = Vec::from_iter((0..max_allowed).map(|id| id + 1));

    let expected = false;
    let result = is_self_weight(uid, &uids, &weights);

    assert_eq!(
        expected, result,
        "Failed get expected result when `weights.len() != 1`"
    );
}

#[test]
fn test_is_self_weight_uid_not_in_uids() {
    let max_allowed: u16 = 3;

    let uids: Vec<u16> = Vec::from_iter((0..max_allowed).map(|id| id + 1));
    let uid: u16 = uids[1].clone();
    let weights: Vec<u16> = vec![0];

    let expected = false;
    let result = is_self_weight(uid, &uids, &weights);

    assert_eq!(
        expected, result,
        "Failed get expected result when `uid != uids[0]`"
    );
}

#[test]
fn test_is_self_weight_uid_in_uids() {
    let max_allowed: u16 = 1;

    let uids: Vec<u16> = Vec::from_iter((0..max_allowed).map(|id| id + 1));
    let uid: u16 = uids[0].clone();
    let weights: Vec<u16> = vec![0];

    let expected = true;
    let result = is_self_weight(uid, &uids, &weights);

    assert_eq!(
        expected, result,
        "Failed get expected result when everything _should_ be fine"
    );
}

#[test]
fn test_check_len_uids_within_allowed_within_network_pool() {
    let (mut deps, env) = instantiate_contract();

    let netuid = 2;
    let tempo: u16 = 13;
    add_network(&mut deps.storage, netuid, tempo, 0);

    let max_registrations_per_block: u16 = 100;

    register_ok_neuron(deps.as_mut(), env.clone(), netuid, "addr1", "addr1", 0);
    register_ok_neuron(deps.as_mut(), env.clone(), netuid, "addr3", "addr3", 65555);
    register_ok_neuron(deps.as_mut(), env.clone(), netuid, "addr5", "addr5", 75555);

    let max_allowed = get_subnetwork_n(&deps.storage, netuid);

    let uids: Vec<u16> = Vec::from_iter((0..max_allowed).map(|uid| uid));

    let expected = true;

    let result = check_len_uids_within_allowed(&deps.storage, netuid, &uids);
    assert_eq!(
        expected, result,
        "netuid network length and uids length incompatible"
    );
}

#[test]
fn test_check_len_uids_within_allowed_not_within_network_pool() {
    let (mut deps, env) = instantiate_contract();

    let netuid = 2;
    let tempo: u16 = 13;
    add_network(&mut deps.storage, netuid, tempo, 0);

    let max_registrations_per_block: u16 = 100;

    register_ok_neuron(deps.as_mut(), env.clone(), netuid, "addr1", "addr1", 0);
    register_ok_neuron(deps.as_mut(), env.clone(), netuid, "addr3", "addr3", 65555);
    register_ok_neuron(deps.as_mut(), env.clone(), netuid, "addr5", "addr5", 75555);

    let max_allowed = get_subnetwork_n(&deps.storage, netuid);

    let max_default_allowed = 256; // set during add_network as default
    let uids: Vec<u16> = Vec::from_iter((0..(max_default_allowed + 1)).map(|uid| uid));

    let expected = false;
    let result = check_len_uids_within_allowed(&deps.storage, netuid, &uids);
    assert_eq!(
        expected, result,
        "Failed to detect incompatible uids for network"
    );
}
