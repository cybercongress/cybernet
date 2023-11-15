use cosmwasm_std::{Addr, Api, Order, StdError, StdResult, Storage};
use crate::ContractError;
use crate::staking::{unstake_all_coldkeys_from_hotkey_account};
use crate::state::{SUBNETWORK_N, KEYS, UIDS, IS_NETWORK_MEMBER, BLOCK_AT_REGISTRATION, RANK, TRUST, ACTIVE, EMISSION, CONSENSUS, INCENTIVE, DIVIDENDS, LAST_UPDATE, PRUNING_SCORES, VALIDATOR_TRUST, VALIDATOR_PERMIT, TOTAL_HOTKEY_STAKE};
use crate::utils::set_active_for_uid;

pub fn get_subnetwork_n(store: &dyn Storage, netuid: u16 ) -> u16 {
    SUBNETWORK_N.load(store, netuid.clone()).unwrap()
}

// Replace the neuron under this uid.
pub fn replace_neuron(
    store: &mut dyn Storage,
    api: &dyn Api,
    netuid: u16,
    uid_to_replace: u16,
    new_hotkey: Addr,
    block_number:u64
) -> Result<(), ContractError> {
    api.debug(&format!("replace_neuron( netuid: {:?} | uid_to_replace: {:?} | new_hotkey: {:?} ) ", netuid, uid_to_replace, new_hotkey.to_string()));

    // 1. Get the old hotkey under this position.
    let old_hotkey: Addr = KEYS.load(store,(netuid.clone(), uid_to_replace.clone()))?;

    // 2. Remove previous set memberships.
    UIDS.remove(store,(netuid, &old_hotkey.clone()));
    IS_NETWORK_MEMBER.remove(store, (&old_hotkey.clone(), netuid.clone()));
    KEYS.remove(store, (netuid.clone(), uid_to_replace));

    // 2a. Check if the uid is registered in any other subnetworks.
    let hotkey_is_registered_on_any_network: bool = is_hotkey_registered_on_any_network(store, old_hotkey.clone());
    if !hotkey_is_registered_on_any_network {
        // If not, unstake all coldkeys under this hotkey.
        unstake_all_coldkeys_from_hotkey_account(store, old_hotkey.clone() );
    }

    // 3. Create new set memberships.
    set_active_for_uid(store, netuid, uid_to_replace, true ); // Set to active by default.
    KEYS.save(store, (netuid.clone(), uid_to_replace.clone()), &new_hotkey.clone())?; // Make hotkey - uid association.
    UIDS.save(store, (netuid.clone(), &new_hotkey.clone()), &uid_to_replace)?; // Make uid - hotkey association.
    BLOCK_AT_REGISTRATION.save(store,(netuid.clone(), uid_to_replace.clone()), &block_number)?; // Fill block at registration.
    IS_NETWORK_MEMBER.save(store, (&new_hotkey.clone(), netuid.clone()), &true)?; // Fill network is member.

    Ok(())
}

