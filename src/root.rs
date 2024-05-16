use cosmwasm_std::StdError::GenericErr;
use cosmwasm_std::{ensure, Addr, Api, DepsMut, Env, MessageInfo, Order, StdResult, Storage};
use cw_utils::must_pay;
use cyber_std::Response;
use substrate_fixed::types::I64F64;

use crate::block_step::blocks_until_next_epoch;
use crate::epoch::get_float_kappa;
use crate::math::{inplace_normalize_64, matmul_64, vec_fixed64_to_u64};
use crate::staking::{
    create_account_if_non_existent, delegate_hotkey, get_total_stake_for_hotkey, hotkey_is_delegate,
};
use crate::state::{
    Metadata, ACTIVE, ACTIVITY_CUTOFF, ADJUSTMENTS_ALPHA, ADJUSTMENT_INTERVAL, BLOCKS_SINCE_LAST_STEP, BONDS,
    BONDS_MOVING_AVERAGE, BURN, BURN_REGISTRATIONS_THIS_INTERVAL, CONSENSUS, DENOM, DIFFICULTY,
    DIVIDENDS, EMISSION, EMISSION_VALUES, IMMUNITY_PERIOD, INCENTIVE, KAPPA, KEYS,
    LAST_ADJUSTMENT_BLOCK, LAST_UPDATE, MAX_ALLOWED_UIDS, MAX_ALLOWED_VALIDATORS, MAX_BURN,
    MAX_DIFFICULTY, MAX_REGISTRATION_PER_BLOCK, MAX_WEIGHTS_LIMIT, METADATA2, MIN_ALLOWED_WEIGHTS,
    MIN_BURN, MIN_DIFFICULTY, NETWORKS_ADDED, NETWORK_IMMUNITY_PERIOD, NETWORK_LAST_LOCK_COST,
    NETWORK_LAST_REGISTERED, NETWORK_LOCK_REDUCTION_INTERVAL, NETWORK_MIN_LOCK_COST,
    NETWORK_MODALITY, NETWORK_RATE_LIMIT, NETWORK_REGISTERED_AT, NETWORK_REGISTRATION_ALLOWED,
    PENDING_EMISSION, POW_REGISTRATIONS_THIS_INTERVAL, PRUNING_SCORES, RANK,
    RAO_RECYCLED_FOR_REGISTRATION, REGISTRATIONS_THIS_BLOCK, REGISTRATIONS_THIS_INTERVAL, RHO,
    SERVING_RATE_LIMIT, SUBNETWORK_N, SUBNET_LIMIT, SUBNET_OWNER,
    TARGET_REGISTRATIONS_PER_INTERVAL, TEMPO, TOTAL_NETWORKS, TRUST, UIDS, VALIDATOR_PERMIT,
    VALIDATOR_TRUST, WEIGHTS, WEIGHTS_SET_RATE_LIMIT, WEIGHTS_VERSION_KEY,
};
use crate::uids::{append_neuron, get_hotkey_for_net_and_uid, get_subnetwork_n, replace_neuron};
use crate::utils::{
    get_block_emission, get_emission_value, get_max_allowed_uids, get_max_registrations_per_block,
    get_registrations_this_block, get_registrations_this_interval, get_rho, get_subnet_owner,
    get_target_registrations_per_interval, get_tempo, set_subnet_locked_balance,
};
use crate::ContractError;

// Retrieves the unique identifier (UID) for the root network.
//
// The root network is a special case and has a fixed UID of 0.
//
// # Returns:
// * 'u16': The UID for the root network.
//
pub fn get_root_netuid() -> u16 {
    0
}

// Fetches the total count of subnets.
//
// This function retrieves the total number of subnets present on the chain.
//
// # Returns:
// * 'u16': The total number of subnets.
//
pub fn get_num_subnets(store: &dyn Storage) -> u16 {
    TOTAL_NETWORKS.load(store).unwrap()
}

// Fetches the total count of subnet validators (those that set weights.)
//
// This function retrieves the total number of subnet validators.
//
// # Returns:
// * 'u16': The total number of validators
//
pub fn get_max_subnets(store: &dyn Storage) -> u16 {
    SUBNET_LIMIT.load(store).unwrap()
}

// Fetches the total count of subnet validators (those that set weights.)
//
// This function retrieves the total number of subnet validators.
//
// # Returns:
// * 'u16': The total number of validators
//
pub fn get_num_root_validators(store: &dyn Storage) -> u16 {
    get_subnetwork_n(store, get_root_netuid())
}

// Fetches the total allowed number of root validators.
//
// This function retrieves the max allowed number of validators
// it is equal to SenateMaxMembers
//
// # Returns:
// * 'u16': The max allowed root validators.
//
pub fn get_max_root_validators(store: &dyn Storage) -> u16 {
    get_max_allowed_uids(store, get_root_netuid())
}

// Returns the emission value for the given subnet.
//
// This function retrieves the emission value for the given subnet.
//
// # Returns:
// * 'u64': The emission value for the given subnet.
#[cfg(test)]
pub fn get_subnet_emission_value(store: &dyn Storage, netuid: u16) -> u64 {
    EMISSION_VALUES.load(store, netuid).unwrap()
}

// Returns true if the subnetwork exists.
//
// This function checks if a subnetwork with the given UID exists.
//
// # Returns:
// * 'bool': Whether the subnet exists.
//
pub fn if_subnet_exist(store: &dyn Storage, netuid: u16) -> bool {
    let exist = NETWORKS_ADDED.load(store, netuid);
    if exist.is_ok() {
        return exist.unwrap();
    } else {
        false
    }
}

