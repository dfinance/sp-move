#![cfg(feature = "runtime-benchmarks")]

use frame_benchmarking::{benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_system::RawOrigin;
use frame_support::assert_ok;
use move_core_types::language_storage::{CORE_CODE_ADDRESS, ModuleId, StructTag};
use move_vm::data::AccessKey;
use sp_std::prelude::*;

use sp_mvm::benchmarking::*;
// use sp_mvm::benchmarking::store::container;
use sp_mvm::benchmarking::store_::container;

mod common;
use common::mock::{self, *};

impl_benchmark_test_suite!(Mvm, crate::common::mock::new_test_ext(), crate::common::mock::Test);

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
