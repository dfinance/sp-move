#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::prelude::*;
use sp_runtime::traits::Saturating;
use frame_system::Config as SystemConfig;
use cumulus_pallet_xcm::{Origin as CumulusOrigin};
use xcm::v0::{Xcm, Error as XcmError, SendXcm, OriginKind, MultiLocation, Junction};

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use super::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	/// The module configuration trait.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        type MvmConfig: sp_mvm::pallet::Config;

		type XcmSender: SendXcm;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	#[pallet::metadata(T::BlockNumber = "BlockNumber")]
	pub enum Event<T: Config> {
		PingSent(u32, u64),
		ErrorSendingPing(XcmError, u32, Vec<u8>),
	}

	#[pallet::error]
	pub enum Error<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(0)]
		pub fn execute_on_parachain(origin: OriginFor<T>, para_id: u32, payload: Vec<u8>) -> DispatchResult {
			// Only accept pings from other chains.
			let para = ensure_signed(origin)?;
            let call = sp_mvm::pallet::Call::<<T as Config>::MvmConfig>::execute(payload.clone(), 1_000_000).encode().into();

			match T::XcmSender::send_xcm(
				MultiLocation::X2(Junction::Parent, Junction::Parachain(para_id)),
				Xcm::Transact {
					origin_type: OriginKind::Native,
					require_weight_at_most: 1_000_000,
					call,
				},
			) {
				Ok(()) => Self::deposit_event(Event::PingSent(para_id, payload.len() as u64)),
				Err(e) => Self::deposit_event(Event::ErrorSendingPing(e, para_id, payload)),
			}
			Ok(())
		}
	}
}
