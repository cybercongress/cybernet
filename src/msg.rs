use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, CosmosMsg, Empty};

#[cw_serde]
pub struct InstantiateMsg {
    pub stakes: Vec<(String, Vec<(String, (u64, u16))>)>,
    pub balances_issuance: u64,
}

#[cw_serde]
pub enum ExecuteMsg {
    // TODO move to sudo and call as module from sdk
    BlockStep {},

    SetWeights {
        netuid: u16,
        dests: Vec<u16>,
        weights: Vec<u16>,
        version_key: u64,
    },
    BecomeDelegate {
        hotkey: String,
        take: u16,
    },
    AddStake {
        hotkey: String,
        amount_staked: u64,
    },
    RemoveStake {
        hotkey: String,
        amount_unstaked: u64,
    },
    ServeAxon {
        netuid: u16,
        version: u32,
        ip: u128,
        port: u16,
        ip_type: u8,
        protocol: u8,
        placeholder1: u8,
        placeholder2: u8,
    },
    ServePrometheus {
        netuid: u16,
        version: u32,
        ip: u128,
        port: u16,
        ip_type: u8,
    },
    Register {
        netuid: u16,
        block_number: u64,
        nonce: u64,
        work: Vec<u8>,
        hotkey: String,
        coldkey: String,
    },
    RootRegister {
        hotkey: String,
    },
    BurnedRegister {
        netuid: u16,
        hotkey: String,
    },

    RegisterNetwork {},
    DissolveNetwork {
        netuid: u16,
    },
    // Faucet { block_number: u64, nonce: u64, work: Vec<u8> },
    SudoRegister {
        netuid: u16,
        hotkey: String,
        coldkey: String,
        stake: u64,
        balance: u64,
    },
    SudoSetDefaultTake {
        default_take: u16,
    },
    SudoSetServingRateLimit {
        netuid: u16,
        serving_rate_limit: u64,
    },
    SudoSetTxRateLimit {
        tx_rate_limit: u64,
    },
    SudoSetMaxBurn {
        netuid: u16,
        max_burn: u64,
    },
    SudoSetMinBurn {
        netuid: u16,
        min_burn: u64,
    },
    SudoSetMaxDifficulty {
        netuid: u16,
        max_difficulty: u64,
    },
    SudoSetMinDifficulty {
        netuid: u16,
        min_difficulty: u64,
    },
    SudoSetWeightsSetRateLimit {
        netuid: u16,
        weights_set_rate_limit: u64,
    },
    SudoSetWeightsVersionKey {
        netuid: u16,
        weights_version_key: u64,
    },
    SudoSetBondsMovingAverage {
        netuid: u16,
        bonds_moving_average: u64,
    },
    SudoSetMaxAllowedValidators {
        netuid: u16,
        max_allowed_validators: u16,
    },
    SudoSetDifficulty {
        netuid: u16,
        difficulty: u64,
    },
    SudoSetAdjustmentInterval {
        netuid: u16,
        adjustment_interval: u16,
    },
    SudoSetTargetRegistrationsPerInterval {
        netuid: u16,
        target_registrations_per_interval: u16,
    },
    SudoSetActivityCutoff {
        netuid: u16,
        activity_cutoff: u16,
    },
    SudoSetRho {
        netuid: u16,
        rho: u16,
    },
    SudoSetKappa {
        netuid: u16,
        kappa: u16,
    },
    SudoSetMaxAllowedUids {
        netuid: u16,
        max_allowed_uids: u16,
    },
    SudoSetMinAllowedWeights {
        netuid: u16,
        min_allowed_weights: u16,
    },
    SudoSetValidatorPruneLen {
        netuid: u16,
        validator_prune_len: u64,
    },
    SudoSetScalingLawPower {
        netuid: u16,
        scaling_law_power: u16,
    },
    SudoSetImmunityPeriod {
        netuid: u16,
        immunity_period: u16,
    },
    SudoSetMaxWeightLimit {
        netuid: u16,
        max_weight_limit: u16,
    },
    SudoSetMaxRegistrationsPerBlock {
        netuid: u16,
        max_registrations_per_block: u16,
    },
    SudoSetTotalIssuance {
        total_issuance: u64,
    },
    SudoSetTempo {
        netuid: u16,
        tempo: u16,
    },
    SudoSetRaoRecycled {
        netuid: u16,
        rao_recycled: u64,
    },
    // Sudo { call: CosmosMsg<Empty> },
    SudoSetRegistrationAllowed {
        netuid: u16,
        registration_allowed: bool,
    },
    SudoSetAdjustmentAlpha {
        netuid: u16,
        adjustment_alpha: u64,
    },
    SudoSetSubnetOwnerCut {
        cut: u16,
    },
    SudoSetNetworkRateLimit {
        rate_limit: u64,
    },
    SudoSetNetworkImmunityPeriod {
        immunity_period: u64,
    },
    SudoSetNetworkMinLockCost {
        lock_cost: u64,
    },
    SudoSetSubnetLimit {
        max_subnets: u16,
    },
    SudoSetLockReductionInterval {
        interval: u64,
    },

    SudoSetValidatorPermitForUid {
        netuid: u16,
        uid: u16,
        permit: bool,
    },
    SudoSetBlockEmission {
        emission: u64,
    },
}

#[cw_serde]
pub enum SudoMsg {
    BlockStep {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Vec<crate::delegate_info::DelegateInfo>)]
    GetDelegates {},
    #[returns(Option<crate::delegate_info::DelegateInfo>)]
    GetDelegate { delegate_account: String },
    #[returns(Vec<(crate::delegate_info::DelegateInfo, u64)>)]
    GetDelegated { delegatee_account: String },

    #[returns(Vec<crate::neuron_info::NeuronInfoLite>)]
    GetNeuronsLite { netuid: u16 },
    #[returns(Option<crate::neuron_info::NeuronInfoLite>)]
    GetNeuronLite { netuid: u16, uid: u16 },
    #[returns(Vec<crate::neuron_info::NeuronInfo>)]
    GetNeurons { netuid: u16 },
    #[returns(Option<crate::neuron_info::NeuronInfo>)]
    GetNeuron { netuid: u16, uid: u16 },

    #[returns(crate::subnet_info::SubnetInfo)]
    GetSubnetInfo { netuid: u16 },
    #[returns(Vec<crate::subnet_info::SubnetInfo>)]
    GetSubnetsInfo {},
    #[returns(crate::subnet_info::SubnetHyperparams)]
    GetSubnetHyperparams { netuid: u16 },

    #[returns(crate::stake_info::StakeInfo)]
    GetStakeInfoForColdkey { coldkey_account: String },
    #[returns(Vec<crate::stake_info::StakeInfo>)]
    GetStakeInfoForColdkeys { coldkey_accounts: Vec<String> },

    #[returns(u64)]
    GetNetworkRegistrationCost {},

    // TODO added for debugging, remove later
    #[returns(Vec<Vec<u16>>)]
    GetWeights { netuid: u16 },

    #[returns(crate::state_info::StateInfo)]
    GetState {},
}

#[cw_serde]
pub struct MigrateMsg {}