// Returns true if the subnetwork allows registration.
//
//
// This function checks if a subnetwork allows registrations.
//
// # Returns:
// * 'bool': Whether the subnet allows registrations.
//
pub fn if_subnet_allows_registration(store: &dyn Storage, netuid: u16) -> bool {
    NETWORK_REGISTRATION_ALLOWED.load(store, netuid).unwrap()
}

// Returns a list of subnet netuid equal to total networks.
//
//
// This iterates through all the networks and returns a list of netuids.
//
// # Returns:
// * 'Vec<u16>': Netuids of added subnets.
//
pub fn get_all_subnet_netuids(store: &dyn Storage) -> Vec<u16> {
    return NETWORKS_ADDED
        .range(store, None, None, Order::Ascending)
        .map(|item| item.unwrap().0)
        .collect();
}

// Checks for any UIDs in the given list that are either equal to the root netuid or exceed the total number of subnets.
//
// It's important to check for invalid UIDs to ensure data integrity and avoid referencing nonexistent subnets.
//
// # Arguments:
// * 'uids': A reference to a vector of UIDs to check.
//
// # Returns:
// * 'bool': 'true' if any of the UIDs are invalid, 'false' otherwise.
//
pub fn contains_invalid_root_uids(store: &dyn Storage, api: &dyn Api, netuids: &Vec<u16>) -> bool {
    for netuid in netuids {
        if !if_subnet_exist(store, *netuid) {
            api.debug(&format!(
                "🔵 contains_invalid_root_uids: netuid {:?} does not exist",
                netuid
            ));
            return true;
        }
    }
    false
}

// Sets the emission values for each netuid
//
//
pub fn set_emission_values(
    store: &mut dyn Storage,
    api: &dyn Api,
    netuids: &Vec<u16>,
    emission: Vec<u64>,
) -> Result<(), ContractError> {
    api.debug(&format!(
        "🔵 set_emission_values: netuids: {:?}, emission:{:?}",
        netuids, emission
    ));

    // Be careful this function can fail.
    if contains_invalid_root_uids(store, api, netuids) {
        api.debug(&format!(
            "🔵 error set_emission_values: contains_invalid_root_uids"
        ));
        return Err(ContractError::InvalidUid {});
    }
    if netuids.len() != emission.len() {
        api.debug(&format!(
            "🔵 error set_emission_values: netuids.len() != emission.len()"
        ));
        return Err(ContractError::Std(GenericErr {
            msg: "🔵 netuids and emission must have the same length".to_string(),
        }));
    }
    for (i, netuid_i) in netuids.iter().enumerate() {
        api.debug(&format!(
            "🔵 set netuid:{:?}, emission:{:?}",
            netuid_i, emission[i]
        ));
        EMISSION_VALUES.save(store, *netuid_i, &emission[i])?;
    }
    Ok(())
}

// Retrieves weight matrix associated with the root network.
//  Weights represent the preferences for each subnetwork.
//
// # Returns:
// A 2D vector ('Vec<Vec<I32F32>>') where each entry [i][j] represents the weight of subnetwork
// 'j' with according to the preferences of key. 'j' within the root network.
//
pub fn get_root_weights(store: &dyn Storage, api: &dyn Api) -> Vec<Vec<I64F64>> {
    // --- 0. The number of validators on the root network.
    let n: usize = get_num_root_validators(store) as usize;

    // --- 1 The number of subnets to validate.
    // api.debug(&format!(
    //     "🔵 subnet size before cast: {:?}",
    //     get_num_subnets(store)
    // ));
    let k: usize = get_num_subnets(store) as usize;
    api.debug(&format!("🔵 root_validators: {:?}, subnets: {:?}", n, k));

    // --- 2. Initialize a 2D vector with zeros to store the weights. The dimensions are determined
    // by `n` (number of validators) and `k` (total number of subnets).
    let mut weights: Vec<Vec<I64F64>> = vec![vec![I64F64::from_num(0.0); k]; n];
    // deps.api.debug(&format!("weights:\n{:?}\n", weights));

    let subnet_list = get_all_subnet_netuids(store);

    // --- 3. Iterate over stored weights and fill the matrix.
    for item in WEIGHTS
        .prefix(get_root_netuid())
        .range(store, None, None, Order::Ascending)
    {
        let (uid_i, weights_i) = item.unwrap();
        // --- 4. Iterate over each weight entry in `weights_i` to update the corresponding value in the
        // initialized `weights` 2D vector. Here, `uid_j` represents a subnet, and `weight_ij` is the
        // weight of `uid_i` with respect to `uid_j`.
        for (netuid, weight_ij) in weights_i.iter() {
            let option = subnet_list.iter().position(|item| item == netuid);

            let idx = uid_i as usize;
            if let Some(weight) = weights.get_mut(idx) {
                if let Some(netuid_idx) = option {
                    weight[netuid_idx] = I64F64::from_num(*weight_ij);
                }
            }
        }
    }

    // --- 5. Return the filled weights matrix.
    weights
}

pub fn get_network_rate_limit(store: &dyn Storage) -> u64 {
    NETWORK_RATE_LIMIT.load(store).unwrap()
}

