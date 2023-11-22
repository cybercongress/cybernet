use crate::serving::{get_axon_info, get_prometheus_info, is_valid_ip_address, is_valid_ip_type};
use crate::test_helpers::{
    add_network, instantiate_contract, register_ok_neuron, run_step_to_block, serve_axon,
    serve_prometheus, step_block,
};
use crate::utils::set_serving_rate_limit;
use crate::ContractError;
use cosmwasm_std::Addr;

mod test {
    use std::net::{Ipv4Addr, Ipv6Addr};

    // Generates an ipv6 address based on 8 ipv6 words and returns it as u128
    pub fn ipv6(a: u16, b: u16, c: u16, d: u16, e: u16, f: u16, g: u16, h: u16) -> u128 {
        return Ipv6Addr::new(a, b, c, d, e, f, g, h).into();
    }

    // Generate an ipv4 address based on 4 bytes and returns the corresponding u128, so it can be fed
    // to the module::subscribe() function
    pub fn ipv4(a: u8, b: u8, c: u8, d: u8) -> u128 {
        let ipv4: Ipv4Addr = Ipv4Addr::new(a, b, c, d);
        let integer: u32 = ipv4.into();
        return u128::from(integer);
    }
}
#[test]
fn test_serving_ok() {
    let (mut deps, mut env) = instantiate_contract();

    let hotkey_account_id = "addr1";
    let netuid: u16 = 1;
    let tempo: u16 = 13;
    let version: u32 = 2;
    let ip: u128 = 1676056785;
    let port: u16 = 128;
    let ip_type: u8 = 4;
    let modality: u16 = 0;
    let protocol: u8 = 0;
    let placeholder1: u8 = 0;
    let placeholder2: u8 = 0;

    add_network(&mut deps.storage, netuid, tempo, modality);
    register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        hotkey_account_id,
        "addr66",
        0,
    );

    let result = serve_axon(
        deps.as_mut(),
        env.clone(),
        hotkey_account_id,
        netuid,
        version,
        ip,
        port,
        ip_type,
        protocol,
        placeholder1,
        placeholder2,
    );
    assert!(result.is_ok());

    let neuron = get_axon_info(&deps.storage, netuid, &Addr::unchecked(hotkey_account_id));
    assert_eq!(neuron.ip, ip);
    assert_eq!(neuron.version, version);
    assert_eq!(neuron.port, port);
    assert_eq!(neuron.ip_type, ip_type);
    assert_eq!(neuron.protocol, protocol);
    assert_eq!(neuron.placeholder1, placeholder1);
    assert_eq!(neuron.placeholder2, placeholder2);
}

#[test]
fn test_serving_set_metadata_update() {
    let (mut deps, mut env) = instantiate_contract();

    let hotkey_account_id = "addr1";
    let netuid: u16 = 1;
    let tempo: u16 = 13;
    let version: u32 = 2;
    let ip: u128 = 1676056785;
    let port: u16 = 128;
    let ip_type: u8 = 4;
    let modality: u16 = 0;
    let protocol: u8 = 0;
    let placeholder1: u8 = 0;
    let placeholder2: u8 = 0;

    add_network(&mut deps.storage, netuid, tempo, modality);
    register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        hotkey_account_id,
        "addr66",
        0,
    );

    set_serving_rate_limit(&mut deps.storage, netuid, 0);

    let result = serve_axon(
        deps.as_mut(),
        env.clone(),
        hotkey_account_id,
        netuid,
        version,
        ip,
        port,
        ip_type,
        protocol,
        placeholder1,
        placeholder2,
    );
    assert!(result.is_ok());

    let neuron = get_axon_info(&deps.storage, netuid, &Addr::unchecked(hotkey_account_id));
    assert_eq!(neuron.ip, ip);
    assert_eq!(neuron.version, version);
    assert_eq!(neuron.port, port);
    assert_eq!(neuron.ip_type, ip_type);
    assert_eq!(neuron.protocol, protocol);
    assert_eq!(neuron.placeholder1, placeholder1);
    assert_eq!(neuron.placeholder2, placeholder2);

    let version2: u32 = version + 1;
    let ip2: u128 = ip + 1;
    let port2: u16 = port + 1;
    let ip_type2: u8 = 6;
    let protocol2: u8 = protocol + 1;
    let placeholder12: u8 = placeholder1 + 1;
    let placeholder22: u8 = placeholder2 + 1;
    let result = serve_axon(
        deps.as_mut(),
        env.clone(),
        hotkey_account_id,
        netuid,
        version2,
        ip2,
        port2,
        ip_type2,
        protocol2,
        placeholder12,
        placeholder22,
    );
    assert!(result.is_ok());

    let neuron = get_axon_info(&deps.storage, netuid, &Addr::unchecked(hotkey_account_id));
    assert_eq!(neuron.ip, ip2);
    assert_eq!(neuron.version, version2);
    assert_eq!(neuron.port, port2);
    assert_eq!(neuron.ip_type, ip_type2);
    assert_eq!(neuron.protocol, protocol2);
    assert_eq!(neuron.placeholder1, placeholder12);
    assert_eq!(neuron.placeholder2, placeholder22);
}

