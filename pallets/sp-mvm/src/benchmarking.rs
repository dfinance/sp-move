#![cfg(feature = "runtime-benchmarks")]
//! Benchmarking setup for pallet-template

// How to use:
// 1. Build node with feature `runtime-benchmarks`
// 2. Run `./target/release/mv-node benchmark --dev -lsp_mvm=trace --pallet=sp_mvm --execution=wasm --wasm-execution=compiled --extrinsic='*' --steps=20 --repeat=10 --output=./target/sp-bench/`

use frame_benchmarking::{benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_system::RawOrigin;
use move_core_types::account_address::AccountAddress;
use move_core_types::identifier::Identifier;
use move_core_types::language_storage::{CORE_CODE_ADDRESS, ModuleId, StructTag};
use move_vm::data::AccessKey;
use sp_std::prelude::*;

use crate::benchmarking::store::container;

use super::*;
#[allow(unused)]
use super::Pallet as Mvm;

benchmarks! {
    publish_std {
        let s in 0 .. 100;
        let stdlib_modules = stdlib()
        .into_iter()
        .map(|(_, m)|m)
        .collect::<Vec<_>>();
    }: _(RawOrigin::Root, stdlib_modules, 100_000_000)
    verify {
        for (name, _) in stdlib() {
            assert!(VMStorage::<T>::contains_key(module_access_core(name)));
        }
    }
    publish_empty_module {
        let s in 0 .. 100;
        let caller: T::AccountId = whitelisted_caller();
        let module = include_bytes!("../tests/benchmark_assets/target/modules/2_Empty.mv").to_vec();
    }: publish_module(RawOrigin::Signed(caller), module, 100_000_000)
    verify {
        assert!(VMStorage::<T>::contains_key(module_access("Empty")));
    }
    publish_many_deps_module {
        let s in 0 .. 100;
        for (name, module) in stdlib() {
            VMStorage::<T>::insert(module_access_core(name), module);
        }
        let caller: T::AccountId = whitelisted_caller();
        let module = include_bytes!("../tests/benchmark_assets/target/modules/22_StdImport.mv").to_vec();
    }: publish_module(RawOrigin::Signed(caller), module, 100_000_000)
    verify {
        assert!(VMStorage::<T>::contains_key(module_access("StdImport")));
    }
    publish_s_module {
        let s in 0 .. 100;
        let caller: T::AccountId = whitelisted_caller();
        let module = include_bytes!("../tests/benchmark_assets/target/modules/6_S.mv").to_vec();
    }: publish_module(RawOrigin::Signed(caller), module, 100_000_000)
    verify {
        assert!(VMStorage::<T>::contains_key(module_access("S")));
    }
    publish_m_module {
        let s in 0 .. 100;
        let caller: T::AccountId = whitelisted_caller();
        let module = include_bytes!("../tests/benchmark_assets/target/modules/5_M.mv").to_vec();
    }: publish_module(RawOrigin::Signed(caller), module, 100_000_000)
    verify {
        assert!(VMStorage::<T>::contains_key(module_access("M")));
    }
    publish_l_module {
        let s in 0 .. 100;
        let caller: T::AccountId = whitelisted_caller();
        let module = include_bytes!("../tests/benchmark_assets/target/modules/4_L.mv").to_vec();
    }: publish_module(RawOrigin::Signed(caller), module, 100_000_000)
    verify {
        assert!(VMStorage::<T>::contains_key(module_access("L")));
    }
    execute_many_params {
        let s in 0 .. 100;
        let caller: T::AccountId = whitelisted_caller();
        let tx = include_bytes!("../tests/benchmark_assets/target/transactions/many_params.mvt").to_vec();
    }: execute(RawOrigin::Signed(caller), tx, 500_000)
    verify {
        // no-op
    }
    execute_store {
        let s in 0 .. 100;
         for (name, module) in stdlib() {
            VMStorage::<T>::insert(module_access_core(name), module);
        }
        VMStorage::<T>::insert(module_access_core("Store"), include_bytes!("../tests/benchmark_assets/target/modules/1_Store.mv").to_vec());
        let caller: T::AccountId = whitelisted_caller();
        let tx = include_bytes!("../tests/benchmark_assets/target/transactions/store.mvt").to_vec();
    }: execute(RawOrigin::Signed(caller), tx, 500_000)
    verify {

        let tag = StructTag {
            address: CORE_CODE_ADDRESS,
            module: Identifier::new("Store").unwrap(),
            name: Identifier::new("Container").unwrap(),
            type_params: vec![]
         };

        let ak = AccessKey::from((&AccountAddress::from_hex_literal("0xd861ea1ebf4800d4b89f4ff787ad79ee96d9a708c85b57da7eb8f9ddeda61291").unwrap(), &tag));

        assert!(VMStorage::<T>::contains_key(ak.as_ref()));
    }
    execute_load {
        let s in 0 .. 100;
         for (name, module) in stdlib() {
            VMStorage::<T>::insert(module_access_core(name), module);
        }

        let tag = StructTag {
            address: CORE_CODE_ADDRESS,
            module: Identifier::new("Store").unwrap(),
            name: Identifier::new("Container").unwrap(),
            type_params: vec![]
         };

        let ak = AccessKey::from((&CORE_CODE_ADDRESS, &tag));

        VMStorage::<T>::insert(ak.as_ref().to_vec(), bcs::to_bytes(&container()).unwrap());

        VMStorage::<T>::insert(module_access_core("Store"), include_bytes!("../tests/benchmark_assets/target/modules/1_Store.mv").to_vec());
        let caller: T::AccountId = whitelisted_caller();
        let tx = include_bytes!("../tests/benchmark_assets/target/transactions/load.mvt").to_vec();
    }: execute(RawOrigin::Signed(caller), tx, 500_000)
    verify {
    }
    execute_store_event {
        let s in 0 .. 100;
         for (name, module) in stdlib() {
            VMStorage::<T>::insert(module_access_core(name), module);
        }
        let caller: T::AccountId = whitelisted_caller();
        let tx = include_bytes!("../tests/benchmark_assets/target/transactions/store_events.mvt").to_vec();
    }: execute(RawOrigin::Signed(caller), tx, 500_000)
    verify {
    }
    execute_vec_input {
        let s in 0 .. 100;
        let caller: T::AccountId = whitelisted_caller();
        let tx = include_bytes!("../tests/benchmark_assets/target/transactions/vector_input.mvt").to_vec();
    }: execute(RawOrigin::Signed(caller), tx, 500_000)
    verify {
    }
    execute_loop {
        let s in 0 .. 100;
        let caller: T::AccountId = whitelisted_caller();
        let tx = include_bytes!("../tests/benchmark_assets/target/transactions/lp.mvt").to_vec();
    }: execute(RawOrigin::Signed(caller), tx, 100_000_000)
    verify {
    }
}

impl_benchmark_test_suite!(Mvm, crate::mock::new_test_ext(), crate::mock::Test,);

pub fn module_access_core(name: &str) -> Vec<u8> {
    ModuleId::new(CORE_CODE_ADDRESS, Identifier::new(name).unwrap()).access_vector()
}

pub fn module_access(name: &str) -> Vec<u8> {
    ModuleId::new(
        AccountAddress::from_hex_literal(
            "0xd861ea1ebf4800d4b89f4ff787ad79ee96d9a708c85b57da7eb8f9ddeda61291",
        )
        .unwrap(),
        Identifier::new(name).unwrap(),
    )
    .access_vector()
}

pub fn stdlib() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        (
            "Signer",
            include_bytes!("../tests/benchmark_assets/target/modules/0_Signer.mv").to_vec(),
        ),
        (
            "Event",
            include_bytes!("../tests/benchmark_assets/target/modules/7_Event.mv").to_vec(),
        ),
        (
            "Pontem",
            include_bytes!("../tests/benchmark_assets/target/modules/8_Pontem.mv").to_vec(),
        ),
        (
            "Account",
            include_bytes!("../tests/benchmark_assets/target/modules/9_Account.mv").to_vec(),
        ),
        (
            "Vector",
            include_bytes!("../tests/benchmark_assets/target/modules/10_Vector.mv").to_vec(),
        ),
        (
            "Compare",
            include_bytes!("../tests/benchmark_assets/target/modules/11_Compare.mv").to_vec(),
        ),
        (
            "U256",
            include_bytes!("../tests/benchmark_assets/target/modules/12_U256.mv").to_vec(),
        ),
        (
            "Math",
            include_bytes!("../tests/benchmark_assets/target/modules/13_Math.mv").to_vec(),
        ),
        (
            "Offer",
            include_bytes!("../tests/benchmark_assets/target/modules/14_Offer.mv").to_vec(),
        ),
        (
            "Time",
            include_bytes!("../tests/benchmark_assets/target/modules/15_Time.mv").to_vec(),
        ),
        (
            "Security",
            include_bytes!("../tests/benchmark_assets/target/modules/16_Security.mv").to_vec(),
        ),
        (
            "PONT",
            include_bytes!("../tests/benchmark_assets/target/modules/17_PONT.mv").to_vec(),
        ),
        (
            "FixedPoint32",
            include_bytes!("../tests/benchmark_assets/target/modules/18_FixedPoint32.mv")
                .to_vec(),
        ),
        (
            "Debug",
            include_bytes!("../tests/benchmark_assets/target/modules/19_Debug.mv").to_vec(),
        ),
        (
            "Coins",
            include_bytes!("../tests/benchmark_assets/target/modules/20_Coins.mv").to_vec(),
        ),
        (
            "Block",
            include_bytes!("../tests/benchmark_assets/target/modules/21_Block.mv").to_vec(),
        ),
    ]
}

