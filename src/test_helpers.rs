#![cfg(test)]
use std::fs::File;
use std::io::Write;
use cosmwasm_std::{Addr, Empty};
use cw_multi_test::{App, Contract, ContractWrapper, Executor};

use crate::msg::{ExecuteMsg};

const CT_ADDR: &str = "contract0";
const ROOT: &str = "root";
const ADDR1: &str = "addr1";
const ADDR2: &str = "addr2";
const ADDR3: &str = "addr3";
const ADDR4: &str = "addr4";
const ADDR5: &str = "addr5";
const ADDR6: &str = "addr6";
const ADDR7: &str = "addr7";
const ADDR8: &str = "addr8";

pub fn cn_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}

pub fn instantiate_cn(app: &mut App) -> Addr {
    let cn_id = app.store_code(cn_contract());
    let msg = crate::msg::InstantiateMsg {
        stakes: vec![
            (Addr::unchecked(ROOT), vec![(Addr::unchecked(ROOT), (100, 1))]),
            (Addr::unchecked(ADDR1), vec![(Addr::unchecked(ADDR2), (100, 1))]),
            (Addr::unchecked(ADDR3), vec![(Addr::unchecked(ADDR4), (100, 1))]),
        ],
        balances_issuance: 300,
    };

    app.instantiate_contract(
        cn_id,
        Addr::unchecked(ROOT),
        &msg,
        &[],
        "cybernet",
        None,
    )
        .unwrap()
}

pub fn register_ok_neuron(app: &mut App, netuid: u16, hotkey: Addr, coldkey: Addr, nonce: u64) {
    let msg = ExecuteMsg::Register {
        netuid,
        block_number: app.block_info().height,
        nonce,
        work: vec![],
        hotkey: hotkey.clone(),
        coldkey,
    };

    let res = app.execute_contract(hotkey, Addr::unchecked(CT_ADDR), &msg, &[]);
    assert_eq!(res.is_ok(), true);
}

pub fn add_network(app: &mut App) -> u16 {
    let msg = ExecuteMsg::RegisterNetwork {};

    let res = app.execute_contract(Addr::unchecked(ROOT), Addr::unchecked(CT_ADDR), &msg, &[]).unwrap();
    let attrs = res.custom_attrs(res.events.len() - 1);
    return res.custom_attrs(1)[1].value.parse().unwrap();
}

pub fn print_state(app: &mut App, cn_addr: Addr) {
    let mut file = File::create("state_dump.md").unwrap();
    let mut data = String::new();
    let state = app.dump_wasm_raw(&cn_addr);
    data += "| Key | Value |\n | --- | ----- |\n";
    for (k, v) in state.iter() {
        data += &format!("| {:?} | {:?} |\n", String::from_utf8(k.clone()).unwrap(), String::from_utf8(v.clone()).unwrap()).to_string();
    }
    let data = data.replace("\\0\\u", "");
    let data = data.replace("\"", "");
    file.write(data.as_bytes()).unwrap();
}

#[test]
fn test_instantiate() {
    let mut app = App::default();

    let cn_addr = instantiate_cn(&mut app);
    assert_eq!(cn_addr, Addr::unchecked("contract0"))
}

