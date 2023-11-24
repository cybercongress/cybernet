// ==========================
// ==== Helper functions ====
// ==========================

use cosmwasm_std::{ensure, Api, DepsMut, Env, MessageInfo, Order, Response, StdResult, Storage};

use crate::math::{check_vec_max_limited, vec_u16_max_upscale_to_u16};
use crate::root::{contains_invalid_root_uids, get_root_netuid, if_subnet_exist};
use crate::state::{MIN_ALLOWED_WEIGHTS, WEIGHTS, WEIGHTS_VERSION_KEY};
use crate::uids::{
    get_subnetwork_n, get_uid_for_net_and_hotkey, is_hotkey_registered_on_network,
    is_uid_exist_on_network,
};
use crate::utils::{
    get_last_update_for_uid, get_max_weight_limit, get_validator_permit_for_uid,
    get_weights_set_rate_limit, set_last_update_for_uid,
};
use crate::ContractError;

// ---- The implementation for the extrinsic set_weights.
//
// # Args:
// 	* 'origin': (<T as frame_system::Config>RuntimeOrigin):
// 		- The signature of the calling hotkey.
//
// 	* 'netuid' (u16):
// 		- The u16 network identifier.
//
// 	* 'uids' ( Vec<u16> ):
// 		- The uids of the weights to be set on the chain.
//
// 	* 'values' ( Vec<u16> ):
// 		- The values of the weights to set on the chain.
//
// 	* 'version_key' ( u64 ):
// 		- The network version key.
//
// # Event:
// 	* WeightsSet;
// 		- On successfully setting the weights on chain.
//
// # Raises:
// 	* 'NetworkDoesNotExist':
// 		- Attempting to set weights on a non-existent network.
//
// 	* 'NotRegistered':
// 		- Attempting to set weights from a non registered account.
//
// 	* 'IncorrectNetworkVersionKey':
// 		- Attempting to set weights without having an up-to-date version_key.
//
// 	* 'SettingWeightsTooFast':
// 		- Attempting to set weights faster than the weights_set_rate_limit.
//
// 	* 'NoValidatorPermit':
// 		- Attempting to set non-self weights without a validator permit.
//
// 	* 'WeightVecNotEqualSize':
// 		- Attempting to set weights with uids not of same length.
//
// 	* 'DuplicateUids':
// 		- Attempting to set weights with duplicate uids.
//
//     * 'TooManyUids':
// 		- Attempting to set weights above the max allowed uids.
//
// 	* 'InvalidUid':
// 		- Attempting to set weights with invalid uids.
//
// 	* 'NotSettingEnoughWeights':
// 		- Attempting to set weights with fewer weights than min.
//
// 	* 'MaxWeightExceeded':
// 		- Attempting to set weights with max value exceeding limit.
//
pub fn do_set_weights(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    netuid: u16,
    uids: Vec<u16>,
    values: Vec<u16>,
    version_key: u64,
) -> Result<Response, ContractError> {
    // --- 1. Check the caller's signature. This is the hotkey of a registered account.
    let hotkey = info.sender;
    // deps.api.debug(&format!(
    //     "do_set_weights( origin:{:?} netuid:{:?}, uids:{:?}, values:{:?})",
    //     hotkey,
    //     netuid,
    //     uids,
    //     values
    // ));

    // --- 2. Check that the length of uid list and value list are equal for this network.
    ensure!(
        uids_match_values(&uids, &values),
        ContractError::WeightVecNotEqualSize {}
    );

    // --- 3. Check to see if this is a valid network.
    ensure!(
        if_subnet_exist(deps.storage, netuid),
        ContractError::NetworkDoesNotExist {}
    );

    // --- 4. Check to see if the number of uids is within the max allowed uids for this network.
    // For the root network this number is the number of subnets.
    if netuid == get_root_netuid() {
        // --- 4.a. Ensure that the passed uids are valid for the network.
        ensure!(
            !contains_invalid_root_uids(deps.storage, deps.api, &uids),
            ContractError::InvalidUid {}
        );
    } else {
        ensure!(
            check_len_uids_within_allowed(deps.storage, netuid, &uids),
            ContractError::TooManyUids {}
        );
    }

    // --- 5. Check to see if the hotkey is registered to the passed network.
    ensure!(
        is_hotkey_registered_on_network(deps.storage, netuid, &hotkey),
        ContractError::NotRegistered {}
    );

    // --- 6. Ensure version_key is up-to-date.
    ensure!(
        check_version_key(deps.storage, deps.api, netuid, version_key),
        ContractError::IncorrectNetworkVersionKey {}
    );

    // --- 7. Get the neuron uid of associated hotkey on network netuid.
    let neuron_uid;
    let net_neuron_uid = get_uid_for_net_and_hotkey(deps.storage, netuid, &hotkey);
    ensure!(net_neuron_uid.is_ok(), ContractError::NotRegistered {});

    neuron_uid = net_neuron_uid.unwrap();

    // --- 8. Ensure the uid is not setting weights faster than the weights_set_rate_limit.
    let current_block: u64 = env.block.height;
    ensure!(
        check_rate_limit(deps.storage, netuid, neuron_uid, current_block),
        ContractError::SettingWeightsTooFast {}
    );

    // --- 9. Check that the neuron uid is an allowed validator permitted to set non-self weights.
    if netuid != get_root_netuid() {
        ensure!(
            check_validator_permit(deps.storage, netuid, neuron_uid, &uids, &values),
            ContractError::NoValidatorPermit {}
        );
    }

    // --- 10. Ensure the passed uids contain no duplicates.
    ensure!(!has_duplicate_uids(&uids), ContractError::DuplicateUids {});

    // --- 11. Ensure that the passed uids are valid for the network.
    if netuid != get_root_netuid() {
        ensure!(
            !contains_invalid_uids(deps.storage, deps.api, netuid, &uids),
            ContractError::InvalidUid {}
        );
    }

    // --- 12. Ensure that the weights have the required length.
    ensure!(
        check_length(deps.storage, netuid, neuron_uid, &uids, &values),
        ContractError::NotSettingEnoughWeights {}
    );

    // --- 13. Max-upscale the weights.
    let max_upscaled_weights: Vec<u16> = vec_u16_max_upscale_to_u16(&values);

    // --- 14. Ensure the weights are max weight limited
    ensure!(
        max_weight_limited(
            deps.storage,
            netuid,
            neuron_uid,
            &uids,
            &max_upscaled_weights
        ),
        ContractError::MaxWeightExceeded {}
    );

    // --- 15. Zip weights for sinking to storage map.
    let mut zipped_weights: Vec<(u16, u16)> = vec![];
    for (uid, val) in uids.iter().zip(max_upscaled_weights.iter()) {
        zipped_weights.push((*uid, *val))
    }

    // --- 16. Set weights under netuid, uid double map entry.
    WEIGHTS.save(deps.storage, (netuid, neuron_uid), &zipped_weights)?;

    // --- 17. Set the activity for the weights on this network.
    set_last_update_for_uid(deps.storage, netuid, neuron_uid, current_block);

    // --- 18. Emit the tracking event.
    deps.api.debug(&format!(
        "WeightsSet( netuid:{:?}, neuron_uid:{:?} )",
        netuid, neuron_uid
    ));

    // --- 19. Return ok.
    Ok(Response::default()
        .add_attribute("active", "weights_set")
        .add_attribute("netuid", format!("{}", netuid))
        .add_attribute("neuron_uid", format!("{}", neuron_uid)))
}

