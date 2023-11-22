use std::fs::File;
use std::io::Write;

use cosmwasm_std::testing::{mock_env, mock_info, MockApi, MockQuerier};
use cosmwasm_std::{Addr, Coin, DepsMut, Empty, Env, MemoryStorage, OwnedDeps, Response, Storage};
use cw_multi_test::{App, AppBuilder, Contract, ContractWrapper, Executor};
use cw_storage_gas_meter::MemoryStorageWithGas;

use crate::contract::{execute, instantiate, query};
use crate::msg::ExecuteMsg;
use crate::root::init_new_network;
use crate::utils::{get_difficulty_as_u64, set_difficulty, set_network_registration_allowed};
use crate::ContractError;

const CT_ADDR: &str = "contract0";
pub(crate) const ROOT: &str = "root";
const ADDR1: &str = "addr1";
const ADDR2: &str = "addr2";
const ADDR3: &str = "addr3";
const ADDR4: &str = "addr4";
const ADDR5: &str = "addr5";
const ADDR6: &str = "addr6";
const ADDR7: &str = "addr7";
const ADDR8: &str = "addr8";

fn mock_app(contract_balance: &[Coin]) -> App {
    AppBuilder::new()
        .with_storage(MemoryStorage::new())
        .build(|r, _, storage| {
            r.bank
                .init_balance(
                    storage,
                    &Addr::unchecked("contract0"),
                    contract_balance.to_vec(),
                )
                .unwrap();
        })
}

pub fn mock_dependencies(
    contract_balance: &[Coin],
) -> OwnedDeps<MemoryStorageWithGas, MockApi, MockQuerier> {
    let contract_addr = Addr::unchecked(CT_ADDR);
    OwnedDeps {
        storage: MemoryStorageWithGas::new(),
        api: MockApi::default(),
        querier: MockQuerier::new(&[(&contract_addr.to_string(), contract_balance)]),
        custom_query_type: Default::default(),
    }
}

pub fn cn_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(execute, instantiate, query);
    Box::new(contract)
}

pub type TestDeps = OwnedDeps<MemoryStorageWithGas, MockApi, MockQuerier<Empty>>;

pub fn instantiate_contract() -> (TestDeps, Env) {
    let mut deps = mock_dependencies(&[]);
    let msg = crate::msg::InstantiateMsg {
        stakes: vec![
            (
                Addr::unchecked(ROOT),
                vec![(Addr::unchecked(ROOT), (100, 1))],
            ),
            (
                Addr::unchecked(ADDR1),
                vec![(Addr::unchecked(ADDR2), (100, 1))],
            ),
            (
                Addr::unchecked(ADDR3),
                vec![(Addr::unchecked(ADDR4), (100, 1))],
            ),
        ],
        balances_issuance: 300,
    };

    let mut env = mock_env();
    env.block.height = 1;

    let info = mock_info(ROOT, &[]);
    let res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();
    root_register(deps.as_mut(), env.clone(), Addr::unchecked(ROOT));
    assert_eq!(res.messages.len(), 0);
    (deps, env)
}

pub fn instantiate_contract_app(app: &mut App) -> Addr {
    let cn_id = app.store_code(cn_contract());
    let msg = crate::msg::InstantiateMsg {
        stakes: vec![
            // (Addr::unchecked(ROOT), vec![(Addr::unchecked(ROOT), (100, 1))]),
            // (Addr::unchecked(ADDR1), vec![(Addr::unchecked(ADDR2), (100, 1))]),
            // (Addr::unchecked(ADDR3), vec![(Addr::unchecked(ADDR4), (100, 1))]),
        ],
        balances_issuance: 300,
    };

    app.instantiate_contract(cn_id, Addr::unchecked(ROOT), &msg, &[], "cybernet", None)
        .unwrap()
}

pub fn register_ok_neuron_app(app: &mut App, netuid: u16, hotkey: Addr, coldkey: Addr, nonce: u64) {
    let msg = ExecuteMsg::Register {
        netuid,
        block_number: app.block_info().height,
        nonce,
        work: vec![],
        hotkey: hotkey.clone(),
        coldkey,
    };

    let res = app.execute_contract(hotkey, Addr::unchecked(CT_ADDR), &msg, &[]);
    // app.update_block(|block| block.height += 100);
    assert_eq!(res.is_ok(), true);
}

pub fn register_ok_neuron(
    deps: DepsMut,
    env: Env,
    netuid: u16,
    hotkey: &Addr,
    coldkey: &Addr,
    start_nonce: u64,
) {
    let msg = ExecuteMsg::Register {
        netuid,
        block_number: env.block.height,
        nonce: start_nonce,
        work: vec![],
        hotkey: hotkey.clone(),
        coldkey: coldkey.clone(),
    };

    let info = mock_info(hotkey.as_str(), &[]);
    let res = execute(deps, env, info, msg);
    assert_eq!(res.is_ok(), true);
}

pub fn pow_register_ok_neuron(
    deps: DepsMut,
    env: Env,
    netuid: u16,
    block_number: u64,
    start_nonce: u64,
    work: Vec<u8>,
    hotkey: &Addr,
    coldkey: &Addr,
) -> Result<Response, ContractError> {
    let msg = ExecuteMsg::Register {
        netuid,
        block_number,
        nonce: start_nonce,
        work,
        hotkey: hotkey.clone(),
        coldkey: coldkey.clone(),
    };

    let info = mock_info(hotkey.as_str(), &[]);
    let result = execute(deps, env, info, msg);
    result
}

