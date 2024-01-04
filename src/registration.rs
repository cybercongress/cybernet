use crate::root::{get_root_netuid, if_subnet_allows_registration, if_subnet_exist};
use crate::staking::coldkey_owns_hotkey;
use crate::staking::{create_account_if_non_existent, increase_stake_on_coldkey_hotkey_account};
use crate::state::{
    BURN_REGISTRATIONS_THIS_INTERVAL, DENOM, POW_REGISTRATIONS_THIS_INTERVAL,
    REGISTRATIONS_THIS_BLOCK, REGISTRATIONS_THIS_INTERVAL, UIDS, USED_WORK,
};
use crate::uids::{append_neuron, get_subnetwork_n, replace_neuron};
use crate::utils::{
    burn_tokens, ensure_root, get_burn_as_u64, get_difficulty_as_u64, get_immunity_period,
    get_max_allowed_uids, get_max_registrations_per_block, get_neuron_block_at_registration,
    get_pruning_score_for_uid, get_registrations_this_block, get_registrations_this_interval,
    get_target_registrations_per_interval, increase_rao_recycled, set_pruning_score_for_uid,
};
use crate::ContractError;
use cosmwasm_std::{ensure, Api, DepsMut, Env, MessageInfo, StdResult, Storage};
use cw_utils::must_pay;

use primitive_types::{H256, U256};
// use sp_io::hashing::{keccak_256, sha2_256};
use cyber_std::Response;
use sp_core_hashing::{keccak_256, sha2_256};

pub fn do_sudo_registration(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    netuid: u16,
    hotkey_address: String,
    coldkey_address: String,
) -> Result<Response, ContractError> {
    let denom = DENOM.load(deps.storage)?;
    let stake = must_pay(&info, &denom).map_err(|_| ContractError::CouldNotConvertToBalance {})?;

    let hotkey = deps.api.addr_validate(&hotkey_address)?;
    let coldkey = deps.api.addr_validate(&coldkey_address)?;

    deps.api.debug(&format!(
        "👾 do_sudo_registration ( netuid:{:?} hotkey:{:?}, coldkey:{:?} )",
        netuid, hotkey, coldkey
    ));

    ensure_root(deps.storage, &info.sender)?;

    ensure!(
        netuid != get_root_netuid(),
        ContractError::OperationNotPermittedOnRootSubnet {}
    );

    ensure!(
        if_subnet_exist(deps.storage, netuid),
        ContractError::NetworkDoesNotExist {}
    );

    ensure!(
        !UIDS.has(deps.storage, (netuid, &hotkey)),
        ContractError::AlreadyRegistered {}
    );

    create_account_if_non_existent(deps.storage, &coldkey, &hotkey);
    ensure!(
        coldkey_owns_hotkey(deps.storage, &coldkey, &hotkey),
        ContractError::NonAssociatedColdKey {}
    );
    increase_stake_on_coldkey_hotkey_account(deps.storage, &coldkey, &hotkey, stake.u128() as u64);

    let subnetwork_uid: u16;
    let current_block_number: u64 = env.block.height;
    let current_subnetwork_n: u16 = get_subnetwork_n(deps.storage, netuid);
    if current_subnetwork_n < get_max_allowed_uids(deps.storage, netuid) {
        // --- 12.1.1 No replacement required, the uid appends the subnetwork.
        // We increment the subnetwork count here but not below.
        subnetwork_uid = current_subnetwork_n;

        // --- 12.1.2 Expand subnetwork with new account.
        append_neuron(
            deps.storage,
            deps.api,
            netuid,
            &hotkey,
            current_block_number,
        )?;
        deps.api.debug(&format!("👾 add new neuron account"));
    } else {
        // --- 12.1.1 Replacement required.
        // We take the neuron with the lowest pruning score here.
        subnetwork_uid = get_neuron_to_prune(deps.storage, deps.api, netuid, env.block.height);

        // --- 12.1.1 Replace the neuron account with the new info.
        let _msgs = replace_neuron(
            deps.storage,
            deps.api,
            netuid,
            subnetwork_uid,
            &hotkey,
            current_block_number,
        )?;
        deps.api.debug(&format!("👾 prune neuron"));
    }

    deps.api.debug(&format!(
        "👾 Neuron Registration Sudo ( netuid:{:?} uid:{:?} hotkey:{:?} ) ",
        netuid, subnetwork_uid, hotkey
    ));

    Ok(Response::default()
        .add_attribute("action", "neuron_registered")
        .add_attribute("subnetwork_uid", format!("{}", subnetwork_uid))
        .add_attribute("hotkey", hotkey))
}