// ==========================
// ==== Helper functions ====
// ==========================

// Returns true if version_key is up-to-date.
//
pub fn check_version_key(
    store: &dyn Storage,
    api: &dyn Api,
    netuid: u16,
    version_key: u64,
) -> bool {
    let network_version_key: u64 = WEIGHTS_VERSION_KEY.load(store, netuid).unwrap();
    api.debug(&format!(
        "check_version_key( network_version_key:{:?}, version_key:{:?} )",
        network_version_key.clone(),
        version_key
    ));
    return network_version_key.clone() == 0 || version_key >= network_version_key;
}

// Checks if the neuron has set weights within the weights_set_rate_limit.
//
pub fn check_rate_limit(
    store: &dyn Storage,
    netuid: u16,
    neuron_uid: u16,
    current_block: u64,
) -> bool {
    if is_uid_exist_on_network(store, netuid, neuron_uid) {
        // --- 1. Ensure that the diff between current and last_set weights is greater than limit.
        let last_set_weights: u64 = get_last_update_for_uid(store, netuid, neuron_uid);
        if last_set_weights == 0 {
            return true;
        } // (Storage default) Never set weights.
        let rate_limit = get_weights_set_rate_limit(store, netuid);
        return current_block - last_set_weights >= rate_limit;
    }
    // --- 3. Non registered peers cant pass.
    return false;
}

// Checks for any invalid uids on this network.
pub fn contains_invalid_uids(
    store: &dyn Storage,
    api: &dyn Api,
    netuid: u16,
    uids: &Vec<u16>,
) -> bool {
    for uid in uids {
        if !is_uid_exist_on_network(store, netuid, uid.clone()) {
            api.debug(&format!(
                "contains_invalid_uids( netuid:{:?}, uid:{:?} does not exist on network. )",
                netuid, uids
            ));
            return true;
        }
    }
    return false;
}

