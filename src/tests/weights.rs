#[cfg(test)]
mod weights {
    use cosmwasm_std::Addr;
    use cw_multi_test::{App, Executor, next_block};

    use crate::ContractError;
    use crate::msg::{ExecuteMsg, QueryMsg};
    use crate::test_helpers::{add_network, instantiate_cn, print_state, register_ok_neuron};
    use crate::weights::{is_self_weight, normalize_weights};

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


    #[test]
    fn test_instantiate() {
        let mut app = App::default();

        let cn_addr = instantiate_cn(&mut app);
        assert_eq!(cn_addr, Addr::unchecked("contract0"))
    }

    #[test]
    fn test_weights_err_no_validator_permit() {
        let mut app = App::default();

        let cn_addr = instantiate_cn(&mut app);

        let hotkey_account_id = Addr::unchecked("55");
        let netuid: u16 = 2;
        let tempo: u16 = 13;
        add_network(&mut app);

        app.execute_contract(
            Addr::unchecked(ROOT),
            cn_addr.clone(),
            &ExecuteMsg::SudoSetMinAllowedWeights {
                netuid,
                min_allowed_weights: 0,
            },
            &[],
        ).unwrap();

        app.execute_contract(
            Addr::unchecked(ROOT),
            cn_addr.clone(),
            &ExecuteMsg::SudoSetMaxAllowedUids {
                netuid,
                max_allowed_uids: 3,
            },
            &[],
        ).unwrap();

        app.execute_contract(
            Addr::unchecked(ROOT),
            cn_addr.clone(),
            &ExecuteMsg::SudoSetMaxWeightLimit {
                netuid,
                max_weight_limit: u16::MAX,
            },
            &[],
        ).unwrap();

        register_ok_neuron(&mut app, netuid, hotkey_account_id.clone(), Addr::unchecked("55"), 0);
        app.update_block(next_block);
        register_ok_neuron(&mut app, netuid, Addr::unchecked(ADDR5), Addr::unchecked(ADDR5), 65555);
        app.update_block(next_block);
        register_ok_neuron(&mut app, netuid, Addr::unchecked(ADDR6), Addr::unchecked(ADDR6), 75555);

        let weights_keys: Vec<u16> = vec![1, 2];
        let weight_values: Vec<u16> = vec![1, 2];
        let msg = ExecuteMsg::SetWeights {
            netuid,
            dests: weights_keys,
            weights: weight_values,
            version_key: 0,
        };
        let err = app.execute_contract(Addr::unchecked(hotkey_account_id.clone()), cn_addr.clone(), &msg, &[])
            .unwrap_err();
        assert_eq!(ContractError::NoValidatorPermit {}, err.downcast().unwrap());

        let neuron_uid: u16 = app.wrap().query_wasm_smart(
            cn_addr.clone(),
            &QueryMsg::GetUidForNetAndHotkey {
                netuid,
                hotkey_account: hotkey_account_id.clone(),
            },
        ).unwrap();

        app.execute_contract(
            Addr::unchecked(ROOT),
            cn_addr.clone(),
            &ExecuteMsg::SudoSetValidatorPermitForUid {
                netuid,
                uid: neuron_uid,
                permit: true,
            },
            &[],
        ).unwrap();

        let weights_keys: Vec<u16> = vec![1, 2];
        let weight_values: Vec<u16> = vec![1, 2];
        let msg = ExecuteMsg::SetWeights {
            netuid,
            dests: weights_keys,
            weights: weight_values,
            version_key: 0,
        };
        let result = app.execute_contract(Addr::unchecked(hotkey_account_id.clone()), cn_addr.clone(), &msg, &[]);
        assert_eq!(result.is_ok(), true)
    }

