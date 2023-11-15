#[cfg(test)]
mod test {
    use cosmwasm_std::{Addr, DepsMut, Empty};
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cw_multi_test::{next_block, App, Contract, ContractWrapper, Executor};
    use crate::contract::instantiate;
    use crate::msg::InstantiateMsg;
    use crate::weights::do_set_weights;

    const CT_ADDR: &str = "cybertensor";
    const ADDR1: &str = "addr1";
    const ADDR2: &str = "addr2";
    const ADDR3: &str = "addr3";
    const ADDR4: &str = "addr4";

    fn do_instantiate(deps: DepsMut) {
        let msg = InstantiateMsg {
            stakes: vec![
                (Addr::unchecked(ADDR1), vec![(Addr::unchecked(ADDR2), (100, 1))]),
                (Addr::unchecked(ADDR3), vec![(Addr::unchecked(ADDR4), (100, 1))]),
            ],
            balances_issuance: 200,
        };
        let info = mock_info("creator", &[]);
        instantiate(deps, mock_env(), info, msg).unwrap();
    }

    #[test]
    fn test_instantiate() {
        let mut deps = mock_dependencies();
        do_instantiate(deps.as_mut());
    }

    #[test]
    fn test_set_weights_dispatch_info_ok() {
        let mut deps = mock_dependencies();
        do_instantiate(deps.as_mut());

        let info = mock_info("genesis", &[]);
        let env = mock_env();

        let dests = vec![1, 1];
        let weights = vec![1, 1];
        let netuid: u16 = 1;
        let version_key: u64 = 0;
        let call = do_set_weights(
            deps.as_mut(),
            env,
            info,
            netuid,
            dests,
            weights,
            version_key,
        );

        assert_eq!(call.is_err(), false);
    }
}