// Returns true if the passed uids have the same length of the passed values.
pub fn uids_match_values(uids: &Vec<u16>, values: &Vec<u16>) -> bool {
    return uids.len() == values.len();
}

// Returns true if the items contain duplicates.
pub fn has_duplicate_uids(items: &Vec<u16>) -> bool {
    let mut parsed: Vec<u16> = Vec::new();
    for item in items {
        if parsed.contains(&item) {
            return true;
        }
        parsed.push(item.clone());
    }
    return false;
}

// Returns True if setting self-weight or has validator permit.
pub fn check_validator_permit(
    store: &dyn Storage,
    netuid: u16,
    uid: u16,
    uids: &Vec<u16>,
    weights: &Vec<u16>,
) -> bool {
    // Check self weight. Allowed to set single value for self weight.
    if is_self_weight(uid.clone(), uids, weights) {
        return true;
    }
    // Check if uid has validator permit.
    get_validator_permit_for_uid(store, netuid, uid)
}

// Returns True if the uids and weights are have a valid length for uid on network.
pub fn check_length(
    store: &dyn Storage,
    netuid: u16,
    uid: u16,
    uids: &Vec<u16>,
    weights: &Vec<u16>,
) -> bool {
    let subnet_n: usize = get_subnetwork_n(store, netuid.clone()) as usize;
    let min_allowed_length: usize = MIN_ALLOWED_WEIGHTS.load(store, netuid).unwrap() as usize;
    let min_allowed: usize = {
        if subnet_n.clone() < min_allowed_length.clone() {
            subnet_n
        } else {
            min_allowed_length
        }
    };

    // Check self weight. Allowed to set single value for self weight.
    // Or check that this is the root netuid.
    if netuid != 0 && is_self_weight(uid, uids, weights) {
        return true;
    }
    // Check if number of weights exceeds min.
    if weights.len() >= min_allowed {
        return true;
    }
    // To few weights.
    return false;
}

// Implace normalizes the passed positive integer weights so that they sum to u16 max value.
pub fn normalize_weights(mut weights: Vec<u16>) -> Vec<u16> {
    let sum: u64 = weights.iter().map(|x| *x as u64).sum();
    if sum.clone() == 0 {
        return weights;
    }
    weights.iter_mut().for_each(|x| {
        *x = (*x as u64 * u16::max_value() as u64 / sum) as u16;
    });
    return weights;
}

// Returns False if the weights exceed the max_weight_limit for this network.
pub fn max_weight_limited(
    store: &dyn Storage,
    netuid: u16,
    uid: u16,
    uids: &Vec<u16>,
    weights: &Vec<u16>,
) -> bool {
    // Allow self weights to exceed max weight limit.
    if is_self_weight(uid, uids, weights) {
        return true;
    }

    // If the max weight limit it u16 max, return true.
    // let max_weight_limit: u16 = MAX_WEIGHTS_LIMIT.load(store, netuid).unwrap();
    let max_weight_limit: u16 = get_max_weight_limit(store, netuid);
    if max_weight_limit == u16::MAX {
        return true;
    }

    // Check if the weights max value is less than or equal to the limit.
    check_vec_max_limited(weights, max_weight_limit)
}

// Returns true if the uids and weights correspond to a self weight on the uid.
pub fn is_self_weight(uid: u16, uids: &Vec<u16>, weights: &Vec<u16>) -> bool {
    if weights.len() != 1 {
        return false;
    }
    if uid != uids[0] {
        return false;
    }
    return true;
}

// Returns False is the number of uids exceeds the allowed number of uids for this network.
pub fn check_len_uids_within_allowed(store: &dyn Storage, netuid: u16, uids: &Vec<u16>) -> bool {
    let subnetwork_n: u16 = get_subnetwork_n(store, netuid);
    // we should expect at most subnetwork_n uids.
    return uids.len() <= subnetwork_n as usize;
}

// TODO added for debugging, remove later
pub fn get_network_weights(store: &dyn Storage, netuid: u16) -> StdResult<Vec<Vec<u16>>> {
    let n: usize = get_subnetwork_n(store, netuid) as usize;
    let mut weights: Vec<Vec<u16>> = vec![vec![0; n]; n];
    for item in WEIGHTS
        .prefix(netuid)
        .range(store, None, None, Order::Ascending)
    {
        let (uid_i, weights_i) = item.unwrap();
        for (uid_j, weight_ij) in weights_i.iter() {
            weights[uid_i as usize][*uid_j as usize] = *weight_ij;
        }
    }
    Ok(weights)
}