mod store {
    use move_core_types::language_storage::CORE_CODE_ADDRESS;
    use serde::Serialize;
    use sp_std::prelude::*;

    #[derive(Serialize)]
    #[serde(crate = "serde_alt")]
    pub struct Container {
        inner_1: Inner,
        inner_2: Inner,
        inner_3: Inner,
        inner_4: Inner,
    }

    #[derive(Serialize)]
    #[serde(crate = "serde_alt")]
    pub struct Inner {
        val: bool,
        val_1: u128,
        val_2: Vec<u8>,
        val_3: u64,
    }

    pub fn container() -> Container {
        Container {
            inner_1: Inner {
                val: true,
                val_1: 1000000000,
                val_2: CORE_CODE_ADDRESS.to_vec(),
                val_3: 13,
            },
            inner_2: Inner {
                val: false,
                val_1: 13,
                val_2: CORE_CODE_ADDRESS.to_vec(),
                val_3: 0,
            },
            inner_3: Inner {
                val: false,
                val_1: 42,
                val_2: CORE_CODE_ADDRESS.to_vec(),
                val_3: 0,
            },
            inner_4: Inner {
                val: false,
                val_1: 0,
                val_2: CORE_CODE_ADDRESS.to_vec(),
                val_3: 0,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests_composite::{ExtBuilder, Test};
    use frame_support::assert_ok;

    #[test]
    fn publish_std() {
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_publish_std::<Test>());
        });
    }

    #[test]
    fn publish_empty_module() {
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_publish_empty_module::<Test>());
        });
    }

    #[test]
    fn publish_many_deps_module() {
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_publish_many_deps_module::<Test>());
        });
    }

    #[test]
    fn publish_s_module() {
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_publish_s_module::<Test>());
        });
    }

    #[test]
    fn publish_m_module() {
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_publish_m_module::<Test>());
        });
    }

    #[test]
    fn publish_l_module() {
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_publish_l_module::<Test>());
        });
    }

    #[test]
    fn execute_many_params() {
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_execute_many_params::<Test>());
        });
    }

    #[test]
    fn execute_store() {
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_execute_store::<Test>());
        });
    }

    #[test]
    fn execute_load() {
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_execute_load::<Test>());
        });
    }

    #[test]
    fn execute_store_event() {
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_execute_store_event::<Test>());
        });
    }

    #[test]
    fn execute_vec_input() {
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_execute_vec_input::<Test>());
        });
    }

    #[test]
    fn execute_execute_loop() {
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_eexecute_loop::<Test>());
        });
    }
}