#[test]
#[cfg(not(tarpaulin))]
fn test_axon_serving_rate_limit_exceeded() {
    let (mut deps, mut env) = instantiate_contract();

    let hotkey_account_id = "addr1";
    let netuid: u16 = 1;
    let tempo: u16 = 13;
    let version: u32 = 2;
    let ip: u128 = 1676056785;
    let port: u16 = 128;
    let ip_type: u8 = 4;
    let modality: u16 = 0;
    let protocol: u8 = 0;
    let placeholder1: u8 = 0;
    let placeholder2: u8 = 0;

    add_network(&mut deps.storage, netuid, tempo, modality);
    register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        hotkey_account_id,
        "addr66",
        0,
    );
    step_block(deps.as_mut(), &mut env).unwrap(); // Go to block 1

    set_serving_rate_limit(&mut deps.storage, netuid, 0);

    // No issue on multiple
    let result = serve_axon(
        deps.as_mut(),
        env.clone(),
        hotkey_account_id,
        netuid,
        version,
        ip,
        port,
        ip_type,
        protocol,
        placeholder1,
        placeholder2,
    );
    assert!(result.is_ok());

    let result = serve_axon(
        deps.as_mut(),
        env.clone(),
        hotkey_account_id,
        netuid,
        version,
        ip,
        port,
        ip_type,
        protocol,
        placeholder1,
        placeholder2,
    );
    assert!(result.is_ok());

    let result = serve_axon(
        deps.as_mut(),
        env.clone(),
        hotkey_account_id,
        netuid,
        version,
        ip,
        port,
        ip_type,
        protocol,
        placeholder1,
        placeholder2,
    );
    assert!(result.is_ok());

    let result = serve_axon(
        deps.as_mut(),
        env.clone(),
        hotkey_account_id,
        netuid,
        version,
        ip,
        port,
        ip_type,
        protocol,
        placeholder1,
        placeholder2,
    );
    assert!(result.is_ok());

    set_serving_rate_limit(&mut deps.storage, netuid, 2);
    run_step_to_block(deps.as_mut(), &mut env, 3).unwrap();

    // Needs to be 2 blocks apart, we are only 1 block apart
    let result = serve_axon(
        deps.as_mut(),
        env.clone(),
        hotkey_account_id,
        netuid,
        version,
        ip,
        port,
        ip_type,
        protocol,
        placeholder1,
        placeholder2,
    );
    assert_eq!(
        result.unwrap_err(),
        ContractError::ServingRateLimitExceeded {}
    );
}

#[test]
fn test_axon_invalid_port() {
    let (mut deps, mut env) = instantiate_contract();

    let hotkey_account_id = "addr1";
    let netuid: u16 = 1;
    let tempo: u16 = 13;
    let version: u32 = 2;
    let ip: u128 = 1676056785;
    let port: u16 = 0;
    let ip_type: u8 = 4;
    let modality: u16 = 0;
    let protocol: u8 = 0;
    let placeholder1: u8 = 0;
    let placeholder2: u8 = 0;

    add_network(&mut deps.storage, netuid, tempo, modality);
    register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        hotkey_account_id,
        "addr66",
        0,
    );

    step_block(deps.as_mut(), &mut env).unwrap(); // Go to block 1

    let result = serve_axon(
        deps.as_mut(),
        env.clone(),
        hotkey_account_id,
        netuid,
        version,
        ip,
        port,
        ip_type,
        protocol,
        placeholder1,
        placeholder2,
    );
    assert_eq!(result.unwrap_err(), ContractError::InvalidPort {});
}