// Computes and sets emission values for the root network which determine the emission for all subnets.
//
// This function is responsible for calculating emission based on network weights, stake values,
// and registered hotkeys.
//
pub fn root_epoch(
    store: &mut dyn Storage,
    api: &dyn Api,
    block_number: u64,
) -> Result<(), ContractError> {
    // --- 0. The unique ID associated with the root network.
    let root_netuid: u16 = get_root_netuid();

    // --- 3. Check if we should update the emission values based on blocks since emission was last set.
    let blocks_until_next_epoch: u64 =
        blocks_until_next_epoch(root_netuid, get_tempo(store, root_netuid), block_number);
    if blocks_until_next_epoch != 0 {
        // Not the block to update emission values.
        api.debug(&format!(
            "🔵 blocks_until_next_epoch: {:?}",
            blocks_until_next_epoch
        ));
        return Err(ContractError::Std(GenericErr {
            msg: "🔵 Not the block to update emission values.".to_string(),
        }));
    }

    // --- 1. Retrieves the number of root validators on subnets.
    let n: u16 = get_num_root_validators(store);
    api.debug(&format!("root_validators: {:?}", n));
    if n == 0 {
        // No validators.
        return Err(ContractError::Std(GenericErr {
            msg: "🔵 No validators to validate emission values.".to_string(),
        }));
    }

    // --- 2. Obtains the number of registered subnets.
    let k: u16 = get_all_subnet_netuids(store).len() as u16;
    api.debug(&format!("subnets 🔵: {:?}", k));
    if k == 0 {
        // No networks to validate.
        return Err(ContractError::Std(GenericErr {
            msg: "🔵 No networks to validate emission values.".to_string(),
        }));
    }

    // --- 4. Determines the total block emission across all the subnetworks. This is the
    // value which will be distributed based on the computation below.
    let block_emission: I64F64 = I64F64::from_num(get_block_emission(store));
    api.debug(&format!("🔵 block_emission: {:?}", block_emission));

    // --- 5. A collection of all registered hotkeys on the root network. Hotkeys
    // pairs with network UIDs and stake values.
    let mut hotkeys: Vec<(u16, Addr)> = vec![];

    for item in KEYS
        .prefix(root_netuid)
        .range(store, None, None, Order::Ascending)
    {
        let (uid_i, hotkey) = item?;
        hotkeys.push((uid_i, hotkey));
    }
    api.debug(&format!("🔵 hotkeys: {:?}\n", hotkeys));

    // --- 6. Retrieves and stores the stake value associated with each hotkey on the root network.
    // Stakes are stored in a 64-bit fixed point representation for precise calculations.
    let mut stake_i64: Vec<I64F64> = vec![I64F64::from_num(0.0); n as usize];
    for (uid_i, hotkey) in hotkeys.iter() {
        stake_i64[*uid_i as usize] = I64F64::from_num(get_total_stake_for_hotkey(store, &hotkey));
    }
    inplace_normalize_64(&mut stake_i64);
    api.debug(&format!("🔵 stake: {:?}\n", &stake_i64));

    // --- 8. Retrieves the network weights in a 2D Vector format. Weights have shape
    // n x k where is n is the number of registered peers and k is the number of subnets.
    let weights: Vec<Vec<I64F64>> = get_root_weights(store, api);
    api.debug(&format!("🔵 weights: {:?}\n", &weights));

    // --- 9. Calculates the rank of networks. Rank is a product of weights and stakes.
    // Ranks will have shape k, a score for each subnet.
    let ranks: Vec<I64F64> = matmul_64(&weights, &stake_i64);
    api.debug(&format!("🔵 ranks: {:?}\n", &ranks));

    // --- 10. Calculates the trust of networks. Trust is a sum of all stake with weights > 0.
    // Trust will have shape k, a score for each subnet.
    let total_networks = get_num_subnets(store);
    let mut trust = vec![I64F64::from_num(0); total_networks as usize];
    let mut total_stake: I64F64 = I64F64::from_num(0);
    for (idx, weights) in weights.iter().enumerate() {
        let hotkey_stake = stake_i64[idx];
        total_stake += hotkey_stake;
        for (weight_idx, weight) in weights.iter().enumerate() {
            if *weight > 0 {
                trust[weight_idx] += hotkey_stake;
            }
        }
    }

    api.debug(&format!("🔵 trust_nn: {:?}\n", &trust));
    api.debug(&format!("🔵 total_stake: {:?}\n", &total_stake));

    if total_stake == 0 {
        return Err(ContractError::Std(GenericErr {
            msg: "🔵 No stake on network".to_string(),
        }));
    }

    for trust_score in trust.iter_mut() {
        match trust_score.checked_div(total_stake) {
            Some(quotient) => {
                *trust_score = quotient;
            }
            None => {}
        }
    }

    // --- 11. Calculates the consensus of networks. Consensus is a sigmoid normalization of the trust scores.
    // Consensus will have shape k, a score for each subnet.
    api.debug(&format!("🔵 trust_n: {:?}\n", &trust));
    let one = I64F64::from_num(1);
    let mut consensus = vec![I64F64::from_num(0); total_networks as usize];
    for (idx, trust_score) in trust.iter_mut().enumerate() {
        let shifted_trust = *trust_score - I64F64::from_num(get_float_kappa(store, 0)); // Range( -kappa, 1 - kappa )
        let temperatured_trust = shifted_trust * I64F64::from_num(get_rho(store, 0)); // Range( -rho * kappa, rho ( 1 - kappa ) )
        let exponentiated_trust: I64F64 = substrate_fixed::transcendental::exp(-temperatured_trust)
            .expect("temperatured_trust is on range( -rho * kappa, rho ( 1 - kappa ) )");

        consensus[idx] = one / (one + exponentiated_trust);
    }

    api.debug(&format!("🔵 consensus: {:?}\n", &consensus));
    let mut weighted_emission = vec![I64F64::from_num(0); total_networks as usize];
    for (idx, emission) in weighted_emission.iter_mut().enumerate() {
        *emission = consensus[idx] * ranks[idx];
    }
    inplace_normalize_64(&mut weighted_emission);
    api.debug(&format!("🔵 emission_w: {:?}\n", &weighted_emission));

    // -- 11. Converts the normalized 64-bit fixed point rank values to u64 for the final emission calculation.
    let emission_as_boot: Vec<I64F64> = weighted_emission
        .iter()
        .map(|v: &I64F64| *v * block_emission)
        .collect();

    // --- 12. Converts the normalized 64-bit fixed point rank values to u64 for the final emission calculation.
    let emission_u64: Vec<u64> = vec_fixed64_to_u64(emission_as_boot);
    api.debug(&format!("🔵 emission_f: {:?}\n", &emission_u64));

    // --- 13. Set the emission values for each subnet directly.
    let netuids: Vec<u16> = get_all_subnet_netuids(store);
    api.debug(&format!(
        "🔵 netuids: {:?}, values: {:?}",
        netuids, emission_u64
    ));

    set_emission_values(store, api, &netuids, emission_u64)?;

    Ok(())
}