//
// # Args:
// 	* 'origin': (<T as frame_system::Config>RuntimeOrigin):
// 		- The signature of the calling coldkey.
//             Burned registers can only be created by the coldkey.
//
// 	* 'netuid' (u16):
// 		- The u16 network identifier.
//
// 	* 'hotkey' ( Addr ):
// 		- Hotkey to be registered to the network.
//
// # Event:
// 	* NeuronRegistered;
// 		- On successfully registereing a uid to a neuron slot on a subnetwork.
//
// # Raises:
// 	* 'NetworkDoesNotExist':
// 		- Attempting to registed to a non existent network.
//
// 	* 'TooManyRegistrationsThisBlock':
// 		- This registration exceeds the total allowed on this network this block.
//
// 	* 'AlreadyRegistered':
// 		- The hotkey is already registered on this network.
//
pub fn do_burned_registration(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    netuid: u16,
    hotkey_address: String,
) -> Result<Response, ContractError> {
    // TODO update burn when token factory bindings will be available
    let denom = DENOM.load(deps.storage)?;
    let amount = must_pay(&info, &denom).map_err(|_| ContractError::CouldNotConvertToBalance {})?;

    // --- 1. Check that the caller has signed the transaction. (the coldkey of the pairing)
    let coldkey = info.sender;
    let hotkey = deps.api.addr_validate(&hotkey_address)?;

    deps.api.debug(&format!(
        "👾 do_burned_registration ( netuid:{:?} hotkey:{:?}, coldkey:{:?} )",
        netuid, hotkey, coldkey
    ));

    // --- 2. Ensure the passed network is valid.
    ensure!(
        netuid != get_root_netuid(),
        ContractError::OperationNotPermittedOnRootSubnet {}
    );
    ensure!(
        if_subnet_exist(deps.storage, netuid),
        ContractError::NetworkDoesNotExist {}
    );

    // --- 3. Ensure the passed network allows registrations.
    ensure!(
        if_subnet_allows_registration(deps.storage, netuid),
        ContractError::RegistrationDisabled {}
    );

    // --- 4. Ensure we are not exceeding the max allowed registrations per block.
    ensure!(
        get_registrations_this_block(deps.storage, netuid)
            < get_max_registrations_per_block(deps.storage, netuid),
        ContractError::TooManyRegistrationsThisBlock {}
    );

    // --- 4. Ensure we are not exceeding the max allowed registrations per interval.
    ensure!(
        get_registrations_this_interval(deps.storage, netuid)
            < get_target_registrations_per_interval(deps.storage, netuid) * 3,
        ContractError::TooManyRegistrationsThisInterval {}
    );

    // --- 4. Ensure that the key is not already registered.
    ensure!(
        !UIDS.has(deps.storage, (netuid, &hotkey)),
        ContractError::AlreadyRegistered {}
    );

    // --- 7. Ensure the callers coldkey has enough stake to perform the transaction.
    let current_block_number: u64 = env.block.height;
    let registration_cost_as_u64 = get_burn_as_u64(deps.storage, netuid);

    ensure!(
        amount.u128() as u64 >= registration_cost_as_u64,
        ContractError::NotEnoughTokens {}
    );

    // The burn occurs here.
    // same as below
    let burn_amount = get_burn_as_u64(deps.storage, netuid);
    burn_tokens(deps.storage, burn_amount)?;

    // --- 9. If the network account does not exist we will create it here.
    create_account_if_non_existent(deps.storage, &coldkey, &hotkey);

    // --- 10. Ensure that the pairing is correct.
    ensure!(
        coldkey_owns_hotkey(deps.storage, &coldkey, &hotkey),
        ContractError::NonAssociatedColdKey {}
    );

    // --- 11. Append neuron or prune it.
    let subnetwork_uid: u16;
    let current_subnetwork_n: u16 = get_subnetwork_n(deps.storage, netuid);

    // Possibly there is no neuron slots at all.
    ensure!(
        get_max_allowed_uids(deps.storage, netuid) != 0,
        ContractError::NetworkDoesNotExist {}
    );

    if current_subnetwork_n < get_max_allowed_uids(deps.storage, netuid) {
        // --- 12.1.1 No replacement required, the uid appends the subnetwork.
        // We increment the subnetwork count here but not below.
        subnetwork_uid = current_subnetwork_n;

        // --- 12.1.2 Expand subnetwork with new account.
        append_neuron(
            deps.storage,
            deps.api,
            netuid,
            &hotkey,
            current_block_number,
        )?;
        deps.api.debug(&format!("👾 add new neuron account"));
    } else {
        // --- 13.1.1 Replacement required.
        // We take the neuron with the lowest pruning score here.
        subnetwork_uid = get_neuron_to_prune(deps.storage, deps.api, netuid, env.block.height);

        // --- 13.1.1 Replace the neuron account with the new info.
        let _msgs = replace_neuron(
            deps.storage,
            deps.api,
            netuid,
            subnetwork_uid,
            &hotkey,
            current_block_number,
        )?;
        deps.api.debug(&format!("👾 prune neuron"));
    }

    // --- 14. Record the registration and increment block and interval counters.
    BURN_REGISTRATIONS_THIS_INTERVAL.update(deps.storage, netuid, |val| -> StdResult<_> {
        match val {
            Some(val) => Ok(val.saturating_add(1)),
            None => Ok(1),
        }
    })?;
    REGISTRATIONS_THIS_INTERVAL.update(deps.storage, netuid, |val| -> StdResult<_> {
        match val {
            Some(val) => Ok(val.saturating_add(1)),
            None => Ok(1),
        }
    })?;
    REGISTRATIONS_THIS_BLOCK.update(deps.storage, netuid, |val| -> StdResult<_> {
        match val {
            Some(val) => Ok(val.saturating_add(1)),
            None => Ok(1),
        }
    })?;
    let burn = get_burn_as_u64(deps.storage, netuid);
    increase_rao_recycled(deps.storage, netuid, burn);

    // --- 15. Deposit successful event.
    deps.api.debug(&format!(
        "👾 Neuron Registration Burned ( netuid:{:?} uid:{:?} hotkey:{:?} ) ",
        netuid,
        subnetwork_uid,
        hotkey.clone()
    ));

    // --- 16. Ok and done.
    Ok(Response::default()
        .add_attribute("action", "neuron_registered")
        .add_attribute("subnetwork_uid", format!("{}", subnetwork_uid))
        .add_attribute("hotkey", hotkey))
}