#[test]
fn test_prometheus_serving_ok() {
    let (mut deps, mut env) = instantiate_contract();

    let hotkey_account_id = "addr1";
    let netuid: u16 = 1;
    let tempo: u16 = 13;
    let version: u32 = 2;
    let ip: u128 = 1676056785;
    let port: u16 = 128;
    let ip_type: u8 = 4;
    let modality: u16 = 0;

    add_network(&mut deps.storage, netuid, tempo, modality);
    register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        hotkey_account_id,
        "addr66",
        0,
    );

    let result = serve_prometheus(
        deps.as_mut(),
        env.clone(),
        hotkey_account_id,
        netuid,
        version,
        ip,
        port,
        ip_type,
    );
    assert!(result.is_ok());

    let neuron = get_prometheus_info(&deps.storage, netuid, &Addr::unchecked(hotkey_account_id));
    assert_eq!(neuron.ip, ip);
    assert_eq!(neuron.version, version);
    assert_eq!(neuron.port, port);
    assert_eq!(neuron.ip_type, ip_type);
}

#[test]
fn test_prometheus_serving_set_metadata_update() {
    let (mut deps, mut env) = instantiate_contract();

    let hotkey_account_id = "addr1";
    let netuid: u16 = 1;
    let tempo: u16 = 13;
    let version: u32 = 2;
    let ip: u128 = 1676056785;
    let port: u16 = 128;
    let ip_type: u8 = 4;
    let modality: u16 = 0;

    add_network(&mut deps.storage, netuid, tempo, modality);
    register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        hotkey_account_id,
        "addr66",
        0,
    );

    set_serving_rate_limit(&mut deps.storage, netuid, 0);

    let result = serve_prometheus(
        deps.as_mut(),
        env.clone(),
        hotkey_account_id,
        netuid,
        version,
        ip,
        port,
        ip_type,
    );
    assert!(result.is_ok());

    let neuron = get_prometheus_info(&deps.storage, netuid, &Addr::unchecked(hotkey_account_id));
    assert_eq!(neuron.ip, ip);
    assert_eq!(neuron.version, version);
    assert_eq!(neuron.port, port);
    assert_eq!(neuron.ip_type, ip_type);

    let version2: u32 = version + 1;
    let ip2: u128 = ip + 1;
    let port2: u16 = port + 1;
    let ip_type2: u8 = 6;

    let result = serve_prometheus(
        deps.as_mut(),
        env.clone(),
        hotkey_account_id,
        netuid,
        version2,
        ip2,
        port2,
        ip_type2,
    );
    assert!(result.is_ok());

    let neuron = get_prometheus_info(&deps.storage, netuid, &Addr::unchecked(hotkey_account_id));
    assert_eq!(neuron.ip, ip2);
    assert_eq!(neuron.version, version2);
    assert_eq!(neuron.port, port2);
    assert_eq!(neuron.ip_type, ip_type2);
}

#[test]
#[cfg(not(tarpaulin))]
fn test_prometheus_serving_rate_limit_exceeded() {
    let (mut deps, mut env) = instantiate_contract();

    let hotkey_account_id = "addr1";
    let netuid: u16 = 1;
    let tempo: u16 = 13;
    let version: u32 = 2;
    let ip: u128 = 1676056785;
    let port: u16 = 128;
    let ip_type: u8 = 4;
    let modality: u16 = 0;

    add_network(&mut deps.storage, netuid, tempo, modality);
    register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        hotkey_account_id,
        "addr66",
        0,
    );
    step_block(deps.as_mut(), &mut env).unwrap(); // Go to block 1

    set_serving_rate_limit(&mut deps.storage, netuid, 0);

    // No issue on multiple
    let result = serve_prometheus(
        deps.as_mut(),
        env.clone(),
        hotkey_account_id,
        netuid,
        version,
        ip,
        port,
        ip_type,
    );
    assert!(result.is_ok());

    let result = serve_prometheus(
        deps.as_mut(),
        env.clone(),
        hotkey_account_id,
        netuid,
        version,
        ip,
        port,
        ip_type,
    );
    assert!(result.is_ok());

    let result = serve_prometheus(
        deps.as_mut(),
        env.clone(),
        hotkey_account_id,
        netuid,
        version,
        ip,
        port,
        ip_type,
    );
    assert!(result.is_ok());

    let result = serve_prometheus(
        deps.as_mut(),
        env.clone(),
        hotkey_account_id,
        netuid,
        version,
        ip,
        port,
        ip_type,
    );
    assert!(result.is_ok());

    set_serving_rate_limit(&mut deps.storage, netuid, 1);
    // Same block, need 1 block to pass
    let result = serve_prometheus(
        deps.as_mut(),
        env.clone(),
        hotkey_account_id,
        netuid,
        version,
        ip,
        port,
        ip_type,
    );
    assert_eq!(
        result.unwrap_err(),
        ContractError::ServingRateLimitExceeded {}
    );
}