// Registers a user's hotkey to the root network.
//
// This function is responsible for registering the hotkey of a user.
// The root key with the least stake if pruned in the event of a filled network.
//
// # Arguments:
// * 'origin': Represents the origin of the call.
// * 'hotkey': The hotkey that the user wants to register to the root network.
//
// # Returns:
// * 'DispatchResult': A result type indicating success or failure of the registration.
//
pub fn do_root_register(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    hotkey_address: String,
) -> Result<Response, ContractError> {
    let hotkey = deps.api.addr_validate(&hotkey_address)?;

    // --- 0. Get the unique identifier (UID) for the root network.
    let root_netuid: u16 = get_root_netuid();
    let current_block_number: u64 = env.block.height;
    ensure!(
        if_subnet_exist(deps.storage, root_netuid),
        ContractError::NetworkDoesNotExist {}
    );

    // --- 1. Ensure that the call originates from a signed source and retrieve the caller's account ID (coldkey).
    let coldkey = info.sender;
    deps.api.debug(&format!(
        "🔵 do_root_register ( coldkey: {:?}, hotkey: {:?} )",
        coldkey,
        hotkey.clone()
    ));

    // --- 2. Ensure that the number of registrations in this block doesn't exceed the allowed limit.
    ensure!(
        get_registrations_this_block(deps.storage, root_netuid)
            < get_max_registrations_per_block(deps.storage, root_netuid),
        ContractError::TooManyRegistrationsThisBlock {}
    );

    // --- 3. Ensure that the number of registrations in this interval doesn't exceed thrice the target limit.
    ensure!(
        get_registrations_this_interval(deps.storage, root_netuid)
            < get_target_registrations_per_interval(deps.storage, root_netuid) * 3,
        ContractError::TooManyRegistrationsThisInterval {}
    );

    // --- 4. Check if the hotkey is already registered. If so, error out.
    ensure!(
        !UIDS.has(deps.storage, (root_netuid, &hotkey)),
        ContractError::AlreadyRegistered {}
    );

    // --- 6. Create a network account for the user if it doesn't exist.
    create_account_if_non_existent(deps.storage, &coldkey, &hotkey);

    // --- 7. Fetch the current size of the subnetwork.
    let current_num_root_validators: u16 = get_num_root_validators(deps.storage);

    // Declare a variable to hold the root UID.
    let subnetwork_uid: u16;

    // --- 8. Check if the root net is below its allowed size.
    // max allowed is senate size.
    if current_num_root_validators < get_max_root_validators(deps.storage) {
        // --- 12.1.1 We can append to the subnetwork as it's not full.
        subnetwork_uid = current_num_root_validators;

        // --- 12.1.2 Add the new account and make them a member of the Senate.
        append_neuron(
            deps.storage,
            deps.api,
            root_netuid,
            &hotkey,
            current_block_number,
        )?;
        deps.api.debug(&format!(
            "🔵 add new neuron: {:?} on uid {:?}",
            hotkey, subnetwork_uid
        ));
    } else {
        // --- 13.1.1 The network is full. Perform replacement.
        // Find the neuron with the lowest stake value to replace.
        let mut lowest_stake: u64 = u64::MAX;
        let mut lowest_uid: u16 = 0;

        // Iterate over all keys in the root network to find the neuron with the lowest stake.
        for item in KEYS
            .prefix(root_netuid)
            .range(deps.storage, None, None, Order::Ascending)
        {
            let (uid_i, hotkey_i) = item?;
            let stake_i: u64 = get_total_stake_for_hotkey(deps.storage, &hotkey_i);
            if stake_i < lowest_stake {
                lowest_stake = stake_i;
                lowest_uid = uid_i;
            }
        }
        subnetwork_uid = lowest_uid;
        let replaced_hotkey: Addr =
            get_hotkey_for_net_and_uid(deps.storage, root_netuid, subnetwork_uid).unwrap();

        // --- 13.1.2 The new account has a higher stake than the one being replaced.
        ensure!(
            lowest_stake < get_total_stake_for_hotkey(deps.storage, &hotkey),
            ContractError::StakeTooLowForRoot {}
        );

        // --- 13.1.3 The new account has a higher stake than the one being replaced.
        // Replace the neuron account with new information.
        let _msgs = replace_neuron(
            deps.storage,
            deps.api,
            root_netuid,
            lowest_uid,
            &hotkey,
            current_block_number,
        )?;

        deps.api.debug(&format!(
            "🔵 replace neuron: {:?} with {:?} on uid {:?}",
            replaced_hotkey, hotkey, subnetwork_uid
        ));
    }

    // TODO revisit this as we don't have a senate and need migration to dao
    // let current_stake = get_total_stake_for_hotkey(deps.storage, hotkey.clone());
    // If we're full, we'll swap out the lowest stake member.
    // let members = T::SenateMembers::members();
    // if (members.len() as u32) == T::SenateMembers::max_members() {
    //     let mut sorted_members = members.clone();
    //     sorted_members.sort_by(|a, b| {
    //         let a_stake = get_total_stake_for_hotkey(a);
    //         let b_stake = get_total_stake_for_hotkey(b);
    //
    //         b_stake.cmp(&a_stake)
    //     });
    //
    //     if let Some(last) = sorted_members.last() {
    //         let last_stake = get_total_stake_for_hotkey(last);
    //
    //         if last_stake < current_stake {
    //             T::SenateMembers::swap_member(last, &hotkey)?;
    //             T::TriumvirateInterface::remove_votes(&last)?;
    //         }
    //     }
    // } else {
    //     T::SenateMembers::add_member(&hotkey)?;
    // }

    // --- 13. Force all members on root to become a delegate.
    if !hotkey_is_delegate(deps.storage, &hotkey) {
        delegate_hotkey(deps.storage, &hotkey, 11796);
    }

    // --- 14. Update the registration counters for both the block and interval.
    REGISTRATIONS_THIS_INTERVAL.update(deps.storage, root_netuid, |val| -> StdResult<_> {
        let mut new_val = val.unwrap();
        new_val += 1;
        Ok(new_val)
    })?;
    REGISTRATIONS_THIS_BLOCK.update(deps.storage, root_netuid, |val| -> StdResult<_> {
        let mut new_val = val.unwrap();
        new_val += 1;
        Ok(new_val)
    })?;

    // --- 15. Log and announce the successful registration.
    deps.api.debug(&format!(
        "🔵 RootRegistered (netuid:{:?} uid:{:?} hotkey:{:?})",
        root_netuid, subnetwork_uid, hotkey
    ));

    // --- 16. Finish and return success.
    Ok(Response::default()
        .add_attribute("active", "neuron_registered")
        .add_attribute("root_netuid", format!("{}", root_netuid))
        .add_attribute("subnetwork_uid", format!("{}", subnetwork_uid))
        .add_attribute("hotkey", hotkey))
}