// ---- The implementation for the extrinsic do_registration.
//
// # Args:
// 	* 'origin': (<T as frame_system::Config>RuntimeOrigin):
// 		- The signature of the calling hotkey.
//
// 	* 'netuid' (u16):
// 		- The u16 network identifier.
//
// 	* 'block_number' ( u64 ):
// 		- Block hash used to prove work done.
//
// 	* 'nonce' ( u64 ):
// 		- Positive integer nonce used in POW.
//
// 	* 'work' ( Vec<u8> ):
// 		- Vector encoded bytes representing work done.
//
// 	* 'hotkey' ( Addr ):
// 		- Hotkey to be registered to the network.
//
// 	* 'coldkey' ( Addr ):
// 		- Associated coldkey account.
//
// # Event:
// 	* NeuronRegistered;
// 		- On successfully registereing a uid to a neuron slot on a subnetwork.
//
// # Raises:
// 	* 'NetworkDoesNotExist':
// 		- Attempting to registed to a non existent network.
//
// 	* 'TooManyRegistrationsThisBlock':
// 		- This registration exceeds the total allowed on this network this block.
//
// 	* 'AlreadyRegistered':
// 		- The hotkey is already registered on this network.
//
// 	* 'InvalidWorkBlock':
// 		- The work has been performed on a stale, future, or non existent block.
//
// 	* 'InvalidDifficulty':
// 		- The work does not match the difficutly.
//
// 	* 'InvalidSeal':
// 		- The seal is incorrect.
//
pub fn do_registration(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    netuid: u16,
    block_number: u64,
    nonce: u64,
    work: Vec<u8>,
    hotkey_address: String,
    coldkey_address: String,
) -> Result<Response, ContractError> {
    // --- 1. Check that the caller has signed the transaction.
    // TODO( const ): This not be the hotkey signature or else an exterior actor can register the hotkey and potentially control it?
    // TODO seriously consider this. Add signatures to the registration.
    let signing_origin = info.sender;
    let hotkey = deps.api.addr_validate(&hotkey_address)?;
    let coldkey = deps.api.addr_validate(&coldkey_address)?;

    deps.api.debug(&format!(
        "👾 do_registration ( origin:{:?} netuid:{:?} hotkey:{:?}, coldkey:{:?} )",
        signing_origin, netuid, hotkey, coldkey
    ));

    ensure!(
        signing_origin == hotkey,
        ContractError::HotkeyOriginMismatch {}
    );

    // --- 2. Ensure the passed network is valid.
    ensure!(
        netuid != get_root_netuid(),
        ContractError::OperationNotPermittedOnRootSubnet {}
    );
    ensure!(
        if_subnet_exist(deps.storage, netuid),
        ContractError::NetworkDoesNotExist {}
    );

    // --- 3. Ensure the passed network allows registrations.
    ensure!(
        if_subnet_allows_registration(deps.storage, netuid),
        ContractError::RegistrationDisabled {}
    );

    // --- 4. Ensure we are not exceeding the max allowed registrations per block.
    // TODO TESTS FAIL HERE
    ensure!(
        get_registrations_this_block(deps.storage, netuid)
            < get_max_registrations_per_block(deps.storage, netuid),
        ContractError::TooManyRegistrationsThisBlock {}
    );

    // --- 5. Ensure we are not exceeding the max allowed registrations per interval.
    ensure!(
        get_registrations_this_interval(deps.storage, netuid)
            < get_target_registrations_per_interval(deps.storage, netuid) * 3,
        ContractError::TooManyRegistrationsThisInterval {}
    );

    // --- 6. Ensure that the key is not already registered.
    ensure!(
        !UIDS.has(deps.storage, (netuid, &hotkey.clone())),
        ContractError::AlreadyRegistered {}
    );

    // --- 7. Ensure the passed block number is valid, not in the future or too old.
    // Work must have been done within 3 blocks (stops long range attacks).
    let current_block_number: u64 = env.block.height;
    ensure!(
        block_number <= current_block_number,
        ContractError::InvalidWorkBlock {}
    );
    ensure!(
        current_block_number - block_number < 3,
        ContractError::InvalidWorkBlock {}
    );

    // --- 8. Ensure the supplied work passes the difficulty.
    let difficulty = get_difficulty(deps.storage, netuid);
    let work_hash: H256 = vec_to_hash(work.clone());
    ensure!(
        hash_meets_difficulty(&work_hash, difficulty),
        ContractError::InvalidDifficulty {}
    ); // Check that the work meets difficulty.

    // --- 7. Check Work is the product of the nonce, the block number, and hotkey. Add this as used work.
    let seal: H256 = create_seal_hash(block_number, nonce, hotkey.as_str());
    ensure!(seal == work_hash, ContractError::InvalidSeal {});
    USED_WORK.save(deps.storage, work.clone(), &current_block_number)?;

    // --- 9. If the network account does not exist we will create it here.
    create_account_if_non_existent(deps.storage, &coldkey, &hotkey);

    // --- 10. Ensure that the pairing is correct.
    ensure!(
        coldkey_owns_hotkey(deps.storage, &coldkey, &hotkey),
        ContractError::NonAssociatedColdKey {}
    );

    // --- 11. Append neuron or prune it.
    let subnetwork_uid: u16;
    let current_subnetwork_n: u16 = get_subnetwork_n(deps.storage, netuid);

    // Possibly there is no neuron slots at all.
    ensure!(
        get_max_allowed_uids(deps.storage, netuid) != 0,
        ContractError::NetworkDoesNotExist {}
    );

    if current_subnetwork_n < get_max_allowed_uids(deps.storage, netuid) {
        // --- 11.1.1 No replacement required, the uid appends the subnetwork.
        // We increment the subnetwork count here but not below.
        subnetwork_uid = current_subnetwork_n;

        // --- 11.1.2 Expand subnetwork with new account.
        append_neuron(
            deps.storage,
            deps.api,
            netuid,
            &hotkey.clone(),
            current_block_number,
        )?;
        deps.api.debug(&format!("👾 add new neuron account"));
    } else {
        // --- 11.1.1 Replacement required.
        // We take the neuron with the lowest pruning score here.
        subnetwork_uid = get_neuron_to_prune(deps.storage, deps.api, netuid, current_block_number);

        // --- 11.1.1 Replace the neuron account with the new info.
        let _msgs = replace_neuron(
            deps.storage,
            deps.api,
            netuid,
            subnetwork_uid,
            &hotkey.clone(),
            current_block_number,
        )?;
        deps.api.debug(&format!("👾 prune neuron"));
    }

    // --- 12. Record the registration and increment block and interval counters.
    POW_REGISTRATIONS_THIS_INTERVAL.update(deps.storage, netuid, |val| -> StdResult<_> {
        match val {
            Some(val) => Ok(val.saturating_add(1)),
            None => Ok(1),
        }
    })?;
    REGISTRATIONS_THIS_INTERVAL.update(deps.storage, netuid, |val| -> StdResult<_> {
        match val {
            Some(val) => Ok(val.saturating_add(1)),
            None => Ok(1),
        }
    })?;
    REGISTRATIONS_THIS_BLOCK.update(deps.storage, netuid, |val| -> StdResult<_> {
        match val {
            Some(val) => Ok(val.saturating_add(1)),
            None => Ok(1),
        }
    })?;

    // --- 13. Deposit successful event.
    deps.api.debug(&format!(
        "👾 Neuron Registration PoW( netuid:{:?} uid:{:?} hotkey:{:?} ) ",
        netuid, subnetwork_uid, hotkey
    ));

    // --- 14. Ok and done.
    Ok(Response::default()
        .add_attribute("action", "neuron_registered")
        .add_attribute("subnetwork_uid", format!("{}", subnetwork_uid))
        .add_attribute("hotkey", hotkey))
}