// Appends the uid to the network.
pub fn append_neuron(
    store: &mut dyn Storage,
    api: &dyn Api,
    netuid: u16,
    new_hotkey: Addr,
    block_number:u64
) -> Result<(), StdError> {

    // 1. Get the next uid. This is always equal to subnetwork_n.
    let next_uid: u16 = get_subnetwork_n(store, netuid.clone());
    api.debug(&format!("append_neuron( netuid: {:?} | next_uid: {:?} | new_hotkey: {:?} ) ", netuid, new_hotkey.to_string(), next_uid.clone() ));
    // 2. Get and increase the uid count.
    SUBNETWORK_N.save(store, netuid.clone(), &(next_uid.clone() + 1))?;

    // 3. Expand Yuma Consensus with new position.
    // TODO revisit updates
    let action = |vec: Option<Vec<u16>>| -> StdResult<_> {
        match vec {
            Some(mut v) => {
                v.push(0);
                Ok(v)
            },
            None => Ok(vec!(0)),
        }
    };
    RANK.update(store, netuid.clone(), action)?;
    TRUST.update(store, netuid.clone(), action)?;
    ACTIVE.update(store, netuid.clone(), |vec| -> StdResult<_> {
        match vec {
            Some(mut v) => {
                v.push(true);
                Ok(v)
            },
            None => Ok(vec!(true)),
        }
    })?;
    EMISSION.update(store, netuid.clone(), |vec| -> StdResult<_> {
        match vec {
            Some(mut v) => {
                v.push(0);
                Ok(v)
            },
            None => Ok(vec!(0)),
        }
    })?;
    CONSENSUS.update(store, netuid.clone(), action)?;
    INCENTIVE.update(store, netuid.clone(), action)?;
    DIVIDENDS.update(store, netuid.clone(), action)?;
    LAST_UPDATE.update(store, netuid.clone(), |vec| -> StdResult<_> {
        match vec {
            Some(mut v) => {
                v.push(0);
                Ok(v)
            },
            None => Ok(vec!(0)),
        }
    })?;
    PRUNING_SCORES.update(store, netuid.clone(), action)?;
    VALIDATOR_TRUST.update(store, netuid.clone(), action)?;
    VALIDATOR_PERMIT.update(store, netuid.clone(), |vec| -> StdResult<_> {
        match vec {
            Some(mut v) => {
                v.push(true);
                Ok(v)
            },
            None => Ok(vec!(true)),
        }
    })?;

    // 4. Insert new account information.
    KEYS.save(store, (netuid.clone(), next_uid.clone()), &new_hotkey.clone())?; // Make hotkey - uid association.
    UIDS.save(store, (netuid.clone(), &new_hotkey.clone()), &next_uid.clone())?; // Make uid - hotkey association.
    BLOCK_AT_REGISTRATION.save(store, (netuid.clone(), next_uid), &block_number)?; // Fill block at registration.
    IS_NETWORK_MEMBER.save(store, (&new_hotkey, netuid), &true )?; // Fill network is member.

    Ok(())
}

// Returns true if the hotkey holds a slot on the network.
//
pub fn is_hotkey_registered_on_network(store: &dyn Storage, netuid: u16, hotkey: &Addr ) -> bool {
    return UIDS.has(store, (netuid, hotkey))
}

// Returs the hotkey under the network uid as a Result. Ok if the uid is taken.
//
pub fn get_hotkey_for_net_and_uid(store: &dyn Storage, netuid: u16, neuron_uid: u16) -> Result<Addr, ContractError> {
    let key = KEYS.may_load(store, (netuid, neuron_uid))?;
    match key {
        Some(key) => Ok(key),
        None => Err(ContractError::NotRegistered {}),
    }
}

// Returns the uid of the hotkey in the network as a Result. Ok if the hotkey has a slot.
//
pub fn get_uid_for_net_and_hotkey(store: &dyn Storage, netuid: u16, hotkey: &Addr) -> Result<u16, ContractError> {
    let key = UIDS.may_load(store, (netuid, hotkey))?;
    match key {
        Some(key) => Ok(key),
        None => Err(ContractError::NotRegistered {}),
    }
}

// Returns the stake of the uid on network or 0 if it doesnt exist.
//
pub fn get_stake_for_uid_and_subnetwork(store: &dyn Storage, netuid: u16, neuron_uid: u16) -> u64 {
    return if KEYS.has(store, (netuid, neuron_uid)) {
        let hotkey = get_hotkey_for_net_and_uid(store, netuid, neuron_uid).unwrap();
        TOTAL_HOTKEY_STAKE.load(store, hotkey).unwrap()
    } else {
        0
    }
}


// Return the total number of subnetworks available on the chain.
//
pub fn get_number_of_subnets(store: &dyn Storage)-> u16 {
    let mut number_of_subnets : u16 = 0;
    for _ in SUBNETWORK_N.range(store, None, None, Order::Descending) {
        number_of_subnets = number_of_subnets + 1;
    };
    return number_of_subnets
}

// Return a list of all networks a hotkey is registered on.
//
pub fn get_registered_networks_for_hotkey(store: &dyn Storage, hotkey: Addr )-> Vec<u16> {
    let mut all_networks: Vec<u16> = vec![];
    for item in IS_NETWORK_MEMBER
        .prefix(&hotkey)
        .range(store, None, None, Order::Ascending)
    {
        let (network, is_registered) = item.unwrap();
        if is_registered { all_networks.push( network ) }
    }

    all_networks
}

// Return true if a hotkey is registered on any network.
//
pub fn is_hotkey_registered_on_any_network(store: &dyn Storage, hotkey: Addr ) -> bool {
    for item in IS_NETWORK_MEMBER
        .prefix(&hotkey)
        .range(store, None, None, Order::Ascending)
    {
        let (_, is_registered) = item.unwrap();
        if is_registered { return true }
    }

    false
}