// Facilitates user registration of a new subnetwork.
//
// # Args:
// 	* 'origin': ('T::RuntimeOrigin'): The calling origin. Must be signed.
//
// # Event:
// 	* 'NetworkAdded': Emitted when a new network is successfully added.
//
// # Raises:
// 	* 'TxRateLimitExceeded': If the rate limit for network registration is exceeded.
// 	* 'NotEnoughBalanceToStake': If there isn't enough balance to stake for network registration.
// 	* 'BalanceWithdrawalError': If an error occurs during balance withdrawal for network registration.
//
pub fn user_add_network(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let denom = DENOM.load(deps.storage)?;
    let amount = must_pay(&info, &denom).map_err(|_| ContractError::CouldNotConvertToBalance {})?;

    // --- 0. Ensure the caller is a signed user.
    let coldkey = info.sender;

    // --- 1. Rate limit for network registrations.
    let current_block = env.block.height;
    let last_lock_block = get_network_last_lock_block(deps.storage);
    ensure!(
        current_block - last_lock_block >= get_network_rate_limit(deps.storage),
        ContractError::TxRateLimitExceeded {}
    );

    // --- 2. Calculate and lock the required tokens.
    let lock_amount: u64 = get_network_lock_cost(deps.storage, deps.api, env.block.height)?;
    deps.api
        .debug(&format!("🔵 network lock_amount: {:?}", lock_amount));

    ensure!(
        amount.u128() as u64 >= lock_amount,
        ContractError::NotEnoughTokens {}
    );

    // --- 4. Determine the netuid to register.
    let netuid_to_register: u16 = {
        deps.api.debug(&format!(
            "🔵 subnet count: {:?}\nmax subnets: {:?}",
            get_num_subnets(deps.storage),
            get_max_subnets(deps.storage)
        ));
        if get_num_subnets(deps.storage) - 1 < get_max_subnets(deps.storage) {
            // We subtract one because we don't want root subnet to count towards total
            let mut next_available_netuid = 0;
            loop {
                next_available_netuid += 1;
                if !if_subnet_exist(deps.storage, next_available_netuid) {
                    deps.api
                        .debug(&format!("got subnet id: {:?}", next_available_netuid));
                    break next_available_netuid;
                }
            }
        } else {
            let netuid_to_prune = get_subnet_to_prune(deps.storage, env.block.height)?;
            ensure!(netuid_to_prune > 0, ContractError::AllNetworksInImmunity {});

            remove_network(deps.storage, netuid_to_prune)?;
            deps.api
                .debug(&format!("remove_network: {:?}", netuid_to_prune));
            netuid_to_prune
        }
    };

    // --- 5. Perform the lock operation.
    set_subnet_locked_balance(deps.storage, netuid_to_register, lock_amount);
    set_network_last_lock(deps.storage, lock_amount);

    // --- 6. Set initial and custom parameters for the network.
    init_new_network(deps.storage, netuid_to_register, 360)?;
    deps.api
        .debug(&format!("init_new_network: {:?}", netuid_to_register));

    // --- 7. Set netuid storage.
    let current_block_number: u64 = env.block.height;
    NETWORK_LAST_REGISTERED.save(deps.storage, &current_block_number)?;
    NETWORK_REGISTERED_AT.save(deps.storage, netuid_to_register, &current_block_number)?;
    SUBNET_OWNER.save(deps.storage, netuid_to_register, &coldkey)?;

    // --- 8. Emit the NetworkAdded event.
    deps.api.debug(&format!(
        "🔵 NetworkAdded ( netuid:{:?}, modality:{:?} )",
        netuid_to_register, 0
    ));

    // --- 9. Return success.
    Ok(Response::default()
        .add_attribute("active", "network_added")
        .add_attribute("netuid_to_register", format!("{}", netuid_to_register)))
}