#[cfg(feature = "pow-faucet")]
pub fn do_faucet(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    block_number: u64,
    nonce: u64,
    work: Vec<u8>,
) -> Result<Response, ContractError> {
    // --- 0. Ensure the faucet is enabled.
    ensure!(
        ALLOW_FAUCET.load(deps.storage)?,
        ContractError::FaucetDisabled {}
    );

    // --- 1. Check that the caller has signed the transaction.
    let coldkey = info.sender;
    deps.api
        .debug(&format!("do_faucet( coldkey:{:?} )", coldkey));

    // --- 2. Ensure the passed block number is valid, not in the future or too old.
    // Work must have been done within 3 blocks (stops long range attacks).
    let current_block_number: u64 = env.block.height;
    ensure!(
        block_number <= env.block.height,
        ContractError::InvalidWorkBlock {}
    );
    ensure!(
        env.block.height - block_number < 3,
        ContractError::InvalidWorkBlock {}
    );

    // --- 3. Ensure the supplied work passes the difficulty.
    // let difficulty = U256::from(1_000_000); // Base faucet difficulty.
    // let work_hash: H256 = vec_to_hash(work.clone());
    // ensure!(
    //     hash_meets_difficulty(&work_hash, difficulty),
    //     ContractError::InvalidDifficulty {}
    // ); // Check that the work meets difficulty.

    // --- 4. Check Work is the product of the nonce, the block number, and hotkey. Add this as used work.
    // let seal: H256 = create_seal_hash(block_number, nonce, coldkey.as_str());
    // ensure!(seal == work_hash, ContractError::InvalidSeal {});
    // USED_WORK.save(deps.storage, work.clone(), &current_block_number)?;

    // --- 5. Add Balance via faucet.
    let balance_to_add: u64 = 100_000_000_000;
    let balance_to_be_added_as_balance = u64_to_balance(balance_to_add);
    add_balance_to_coldkey_account(&coldkey, balance_to_be_added_as_balance.unwrap());
    TOTAL_ISSUANCE.update(deps.storage, |total_issuance| -> StdResult<_> {
        Ok(total_issuance.saturating_add(balance_to_add))
    })?;

    // --- 6. Deposit successful event.
    deps.api.debug(&format!(
        "Faucet( coldkey:{:?} amount:{:?} ) ",
        coldkey, balance_to_add
    ));
    // TODO Create send tokens msgs and add to response

    // --- 7. Ok and done.
    Ok(Response::default()
        .add_attribute("action", "faucet")
        .add_attribute("coldkey", coldkey)
        .add_attribute("amount", format!("{}", balance_to_add)))
}