    #[test]
    fn test_weights_version_key() {
        let mut app = App::default();

        let cn_addr = instantiate_cn(&mut app);

        let hotkey = Addr::unchecked("55");
        let coldkey = Addr::unchecked("66");
        let netuid0: u16 = 2;
        let netuid1: u16 = 3;

        add_network(&mut app);
        add_network(&mut app);

        register_ok_neuron(&mut app, netuid0, hotkey.clone(), coldkey.clone(), 2143124);
        app.update_block(next_block);
        register_ok_neuron(&mut app, netuid1, hotkey.clone(), coldkey.clone(), 2143124);
        app.update_block(next_block);

        let weights_keys: Vec<u16> = vec![0];
        let weight_values: Vec<u16> = vec![1];
        let msg = ExecuteMsg::SetWeights {
            netuid: netuid0,
            dests: weights_keys.clone(),
            weights: weight_values.clone(),
            version_key: 0,
        };
        let result = app.execute_contract(Addr::unchecked(hotkey.clone()), cn_addr.clone(), &msg, &[]);
        assert_eq!(result.is_ok(), true);
        app.update_block(|block| block.height += 100);

        let msg = ExecuteMsg::SetWeights {
            netuid: netuid1,
            dests: weights_keys.clone(),
            weights: weight_values.clone(),
            version_key: 0,
        };
        let result = app.execute_contract(Addr::unchecked(hotkey.clone()), cn_addr.clone(), &msg, &[]);
        assert_eq!(result.is_ok(), true);
        app.update_block(|block| block.height += 100);

        // Set version keys.
        let key0: u64 = 12312;
        let key1: u64 = 20313;

        app.execute_contract(
            Addr::unchecked(ROOT),
            cn_addr.clone(),
            &ExecuteMsg::SudoSetWeightsVersionKey {
                netuid: netuid0,
                weights_version_key: key0,
            },
            &[],
        ).unwrap();

        app.execute_contract(
            Addr::unchecked(ROOT),
            cn_addr.clone(),
            &ExecuteMsg::SudoSetWeightsVersionKey {
                netuid: netuid1,
                weights_version_key: key1,
            },
            &[],
        ).unwrap();

        // Setting works with version key.
        let msg = ExecuteMsg::SetWeights {
            netuid: netuid0,
            dests: weights_keys.clone(),
            weights: weight_values.clone(),
            version_key: key0,
        };
        let result = app.execute_contract(Addr::unchecked(hotkey.clone()), cn_addr.clone(), &msg, &[]);
        assert_eq!(result.is_ok(), true);
        app.update_block(|block| block.height += 100);

        let msg = ExecuteMsg::SetWeights {
            netuid: netuid1,
            dests: weights_keys.clone(),
            weights: weight_values.clone(),
            version_key: key1,
        };
        let result = app.execute_contract(Addr::unchecked(hotkey.clone()), cn_addr.clone(), &msg, &[]);
        assert_eq!(result.is_ok(), true);
        app.update_block(|block| block.height += 100);

        // validator:20313 >= network:12312 (accepted: validator newer)
        let msg = ExecuteMsg::SetWeights {
            netuid: netuid0,
            dests: weights_keys.clone(),
            weights: weight_values.clone(),
            version_key: key1,
        };
        let result = app.execute_contract(Addr::unchecked(hotkey.clone()), cn_addr.clone(), &msg, &[]);
        assert_eq!(result.is_ok(), true);
        app.update_block(|block| block.height += 100);

        // Setting fails with incorrect keys.
        // validator:12312 < network:20313 (rejected: validator not updated)
        let msg = ExecuteMsg::SetWeights {
            netuid: netuid1,
            dests: weights_keys.clone(),
            weights: weight_values.clone(),
            version_key: key0,
        };
        let result = app.execute_contract(Addr::unchecked(hotkey.clone()), cn_addr.clone(), &msg, &[]);
        assert_eq!(result.is_err(), true);

        print_state(&mut app, cn_addr.clone());
    }

    #[test]
    fn test_weights_err_setting_weights_too_fast() {
        let mut app = App::default();

        let cn_addr = instantiate_cn(&mut app);

        let hotkey = Addr::unchecked("55");

        let netuid = add_network(&mut app);

        app.execute_contract(
            Addr::unchecked(ROOT),
            cn_addr.clone(),
            &ExecuteMsg::SudoSetMinAllowedWeights {
                netuid,
                min_allowed_weights: 0,
            },
            &[],
        ).unwrap();

        app.execute_contract(
            Addr::unchecked(ROOT),
            cn_addr.clone(),
            &ExecuteMsg::SudoSetMaxAllowedUids {
                netuid,
                max_allowed_uids: 3,
            },
            &[],
        ).unwrap();

        app.execute_contract(
            Addr::unchecked(ROOT),
            cn_addr.clone(),
            &ExecuteMsg::SudoSetMaxWeightLimit {
                netuid,
                max_weight_limit: u16::MAX,
            },
            &[],
        ).unwrap();

        register_ok_neuron(&mut app, netuid, hotkey.clone(), Addr::unchecked("66"), 0);
        app.update_block(next_block);
        register_ok_neuron(&mut app, netuid, Addr::unchecked("1"), Addr::unchecked("1"), 65555);
        app.update_block(next_block);
        register_ok_neuron(&mut app, netuid, Addr::unchecked("2"), Addr::unchecked("2"), 75555);
        app.update_block(next_block);

        let neuron_uid: u16 = app.wrap().query_wasm_smart(
            cn_addr.clone(),
            &QueryMsg::GetUidForNetAndHotkey {
                netuid,
                hotkey_account: hotkey.clone(),
            },
        ).unwrap();

        app.execute_contract(
            Addr::unchecked(ROOT),
            cn_addr.clone(),
            &ExecuteMsg::SudoSetValidatorPermitForUid {
                netuid,
                uid: neuron_uid,
                permit: true,
            },
            &[],
        ).unwrap();

        app.execute_contract(
            Addr::unchecked(ROOT),
            cn_addr.clone(),
            &ExecuteMsg::SudoSetWeightsSetRateLimit {
                netuid,
                weights_set_rate_limit: 10,
            },
            &[],
        ).unwrap();

        let weights_keys: Vec<u16> = vec![1, 2];
        let weight_values: Vec<u16> = vec![1, 2];

        for i in 1..100 {
            let msg = ExecuteMsg::SetWeights {
                netuid,
                dests: weights_keys.clone(),
                weights: weight_values.clone(),
                version_key: 0,
            };
            let result = app.execute_contract(Addr::unchecked(hotkey.clone()), cn_addr.clone(), &msg, &[]);
            if i % 10 == 1 {
                assert_eq!(result.is_ok(), true);
            } else {
                assert_eq!(ContractError::SettingWeightsTooFast {}, result.unwrap_err().downcast().unwrap());
            }
            app.update_block(next_block);
        }

        print_state(&mut app, cn_addr.clone());
    }

