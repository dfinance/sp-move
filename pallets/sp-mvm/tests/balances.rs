use frame_support::assert_ok;
use sp_runtime::DispatchError;
use serde::Deserialize;
use move_core_types::identifier::Identifier;
use move_core_types::language_storage::StructTag;
use move_core_types::account_address::AccountAddress;

mod common;
use common::assets::*;
use common::mock::*;
use common::addr::*;
use common::utils::*;

const GAS_LIMIT: u64 = 1_000_000;

#[derive(Deserialize, Debug, PartialEq)]
struct StoreU128 {
    pub val: u128,
}

fn check_storage_u128<T>(address: AccountAddress, expected: T)
where
    T: Into<u128>,
{
    let expected = StoreU128 {
        val: expected.into(),
    };
    let tag = StructTag {
        address,
        module: Identifier::new(UserMod::Store.name()).unwrap(),
        name: Identifier::new("U128").unwrap(),
        type_params: vec![],
    };
    check_storage_res(address, tag, expected);
}

fn check_storage_pont(address: AccountAddress, expected: u64) {
    let tt = get_type_tag_balance_pont();
    check_storage_res(address, tt, expected);
}

#[test]
fn execute_get_balance() {
    new_test_ext().execute_with(|| {
        let account = origin_ps_acc();

        // publish entire std lib:
        publish_std();

        // publish user module:
        publish_module(account, UserMod::Store);

        // execute tx:
        let signer = Origin::signed(account);
        let result = execute_tx_unchecked(signer, UserTx::StoreGetBalance, GAS_LIMIT);
        assert_ok!(result);

        // check storage:
        check_storage_u128(to_move_addr(account), INITIAL_BALANCE);

        let balance = balances::Pallet::<Test>::free_balance(&account);
        assert_eq!(INITIAL_BALANCE, balance);
    });
}

#[test]
fn execute_get_missing_balance_err() {
    new_test_ext().execute_with(|| {
        let account = origin_ps_acc();

        // publish entire std lib:
        publish_std();

        // execute tx:
        let signer = Origin::signed(account);
        let result = execute_tx_unchecked(signer, UserTx::MissedNativeBalance, GAS_LIMIT);
        assert!(result.is_err());

        match result.unwrap_err().error {
            DispatchError::Module {
                message: Some("ResourceDoesNotExist"),
                ..
            } => { /* OK */ }
            _ => panic!("should be an error"),
        }
    });
}

#[test]
fn execute_deposit_balance() {
    new_test_ext().execute_with(|| {
        let account = origin_ps_acc();

        // publish entire std lib:
        publish_std();

        // publish user module:
        publish_module(account, UserMod::Store);

        // execute tx:
        let signer = Origin::signed(account);
        let result = execute_tx_unchecked(signer, UserTx::StoreNativeDepositReg, GAS_LIMIT);
        assert_ok!(result);

        // check storage:
        check_storage_u128(to_move_addr(account), INITIAL_BALANCE / 2);
        // check balance for PONT assets (equivalent):
        check_storage_pont(to_move_addr(account), INITIAL_BALANCE / 2);

        // let total = balances::TotalIssuance::<Test>::get();
        let balance = balances::Pallet::<Test>::free_balance(&account);
        assert_eq!(INITIAL_BALANCE / 2, balance);
    });
}

#[test]
fn execute_deposit_withdraw_balance() {
    new_test_ext().execute_with(|| {
        let account = origin_ps_acc();

        // publish entire std lib:
        publish_std();

        // publish user module:
        publish_module(account, UserMod::Store);

        // execute tx:
        let signer = Origin::signed(account);
        let result = execute_tx_unchecked(signer, UserTx::StoreNativeWithdrawReg, GAS_LIMIT);
        assert_ok!(result);

        // check storage:
        check_storage_u128(to_move_addr(account), INITIAL_BALANCE);

        let balance = balances::Pallet::<Test>::free_balance(&account);
        assert_eq!(INITIAL_BALANCE, balance);
    });
}

mod adapter {
    use move_vm::io::traits::BalanceAccess;
    use sp_mvm::balance::BalancesAdapter;
    use sp_mvm::balance::boxed::BalancesAdapter as BoxedBalancesAdapter;

    use super::*;

    fn test_get_balance_with<T: BalanceAccess>(adapter: &T) {
        new_test_ext().execute_with(|| {
            let origin = origin_ps_acc();
            let account = to_move_addr(origin.clone());
            let expected = balances::Pallet::<Test>::free_balance(&origin);
            let value = adapter.get_balance(&account, "PONT".as_bytes());
            assert_eq!(Some(expected), value);
        });
    }

    fn test_deposit_with<T: BalanceAccess>(adapter: &T) {
        new_test_ext().execute_with(|| {
            let origin = origin_ps_acc();
            let account = to_move_addr(origin.clone());
            let initial_balance = balances::Pallet::<Test>::free_balance(&origin);

            let expected_balance = initial_balance / 2;

            adapter.add(&account, "PONT".as_bytes(), expected_balance);

            let actual_balance = balances::Pallet::<Test>::free_balance(&origin);

            assert_eq!(expected_balance, actual_balance);
        });
    }

    fn test_withdraw_with<T: BalanceAccess>(adapter: &T) {
        new_test_ext().execute_with(|| {
            let origin = origin_ps_acc();
            let account = to_move_addr(origin.clone());
            let initial_balance = balances::Pallet::<Test>::free_balance(&origin);

            adapter.sub(&account, "PONT".as_bytes(), initial_balance);

            let actual_balance = balances::Pallet::<Test>::free_balance(&origin);
            assert_eq!(initial_balance * 2, actual_balance);
        });
    }

    #[test]
    fn get_balance() {
        let adapter = BalancesAdapter::<Test>::new();
        test_get_balance_with(&adapter);
    }

    #[test]
    fn get_balance_boxed() {
        let adapter = BoxedBalancesAdapter::from(BalancesAdapter::<Test>::new());
        test_get_balance_with(&adapter);
    }

    #[test]
    fn deposit() {
        let adapter = BalancesAdapter::<Test>::new();
        test_deposit_with(&adapter);
    }

    #[test]
    fn deposit_boxed() {
        let adapter = BoxedBalancesAdapter::from(BalancesAdapter::<Test>::new());
        test_deposit_with(&adapter);
    }

    #[test]
    fn withdraw() {
        let adapter = BalancesAdapter::<Test>::new();
        test_withdraw_with(&adapter);
    }

    #[test]
    fn withdraw_boxed() {
        let adapter = BoxedBalancesAdapter::from(BalancesAdapter::<Test>::new());
        test_withdraw_with(&adapter);
    }
}