pub fn sudo_register_ok_neuron(deps: DepsMut, env: Env, netuid: u16, hotkey: Addr, coldkey: Addr) {
    let msg = ExecuteMsg::SudoRegister {
        netuid,
        hotkey: hotkey.clone(),
        coldkey,
        stake: 300,
        balance: 300,
    };

    let env = mock_env();
    let info = mock_info(&ROOT, &[]);
    let res = execute(deps, env, info, msg);
    assert_eq!(res.is_ok(), true);
}

pub fn root_register(deps: DepsMut, env: Env, hotkey: Addr) {
    let msg = ExecuteMsg::RootRegister {
        hotkey: hotkey.clone(),
    };

    let info = mock_info(&ROOT, &[]);
    let res = execute(deps, env, info, msg);
    assert_eq!(res.is_ok(), true);
}

pub fn burned_register_ok_neuron(deps: DepsMut, env: Env, netuid: u16, hotkey: &Addr) {
    let msg = ExecuteMsg::BurnedRegister {
        netuid,
        hotkey: hotkey.clone(),
    };

    let info = mock_info(hotkey.as_str(), &[]);
    let res = execute(deps, env, info, msg);
    assert_eq!(res.is_ok(), true);
}

pub fn add_network_app(app: &mut App) -> u16 {
    let msg = ExecuteMsg::RegisterNetwork {};

    let res = app
        .execute_contract(Addr::unchecked(ROOT), Addr::unchecked(CT_ADDR), &msg, &[])
        .unwrap();
    // let attrs = res.custom_attrs(res.events.len() - 1);
    return res.custom_attrs(1)[1].value.parse().unwrap();
}

pub fn add_network(store: &mut dyn Storage, netuid: u16, tempo: u16, _modality: u16) {
    init_new_network(store, netuid, tempo).unwrap();
    set_network_registration_allowed(store, netuid, true);
}

// TODO revisit block increasing logic before or after step
pub fn step_block(deps: DepsMut, mut env: &mut Env) -> Result<Response, ContractError> {
    env.block.height += 1;
    let result = execute(
        deps,
        env.clone(),
        mock_info("ROOT", &[]),
        ExecuteMsg::BlockStep {},
    );
    result
}

// TODO revisit block increasing logic before or after step
pub fn run_step_to_block(
    mut deps: DepsMut,
    mut env: &mut Env,
    block_number: u64,
) -> Result<Response, ContractError> {
    while env.block.height < block_number {
        env.block.height += 1;
        let result = execute(
            deps.branch(),
            env.clone(),
            mock_info("ROOT", &[]),
            ExecuteMsg::BlockStep {},
        );
        assert!(result.is_ok());
    }
    Ok(Response::default())
}

pub fn set_weights(
    deps: DepsMut,
    env: Env,
    address: &Addr,
    netuid: u16,
    dests: Vec<u16>,
    weights: Vec<u16>,
    version_key: u64,
) -> Result<Response, ContractError> {
    let result = execute(
        deps,
        env.clone(),
        mock_info(address.as_str(), &[]),
        ExecuteMsg::SetWeights {
            netuid,
            dests,
            weights,
            version_key,
        },
    );
    result
}

pub fn serve_axon(
    deps: DepsMut,
    env: Env,
    address: &Addr,
    netuid: u16,
    version: u32,
    ip: u128,
    port: u16,
    ip_type: u8,
    protocol: u8,
    placeholder1: u8,
    placeholder2: u8,
) -> Result<Response, ContractError> {
    let result = execute(
        deps,
        env.clone(),
        mock_info(address.as_str(), &[]),
        ExecuteMsg::ServeAxon {
            netuid,
            version,
            ip,
            port,
            ip_type,
            protocol,
            placeholder1,
            placeholder2,
        },
    );
    result
}

pub fn serve_prometheus(
    deps: DepsMut,
    env: Env,
    address: &Addr,
    netuid: u16,
    version: u32,
    ip: u128,
    port: u16,
    ip_type: u8,
) -> Result<Response, ContractError> {
    let result = execute(
        deps,
        env.clone(),
        mock_info(address.as_str(), &[]),
        ExecuteMsg::ServePrometheus {
            netuid,
            version,
            ip,
            port,
            ip_type,
        },
    );
    result
}

pub fn print_state(app: &mut App, cn_addr: Addr) {
    let mut file = File::create("state_dump.md").unwrap();
    let mut data = String::new();

    let state = app.dump_wasm_raw(&cn_addr);

    data += "| Key | Value |\n | --- | ----- |\n";
    for (k, v) in state.iter() {
        data += &format!(
            "| {:?} | {:?} |\n",
            String::from_utf8(k.clone()).unwrap(),
            String::from_utf8(v.clone()).unwrap()
        )
        .to_string();
    }
    let data = data.replace("\\0\\u", "");
    let data = data.replace("\"", "");

    file.write(data.as_bytes()).unwrap();
}

#[test]
fn test_instantiate() {
    let mut app = mock_app(&[]);

    let cn_addr = instantiate_contract_app(&mut app);
    assert_eq!(cn_addr, Addr::unchecked("contract0"))
}

#[test]
fn test_deps() {
    let (mut deps, _) = instantiate_contract();

    let before = get_difficulty_as_u64(&deps.storage, 1);
    assert_eq!(before, 10000000);

    set_difficulty(&mut deps.storage, 1, 1);

    let after = get_difficulty_as_u64(&deps.storage, 1);
    assert_eq!(after, 1);
}
