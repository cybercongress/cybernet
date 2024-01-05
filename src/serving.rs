use cosmwasm_std::{ensure, Addr, DepsMut, Env, MessageInfo, Storage};

use crate::state::{AxonInfo, AxonInfoOf, PrometheusInfo, PrometheusInfoOf, AXONS, PROMETHEUS};
use crate::uids::is_hotkey_registered_on_any_network;
use crate::utils::get_serving_rate_limit;
use crate::ContractError;
use cyber_std::Response;

// ---- The implementation for the extrinsic serve_axon which sets the ip endpoint information for a uid on a network.
//
// # Args:
// 	* 'origin': (<T as frame_system::Config>RuntimeOrigin):
// 		- The signature of the caller.
//
// 	* 'netuid' (u16):
// 		- The u16 network identifier.
//
// 	* 'version' (u64):
// 		- The bittensor version identifier.
//
// 	* 'ip' (u64):
// 		- The endpoint ip information as a u128 encoded integer.
//
// 	* 'port' (u16):
// 		- The endpoint port information as a u16 encoded integer.
//
// 	* 'ip_type' (u8):
// 		- The endpoint ip version as a u8, 4 or 6.
//
// 	* 'protocol' (u8):
// 		- UDP:1 or TCP:0
//
// 	* 'placeholder1' (u8):
// 		- Placeholder for further extra params.
//
// 	* 'placeholder2' (u8):
// 		- Placeholder for further extra params.
//
// # Event:
// 	* AxonServed;
// 		- On successfully serving the axon info.
//
// # Raises:
// 	* 'NetworkDoesNotExist':
// 		- Attempting to set weights on a non-existent network.
//
// 	* 'NotRegistered':
// 		- Attempting to set weights from a non registered account.
//
// 	* 'InvalidIpType':
// 		- The ip type is not 4 or 6.
//
// 	* 'InvalidIpAddress':
// 		- The numerically encoded ip address does not resolve to a proper ip.
//
// 	* 'ServingRateLimitExceeded':
// 		- Attempting to set prometheus information withing the rate limit min.
//
pub fn do_serve_axon(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    netuid: u16,
    version: u32,
    ip: u128,
    port: u16,
    ip_type: u8,
    protocol: u8,
    placeholder1: u8,
    placeholder2: u8,
) -> Result<Response, ContractError> {
    // --- 1. We check the callers (hotkey) signature.
    let hotkey_id = info.sender;

    // --- 2. Ensure the hotkey is registered somewhere.
    ensure!(
        is_hotkey_registered_on_any_network(deps.storage, &hotkey_id),
        ContractError::NotRegistered {}
    );

    // --- 3. Check the ip signature validity.
    ensure!(is_valid_ip_type(ip_type), ContractError::InvalidIpType {});
    ensure!(
        is_valid_ip_address(ip_type, ip),
        ContractError::InvalidIpAddress {}
    );

    // --- 4. Get the previous axon information.
    let mut prev_axon = get_axon_info(deps.storage, netuid, &hotkey_id.clone());
    ensure!(
        axon_passes_rate_limit(deps.storage, netuid, &prev_axon, env.block.height),
        ContractError::ServingRateLimitExceeded {}
    );

    // --- 6. We insert the axon meta.
    prev_axon.block = env.block.height;
    prev_axon.version = version;
    prev_axon.ip = ip;
    prev_axon.port = port;
    prev_axon.ip_type = ip_type;
    prev_axon.protocol = protocol;
    prev_axon.placeholder1 = placeholder1;
    prev_axon.placeholder2 = placeholder2;

    // --- 7. Validate axon data with delegate func
    let axon_validated = validate_axon_data(&prev_axon);
    ensure!(
        axon_validated.is_ok(),
        axon_validated
            .err()
            .unwrap_or(ContractError::InvalidPort {})
    );

    AXONS.save(deps.storage, (netuid, &hotkey_id), &prev_axon)?;

    // --- 8. We deposit axon served event.
    deps.api.debug(&format!(
        "📡 AxonServed ( hotkey:{:?} ) ",
        hotkey_id.clone()
    ));

    // --- 9. Return is successful dispatch.
    Ok(Response::default()
        .add_attribute("action", "axon_served")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute("hotkey_id", format!("{}", hotkey_id)))
}

