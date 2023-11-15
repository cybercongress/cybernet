use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Thrown when the network does not exist.")]
    NetworkDoesNotExist {},

    #[error("Thrown when the network already exists.")]
    NetworkExist {},

    #[error("Thrown when an invalid modality attempted on serve.")]
    InvalidModality {},

    #[error("Thrown when the user tries to serve an axon which is not of type	4 (IPv4) or 6 (IPv6).")]
    InvalidIpType {},

    #[error("Thrown when an invalid IP address is passed to the serve function.")]
    InvalidIpAddress {},

    #[error("Thrown when an invalid port is passed to the serve function.")]
    InvalidPort {},

    #[error("Thrown when the caller requests setting or removing data from a neuron which does not exist in the active set.")]
    NotRegistered {},

    #[error("Thrown when a stake, unstake or subscribe request is made by a coldkey which is not associated with the hotkey account.")]
    NonAssociatedColdKey {},

    #[error("Thrown when the caller requests removing more stake than there exists in the staking account. See: fn remove_stake.")]
    NotEnoughStaketoWithdraw {},

    #[error("//  ---Thrown when the caller requests adding more stake than there exists in the cold key account. See: fn add_stake")]
    NotEnoughBalanceToStake {},

    #[error("Thrown when the caller tries to add stake, but for some reason the requested amount could not be withdrawn from the coldkey account.")]
    BalanceWithdrawalError {},

    #[error("Thrown when the caller attempts to set non-self weights without being a permitted validator.")]
    NoValidatorPermit {},

    #[error("Thrown when the caller attempts to set the weight keys and values but these vectors have different size.")]
    WeightVecNotEqualSize {},

    #[error("Thrown when the caller attempts to set weights with duplicate uids in the weight matrix.")]
    DuplicateUids {},

    #[error("Thrown when a caller attempts to set weight to at least one uid that does not exist in the metagraph.")]
    InvalidUid {},

    #[error("Thrown when the dispatch attempts to set weights on chain with fewer elements than are allowed.")]
    NotSettingEnoughWeights {},

    #[error("Thrown when registrations this block exceeds allowed number.")]
    TooManyRegistrationsThisBlock {},

    #[error("Thrown when the caller requests registering a neuron which already exists in the active set.")]
    AlreadyRegistered {},

    #[error("Thrown if the supplied pow hash block is in the future or negative.")]
    InvalidWorkBlock {},

    #[error("Thrown if the supplied pow hash block does not meet the network difficulty.")]
    InvalidDifficulty {},

    #[error("Thrown if the supplied pow hash seal does not match the supplied work.")]
    InvalidSeal {},

    #[error("Thrown if the vaule is invalid for MaxAllowedUids.")]
    MaxAllowedUIdsNotAllowed {},

    #[error("Thrown when the dispatch attempts to convert between a u64 and T::balance but the call fails.")]
    CouldNotConvertToBalance {},

    #[error("Thrown when the caller requests adding stake for a hotkey to the total stake which already added.")]
    StakeAlreadyAdded {},

    #[error("Thrown when the dispatch attempts to set weights on chain with where any normalized weight is more than MaxWeightLimit.")]
    MaxWeightExceeded {},

    #[error("Thrown when the caller attempts to set a storage value outside of its allowed range.")]
    StorageValueOutOfRange {},

    #[error("Thrown when tempo has not set.")]
    TempoHasNotSet {},

    #[error("Thrown when tempo is not valid.")]
    InvalidTempo {},

    #[error("Thrown when number or recieved emission rates does not match number of networks.")]
    EmissionValuesDoesNotMatchNetworks {},

    #[error("Thrown when emission ratios are not valid (did not sum up to 10^9).")]
    InvalidEmissionValues {},

    #[error("Thrown if the hotkey attempts to become delegate when they are already.")]
    AlreadyDelegate {},

    #[error("Thrown if the hotkey attempts to set weights twice within net_tempo/2 blocks.")]
    SettingWeightsTooFast {},

    #[error("Thrown when a validator attempts to set weights from a validator with incorrect code base key.")]
    IncorrectNetworkVersionKey {},

    #[error("Thrown when an axon or prometheus serving exceeds the rate limit for a registered neuron.")]
    ServingRateLimitExceeded {},

    #[error("Thrown when an error occurs while setting a balance.")]
    BalanceSetError {},

    #[error("Thrown when number of accounts going to be registered exceeds MaxAllowedUids for the network.")]
    MaxAllowedUidsExceeded {},

    #[error("Thrown when the caller attempts to set weights with more uids than allowed.")]
    TooManyUids {},

    #[error("Thrown when a transactor exceeds the rate limit for transactions.")]
    TxRateLimitExceeded {},

    #[error("Thrown when registration is disabled")]
    RegistrationDisabled {},

    #[error("Thrown when registration attempt exceeds allowed in interval")]
    TooManyRegistrationsThisInterval {},

    #[error("Thrown when a function is only available for benchmarking")]
    BenchmarkingOnly {},

    #[error("Thrown when the hotkey passed is not the origin, but it should be")]
    HotkeyOriginMismatch {},

    // {} Senate errors
    #[error("Thrown when attempting to do something to a senate member that is limited")]
    SenateMember {},

    #[error("Thrown when a hotkey attempts to do something only senate members can do")]
    NotSenateMember {},

    #[error("Thrown when a hotkey attempts to join the senate while already being a member")]
    AlreadySenateMember {},

    #[error("Thrown when a hotkey attempts to join the senate without enough stake")]
    BelowStakeThreshold {},

    #[error("Thrown when a hotkey attempts to join the senate without being a delegate first")]
    NotDelegate {},

    #[error("Thrown when an incorrect amount of Netuids are passed as input")]
    IncorrectNetuidsLength {},

    #[error("Thrown when the faucet is disabled")]
    FaucetDisabled {},

    #[error("Thrown when key not subnet owner")]
    NotSubnetOwner {},

    #[error("Thrown when operation not permitted on root subnet")]
    OperationNotPermittedOnRootSubnet {},

    #[error("Thrown when a hotkey attempts to join the root subnet with too little stake")]
    StakeTooLowForRoot {},

    #[error("Thrown when all subnets are in the immunity period")]
    AllNetworksInImmunity {},
}