    #[test]
    fn test_weights_err_weights_vec_not_equal_size() {
        let mut app = App::default();

        let cn_addr = instantiate_cn(&mut app);

        let hotkey = Addr::unchecked("55");

        let netuid = add_network(&mut app);

        register_ok_neuron(&mut app, netuid, hotkey.clone(), Addr::unchecked("66"), 0);
        app.update_block(next_block);

        let neuron_uid: u16 = app.wrap().query_wasm_smart(
            cn_addr.clone(),
            &QueryMsg::GetUidForNetAndHotkey {
                netuid,
                hotkey_account: hotkey.clone(),
            },
        ).unwrap();

        app.execute_contract(
            Addr::unchecked(ROOT),
            cn_addr.clone(),
            &ExecuteMsg::SudoSetValidatorPermitForUid {
                netuid,
                uid: neuron_uid,
                permit: true,
            },
            &[],
        ).unwrap();

        let weights_keys: Vec<u16> = vec![1, 2, 3, 4, 5, 6];
        let weight_values: Vec<u16> = vec![1, 2, 3, 4, 5]; // Uneven sizes

        let msg = ExecuteMsg::SetWeights {
            netuid,
            dests: weights_keys.clone(),
            weights: weight_values.clone(),
            version_key: 0,
        };
        let result = app.execute_contract(Addr::unchecked(hotkey.clone()), cn_addr.clone(), &msg, &[]);
        assert_eq!(ContractError::WeightVecNotEqualSize {}, result.unwrap_err().downcast().unwrap());

        print_state(&mut app, cn_addr.clone());
    }

    #[test]
    fn test_weights_err_has_duplicate_ids() {
        let mut app = App::default();

        let cn_addr = instantiate_cn(&mut app);

        let hotkey = Addr::unchecked("666");

        let netuid = add_network(&mut app);

        app.execute_contract(
            Addr::unchecked(ROOT),
            cn_addr.clone(),
            &ExecuteMsg::SudoSetMaxAllowedUids {
                netuid,
                max_allowed_uids: 100,
            },
            &[],
        ).unwrap();

        // Allow many registrations per block.
        app.execute_contract(
            Addr::unchecked(ROOT),
            cn_addr.clone(),
            &ExecuteMsg::SudoSetMaxAllowedUids {
                netuid,
                max_allowed_uids: 100,
            },
            &[],
        ).unwrap();

        app.execute_contract(
            Addr::unchecked(ROOT),
            cn_addr.clone(),
            &ExecuteMsg::SudoSetMaxRegistrationsPerBlock {
                netuid,
                max_registrations_per_block: 100,
            },
            &[],
        ).unwrap();

        app.execute_contract(
            Addr::unchecked(ROOT),
            cn_addr.clone(),
            &ExecuteMsg::SudoSetTargetRegistrationsPerInterval {
                netuid,
                target_registrations_per_interval: 100,
            },
            &[],
        ).unwrap();

        // uid 0
        register_ok_neuron(&mut app, netuid, hotkey.clone(), Addr::unchecked("77"), 0);
        let neuron_uid: u16 = app.wrap().query_wasm_smart(
            cn_addr.clone(),
            &QueryMsg::GetUidForNetAndHotkey {
                netuid,
                hotkey_account: hotkey.clone(),
            },
        ).unwrap();

        app.execute_contract(
            Addr::unchecked(ROOT),
            cn_addr.clone(),
            &ExecuteMsg::SudoSetValidatorPermitForUid {
                netuid,
                uid: neuron_uid,
                permit: true,
            },
            &[],
        ).unwrap();

        // uid 1
        register_ok_neuron(&mut app, netuid, Addr::unchecked("1"), Addr::unchecked("1"), 100000);
        let neuron_uid: u16 = app.wrap().query_wasm_smart(
            cn_addr.clone(),
            &QueryMsg::GetUidForNetAndHotkey {
                netuid,
                hotkey_account: Addr::unchecked("1"),
            },
        ).unwrap();

        // uid 2
        register_ok_neuron(&mut app, netuid, Addr::unchecked("2"), Addr::unchecked("2"), 100000);
        let neuron_uid: u16 = app.wrap().query_wasm_smart(
            cn_addr.clone(),
            &QueryMsg::GetUidForNetAndHotkey {
                netuid,
                hotkey_account: Addr::unchecked("2"),
            },
        ).unwrap();

        // uid 3
        register_ok_neuron(&mut app, netuid, Addr::unchecked("3"), Addr::unchecked("3"), 100000);
        let neuron_uid: u16 = app.wrap().query_wasm_smart(
            cn_addr.clone(),
            &QueryMsg::GetUidForNetAndHotkey {
                netuid,
                hotkey_account: Addr::unchecked("3"),
            },
        ).unwrap();

        let weights_keys: Vec<u16> = vec![1, 1, 1]; // Contains duplicates
        let weight_values: Vec<u16> = vec![1, 2, 3];

        let msg = ExecuteMsg::SetWeights {
            netuid,
            dests: weights_keys.clone(),
            weights: weight_values.clone(),
            version_key: 0,
        };
        let result = app.execute_contract(Addr::unchecked(hotkey.clone()), cn_addr.clone(), &msg, &[]);
        assert_eq!(ContractError::DuplicateUids {}, result.unwrap_err().downcast().unwrap());

        print_state(&mut app, cn_addr.clone());
    }