#[test]
fn test_prometheus_invalid_port() {
    let (mut deps, mut env) = instantiate_contract();

    let hotkey_account_id = "addr1";
    let netuid: u16 = 1;
    let tempo: u16 = 13;
    let version: u32 = 2;
    let ip: u128 = 1676056785;
    let port: u16 = 0;
    let ip_type: u8 = 4;
    let modality: u16 = 0;

    add_network(&mut deps.storage, netuid, tempo, modality);
    register_ok_neuron(
        deps.as_mut(),
        env.clone(),
        netuid,
        hotkey_account_id,
        "addr66",
        0,
    );
    step_block(deps.as_mut(), &mut env).unwrap(); // Go to block 1

    let result = serve_prometheus(
        deps.as_mut(),
        env.clone(),
        hotkey_account_id,
        netuid,
        version,
        ip,
        port,
        ip_type,
    );
    assert_eq!(result.unwrap_err(), ContractError::InvalidPort {});
}

#[test]
fn test_serving_is_valid_ip_type_ok_ipv4() {
    let (mut deps, mut env) = instantiate_contract();

    assert_eq!(is_valid_ip_type(4), true);
}

#[test]
fn test_serving_is_valid_ip_type_ok_ipv6() {
    let (mut deps, mut env) = instantiate_contract();

    assert_eq!(is_valid_ip_type(6), true);
}

#[test]
fn test_serving_is_valid_ip_type_nok() {
    let (mut deps, mut env) = instantiate_contract();

    assert_eq!(is_valid_ip_type(10), false);
}

#[test]
fn test_serving_is_valid_ip_address_ipv4() {
    let (mut deps, mut env) = instantiate_contract();
    assert_eq!(is_valid_ip_address(4, test::ipv4(8, 8, 8, 8)), true);
}

#[test]
fn test_serving_is_valid_ip_address_ipv6() {
    let (mut deps, mut env) = instantiate_contract();

    assert_eq!(
        is_valid_ip_address(6, test::ipv6(1, 2, 3, 4, 5, 6, 7, 8)),
        true
    );
    assert_eq!(
        is_valid_ip_address(6, test::ipv6(1, 2, 3, 4, 5, 6, 7, 8)),
        true
    );
}

#[test]
fn test_serving_is_invalid_ipv4_address() {
    let (mut deps, mut env) = instantiate_contract();

    assert_eq!(is_valid_ip_address(4, test::ipv4(0, 0, 0, 0)), false);
    assert_eq!(
        is_valid_ip_address(4, test::ipv4(255, 255, 255, 255)),
        false
    );
    assert_eq!(is_valid_ip_address(4, test::ipv4(127, 0, 0, 1)), false);
    assert_eq!(
        is_valid_ip_address(4, test::ipv6(0xffff, 2, 3, 4, 5, 6, 7, 8)),
        false
    );
}

#[test]
fn test_serving_is_invalid_ipv6_address() {
    let (mut deps, mut env) = instantiate_contract();

    assert_eq!(
        is_valid_ip_address(6, test::ipv6(0, 0, 0, 0, 0, 0, 0, 0)),
        false
    );
    assert_eq!(
        is_valid_ip_address(
            4,
            test::ipv6(0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff)
        ),
        false
    );
}