pub fn vec_to_hash(vec_hash: Vec<u8>) -> H256 {
    let de_ref_hash = &vec_hash; // b: &Vec<u8>
    let de_de_ref_hash: &[u8] = &de_ref_hash; // c: &[u8]
    let real_hash: H256 = H256::from_slice(de_de_ref_hash);
    return real_hash;
}

// Determine which peer to prune from the network by finding the element with the lowest pruning score out of
// immunity period. If all neurons are in immunity period, return node with lowest prunning score.
// This function will always return an element to prune.
pub fn get_neuron_to_prune(
    store: &mut dyn Storage,
    api: &dyn Api,
    netuid: u16,
    current_block: u64,
) -> u16 {
    let mut min_score: u16 = u16::MAX;
    let mut min_score_in_immunity_period = u16::MAX;
    let mut uid_with_min_score = 0;
    let mut uid_with_min_score_in_immunity_period: u16 = 0;
    if get_subnetwork_n(store, netuid) == 0 {
        return 0;
    } // If there are no neurons in this network.
    for neuron_uid_i in 0..get_subnetwork_n(store, netuid) {
        let pruning_score: u16 = get_pruning_score_for_uid(store, netuid, neuron_uid_i);
        let block_at_registration: u64 =
            get_neuron_block_at_registration(store, netuid, neuron_uid_i);
        let current_block: u64 = current_block;
        let immunity_period: u64 = get_immunity_period(store, netuid) as u64;
        if min_score == pruning_score {
            if current_block - block_at_registration < immunity_period {
                //neuron is in immunity period
                if min_score_in_immunity_period > pruning_score {
                    min_score_in_immunity_period = pruning_score;
                    uid_with_min_score_in_immunity_period = neuron_uid_i;
                }
            } else {
                min_score = pruning_score;
                uid_with_min_score = neuron_uid_i;
            }
        }
        // Find min pruning score.
        else if min_score > pruning_score {
            if current_block - block_at_registration < immunity_period {
                //neuron is in immunity period
                if min_score_in_immunity_period > pruning_score {
                    min_score_in_immunity_period = pruning_score;
                    uid_with_min_score_in_immunity_period = neuron_uid_i;
                }
            } else {
                min_score = pruning_score;
                uid_with_min_score = neuron_uid_i;
            }
        }
    }
    if min_score == u16::MAX {
        //all neuorns are in immunity period
        set_pruning_score_for_uid(
            store,
            api,
            netuid,
            uid_with_min_score_in_immunity_period,
            u16::MAX,
        );
        return uid_with_min_score_in_immunity_period;
    } else {
        // We replace the pruning score here with u16 max to ensure that all peers always have a
        // pruning score. In the event that every peer has been pruned this function will prune
        // the last element in the network continually.
        set_pruning_score_for_uid(store, api, netuid, uid_with_min_score, u16::MAX);
        return uid_with_min_score;
    }
}