    #[test]
    fn test_weights_err_max_weight_limit() {
        let mut app = App::default();

        let cn_addr = instantiate_cn(&mut app);

        let netuid = add_network(&mut app);

        app.execute_contract(
            Addr::unchecked(ROOT),
            cn_addr.clone(),
            &ExecuteMsg::SudoSetMaxAllowedUids {
                netuid,
                max_allowed_uids: 5,
            },
            &[],
        ).unwrap();

        app.execute_contract(
            Addr::unchecked(ROOT),
            cn_addr.clone(),
            &ExecuteMsg::SudoSetTargetRegistrationsPerInterval {
                netuid,
                target_registrations_per_interval: 5,
            },
            &[],
        ).unwrap();

        app.execute_contract(
            Addr::unchecked(ROOT),
            cn_addr.clone(),
            &ExecuteMsg::SudoSetMaxWeightLimit {
                netuid,
                max_weight_limit: u16::MAX / 5,
            },
            &[],
        ).unwrap();

        app.execute_contract(
            Addr::unchecked(ROOT),
            cn_addr.clone(),
            &ExecuteMsg::SudoSetMinAllowedWeights {
                netuid,
                min_allowed_weights: 0,
            },
            &[],
        ).unwrap();

        app.execute_contract(
            Addr::unchecked(ROOT),
            cn_addr.clone(),
            &ExecuteMsg::SudoSetMaxRegistrationsPerBlock {
                netuid,
                max_registrations_per_block: 100,
            },
            &[],
        ).unwrap();

        // uid 0
        register_ok_neuron(&mut app, netuid, Addr::unchecked("0"), Addr::unchecked("0"), 55555);
        let neuron_uid: u16 = app.wrap().query_wasm_smart(
            cn_addr.clone(),
            &QueryMsg::GetUidForNetAndHotkey {
                netuid,
                hotkey_account: Addr::unchecked("0"),
            },
        ).unwrap();

        app.execute_contract(
            Addr::unchecked(ROOT),
            cn_addr.clone(),
            &ExecuteMsg::SudoSetValidatorPermitForUid {
                netuid,
                uid: neuron_uid,
                permit: true,
            },
            &[],
        ).unwrap();

        register_ok_neuron(&mut app, netuid, Addr::unchecked("1"), Addr::unchecked("1"), 65555);
        register_ok_neuron(&mut app, netuid, Addr::unchecked("2"), Addr::unchecked("2"), 75555);
        register_ok_neuron(&mut app, netuid, Addr::unchecked("3"), Addr::unchecked("3"), 95555);
        register_ok_neuron(&mut app, netuid, Addr::unchecked("4"), Addr::unchecked("4"), 35555);

        // Non self-weight fails.
        let uids: Vec<u16> = vec![1, 2, 3, 4];
        let values: Vec<u16> = vec![u16::MAX / 4, u16::MAX / 4, u16::MAX / 54, u16::MAX / 4];
        let msg = ExecuteMsg::SetWeights {
            netuid,
            dests: uids,
            weights: values,
            version_key: 0,
        };
        let result = app.execute_contract(Addr::unchecked("0"), cn_addr.clone(), &msg, &[]);
        assert_eq!(ContractError::MaxWeightExceeded {}, result.unwrap_err().downcast().unwrap());

        // Self-weight is a success.
        let uids: Vec<u16> = vec![0]; // Self.
        let values: Vec<u16> = vec![u16::MAX]; // normalizes to u32::MAX
        let msg = ExecuteMsg::SetWeights {
            netuid,
            dests: uids,
            weights: values,
            version_key: 0,
        };
        let result = app.execute_contract(Addr::unchecked("0"), cn_addr.clone(), &msg, &[]);
        assert_eq!(result.is_ok(), true);
    }

    #[test]
    fn test_set_weights_err_not_active() {
        let mut app = App::default();

        let cn_addr = instantiate_cn(&mut app);

        let netuid = add_network(&mut app);

        // uid 0
        register_ok_neuron(&mut app, netuid, Addr::unchecked("666"), Addr::unchecked("2"), 100000);
        let neuron_uid: u16 = app.wrap().query_wasm_smart(
            cn_addr.clone(),
            &QueryMsg::GetUidForNetAndHotkey {
                netuid,
                hotkey_account: Addr::unchecked("666"),
            },
        ).unwrap();

        let weights_keys: Vec<u16> = vec![0]; // Uid 0 is valid.
        let weight_values: Vec<u16> = vec![1];
        // This hotkey is NOT registered.
        let msg = ExecuteMsg::SetWeights {
            netuid,
            dests: weights_keys,
            weights: weight_values,
            version_key: 0,
        };
        let result = app.execute_contract(Addr::unchecked("1"), cn_addr.clone(), &msg, &[]);
        assert_eq!(ContractError::NotRegistered {}, result.unwrap_err().downcast().unwrap());
    }

    #[test]
    fn test_set_weights_err_invalid_uid() {
        let mut app = App::default();

        let cn_addr = instantiate_cn(&mut app);

        let netuid = add_network(&mut app);

        let hotkey_account = Addr::unchecked("55");

        register_ok_neuron(&mut app, netuid, hotkey_account.clone(), Addr::unchecked("66"), 100000);
        let neuron_uid: u16 = app.wrap().query_wasm_smart(
            cn_addr.clone(),
            &QueryMsg::GetUidForNetAndHotkey {
                netuid,
                hotkey_account: hotkey_account.clone(),
            },
        ).unwrap();

        app.execute_contract(
            Addr::unchecked(ROOT),
            cn_addr.clone(),
            &ExecuteMsg::SudoSetValidatorPermitForUid {
                netuid,
                uid: neuron_uid,
                permit: true,
            },
            &[],
        ).unwrap();

        let weight_keys: Vec<u16> = vec![9999]; // Does not exist
        let weight_values: Vec<u16> = vec![88]; // random value
        let msg = ExecuteMsg::SetWeights {
            netuid,
            dests: weight_keys,
            weights: weight_values,
            version_key: 0,
        };
        let result = app.execute_contract(hotkey_account, cn_addr.clone(), &msg, &[]);
        assert_eq!(ContractError::InvalidUid {}, result.unwrap_err().downcast().unwrap());
    }