// Facilitates the removal of a user's subnetwork.
//
// # Args:
// 	* 'origin': ('T::RuntimeOrigin'): The calling origin. Must be signed.
//     * 'netuid': ('u16'): The unique identifier of the network to be removed.
//
// # Event:
// 	* 'NetworkRemoved': Emitted when a network is successfully removed.
//
// # Raises:
// 	* 'NetworkDoesNotExist': If the specified network does not exist.
// 	* 'NotSubnetOwner': If the caller does not own the specified subnet.
//
pub fn user_remove_network(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    netuid: u16,
) -> Result<Response, ContractError> {
    // --- 1. Ensure the function caller is a signed user.
    let coldkey = info.sender;

    // --- 2. Ensure this subnet exists.
    ensure!(
        if_subnet_exist(deps.storage, netuid),
        ContractError::NetworkDoesNotExist {}
    );

    // --- 3. Ensure the caller owns this subnet.
    ensure!(
        get_subnet_owner(deps.storage, netuid) == coldkey,
        ContractError::NotSubnetOwner {}
    );

    // --- 4. Explicitly erase the network and all its parameters.
    remove_network(deps.storage, netuid)?;

    // --- 5. Emit the NetworkRemoved event.
    deps.api
        .debug(&format!("🔵 NetworkRemoved ( netuid:{:?} )", netuid));

    // --- 6. Return success.
    Ok(Response::default()
        .add_attribute("active", "network_removed")
        .add_attribute("netuid", format!("{}", netuid)))
}

// Sets initial and custom parameters for a new network.
pub fn init_new_network(
    store: &mut dyn Storage,
    netuid: u16,
    tempo: u16,
) -> Result<(), ContractError> {
    // --- 1. Set network to 0 size.
    SUBNETWORK_N.save(store, netuid, &0)?;

    // --- 2. Set this network uid to alive.
    NETWORKS_ADDED.save(store, netuid, &true)?;

    // --- 3. Fill tempo memory item.
    TEMPO.save(store, netuid, &tempo)?;

    // --- 4 Fill modality item.
    NETWORK_MODALITY.save(store, netuid, &0)?;

    // --- 5. Increase total network count.
    TOTAL_NETWORKS.update(store, |mut n| -> StdResult<_> {
        n += 1;
        Ok(n)
    })?;

    // --- 6. Set all default values **explicitly**.
    NETWORK_REGISTRATION_ALLOWED.save(store, netuid, &true)?;
    MAX_ALLOWED_UIDS.save(store, netuid, &256)?;
    MAX_ALLOWED_VALIDATORS.save(store, netuid, &64)?;
    MIN_ALLOWED_WEIGHTS.save(store, netuid, &1)?;
    MAX_WEIGHTS_LIMIT.save(store, netuid, &u16::MAX)?;
    ADJUSTMENT_INTERVAL.save(store, netuid, &360)?;
    TARGET_REGISTRATIONS_PER_INTERVAL.save(store, netuid, &1)?;
    ADJUSTMENTS_ALPHA.save(store, netuid, &58000)?;
    IMMUNITY_PERIOD.save(store, netuid, &7200)?;

    DIFFICULTY.save(store, netuid, &10_000_000)?;
    MIN_DIFFICULTY.save(store, netuid, &10_000_000)?;
    MAX_DIFFICULTY.save(store, netuid, &(u64::MAX / 4))?;

    // Make network parameters explicit.
    KAPPA.save(store, netuid, &32_767)?; // 0.5 = 65535/2
                                         // IMMUNITY_PERIOD.save(store, netuid, &0)?;
    ACTIVITY_CUTOFF.save(store, netuid, &5000)?;
    EMISSION_VALUES.save(store, netuid, &0)?;

    REGISTRATIONS_THIS_INTERVAL.save(store, netuid, &0)?;
    POW_REGISTRATIONS_THIS_INTERVAL.save(store, netuid, &0)?;
    BURN_REGISTRATIONS_THIS_INTERVAL.save(store, netuid, &0)?;

    // TODO Added initializations
    WEIGHTS_VERSION_KEY.save(store, netuid, &0)?;
    MAX_REGISTRATION_PER_BLOCK.save(store, netuid, &3)?;
    WEIGHTS_SET_RATE_LIMIT.save(store, netuid, &100)?;
    PENDING_EMISSION.save(store, netuid, &0)?;
    BLOCKS_SINCE_LAST_STEP.save(store, netuid, &0)?;
    BONDS_MOVING_AVERAGE.save(store, netuid, &900_000)?;
    LAST_ADJUSTMENT_BLOCK.save(store, netuid, &0)?;
    ADJUSTMENT_INTERVAL.save(store, netuid, &100)?;

    BURN.save(store, netuid, &1_000_000_000)?;
    MIN_BURN.save(store, netuid, &100_000_000)?;
    MAX_BURN.save(store, netuid, &100_000_000_000)?;

    REGISTRATIONS_THIS_BLOCK.save(store, netuid, &0)?;
    // MAX_REGISTRATION_PER_BLOCK.save(store, netuid, &3)?;
    KAPPA.save(store, netuid, &32_767)?;
    RHO.save(store, netuid, &30)?;
    RAO_RECYCLED_FOR_REGISTRATION.save(store, netuid, &0)?;
    SERVING_RATE_LIMIT.save(store, netuid, &50)?;
    ADJUSTMENTS_ALPHA.save(store, netuid, &0)?;
    LAST_UPDATE.save(store, netuid, &vec![])?;
    METADATA2.save(
        store,
        netuid,
        &Metadata {
            name: "empty".to_string(),
            particle: "".to_string(),
            description: "".to_string(),
            logo: "".to_string(),
        }
    )?;

    Ok(())
}

