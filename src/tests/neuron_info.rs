use cosmwasm_std::Addr;

use crate::neuron_info::{get_neuron, get_neurons};
use crate::test_helpers::{add_network, instantiate_contract, register_ok_neuron};

#[test]
fn test_get_neuron_none() {
    let (deps, _) = instantiate_contract();

    let netuid: u16 = 1;
    let uid: u16 = 42;

    let neuron = get_neuron(&deps.storage, netuid, uid).unwrap();
    assert_eq!(neuron, None);
}

#[test]
#[cfg(not(tarpaulin))]
fn test_get_neuron_some() {
    let (mut deps, env) = instantiate_contract();
    let netuid: u16 = 1;

    let tempo: u16 = 2;
    let modality: u16 = 2;

    let uid: u16 = 0;
    let hotkey0 = "addr0";
    let coldkey0 = "addr0";

    add_network(&mut deps.storage, netuid, tempo, modality);
    assert_eq!(
        register_ok_neuron(deps.as_mut(), env, netuid, &hotkey0, &coldkey0, 39420842).is_ok(),
        true
    );

    let neuron = get_neuron(&deps.storage, netuid, uid).unwrap();
    assert_ne!(neuron, None);
}

/* @TODO: Add more neurons to list */
#[test]
fn test_get_neurons_list() {
    let (mut deps, env) = instantiate_contract();
    let netuid: u16 = 1;

    let tempo: u16 = 2;
    let modality: u16 = 2;

    add_network(&mut deps.storage, netuid, tempo, modality);

    let _uid: u16 = 42;

    let neuron_count = 1;
    for index in 0..neuron_count {
        let hotkey = Addr::unchecked((1000 + index).to_string());
        let coldkey = Addr::unchecked((2000 + index).to_string());
        let nonce: u64 = 39420842 + index;
        assert_eq!(
            register_ok_neuron(
                deps.as_mut(),
                env.clone(),
                netuid,
                hotkey.as_str(),
                coldkey.as_str(),
                nonce,
            )
            .is_ok(),
            true
        );
    }

    let neurons = get_neurons(&deps.storage, netuid).unwrap();
    assert_eq!(neurons.len(), neuron_count as usize);
}

#[test]
fn test_get_neurons_empty() {
    let (deps, _) = instantiate_contract();
    let netuid: u16 = 1;

    let neuron_count = 0;
    let neurons = get_neurons(&deps.storage, netuid).unwrap();
    assert_eq!(neurons.len(), neuron_count as usize);
}