    #[test]
    fn test_set_weight_not_enough_values() {
        let mut app = App::default();

        let cn_addr = instantiate_cn(&mut app);

        let netuid = add_network(&mut app);

        let hotkey_account = Addr::unchecked("1");

        register_ok_neuron(&mut app, netuid, hotkey_account.clone(), Addr::unchecked("2"), 100000);
        let neuron_uid: u16 = app.wrap().query_wasm_smart(
            cn_addr.clone(),
            &QueryMsg::GetUidForNetAndHotkey {
                netuid,
                hotkey_account: hotkey_account.clone(),
            },
        ).unwrap();

        app.execute_contract(
            Addr::unchecked(ROOT),
            cn_addr.clone(),
            &ExecuteMsg::SudoSetValidatorPermitForUid {
                netuid,
                uid: neuron_uid,
                permit: true,
            },
            &[],
        ).unwrap();

        register_ok_neuron(&mut app, netuid, Addr::unchecked("3"), Addr::unchecked("4"), 300000);

        app.execute_contract(
            Addr::unchecked(ROOT),
            cn_addr.clone(),
            &ExecuteMsg::SudoSetMaxWeightLimit {
                netuid,
                max_weight_limit: u16::MAX,
            },
            &[],
        ).unwrap();

        app.execute_contract(
            Addr::unchecked(ROOT),
            cn_addr.clone(),
            &ExecuteMsg::SudoSetMinAllowedWeights {
                netuid,
                min_allowed_weights: 2,
            },
            &[],
        ).unwrap();

        // Should fail because we are only setting a single value and its not the self weight.
        let weight_keys: Vec<u16> = vec![1]; // not weight.
        let weight_values: Vec<u16> = vec![88]; // random value.
        let msg = ExecuteMsg::SetWeights {
            netuid,
            dests: weight_keys,
            weights: weight_values,
            version_key: 0,
        };
        let result = app.execute_contract(hotkey_account.clone(), cn_addr.clone(), &msg, &[]);
        assert_eq!(ContractError::NotSettingEnoughWeights {}, result.unwrap_err().downcast().unwrap());

        // Shouldnt fail because we setting a single value but it is the self weight.
        let weight_keys: Vec<u16> = vec![0]; // self weight.
        let weight_values: Vec<u16> = vec![88]; // random value.
        let msg = ExecuteMsg::SetWeights {
            netuid,
            dests: weight_keys,
            weights: weight_values,
            version_key: 0,
        };
        let result = app.execute_contract(hotkey_account.clone(), cn_addr.clone(), &msg, &[]);
        assert_eq!(result.is_ok(), true);
        app.update_block(|block| block.height += 100);

        // Should pass because we are setting enough values.
        let weight_keys: Vec<u16> = vec![0, 1]; // self weight.
        let weight_values: Vec<u16> = vec![10, 10]; // random value.

        app.execute_contract(
            Addr::unchecked(ROOT),
            cn_addr.clone(),
            &ExecuteMsg::SudoSetMinAllowedWeights {
                netuid,
                min_allowed_weights: 2,
            },
            &[],
        ).unwrap();

        let msg = ExecuteMsg::SetWeights {
            netuid,
            dests: weight_keys,
            weights: weight_values,
            version_key: 0,
        };
        let result = app.execute_contract(hotkey_account, cn_addr.clone(), &msg, &[]);
        assert_eq!(result.is_ok(), true);
    }

    #[test]
    fn test_set_weight_too_many_uids() {
        let mut app = App::default();

        let cn_addr = instantiate_cn(&mut app);

        let netuid = add_network(&mut app);

        let hotkey_account = Addr::unchecked("1");

        register_ok_neuron(&mut app, netuid, hotkey_account.clone(), Addr::unchecked("2"), 100000);
        let neuron_uid: u16 = app.wrap().query_wasm_smart(
            cn_addr.clone(),
            &QueryMsg::GetUidForNetAndHotkey {
                netuid,
                hotkey_account: hotkey_account.clone(),
            },
        ).unwrap();

        app.execute_contract(
            Addr::unchecked(ROOT),
            cn_addr.clone(),
            &ExecuteMsg::SudoSetValidatorPermitForUid {
                netuid,
                uid: neuron_uid,
                permit: true,
            },
            &[],
        ).unwrap();

        register_ok_neuron(&mut app, netuid, Addr::unchecked("3"), Addr::unchecked("4"), 300000);

        app.execute_contract(
            Addr::unchecked(ROOT),
            cn_addr.clone(),
            &ExecuteMsg::SudoSetMaxWeightLimit {
                netuid,
                max_weight_limit: u16::MAX,
            },
            &[],
        ).unwrap();

        app.execute_contract(
            Addr::unchecked(ROOT),
            cn_addr.clone(),
            &ExecuteMsg::SudoSetMinAllowedWeights {
                netuid,
                min_allowed_weights: 2,
            },
            &[],
        ).unwrap();

        // Should fail because we are setting more weights than there are neurons.
        let weight_keys: Vec<u16> = vec![0, 1, 2, 3, 4]; // more uids than neurons in subnet.
        let weight_values: Vec<u16> = vec![88, 102, 303, 1212, 11]; // random value.
        let msg = ExecuteMsg::SetWeights {
            netuid,
            dests: weight_keys,
            weights: weight_values,
            version_key: 0,
        };
        let result = app.execute_contract(hotkey_account.clone(), cn_addr.clone(), &msg, &[]);
        assert_eq!(ContractError::TooManyUids {}, result.unwrap_err().downcast().unwrap());

        // Shouldnt fail because we are setting less weights than there are neurons.
        let weight_keys: Vec<u16> = vec![0, 1]; // Only on neurons that exist.
        let weight_values: Vec<u16> = vec![10, 10]; // random value.
        let msg = ExecuteMsg::SetWeights {
            netuid,
            dests: weight_keys,
            weights: weight_values,
            version_key: 0,
        };
        let result = app.execute_contract(hotkey_account.clone(), cn_addr.clone(), &msg, &[]);
        assert_eq!(result.is_ok(), true);
    }

