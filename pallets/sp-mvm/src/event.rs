use core::convert::TryInto;
use move_vm::io::traits::EventHandler;
use sp_core::hexdisplay::AsBytesRef;
use sp_std::prelude::*;
use move_core_types::account_address::AccountAddress;
use move_core_types::language_storage::ModuleId;
use move_core_types::language_storage::TypeTag;
use crate::types;
use crate::{Event, Config};
use crate::addr::address_to_account;

#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(not(feature = "std"))]
use alloc::format;

pub trait DepositMoveEvent {
    /// Emit a Move event with content of passed `MoveEventArguments`
    fn deposit_move_event(e: MoveEventArguments);
}

pub struct EventWriter<F>(F);

pub struct MoveEventArguments {
    pub guid: Vec<u8>,
    pub ty_tag: TypeTag,
    pub message: Vec<u8>,
}

impl<T: Config> TryInto<Event<T>> for MoveEventArguments {
    type Error = codec::Error;

    fn try_into(self) -> Result<Event<T>, Self::Error> {
        let ty_tag_enc = format!("{}", self.ty_tag).as_bytes().to_vec();
        Ok(Event::Event(self.guid, ty_tag_enc, self.message))
    }
}

impl<F: Fn(MoveEventArguments)> EventHandler for EventWriter<F> {
    #[inline]
    fn on_event(&self, guid: Vec<u8>, _seq_num: u64, ty_tag: TypeTag, message: Vec<u8>) {
        self.0(MoveEventArguments {
            guid,
            ty_tag,
            message,
        })
    }
}

impl<F> EventWriter<F> {
    pub fn new(f: F) -> Self {
        Self(f)
    }
}

/// Default EventWriter
pub type DefaultEventHandler = EventWriter<Box<dyn Fn(MoveEventArguments)>>;

/// Boxed fn ptr to something looks like `DepositMoveEvent::deposit_move_event`
pub type DepositMoveEventFnPtr = Box<dyn Fn(MoveEventArguments)>;

pub trait GetDepositMoveEventFn<T: DepositMoveEvent + 'static> {
    fn get_deposit_move_event_fn() -> DepositMoveEventFnPtr {
        Box::new(T::deposit_move_event)
    }
}

impl<T: DepositMoveEvent + 'static> GetDepositMoveEventFn<T> for T {}

pub trait CreateMoveEventHandler<T> {
    type EventHandler: EventHandler;

    fn create_move_event_handler() -> Self::EventHandler;
}

impl<T> CreateMoveEventHandler<T> for T
where
    T: DepositMoveEvent + 'static,
    T: GetDepositMoveEventFn<T>,
{
    type EventHandler = DefaultEventHandler;

    fn create_move_event_handler() -> Self::EventHandler {
        EventWriter::new(T::get_deposit_move_event_fn())
    }
}