// Determine whether the given hash satisfies the given difficulty.
// The test is done by multiplying the two together. If the product
// overflows the bounds of U256, then the product (and thus the hash)
// was too high.
pub fn hash_meets_difficulty(hash: &H256, difficulty: U256) -> bool {
    let bytes: &[u8] = &hash.as_bytes();
    let num_hash = U256::from(bytes);
    let (_, overflowed) = num_hash.overflowing_mul(difficulty);

    // log::trace!(
    //     target: LOG_TARGET,
    //     "Difficulty: hash: {:?}, hash_bytes: {:?}, hash_as_num: {:?}, difficulty: {:?}, value: {:?} overflowed: {:?}",
    //     hash,
    //     bytes,
    //     num_hash,
    //     difficulty,
    //     value,
    //     overflowed
    // );
    !overflowed
}

pub fn get_block_hash_from_u64(_: u64) -> H256 {
    // TODO cosmwasm don't have api to access block hash, not possible to access hash because hash is result of execution
    // let block_number: T::BlockNumber = TryInto::<T::BlockNumber>::try_into(block_number)
    //     .ok()
    //     .expect("convert u64 to block number.");
    // let block_hash_at_number: <T as frame_system::Config>::Hash =
    //     system::Pallet::<T>::block_hash(block_number);
    // let vec_hash: Vec<u8> = block_hash_at_number.as_ref().into_iter().cloned().collect();
    // let deref_vec_hash: &[u8] = &vec_hash; // c: &[u8]
    // let real_hash: H256 = H256::from_slice(deref_vec_hash);

    // log::trace!(
    //     target: LOG_TARGET,
    //     "block_number: {:?}, vec_hash: {:?}, real_hash: {:?}",
    //     block_number,
    //     vec_hash,
    //     real_hash
    // );
    let real_hash = H256::zero();

    return real_hash;
}