    #[test]
    fn test_set_weights_sum_larger_than_u16_max() {
        let mut app = App::default();

        let cn_addr = instantiate_cn(&mut app);

        let netuid = add_network(&mut app);

        let hotkey_account = Addr::unchecked("1");

        register_ok_neuron(&mut app, netuid, hotkey_account.clone(), Addr::unchecked("2"), 100000);
        let neuron_uid: u16 = app.wrap().query_wasm_smart(
            cn_addr.clone(),
            &QueryMsg::GetUidForNetAndHotkey {
                netuid,
                hotkey_account: hotkey_account.clone(),
            },
        ).unwrap();

        app.execute_contract(
            Addr::unchecked(ROOT),
            cn_addr.clone(),
            &ExecuteMsg::SudoSetValidatorPermitForUid {
                netuid,
                uid: neuron_uid,
                permit: true,
            },
            &[],
        ).unwrap();

        register_ok_neuron(&mut app, netuid, Addr::unchecked("3"), Addr::unchecked("4"), 300000);

        app.execute_contract(
            Addr::unchecked(ROOT),
            cn_addr.clone(),
            &ExecuteMsg::SudoSetMaxWeightLimit {
                netuid,
                max_weight_limit: u16::MAX,
            },
            &[],
        ).unwrap();

        app.execute_contract(
            Addr::unchecked(ROOT),
            cn_addr.clone(),
            &ExecuteMsg::SudoSetMinAllowedWeights {
                netuid,
                min_allowed_weights: 2,
            },
            &[],
        ).unwrap();

        // Shouldn't fail because we are setting the right number of weights.
        let weight_keys: Vec<u16> = vec![0, 1];
        let weight_values: Vec<u16> = vec![u16::MAX, u16::MAX];
        // sum of weights is larger than u16 max.
        assert!(weight_values.iter().map(|x| *x as u64).sum::<u64>() > (u16::MAX as u64));

        let msg = ExecuteMsg::SetWeights {
            netuid,
            dests: weight_keys,
            weights: weight_values,
            version_key: 0,
        };
        let result = app.execute_contract(hotkey_account.clone(), cn_addr.clone(), &msg, &[]);
        assert_eq!(result.is_ok(), true);

        // Get max-upscaled unnormalized weights.
        let all_weights: Vec<Vec<u16>> = app.wrap().query_wasm_smart(
            cn_addr.clone(),
            &QueryMsg::GetWeights {
                netuid,
            },
        ).unwrap();
        let weights_set: &Vec<u16> = &all_weights[neuron_uid as usize];
        assert_eq!(weights_set[0], u16::MAX);
        assert_eq!(weights_set[1], u16::MAX);
    }

    #[test]
    fn test_check_length_allows_singleton() {
        let mut app = App::default();

        let cn_addr = instantiate_cn(&mut app);

        let netuid = add_network(&mut app);

        let hotkey_account = Addr::unchecked("1");

        let max_allowed: u16 = 1;
        let min_allowed_weights = max_allowed;

        app.execute_contract(
            Addr::unchecked(ROOT),
            cn_addr.clone(),
            &ExecuteMsg::SudoSetMinAllowedWeights {
                netuid,
                min_allowed_weights,
            },
            &[],
        ).unwrap();

        let uids: Vec<u16> = Vec::from_iter((0..max_allowed).map(|id| id + 1));
        let uid: u16 = uids[0].clone();
        let weights: Vec<u16> = Vec::from_iter((0..max_allowed).map(|id| id + 1));

        let expected = true;
        let result: bool = app.wrap().query_wasm_smart(
            cn_addr.clone(),
            &QueryMsg::CheckLength {
                netuid,
                uid,
                uids,
                weights,
            },
        ).unwrap();

        assert_eq!(expected, result, "Failed get expected result");
    }

    #[test]
    fn test_check_length_weights_length_exceeds_min_allowed() {
        let mut app = App::default();

        let cn_addr = instantiate_cn(&mut app);

        let netuid = add_network(&mut app);

        let hotkey_account = Addr::unchecked("1");

        let max_allowed: u16 = 3;
        let min_allowed_weights = max_allowed;

        app.execute_contract(
            Addr::unchecked(ROOT),
            cn_addr.clone(),
            &ExecuteMsg::SudoSetMinAllowedWeights {
                netuid,
                min_allowed_weights,
            },
            &[],
        ).unwrap();

        let uids: Vec<u16> = Vec::from_iter((0..max_allowed).map(|id| id + 1));
        let uid: u16 = uids[0].clone();
        let weights: Vec<u16> = Vec::from_iter((0..max_allowed).map(|id| id + 1));

        let expected = true;

        let result: bool = app.wrap().query_wasm_smart(
            cn_addr.clone(),
            &QueryMsg::CheckLength {
                netuid,
                uid,
                uids,
                weights,
            },
        ).unwrap();

        assert_eq!(expected, result, "Failed get expected result");
    }

