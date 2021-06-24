use move_vm::io::state::State;
use serde::Deserialize;
use frame_support::assert_ok;
use move_core_types::identifier::Identifier;
use move_core_types::language_storage::StructTag;
use move_vm_runtime::data_cache::RemoteCache;
use sp_mvm::storage::MoveVmStorage;

mod common;
use common::assets::*;
use common::mock::*;
use common::addr::*;
use common::utils;

#[derive(Deserialize)]
struct StoreU64 {
    pub val: u64,
}

fn call_execute_script(origin: Origin) {
    const GAS_LIMIT: u64 = 1_000_000;
    let txbc = UserTx::StoreU64.bc().to_vec();

    // execute VM tx:
    let result = Mvm::execute(origin, txbc, GAS_LIMIT);
    eprintln!("execute_script result: {:?}", result);
    assert_ok!(result);

    // check storage:
    let store = Mvm::move_vm_storage();
    let state = State::new(store);
    let tag = StructTag {
        address: origin_move_addr(),
        module: Identifier::new(UserMod::Store.name()).unwrap(),
        name: Identifier::new("U64").unwrap(),
        type_params: vec![],
    };
    let blob = state
        .get_resource(&origin_move_addr(), &tag)
        .unwrap()
        .unwrap();
    let store: StoreU64 = bcs::from_bytes(&blob).unwrap();
    assert_eq!(42, store.val);
}

#[test]
fn publish_module() {
    new_test_ext().execute_with(|| {
        let origin = origin_ps_acc();
        utils::publish_module(origin, UserMod::Store);
    });
}

#[test]
fn execute_script() {
    new_test_ext().execute_with(|| {
        let origin = origin_ps_acc();
        let signer = Origin::signed(origin);

        utils::publish_module(origin, UserMod::Store);
        call_execute_script(signer);
    });
}