pub fn hash_to_vec(hash: H256) -> Vec<u8> {
    let hash_as_bytes: &[u8] = hash.as_bytes();
    let hash_as_vec: Vec<u8> = hash_as_bytes.iter().cloned().collect();
    return hash_as_vec;
}

pub fn hash_block_and_hotkey(block_hash_bytes: &[u8], hotkey: &str) -> H256 {
    // Get the public key from the account id.
    // let hotkey_pubkey: MultiAddress<Addr, ()> = MultiAddress::Id(hotkey.clone());
    // let binding = hotkey_pubkey.encode();
    // Skip extra 0th byte.
    // let hotkey_bytes: &[u8] = binding[1..].as_ref();

    let hotkey_bytes: [u8; 32] = keccak_256(hotkey.as_bytes());

    let full_bytes: &[u8; 64] = &[
        block_hash_bytes[0],
        block_hash_bytes[1],
        block_hash_bytes[2],
        block_hash_bytes[3],
        block_hash_bytes[4],
        block_hash_bytes[5],
        block_hash_bytes[6],
        block_hash_bytes[7],
        block_hash_bytes[8],
        block_hash_bytes[9],
        block_hash_bytes[10],
        block_hash_bytes[11],
        block_hash_bytes[12],
        block_hash_bytes[13],
        block_hash_bytes[14],
        block_hash_bytes[15],
        block_hash_bytes[16],
        block_hash_bytes[17],
        block_hash_bytes[18],
        block_hash_bytes[19],
        block_hash_bytes[20],
        block_hash_bytes[21],
        block_hash_bytes[22],
        block_hash_bytes[23],
        block_hash_bytes[24],
        block_hash_bytes[25],
        block_hash_bytes[26],
        block_hash_bytes[27],
        block_hash_bytes[28],
        block_hash_bytes[29],
        block_hash_bytes[30],
        block_hash_bytes[31],
        hotkey_bytes[0],
        hotkey_bytes[1],
        hotkey_bytes[2],
        hotkey_bytes[3],
        hotkey_bytes[4],
        hotkey_bytes[5],
        hotkey_bytes[6],
        hotkey_bytes[7],
        hotkey_bytes[8],
        hotkey_bytes[9],
        hotkey_bytes[10],
        hotkey_bytes[11],
        hotkey_bytes[12],
        hotkey_bytes[13],
        hotkey_bytes[14],
        hotkey_bytes[15],
        hotkey_bytes[16],
        hotkey_bytes[17],
        hotkey_bytes[18],
        hotkey_bytes[19],
        hotkey_bytes[20],
        hotkey_bytes[21],
        hotkey_bytes[22],
        hotkey_bytes[23],
        hotkey_bytes[24],
        hotkey_bytes[25],
        hotkey_bytes[26],
        hotkey_bytes[27],
        hotkey_bytes[28],
        hotkey_bytes[29],
        hotkey_bytes[30],
        hotkey_bytes[31],
    ];
    let keccak_256_seal_hash_vec: [u8; 32] = keccak_256(full_bytes);
    let seal_hash: H256 = H256::from_slice(&keccak_256_seal_hash_vec);

    return seal_hash;
}