// Removes a network (identified by netuid) and all associated parameters.
//
// This function is responsible for cleaning up all the data associated with a network.
// It ensures that all the storage values related to the network are removed, and any
// reserved balance is returned to the network owner.
//
// # Args:
// 	* 'netuid': ('u16'): The unique identifier of the network to be removed.
//
// # Note:
// This function does not emit any events, nor does it raise any errors. It silently
// returns if any internal checks fail.
//
pub fn remove_network(store: &mut dyn Storage, netuid: u16) -> Result<(), ContractError> {
    // --- 1. Return balance to subnet owner.
    // let _owner_coldkey = get_subnet_owner(store, netuid);
    // let _reserved_amount = get_subnet_locked_balance(store, netuid);

    // --- 2. Remove network count.
    SUBNETWORK_N.remove(store, netuid);

    // --- 3. Remove network modality storage.
    NETWORK_MODALITY.remove(store, netuid);

    // --- 4. Remove netuid from added networks.
    NETWORKS_ADDED.remove(store, netuid);

    // --- 6. Decrement the network counter.
    TOTAL_NETWORKS.update(store, |mut n| -> StdResult<_> {
        n -= 1;
        Ok(n)
    })?;

    // --- 7. Remove various network-related storages.
    NETWORK_REGISTERED_AT.remove(store, netuid);

    // --- 8. Remove incentive mechanism memory.
    // TODO check correctnes deletation of prefix
    UIDS.prefix(netuid).clear(store, None);
    KEYS.prefix(netuid).clear(store, None);
    BONDS.prefix(netuid).clear(store, None);
    WEIGHTS.prefix(netuid).clear(store, None);

    // --- 9. Remove various network-related parameters.
    RANK.remove(store, netuid);
    TRUST.remove(store, netuid);
    ACTIVE.remove(store, netuid);
    EMISSION.remove(store, netuid);
    INCENTIVE.remove(store, netuid);
    CONSENSUS.remove(store, netuid);
    DIVIDENDS.remove(store, netuid);
    PRUNING_SCORES.remove(store, netuid);
    LAST_UPDATE.remove(store, netuid);
    VALIDATOR_PERMIT.remove(store, netuid);
    VALIDATOR_TRUST.remove(store, netuid);

    // --- 10. Erase network parameters.
    TEMPO.remove(store, netuid);
    KAPPA.remove(store, netuid);
    DIFFICULTY.remove(store, netuid);
    MAX_ALLOWED_UIDS.remove(store, netuid);
    IMMUNITY_PERIOD.remove(store, netuid);
    ACTIVITY_CUTOFF.remove(store, netuid);
    EMISSION_VALUES.remove(store, netuid);
    MAX_WEIGHTS_LIMIT.remove(store, netuid);
    MIN_ALLOWED_WEIGHTS.remove(store, netuid);
    REGISTRATIONS_THIS_INTERVAL.remove(store, netuid);
    POW_REGISTRATIONS_THIS_INTERVAL.remove(store, netuid);
    BURN_REGISTRATIONS_THIS_INTERVAL.remove(store, netuid);

    // --- 11. Add the balance back to the owner.
    // TODO create messages to send token here or burn them during creation, decide later
    set_subnet_locked_balance(store, netuid, 0);
    SUBNET_OWNER.remove(store, netuid);

    // TODO Added
    WEIGHTS_VERSION_KEY.remove(store, netuid);
    MAX_REGISTRATION_PER_BLOCK.remove(store, netuid);
    WEIGHTS_SET_RATE_LIMIT.remove(store, netuid);

    PENDING_EMISSION.remove(store, netuid);
    BLOCKS_SINCE_LAST_STEP.remove(store, netuid);
    BONDS_MOVING_AVERAGE.remove(store, netuid);
    LAST_ADJUSTMENT_BLOCK.remove(store, netuid);
    ADJUSTMENT_INTERVAL.remove(store, netuid);
    BURN.remove(store, netuid);
    MIN_BURN.remove(store, netuid);
    MAX_BURN.remove(store, netuid);
    REGISTRATIONS_THIS_BLOCK.remove(store, netuid);
    // MAX_REGISTRATION_PER_BLOCK.remove(store, netuid);
    KAPPA.remove(store, netuid);
    RHO.remove(store, netuid);
    RAO_RECYCLED_FOR_REGISTRATION.remove(store, netuid);
    SERVING_RATE_LIMIT.remove(store, netuid);
    MIN_DIFFICULTY.remove(store, netuid);
    MAX_DIFFICULTY.remove(store, netuid);
    ADJUSTMENTS_ALPHA.remove(store, netuid);
    NETWORK_REGISTRATION_ALLOWED.remove(store, netuid);
    TARGET_REGISTRATIONS_PER_INTERVAL.remove(store, netuid);
    METADATA2.remove(store, netuid);

    Ok(())
}

