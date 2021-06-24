#![allow(dead_code)]

use std::convert::TryFrom;
use frame_support::dispatch::DispatchResultWithPostInfo as PsResult;
use move_core_types::account_address::AccountAddress;
use move_core_types::identifier::Identifier;
use move_core_types::language_storage::ModuleId;
use move_core_types::language_storage::StructTag;
use move_core_types::language_storage::TypeTag;
use move_vm::io::state::State;
use move_vm_runtime::data_cache::RemoteCache;
use move_vm::types::ModulePackage;

use sp_mvm::storage::MoveVmStorage;

use super::assets::*;
use super::mock::*;
use super::addr::*;

pub type AccountId = <Test as frame_system::Config>::AccountId;

/// Publish module __with__ storage check
pub fn publish_module<Asset: BinAsset>(signer: AccountId, module: Asset) {
    publish_module_raw(signer, module.bc().to_vec(), module.name());
}

/// Publish module __without__ storage check
pub fn publish_module_unchecked<Asset: BinAsset>(signer: AccountId, module: Asset) {
    publish_module_raw_unchecked(signer, module.bc().to_vec())
}

pub fn publish_module_raw(signer: AccountId, bc: Vec<u8>, name: &str) {
    publish_module_raw_with_origin_unchecked(Origin::signed(signer), bc.clone());

    // check storage:
    check_storage_mod_raw(signer, bc, name);
}

/// Publish module __without__ storage check
pub fn publish_module_raw_unchecked(signer: AccountId, bc: Vec<u8>) {
    publish_module_raw_with_origin_unchecked(Origin::signed(signer), bc.clone())
}

/// Publish module __without__ storage check
pub fn publish_module_raw_with_origin_unchecked(origin: Origin, bc: Vec<u8>) {
    const GAS_LIMIT: u64 = 1_000_000;

    // execute VM for publish module:
    Mvm::publish_module(origin, bc, GAS_LIMIT).expect("Publish module");
}

///////////////////////////

/// Publish package.

/// Publish package __with__ storage check
pub fn publish_package<Asset: BinAssetPackage>(
    signer: AccountId,
    package: Asset,
    gas_limit: u64,
) {
    publish_package_raw(signer, package.bc().to_vec(), gas_limit, package.modules());
}

/// Publish package __without__ storage check
pub fn publish_package_unchecked<Asset: BinAssetPackage>(
    signer: AccountId,
    package: Asset,
    gas_limit: u64,
) {
    publish_package_raw_unchecked(signer, package.bc().to_vec(), gas_limit);
}

pub fn publish_package_raw(signer: AccountId, bc: Vec<u8>, gas_limit: u64, names: &[&str]) {
    publish_package_raw_with_origin_unchecked(Origin::signed(signer), bc.clone(), gas_limit);

    let (modules, _) = ModulePackage::try_from(&bc[..])
        .unwrap()
        .into_tx(ROOT_ADDR)
        .into_inner();

    for (i, mbc) in modules.iter().enumerate() {
        check_storage_mod_raw(signer, mbc.to_vec(), names[i]);
    }
}

/// Publish package __without__ storage check
pub fn publish_package_raw_unchecked(signer: AccountId, bc: Vec<u8>, gas_limit: u64) {
    publish_package_raw_with_origin_unchecked(Origin::signed(signer), bc.clone(), gas_limit)
}

/// Publish package __without__ storage check
pub fn publish_package_raw_with_origin_unchecked(origin: Origin, bc: Vec<u8>, gas_limit: u64) {
    // execute VM for publish module:
    Mvm::publish_package(origin, bc, gas_limit).expect("Publish package");
}

///////////////////////////

/// Publish entire stdlib with sudo/root key
pub fn publish_std() {
    let root = root_ps_acc();
    for module in StdMod::all().into_iter() {
        publish_module_raw(root, module.bc().to_vec(), module.name());
    }
}

pub fn execute_tx_unchecked(origin: Origin, tx: UserTx, gas_limit: u64) -> PsResult {
    // get bytecode:
    let bc = tx.bc().to_vec();
    // execute VM tx:
    let result = Mvm::execute(origin, bc, gas_limit);
    eprintln!("execute tx result: {:?}", result);
    result
}

pub fn check_storage_mod_raw<Bc: AsRef<[u8]>>(signer: AccountId, bc: Bc, name: &str) {
    check_storage_mod_raw_with_addr(to_move_addr(signer), bc, name)
}

pub fn check_storage_mod_raw_with_addr<Bc: AsRef<[u8]>>(
    signer: AccountAddress,
    bc: Bc,
    name: &str,
) {
    let module_id = ModuleId::new(signer, Identifier::new(name).unwrap());
    let storage = Mvm::move_vm_storage();
    let state = State::new(storage);
    let stored = state
        .get_module(&module_id)
        .expect("VM state read storage")
        .expect(&format!("Module '{}' should exist", module_id));
    assert_eq!(bc.as_ref(), &stored);
}

pub fn check_storage_res<T>(owner: AccountAddress, ty: StructTag, expected: T)
where
    T: for<'de> serde::Deserialize<'de>,
    T: std::cmp::PartialEq + std::fmt::Debug,
{
    let storage = Mvm::move_vm_storage();
    let state = State::new(storage);
    let blob = state
        .get_resource(&owner, &ty)
        .expect("VM state read storage (resource)")
        .expect(&format!("Resource '{}' should exist", ty));

    let tt_str = format!("{}", ty);
    println!("checking stored resource '{}'", tt_str);
    let stored: T =
        bcs::from_bytes(&blob).expect(&format!("Resource '{}' should exists", tt_str));
    assert_eq!(expected, stored);
}

/// Returns TypeTag 0x1::PONT::T
pub fn get_type_tag_pont_coin() -> StructTag {
    StructTag {
        address: ROOT_ADDR,
        module: Identifier::new("PONT").unwrap(),
        name: Identifier::new("T").unwrap(),
        type_params: vec![],
    }
}

/// Returns TypeTag Pontem::T<0x1::PONT::T>
pub fn get_type_tag_pont_res() -> StructTag {
    StructTag {
        address: ROOT_ADDR,
        module: Identifier::new("Pontem").unwrap(),
        name: Identifier::new("T").unwrap(),
        type_params: vec![TypeTag::Struct(get_type_tag_pont_coin())],
    }
}

/// Returns TypeTag 0x1::Account::Balance<0x1::PONT::T>
pub fn get_type_tag_balance_pont() -> StructTag {
    StructTag {
        address: ROOT_ADDR,
        module: Identifier::new("Account").unwrap(),
        name: Identifier::new("Balance").unwrap(),
        type_params: vec![TypeTag::Struct(get_type_tag_pont_coin())],
    }
}