pub fn create_seal_hash(block_number_u64: u64, nonce_u64: u64, hotkey: &str) -> H256 {
    let nonce = U256::from(nonce_u64);
    let block_hash_at_number: H256 = get_block_hash_from_u64(block_number_u64);
    let block_hash_bytes: &[u8] = block_hash_at_number.as_bytes();
    let binding = hash_block_and_hotkey(block_hash_bytes, hotkey);
    let block_and_hotkey_hash_bytes: &[u8] = binding.as_bytes();

    let full_bytes: &[u8; 40] = &[
        nonce.byte(0),
        nonce.byte(1),
        nonce.byte(2),
        nonce.byte(3),
        nonce.byte(4),
        nonce.byte(5),
        nonce.byte(6),
        nonce.byte(7),
        block_and_hotkey_hash_bytes[0],
        block_and_hotkey_hash_bytes[1],
        block_and_hotkey_hash_bytes[2],
        block_and_hotkey_hash_bytes[3],
        block_and_hotkey_hash_bytes[4],
        block_and_hotkey_hash_bytes[5],
        block_and_hotkey_hash_bytes[6],
        block_and_hotkey_hash_bytes[7],
        block_and_hotkey_hash_bytes[8],
        block_and_hotkey_hash_bytes[9],
        block_and_hotkey_hash_bytes[10],
        block_and_hotkey_hash_bytes[11],
        block_and_hotkey_hash_bytes[12],
        block_and_hotkey_hash_bytes[13],
        block_and_hotkey_hash_bytes[14],
        block_and_hotkey_hash_bytes[15],
        block_and_hotkey_hash_bytes[16],
        block_and_hotkey_hash_bytes[17],
        block_and_hotkey_hash_bytes[18],
        block_and_hotkey_hash_bytes[19],
        block_and_hotkey_hash_bytes[20],
        block_and_hotkey_hash_bytes[21],
        block_and_hotkey_hash_bytes[22],
        block_and_hotkey_hash_bytes[23],
        block_and_hotkey_hash_bytes[24],
        block_and_hotkey_hash_bytes[25],
        block_and_hotkey_hash_bytes[26],
        block_and_hotkey_hash_bytes[27],
        block_and_hotkey_hash_bytes[28],
        block_and_hotkey_hash_bytes[29],
        block_and_hotkey_hash_bytes[30],
        block_and_hotkey_hash_bytes[31],
    ];
    let sha256_seal_hash_vec: [u8; 32] = sha2_256(full_bytes);
    let keccak_256_seal_hash_vec: [u8; 32] = keccak_256(&sha256_seal_hash_vec);
    let seal_hash: H256 = H256::from_slice(&keccak_256_seal_hash_vec);

    // log::trace!(
    //     "\n hotkey:{:?} \nblock_number: {:?}, \nnonce_u64: {:?}, \nblock_hash: {:?}, \nfull_bytes: {:?}, \nsha256_seal_hash_vec: {:?},  \nkeccak_256_seal_hash_vec: {:?}, \nseal_hash: {:?}",
    //     hotkey,
    //     block_number_u64,
    //     nonce_u64,
    //     block_hash_at_number,
    //     full_bytes,
    //     sha256_seal_hash_vec,
    //     keccak_256_seal_hash_vec,
    //     seal_hash
    // );

    return seal_hash;
}

pub fn get_difficulty(store: &dyn Storage, netuid: u16) -> U256 {
    U256::from(get_difficulty_as_u64(store, netuid))
}

// Helper function for creating nonce and work.
// TODO rewrite to use only address and nonce
pub fn create_work_for_block_number(
    store: &dyn Storage,
    netuid: u16,
    block_number: u64,
    start_nonce: u64,
    hotkey: &str,
) -> (u64, Vec<u8>) {
    let difficulty = get_difficulty(store, netuid);
    let mut nonce: u64 = start_nonce;
    let mut work: H256 = create_seal_hash(block_number, nonce, hotkey);
    while !hash_meets_difficulty(&work, difficulty) {
        nonce = nonce + 1;
        work = create_seal_hash(block_number, nonce, hotkey);
    }
    let vec_work: Vec<u8> = hash_to_vec(work);
    return (nonce, vec_work);
}