// This function calculates the lock cost for a network based on the last lock amount, minimum lock cost, last lock block, and current block.
// The lock cost is calculated using the formula:
// lock_cost = (last_lock * mult) - (last_lock / lock_reduction_interval) * (current_block - last_lock_block)
// where:
// - last_lock is the last lock amount for the network
// - mult is the multiplier which increases lock cost each time a registration occurs
// - last_lock_block is the block number at which the last lock occurred
// - lock_reduction_interval the number of blocks before the lock returns to previous value.
// - current_block is the current block number
// - DAYS is the number of blocks in a day
// - min_lock is the minimum lock cost for the network
//
// If the calculated lock cost is less than the minimum lock cost, the minimum lock cost is returned.
//
// # Returns:
// 	* 'u64':
// 		- The lock cost for the network.
//
pub fn get_network_lock_cost(
    store: &dyn Storage,
    api: &dyn Api,
    current_block: u64,
) -> StdResult<u64> {
    let last_lock = get_network_last_lock(store);
    let min_lock = get_network_min_lock(store);
    let last_lock_block = get_network_last_lock_block(store);
    let lock_reduction_interval = get_lock_reduction_interval(store);
    let mult = if last_lock_block == 0 { 1 } else { 2 };

    let mut lock_cost = last_lock.saturating_mul(mult).saturating_sub(
        last_lock
            .saturating_div(lock_reduction_interval)
            .saturating_mul(current_block.saturating_sub(last_lock_block)),
    );

    if lock_cost < min_lock {
        lock_cost = min_lock;
    }

    api.debug(&format!("🔵 last_lock: {:?}, min_lock: {:?}, last_lock_block: {:?}, lock_reduction_interval: {:?}, current_block: {:?}, mult: {:?} lock_cost: {:?}",
                       last_lock, min_lock, last_lock_block, lock_reduction_interval, current_block, mult, lock_cost));

    Ok(lock_cost)
}

// This function is used to determine which subnet to prune when the total number of networks has reached the limit.
// It iterates over all the networks and finds the one with the minimum emission value that is not in the immunity period.
// If all networks are in the immunity period, it returns the one with the minimum emission value.
//
// # Returns:
// 	* 'u16':
// 		- The uid of the network to be pruned.
//
pub fn get_subnet_to_prune(store: &dyn Storage, current_block: u64) -> Result<u16, ContractError> {
    let mut min_score = 1;
    let mut min_score_in_immunity_period = u64::MAX;
    let mut uid_with_min_score = 1;
    // let mut uid_with_min_score_in_immunity_period: u16 = 1;

    // Iterate over all networks
    for netuid in 1..get_num_subnets(store) - 1 {
        let emission_value: u64 = get_emission_value(store, netuid);
        let block_at_registration: u64 = get_network_registered_block(store, netuid);
        let immunity_period: u64 = get_network_immunity_period(store);

        // Check if the network is in the immunity period
        if min_score == emission_value {
            if current_block.saturating_sub(block_at_registration) < immunity_period {
                //neuron is in immunity period
                if min_score_in_immunity_period > emission_value {
                    min_score_in_immunity_period = emission_value;
                    // uid_with_min_score_in_immunity_period = netuid;
                }
            } else {
                min_score = emission_value;
                uid_with_min_score = netuid;
            }
        }
        // Find min emission value.
        else if min_score > emission_value {
            if current_block.saturating_sub(block_at_registration) < immunity_period {
                // network is in immunity period
                if min_score_in_immunity_period > emission_value {
                    min_score_in_immunity_period = emission_value;
                    // uid_with_min_score_in_immunity_period = netuid;
                }
            } else {
                min_score = emission_value;
                uid_with_min_score = netuid;
            }
        }
    }
    // If all networks are in the immunity period, return the one with the minimum emission value.
    if min_score == 1 {
        // all networks are in immunity period
        return Ok(0);
    } else {
        return Ok(uid_with_min_score);
    }
}

pub fn get_network_registered_block(store: &dyn Storage, netuid: u16) -> u64 {
    NETWORK_REGISTERED_AT.load(store, netuid).unwrap()
}

pub fn get_network_immunity_period(store: &dyn Storage) -> u64 {
    NETWORK_IMMUNITY_PERIOD.load(store).unwrap()
}

#[cfg(test)]
pub fn set_network_immunity_period(store: &mut dyn Storage, net_immunity_period: u64) {
    NETWORK_IMMUNITY_PERIOD
        .save(store, &net_immunity_period)
        .unwrap();
}

#[cfg(test)]
pub fn set_network_min_lock(store: &mut dyn Storage, net_min_lock: u64) {
    NETWORK_MIN_LOCK_COST.save(store, &net_min_lock).unwrap();
}

pub fn get_network_min_lock(store: &dyn Storage) -> u64 {
    NETWORK_MIN_LOCK_COST.load(store).unwrap()
}

pub fn set_network_last_lock(store: &mut dyn Storage, net_last_lock: u64) {
    NETWORK_LAST_LOCK_COST.save(store, &net_last_lock).unwrap();
}

pub fn get_network_last_lock(store: &dyn Storage) -> u64 {
    NETWORK_LAST_LOCK_COST.load(store).unwrap()
}

pub fn get_network_last_lock_block(store: &dyn Storage) -> u64 {
    NETWORK_LAST_REGISTERED.load(store).unwrap()
}

#[cfg(test)]
pub fn set_lock_reduction_interval(store: &mut dyn Storage, interval: u64) {
    NETWORK_LOCK_REDUCTION_INTERVAL
        .save(store, &interval)
        .unwrap();
}

pub fn get_lock_reduction_interval(store: &dyn Storage) -> u64 {
    NETWORK_LOCK_REDUCTION_INTERVAL.load(store).unwrap()
}