    #[test]
    fn test_check_length_to_few_weights() {
        let mut app = App::default();

        let cn_addr = instantiate_cn(&mut app);

        let netuid = add_network(&mut app);

        let min_allowed_weights = 3;

        app.execute_contract(
            Addr::unchecked(ROOT),
            cn_addr.clone(),
            &ExecuteMsg::SudoSetMaxRegistrationsPerBlock {
                netuid,
                max_registrations_per_block: 100,
            },
            &[],
        ).unwrap();

        app.execute_contract(
            Addr::unchecked(ROOT),
            cn_addr.clone(),
            &ExecuteMsg::SudoSetTargetRegistrationsPerInterval {
                netuid,
                target_registrations_per_interval: 100,
            },
            &[],
        ).unwrap();

        // register morw than min allowed
        register_ok_neuron(&mut app, netuid, Addr::unchecked("1"), Addr::unchecked("1"), 300001);
        register_ok_neuron(&mut app, netuid, Addr::unchecked("2"), Addr::unchecked("2"), 300002);
        register_ok_neuron(&mut app, netuid, Addr::unchecked("3"), Addr::unchecked("3"), 300003);
        register_ok_neuron(&mut app, netuid, Addr::unchecked("4"), Addr::unchecked("4"), 300004);
        register_ok_neuron(&mut app, netuid, Addr::unchecked("5"), Addr::unchecked("5"), 300005);
        register_ok_neuron(&mut app, netuid, Addr::unchecked("6"), Addr::unchecked("6"), 300006);
        register_ok_neuron(&mut app, netuid, Addr::unchecked("7"), Addr::unchecked("7"), 300007);

        app.execute_contract(
            Addr::unchecked(ROOT),
            cn_addr.clone(),
            &ExecuteMsg::SudoSetMinAllowedWeights {
                netuid,
                min_allowed_weights,
            },
            &[],
        ).unwrap();

        let uids: Vec<u16> = Vec::from_iter((0..2).map(|id| id + 1));
        let weights: Vec<u16> = Vec::from_iter((0..2).map(|id| id + 1));
        let uid: u16 = uids[0].clone();

        let expected = false;
        let result: bool = app.wrap().query_wasm_smart(
            cn_addr.clone(),
            &QueryMsg::CheckLength {
                netuid,
                uid,
                uids,
                weights,
            },
        ).unwrap();
        assert_eq!(expected, result, "Failed get expected result");
    }

    #[test]
    fn test_normalize_weights_does_not_mutate_when_sum_is_zero() {
        let max_allowed: u16 = 3;

        let weights: Vec<u16> = Vec::from_iter((0..max_allowed).map(|_| 0));

        let expected = weights.clone();
        let result = normalize_weights(weights);

        assert_eq!(
            expected, result,
            "Failed get expected result when everything _should_ be fine"
        );
    }

    #[test]
    fn test_normalize_weights_does_not_mutate_when_sum_not_zero() {
        let max_allowed: u16 = 3;

        let weights: Vec<u16> = Vec::from_iter((0..max_allowed).map(|weight| weight));

        let expected = weights.clone();
        let result = normalize_weights(weights);

        assert_eq!(expected.len(), result.len(), "Length of weights changed?!");
    }

    #[test]
    fn test_max_weight_limited_allow_self_weights_to_exceed_max_weight_limit() {
        let mut app = App::default();

        let cn_addr = instantiate_cn(&mut app);

        let netuid = add_network(&mut app);

        let max_allowed: u16 = 1;

        let uids: Vec<u16> = Vec::from_iter((0..max_allowed).map(|id| id + 1));
        let uid: u16 = uids[0].clone();
        let weights: Vec<u16> = vec![0];


        let expected = true;
        let result: bool = app.wrap().query_wasm_smart(
            cn_addr.clone(),
            &QueryMsg::GetMaxWeightLimited {
                netuid,
                uid,
                uids,
                weights,
            },
        ).unwrap();

        assert_eq!(
            expected, result,
            "Failed get expected result when everything _should_ be fine"
        );
    }

    #[test]
    fn test_max_weight_limited_when_weight_limit_is_u16_max() {
        let mut app = App::default();

        let cn_addr = instantiate_cn(&mut app);

        let netuid = add_network(&mut app);

        let max_allowed: u16 = 3;

        let uids: Vec<u16> = Vec::from_iter((0..max_allowed).map(|id| id + 1));
        let uid: u16 = uids[0].clone();
        let weights: Vec<u16> = Vec::from_iter((0..max_allowed).map(|_id| u16::MAX));

        let expected = true;

        let result: bool = app.wrap().query_wasm_smart(
            cn_addr.clone(),
            &QueryMsg::GetMaxWeightLimited {
                netuid,
                uid,
                uids,
                weights,
            },
        ).unwrap();

        assert_eq!(
            expected, result,
            "Failed get expected result when everything _should_ be fine"
        );
    }

    #[test]
    fn test_max_weight_limited_when_max_weight_is_within_limit() {
        let mut app = App::default();

        let cn_addr = instantiate_cn(&mut app);

        let netuid = add_network(&mut app);

        let max_allowed: u16 = 1;
        let max_weight_limit = u16::MAX / 5;

        let uids: Vec<u16> = Vec::from_iter((0..max_allowed).map(|id| id + 1));
        let uid: u16 = uids[0].clone();
        let weights: Vec<u16> = Vec::from_iter((0..max_allowed).map(|id| max_weight_limit - id));

        app.execute_contract(
            Addr::unchecked(ROOT),
            cn_addr.clone(),
            &ExecuteMsg::SudoSetMaxWeightLimit {
                netuid,
                max_weight_limit,
            },
            &[],
        ).unwrap();

        let expected = true;
        let result: bool = app.wrap().query_wasm_smart(
            cn_addr.clone(),
            &QueryMsg::GetMaxWeightLimited {
                netuid,
                uid,
                uids,
                weights,
            },
        ).unwrap();
        assert_eq!(
            expected, result,
            "Failed get expected result when everything _should_ be fine"
        );
    }