// ---- The implementation for the extrinsic serve_prometheus.
//
// # Args:
// 	* 'origin': (<T as frame_system::Config>RuntimeOrigin):
// 		- The signature of the caller.
//
// 	* 'netuid' (u16):
// 		- The u16 network identifier.
//
// 	* 'version' (u64):
// 		- The bittensor version identifier.
//
// 	* 'ip' (u64):
// 		- The prometheus ip information as a u128 encoded integer.
//
// 	* 'port' (u16):
// 		- The prometheus port information as a u16 encoded integer.
//
// 	* 'ip_type' (u8):
// 		- The prometheus ip version as a u8, 4 or 6.
//
// # Event:
// 	* PrometheusServed;
// 		- On successfully serving the axon info.
//
// # Raises:
// 	* 'NetworkDoesNotExist':
// 		- Attempting to set weights on a non-existent network.
//
// 	* 'NotRegistered':
// 		- Attempting to set weights from a non registered account.
//
// 	* 'InvalidIpType':
// 		- The ip type is not 4 or 6.
//
// 	* 'InvalidIpAddress':
// 		- The numerically encoded ip address does not resolve to a proper ip.
//
// 	* 'ServingRateLimitExceeded':
// 		- Attempting to set prometheus information withing the rate limit min.
//
pub fn do_serve_prometheus(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    netuid: u16,
    version: u32,
    ip: u128,
    port: u16,
    ip_type: u8,
) -> Result<Response, ContractError> {
    // --- 1. We check the callers (hotkey) signature.
    let hotkey_id = info.sender;

    // --- 2. Ensure the hotkey is registered somewhere.
    ensure!(
        is_hotkey_registered_on_any_network(deps.storage, &hotkey_id),
        ContractError::NotRegistered {}
    );

    // --- 3. Check the ip signature validity.
    ensure!(is_valid_ip_type(ip_type), ContractError::InvalidIpType {});
    ensure!(
        is_valid_ip_address(ip_type, ip),
        ContractError::InvalidIpAddress {}
    );

    // --- 5. We get the previous axon info assoicated with this ( netuid, uid )
    let mut prev_prometheus = get_prometheus_info(deps.storage, netuid, &hotkey_id.clone());
    ensure!(
        prometheus_passes_rate_limit(deps.storage, netuid, &prev_prometheus, env.block.height),
        ContractError::ServingRateLimitExceeded {}
    );

    // --- 6. We insert the prometheus meta.
    prev_prometheus.block = env.block.height;
    prev_prometheus.version = version;
    prev_prometheus.ip = ip;
    prev_prometheus.port = port;
    prev_prometheus.ip_type = ip_type;

    // --- 7. Validate prometheus data with delegate func
    let prom_validated = validate_prometheus_data(&prev_prometheus);
    if prom_validated.is_err() {
        return Err(prom_validated
            .err()
            .unwrap_or(ContractError::InvalidPort {}));
    }

    // --- 8. Insert new prometheus data
    PROMETHEUS.save(deps.storage, (netuid, &hotkey_id), &prev_prometheus)?;

    // --- 9. We deposit prometheus served event.
    deps.api.debug(&format!(
        "📡 PrometheusServed ( hotkey:{:?} ) ",
        hotkey_id.clone()
    ));

    // --- 10. Return is successful dispatch.
    Ok(
        Response::default().add_attribute("action", "prometheus_served"), // .add_attribute("netuid", netuid)
                                                                          // .add_attribute("hotkey_id", hotkey_id)
    )
}

/********************************
 --==[[  Helper functions   ]]==--
*********************************/

pub fn axon_passes_rate_limit(
    store: &dyn Storage,
    netuid: u16,
    prev_axon_info: &AxonInfoOf,
    current_block: u64,
) -> bool {
    let rate_limit: u64 = get_serving_rate_limit(store, netuid);
    let last_serve = prev_axon_info.block;
    return rate_limit == 0 || last_serve == 0 || current_block - last_serve >= rate_limit;
}

pub fn has_axon_info(store: &dyn Storage, netuid: u16, hotkey: &Addr) -> bool {
    return AXONS.has(store, (netuid, &hotkey));
}

pub fn prometheus_passes_rate_limit(
    store: &dyn Storage,
    netuid: u16,
    prev_prometheus_info: &PrometheusInfoOf,
    current_block: u64,
) -> bool {
    let rate_limit: u64 = get_serving_rate_limit(store, netuid);
    let last_serve = prev_prometheus_info.block;
    return rate_limit == 0 || last_serve == 0 || current_block - last_serve >= rate_limit;
}

pub fn get_axon_info(store: &dyn Storage, netuid: u16, hotkey: &Addr) -> AxonInfoOf {
    return if has_axon_info(store, netuid, hotkey) {
        AXONS.load(store, (netuid, &hotkey)).unwrap()
    } else {
        AxonInfo {
            block: 0,
            version: 0,
            ip: 0,
            port: 0,
            ip_type: 0,
            protocol: 0,
            placeholder1: 0,
            placeholder2: 0,
        }
    };
}

pub fn has_prometheus_info(store: &dyn Storage, netuid: u16, hotkey: &Addr) -> bool {
    return PROMETHEUS.has(store, (netuid, &hotkey));
}

pub fn get_prometheus_info(store: &dyn Storage, netuid: u16, hotkey: &Addr) -> PrometheusInfoOf {
    return if has_prometheus_info(store, netuid, hotkey) {
        PROMETHEUS.load(store, (netuid, &hotkey)).unwrap()
    } else {
        PrometheusInfo {
            block: 0,
            version: 0,
            ip: 0,
            port: 0,
            ip_type: 0,
        }
    };
}

pub fn is_valid_ip_type(ip_type: u8) -> bool {
    let allowed_values: Vec<u8> = vec![4, 6];
    return allowed_values.contains(&ip_type);
}

// @todo (Parallax 2-1-2021) : Implement exclusion of private IP ranges
pub fn is_valid_ip_address(ip_type: u8, addr: u128) -> bool {
    if !is_valid_ip_type(ip_type) {
        return false;
    }
    if addr == 0 {
        return false;
    }
    if ip_type == 4 {
        if addr == 0 {
            return false;
        }
        if addr >= u32::MAX as u128 {
            return false;
        }
        if addr == 0x7f000001 {
            return false;
        } // Localhost
    }
    if ip_type == 6 {
        if addr == 0x0 {
            return false;
        }
        if addr == u128::MAX {
            return false;
        }
        if addr == 1 {
            return false;
        } // IPv6 localhost
    }
    return true;
}

pub fn validate_axon_data(axon_info: &AxonInfoOf) -> Result<bool, ContractError> {
    if axon_info.port.clamp(0, u16::MAX) <= 0 {
        return Err(ContractError::InvalidPort {});
    }

    Ok(true)
}

pub fn validate_prometheus_data(prom_info: &PrometheusInfoOf) -> Result<bool, ContractError> {
    if prom_info.port.clamp(0, u16::MAX) <= 0 {
        return Err(ContractError::InvalidPort {});
    }

    Ok(true)
}
