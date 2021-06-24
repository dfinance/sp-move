#![allow(dead_code)]

use sp_mvm::gas;
use sp_core::H256;
use sp_std::convert::TryFrom;
use frame_system as system;
use frame_support::{
    parameter_types,
    weights::{Weight, constants::WEIGHT_PER_SECOND},
};
use frame_support::traits::{OnInitialize, OnFinalize};
use sp_runtime::traits::{BlakeTwo256, IdentityLookup};
use sp_runtime::{testing::Header};

use super::addr::origin_ps_acc;
use super::addr::root_ps_acc;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

/// Initial balance for all existent test accounts
pub const INITIAL_BALANCE: <Test as balances::Config>::Balance = 42000;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Module, Call, Config, Storage, Event<T>},
        Timestamp: timestamp::{Module, Call, Storage, Inherent},
        Balances: balances::{Module, Call, Storage, Config<T>, Event<T>},
        Mvm: sp_mvm::{Module, Call, Storage, Event<T>},
        // Sudo: pallet_sudo::{Module, Call, Config<T>, Storage, Event<T>},
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
}

impl system::Config for Test {
    type BaseCallFilter = ();
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type Origin = Origin;
    type Call = Call;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = sp_core::sr25519::Public;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = Event;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    // type AccountData = ();
    type AccountData = balances::AccountData<<Self as balances::Config>::Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
}

// --- gas --- //
/// By inheritance from Moonbeam and from Dfinance (based on validators statistic), we believe max 4125000 gas is currently enough for block.
/// In the same time we use same 500ms Weight as Max Block Weight, from which 75% only are used for transactions.
/// So our max gas is GAS_PER_SECOND * 0.500 * 0.65 => 4125000.
pub const GAS_PER_SECOND: u64 = 11_000_000;

/// Approximate ratio of the amount of Weight per Gas.
/// u64 works for approximations because Weight is a very small unit compared to gas.
pub const WEIGHT_PER_GAS: u64 = WEIGHT_PER_SECOND / GAS_PER_SECOND;

pub struct MoveVMGasWeightMapping;

// Just use provided gas.
impl gas::GasWeightMapping for MoveVMGasWeightMapping {
    fn gas_to_weight(gas: u64) -> Weight {
        gas.saturating_mul(WEIGHT_PER_GAS)
    }

    fn weight_to_gas(weight: Weight) -> u64 {
        u64::try_from(weight.wrapping_div(WEIGHT_PER_GAS)).unwrap_or(u32::MAX as u64)
    }
}
// ----------------- //

// --- timestamp --- //

parameter_types! {
    pub const MinimumPeriod: u64 = 5;
}
impl timestamp::Config for Test {
    /// A timestamp: milliseconds since the unix epoch.
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

// --- balances --- //

parameter_types! {
    pub const ExistentialDeposit: u64 = 1;
    pub const MaxLocks: u32 = 50;
}

impl balances::Config for Test {
    type MaxLocks = MaxLocks;
    /// The type for recording an account's balance.
    type Balance = u64;
    /// The ubiquitous event type.
    type Event = Event;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = Sys;
    type WeightInfo = balances::weights::SubstrateWeight<Self>;
}

// ----------------- //

impl sp_mvm::Config for Test {
    // type Event = TestEvent;
    type Event = Event;
    type GasWeightMapping = MoveVMGasWeightMapping;
}

pub type Sys = system::Module<Test>;
pub type Time = timestamp::Module<Test>;
pub type MoveEvent = sp_mvm::Event<Test>;

/// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut sys = system::GenesisConfig::default()
        .build_storage::<Test>()
        .expect("Frame system builds valid default genesis config");
    /*
    let negative = <balances::Module<T> as Currency<T::AccountId>>::withdraw(
        &address,
        amount.try_into().ok().unwrap(),
        WithdrawReasons::RESERVE,
        ExistenceRequirement::AllowDeath,
    )

    */

    balances::GenesisConfig::<Test> {
        balances: vec![
            (root_ps_acc(), INITIAL_BALANCE),
            (origin_ps_acc(), INITIAL_BALANCE),
        ],
        // balances: Vec::<(
        //     <Test as system::Config>::AccountId,
        //     <Test as balances::Config>::Balance,
        // )>::new(),
    }
    .assimilate_storage(&mut sys)
    .expect("Pallet balances storage can be assimilated");

    sys.into()
}

pub const TIME_BLOCK_MULTIPLIER: u64 = 100;
pub fn roll_next_block() {
    // Stake::on_finalize(Sys::block_number());
    Balances::on_finalize(Sys::block_number());
    Mvm::on_finalize(Sys::block_number());
    Sys::on_finalize(Sys::block_number());
    Sys::set_block_number(Sys::block_number() + 1);
    Sys::on_initialize(Sys::block_number());
    Mvm::on_initialize(Sys::block_number());
    Balances::on_initialize(Sys::block_number());
    // Stake::on_initialize(Sys::block_number());

    // set time with multiplier `*MULTIPLIER` by block:
    Time::set_timestamp(Sys::block_number() * TIME_BLOCK_MULTIPLIER);

    println!("now block: {}, time: {}", Sys::block_number(), Time::get());
}

pub fn roll_block_to(n: u64) {
    while Sys::block_number() < n {
        roll_next_block()
    }
}

pub fn last_event() -> Event {
    {
        let events = Sys::events();
        println!("events: {:?}", events);
    }
    Sys::events().pop().expect("Event expected").event
}

pub fn have_no_events() -> bool {
    Sys::events().is_empty()
}