    #[test]
    fn test_max_weight_limited_when_guard_checks_are_not_triggered() {
        let mut app = App::default();

        let cn_addr = instantiate_cn(&mut app);

        let netuid = add_network(&mut app);

        let max_allowed: u16 = 3;
        let max_weight_limit = u16::MAX / 5;

        let netuid: u16 = 1;
        let uids: Vec<u16> = Vec::from_iter((0..max_allowed).map(|id| id + 1));
        let uid: u16 = uids[0].clone();
        let weights: Vec<u16> = Vec::from_iter((0..max_allowed).map(|id| max_weight_limit + id));

        app.execute_contract(
            Addr::unchecked(ROOT),
            cn_addr.clone(),
            &ExecuteMsg::SudoSetMaxWeightLimit {
                netuid,
                max_weight_limit,
            },
            &[],
        ).unwrap();


        let expected = false;
        let result: bool = app.wrap().query_wasm_smart(
            cn_addr.clone(),
            &QueryMsg::GetMaxWeightLimited {
                netuid,
                uid,
                uids,
                weights,
            },
        ).unwrap();

        assert_eq!(
            expected, result,
            "Failed get expected result when guard-checks were not triggered"
        );
    }

    #[test]
    fn test_is_self_weight_weights_length_not_one() {
        let max_allowed: u16 = 3;

        let uids: Vec<u16> = Vec::from_iter((0..max_allowed).map(|id| id + 1));
        let uid: u16 = uids[0].clone();
        let weights: Vec<u16> = Vec::from_iter((0..max_allowed).map(|id| id + 1));

        let expected = false;
        let result = is_self_weight(uid, &uids, &weights);

        assert_eq!(
            expected, result,
            "Failed get expected result when `weights.len() != 1`"
        );
    }

    #[test]
    fn test_is_self_weight_uid_not_in_uids() {
        let max_allowed: u16 = 3;

        let uids: Vec<u16> = Vec::from_iter((0..max_allowed).map(|id| id + 1));
        let uid: u16 = uids[1].clone();
        let weights: Vec<u16> = vec![0];

        let expected = false;
        let result = is_self_weight(uid, &uids, &weights);

        assert_eq!(
            expected, result,
            "Failed get expected result when `uid != uids[0]`"
        );
    }

    #[test]
    fn test_is_self_weight_uid_in_uids() {
        let max_allowed: u16 = 1;

        let uids: Vec<u16> = Vec::from_iter((0..max_allowed).map(|id| id + 1));
        let uid: u16 = uids[0].clone();
        let weights: Vec<u16> = vec![0];

        let expected = true;
        let result = is_self_weight(uid, &uids, &weights);

        assert_eq!(
            expected, result,
            "Failed get expected result when everything _should_ be fine"
        );
    }

    #[test]
    fn test_check_len_uids_within_allowed_within_network_pool() {
        let mut app = App::default();

        let cn_addr = instantiate_cn(&mut app);

        let netuid = add_network(&mut app);

        let max_registrations_per_block: u16 = 100;

        register_ok_neuron(&mut app, netuid, Addr::unchecked("1"), Addr::unchecked("1"), 0);
        register_ok_neuron(&mut app, netuid, Addr::unchecked("3"), Addr::unchecked("3"), 65555);
        register_ok_neuron(&mut app, netuid, Addr::unchecked("5"), Addr::unchecked("5"), 75555);

        let max_allowed: u16 = app.wrap().query_wasm_smart(
            cn_addr.clone(),
            &QueryMsg::GetSubnetworkN {
                netuid,
            },
        ).unwrap();

        let uids: Vec<u16> = Vec::from_iter((0..max_allowed).map(|uid| uid));

        let expected = true;
        let result: bool = app.wrap().query_wasm_smart(
            cn_addr.clone(),
            &QueryMsg::CheckLenUidsWithingAllowed {
                netuid,
                uids,
            },
        ).unwrap();
        assert_eq!(
            expected, result,
            "netuid network length and uids length incompatible"
        );
    }

    #[test]
    fn test_check_len_uids_within_allowed_not_within_network_pool() {
        let mut app = App::default();

        let cn_addr = instantiate_cn(&mut app);

        let netuid = add_network(&mut app);

        let max_registrations_per_block: u16 = 100;

        register_ok_neuron(&mut app, netuid, Addr::unchecked("1"), Addr::unchecked("1"), 0);
        register_ok_neuron(&mut app, netuid, Addr::unchecked("3"), Addr::unchecked("3"), 65555);
        register_ok_neuron(&mut app, netuid, Addr::unchecked("5"), Addr::unchecked("5"), 75555);

        let max_allowed: u16 = app.wrap().query_wasm_smart(
            cn_addr.clone(),
            &QueryMsg::GetSubnetworkN {
                netuid,
            },
        ).unwrap();

        let max_default_allowed = 256; // set during add_network as default
        let uids: Vec<u16> = Vec::from_iter((0..(max_default_allowed + 1)).map(|uid| uid));

        let expected = false;
        let result: bool = app.wrap().query_wasm_smart(
            cn_addr.clone(),
            &QueryMsg::CheckLenUidsWithingAllowed {
                netuid,
                uids,
            },
        ).unwrap();
        assert_eq!(
            expected, result,
            "Failed to detect incompatible uids for network"
        );
    }
}
