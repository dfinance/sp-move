#![cfg_attr(not(feature = "std"), no_std)]

#[macro_use]
extern crate log;

#[cfg(feature = "runtime-benchmarks")]
extern crate serde_alt as serde;
#[cfg(feature = "runtime-benchmarks")]
extern crate bcs_alt as bcs;
#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://substrate.dev/docs/en/knowledgebase/runtime/frame>
pub use pallet::*;
pub mod addr;
pub mod balance;
pub mod event;
pub mod gas;
pub mod mvm;
pub mod oracle;
pub mod result;
pub mod storage;
pub mod types;

#[frame_support::pallet]
pub mod pallet {
    // Clippy didn't love sp- macros
    #![allow(clippy::unused_unit)]

    use super::*;
    use super::storage::MoveVmStorage;
    use gas::GasWeightMapping;
    use event::*;
    use mvm::*;

    #[cfg(not(feature = "no-vm-static"))]
    mod boxed {
        pub use crate::storage::boxed::VmStorageBoxAdapter as StorageAdapter;
        pub use crate::balance::boxed::BalancesAdapter;
    }

    use core::convert::TryInto;
    use core::convert::TryFrom;

    use sp_std::prelude::*;
    use frame_system::pallet_prelude::*;
    use frame_support as support;
    use support::pallet_prelude::*;
    use support::traits::UnixTime;
    use support::dispatch::DispatchResultWithPostInfo;
    use sp_runtime::traits::UniqueSaturatedInto;
    use codec::{FullCodec, FullEncode};

    use move_vm::Vm;
    use move_vm::mvm::Mvm;
    use move_vm::data::ExecutionContext;
    use move_vm::types::Gas;
    use move_vm::types::ModuleTx;
    use move_vm::types::Transaction;
    use move_vm::types::VmResult;
    use move_vm::types::ModulePackage;
    use move_core_types::account_address::AccountAddress;
    use move_core_types::language_storage::CORE_CODE_ADDRESS;

    /// Configure the pallet by specifying the parameters and types on which it depends.
    #[pallet::config]
    pub trait Config: frame_system::Config + timestamp::Config + balances::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Gas to weight convertion settings.
        type GasWeightMapping: gas::GasWeightMapping;

        // doesn't really needed now:
        // type Currency: Currency<Self::AccountId>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub trait Store)]
    pub struct Pallet<T>(_);

    // The pallet's runtime storage items.
    // https://substrate.dev/docs/en/knowledgebase/runtime/storage
    /// Move VM storage. Map with already encoded key-values pairs:
    /// - Key: `AccessPath` as bytes
    /// - Value: `WriteSet` as bytes
    // TODO: Experimentally try hasher [Identity][frame_support::Identity]
    //                           because key are already encoded - hashes.
    #[pallet::storage]
    pub type VMStorage<T> = StorageMap<_, Blake2_128Concat, Vec<u8>, Vec<u8>>;

    // Pallets use events to inform users when important changes are made.
    // https://substrate.dev/docs/en/knowledgebase/runtime/events
    #[pallet::event]
    #[pallet::metadata(T::AccountId = "AccountId")]
    // #[pallet::metadata(T::AccountId = "AccountId", T::Balance = "Balance")]
    #[pallet::generate_deposit(pub fn deposit_event)]
    pub enum Event<T: Config> {
        // Event documentation should end with an array that provides descriptive names for event parameters.
        /// Event provided by Move VM
        /// [account, type_tag, message, module]
        Event(
            T::AccountId, /* transcoded AccountAddress */
            Vec<u8>,      /* encoded TypeTag, TODO: use `MoveTypeTag<T::AccountId>` instead */
            Vec<u8>,      /* encoded String, use Text in web-UI */
            Option<types::MoveModuleId<T::AccountId>>,
        ),

        /// Event about successful move-module publishing
        /// [account]
        ModulePublished(T::AccountId),

        /// Event about successful move-module publishing
        /// [account]
        StdModulePublished,
    }

    // Dispatchable functions allows users to interact with the pallet and invoke state changes.
    // These functions materialize as "extrinsics", which are often compared to transactions.
    // Dispatchable functions must be annotated with a weight and must return a DispatchResult.
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(T::GasWeightMapping::gas_to_weight(*gas_limit))]
        pub fn execute(
            origin: OriginFor<T>,
            tx_bc: Vec<u8>,
            gas_limit: u64,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            debug!("executing `execute` with signed {:?}", who);

            let vm_result = Self::raw_execute_script(&who, tx_bc, gas_limit, false)?;

            // produce result with spended gas:
            let result = result::from_vm_result::<T>(vm_result)?;
            Ok(result)
        }

        #[pallet::weight(T::GasWeightMapping::gas_to_weight(*gas_limit))]
        pub fn publish_module(
            origin: OriginFor<T>,
            module_bc: Vec<u8>,
            gas_limit: u64,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            debug!("executing `publish` with signed {:?}", who);

            // Publish module.
            let vm_result = Self::raw_publish_module(&who, module_bc, gas_limit, false)?;

            // produce result with spended gas:
            let result = result::from_vm_result::<T>(vm_result)?;

            // Emit an event:
            Self::deposit_event(Event::ModulePublished(who));

            Ok(result)
        }

        /// Batch publish module-package produced by Dove compiler
        #[pallet::weight(T::GasWeightMapping::gas_to_weight(*gas_limit))]
        pub fn publish_package(
            origin: OriginFor<T>,
            package: Vec<u8>,
            gas_limit: u64,
        ) -> DispatchResultWithPostInfo {
            let sender = match ensure_root(origin.clone()) {
                Ok(_) => {
                    debug!("executing `publish package` with root");
                    CORE_CODE_ADDRESS
                }
                Err(_) => {
                    let signer = ensure_signed(origin)?;
                    debug!("executing `publish package` with signed {:?}", signer);
                    addr::account_to_account_address(&signer)
                }
            };

            let vm = Self::get_vm()?;
            let gas = Self::get_move_gas_limit(gas_limit)?;

            let package = {
                ModulePackage::try_from(&package[..])
                    .map_err(|_| Error::<T>::TransactionValidationError)?
                    .into_tx(sender)
            };

            let vm_result = vm.publish_module_package(gas, package, false);

            // produce result with spended gas:
            let result = result::from_vm_result::<T>(vm_result)?;

            Ok(result)
        }

        /// Batch publish std-modules by root account only
        // #[pallet::weight(T::GasWeightMapping::gas_to_weight(*gas_limit) * modules.len().into())]
        #[pallet::weight(T::GasWeightMapping::gas_to_weight(*gas_limit))]
        pub fn publish_std(
            origin: OriginFor<T>,
            modules: Vec<Vec<u8>>,
            gas_limit: u64,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            debug!("executing `publish STD` with root");

            let vm = Self::get_vm()?;
            // TODO: use gas_used
            let mut _gas_used = 0;
            let mut results = Vec::with_capacity(modules.len());
            'deploy: for module in modules.into_iter() {
                // Overflow shound't happen.
                // As gas_limit always large or equal to used, otherwise getting out of gas error.
                let gas = Self::get_move_gas_limit(gas_limit - _gas_used)?;

                let tx = ModuleTx::new(module, CORE_CODE_ADDRESS);
                let res = vm.publish_module(gas, tx, false);
                debug!("publish result: {:?}", res);

                let is_ok = result::is_ok(&res);
                _gas_used += res.gas_used;
                results.push(res);
                if !is_ok {
                    break 'deploy;
                }

                // Emit an event:
                Self::deposit_event(Event::StdModulePublished);
            }

            // produce result with spended gas:
            let result = result::from_vm_results::<T>(&results)?;

            Ok(result)
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        #[cfg(not(feature = "no-vm-static"))]
        fn on_finalize(_: BlockNumberFor<T>) {
            if let Some(vm) = Self::get_move_vm_cell().get() {
                vm.clear();
                trace!("VM cache cleared on finalize block");
            }
        }
    }

    // get VM methods unification
    impl<T: Config> Pallet<T> {
        #[cfg(not(feature = "no-vm-static"))]
        fn get_vm() -> Result<&'static VmWrapperTy, Error<T>> {
            let vm = Self::try_get_or_create_move_vm()?;
            Ok(vm)
        }

        #[cfg(feature = "no-vm-static")]
        fn get_vm() -> Result<
            DefaultVm<VMStorage<T>, event::DefaultEventHandler, oracle::DummyOracle, T>,
            Error<T>,
        > {
            let vm = Self::try_create_move_vm()?;
            Ok(vm)
        }
    }

    const GAS_UNIT_PRICE: u64 = 1;

    impl<T: Config> Pallet<T> {
        #![allow(clippy::useless_conversion)]
        fn get_move_gas_limit(gas_limit: u64) -> Result<Gas, Error<T>> {
            Gas::new(gas_limit, GAS_UNIT_PRICE).map_err(|_| Error::InvalidGasAmountMaxValue)
        }

        // TODO: support for multiplay signers.
        pub fn raw_execute_script(
            account: &T::AccountId,
            tx_bc: Vec<u8>,
            gas_limit: u64,
            dry_run: bool,
        ) -> Result<VmResult, Error<T>>
        where
            <T as timestamp::Config>::Moment: UniqueSaturatedInto<u64>,
            // T::BlockNumber: BaseArithmetic,
            // T::BlockNumber: UniqueSaturatedInto<u64>,
            T::BlockNumber: TryInto<u64>,
        {
            // TODO: some minimum gas for processing transaction from bytes?
            let transaction = Transaction::try_from(&tx_bc[..])
                .map_err(|_| Error::<T>::TransactionValidationError)?;

            let vm = Self::get_vm()?;
            let gas = Self::get_move_gas_limit(gas_limit)?;

            let tx = {
                let signers = if transaction.signers_count() == 0 {
                    Vec::with_capacity(0)
                } else {
                    debug!("executing `execute` with signed {:?}", account);
                    let sender = addr::account_to_bytes(account);
                    debug!("converted sender: {:?}", sender);

                    vec![AccountAddress::new(sender)]
                };

                if transaction.signers_count() as usize != signers.len() {
                    error!(
                        "Transaction signers num isn't eq signers: {} != {}",
                        transaction.signers_count(),
                        signers.len()
                    );
                    return Err(Error::<T>::TransactionSignersNumError.into());
                }

                transaction
                    .into_script(signers)
                    .map_err(|_| Error::<T>::TransactionValidationError)?
            };

            let ctx = {
                let height = frame_system::Module::<T>::block_number()
                    .try_into()
                    .map_err(|_| Error::<T>::NumConversionError)?;
                let time = <timestamp::Module<T> as UnixTime>::now().as_millis() as u64;
                ExecutionContext::new(time, height as u64)
            };

            let res = vm.execute_script(gas, ctx, tx, dry_run);
            debug!("execution result: {:?}", res);

            Ok(res)
        }

        pub fn raw_publish_module(
            account: &T::AccountId,
            module_bc: Vec<u8>,
            gas_limit: u64,
            dry_run: bool,
        ) -> Result<VmResult, Error<T>> {
            let vm = Self::get_vm()?;
            let gas = Self::get_move_gas_limit(gas_limit)?;

            let tx = {
                let sender = addr::account_to_bytes(account);
                debug!("converted sender: {:?}", sender);
                ModuleTx::new(module_bc, AccountAddress::new(sender))
            };

            let res = vm.publish_module(gas, tx, dry_run);
            debug!("publication result: {:?}", res);

            Ok(res)
        }
    }

    /// Get storage adapter ready for the VM
    impl<T: Config, K, V> super::storage::MoveVmStorage<T, K, V> for Pallet<T>
    where
        K: FullEncode,
        V: FullCodec,
    {
        type VmStorage = VMStorage<T>;
    }

    impl<T: Config> event::DepositMoveEvent for Pallet<T> {
        fn deposit_move_event(e: MoveEventArguments) {
            debug!(
                "MoveVM Event: {:?} {:?} {:?} {:?}",
                e.addr, e.caller, e.ty_tag, e.message
            );

            // Emit an event:
            // TODO: dispatch up the error by TryInto. Error is almost impossible but who knows..
            Self::deposit_event(e.try_into().expect("Cannot back-convert address"));
        }
    }

    impl<T: Config> mvm::TryCreateMoveVm<T> for Pallet<T> {
        #[cfg(not(feature = "no-vm-static"))]
        type Vm = Mvm<
            boxed::StorageAdapter,
            event::DefaultEventHandler,
            oracle::DummyOracle,
            boxed::BalancesAdapter,
        >;
        #[cfg(feature = "no-vm-static")]
        type Vm = Mvm<
            StorageAdapter<VMStorage<T>>,
            event::DefaultEventHandler,
            oracle::DummyOracle,
            balance::BalancesAdapter<T>,
        >;
        type Error = Error<T>;

        fn try_create_move_vm() -> Result<Self::Vm, Self::Error> {
            trace!("MoveVM created");
            Mvm::new(
                Self::move_vm_storage().into(),
                Self::create_move_event_handler(),
                Default::default(),
                balance::BalancesAdapter::<T>::new().into(),
            )
            .map_err(|err| {
                error!("{}", err);
                Error::InvalidVMConfig
            })
        }
    }

    #[cfg(not(feature = "no-vm-static"))]
    impl<T: Config> GetStaticMoveVmCell for Pallet<T> {
        type Vm = VmWrapper<<Self as mvm::TryCreateMoveVm<T>>::Vm>;

        #[inline(never)]
        fn get_move_vm_cell() -> &'static OnceCell<VmWrapperTy> {
            static VM: OnceCell<VmWrapperTy> = OnceCell::new();
            &VM
        }
    }

    #[cfg(not(feature = "no-vm-static"))]
    impl<T: Config> TryGetStaticMoveVm for Pallet<T> {
        type Vm = <Self as GetStaticMoveVmCell>::Vm;
        type Error = Error<T>;

        fn try_get_or_create_move_vm() -> Result<&'static Self::Vm, Self::Error> {
            Self::get_move_vm_cell().get_or_try_init(|| {
                trace!("Static VM initializing");
                Self::try_create_move_vm_static().map(Into::into)
            })
        }
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Internal: numeric convertion error, overflow
        NumConversionError,

        /// Failed to read or decode VM configuration
        InvalidVMConfig,
        /// `max_gas_amount` value must be in the range from 0 to `u64::MAX / 1000`.
        /// Causes for invalid gas configuration.
        InvalidGasAmountMaxValue,
        /// Script senders should not be empty
        ScriptValidationError,
        /// Transaction deserialization & validation error
        TransactionValidationError,
        /// Transaction signers num isn't eq signers
        TransactionSignersNumError,

        /// Unknown validation status
        UnknownValidationStatus,
        /// The transaction has a bad signature
        InvalidSignature,
        /// Bad account authentication key
        InvalidAuthKey,
        /// Sequence number is too old
        SequenceNumberTooOld,
        /// Sequence number is too new
        SequenceNumberTooNew,
        /// The sequence number is too large and would overflow if the transaction were executed
        SequenceNumberTooBig,
        /// Insufficient balance to pay minimum transaction fee
        InsufficientBalanceForTransactionFee,
        /// The transaction has expired
        TransactionExpired,
        /// The sending account does not exist
        SendingAccountDoesNotExist,
        /// This write set transaction was rejected because it did not meet the requirements for one.
        RejectedWriteSet,
        /// This write set transaction cannot be applied to the current state.
        InvalidWriteSet,
        /// Length of program field in raw transaction exceeded max length
        ExceededMaxTransactionSize,
        /// This script is not in our allowlist of scripts.
        UnknownScript,
        /// Transaction is trying to publish a new module.
        UnknownModule,
        /// Max gas units submitted with transaction exceeds max gas units bound in VM
        MaxGasUnitsExceedsMaxGasUnitsBound,
        /// Max gas units submitted with transaction not enough to cover the intrinsic cost of the transaction.
        MaxGasUnitsBelowMinTransactionGasUnits,
        /// Gas unit price submitted with transaction is below minimum gas price set in the VM.
        GasUnitPriceBelowMinBound,
        /// Gas unit price submitted with the transaction is above the maximum gas price set in the VM.
        GasUnitPriceAboveMaxBound,
        /// Gas specifier submitted is either malformed (not a valid identifier), or does not refer to an accepted gas specifier
        InvalidGasSpecifier,
        /// The sending account is frozen
        SendingAccountFrozen,
        /// Unable to deserialize the account blob
        UnableToDeserializeAccount,
        /// The currency info was unable to be found
        CurrencyInfoDoesNotExist,
        /// The account sender doesn't have permissions to publish modules
        InvalidModulePublisher,
        /// The sending account has no role
        NoAccountRole,
        /// The transaction's chain_id does not match the one published on-chain
        BadChainId,
        /// Unknown verification error
        UnknownVerificationError,
        /// Index out of bounds
        IndexOutOfBounds,
        /// Invalid signature token
        InvalidSignatureToken,
        /// Recursive struct definition
        RecursiveStructDefinition,
        /// Invalid resource field
        InvalidResourceField,
        /// Invalid fall through
        InvalidFallThrough,
        /// Negative stack size within block
        NegativeStackSizeWithinBlock,
        /// Invalid main function signature
        InvalidMainFunctionSignature,
        /// Duplicate element
        DuplicateElement,
        /// Invalid module handle
        InvalidModuleHandle,
        /// Unimplemented handle
        UnimplementedHandle,
        /// Lookup failed
        LookupFailed,
        /// Type mismatch
        TypeMismatch,
        /// Missing dependency
        MissingDependency,
        /// Pop resource error
        PopResourceError,
        /// Br type mismatch
        BrTypeMismatchError,
        /// Abort type mismatch error
        AbortTypeMismatchError,

        /// Stloc type mismatch error
        StlocTypeMismatchError,
        /// Stloc unsafe to destroy error
        StlocUnsafeToDestroyError,
        /// Unsafe ret local or resource still borrowed
        UnsafeRetLocalOrResourceStillBorrowed,
        /// Ret type mismatch error
        RetTypeMismatchError,
        /// Ret borrowed mutable reference error
        RetBorrowedMutableReferenceError,
        /// Freezeref type mismatch error
        FreezerefTypeMismatchError,
        /// Freezeref exists mutable borrow error
        FreezerefExistsMutableBorrowError,
        /// Borrowfield type mismatch error
        BorrowfieldTypeMismatchError,
        /// Borrowfield bad field error
        BorrowfieldBadFieldError,
        /// Borrowfield exists mutable borrow error
        BorrowfieldExistsMutableBorrowError,
        /// Copyloc unavailable error
        CopylocUnavailableError,
        /// Copyloc resource error
        CopylocResourceError,
        /// Copyloc exists borrow error
        CopylocExistsBorrowError,
        /// Moveloc unavailable error
        MovelocUnavailableError,
        /// Moveloc exists borrow error
        MovelocExistsBorrowError,
        /// Borrowloc reference error
        BorrowlocReferenceError,
        /// Borrowloc unavailable error
        BorrowlocUnavailableError,
        /// Borrowloc exists borrow error
        BorrowlocExistsBorrowError,
        /// Call type mismatch error
        CallTypeMismatchError,
        /// Call borrowed mutable reference error
        CallBorrowedMutableReferenceError,
        /// Pack type mismatch error
        PackTypeMismatchError,
        /// Unpack type mismatch error
        UnpackTypeMismatchError,
        /// Readref type mismatch error
        ReadrefTypeMismatchError,
        /// Readref resource error
        ReadrefResourceError,
        /// Readref exists mutable borrow error
        ReadrefExistsMutableBorrowError,
        /// Writeref type mismatch error
        WriterefTypeMismatchError,
        /// Writeref resource error
        WriterefResourceError,
        /// Writeref exists borrow error
        WriterefExistsBorrowError,
        /// Writeref no mutable reference error
        WriterefNoMutableReferenceError,
        /// Integer op type mismatch error
        IntegerOpTypeMismatchError,
        /// Boolean op type mismatch error
        BooleanOpTypeMismatchError,
        /// Equality op type mismatch error
        EqualityOpTypeMismatchError,
        /// Exists resource type mismatch error
        ExistsResourceTypeMismatchError,
        /// Borrowglobal type mismatch error
        BorrowglobalTypeMismatchError,
        /// Borrowglobal no resource error
        BorrowglobalNoResourceError,
        /// Movefrom Type mismatch error
        MovefromTypeMismatchError,
        /// Movefrom no resource error
        MovefromNoResourceError,
        /// Moveto type mismatch error
        MovetoTypeMismatchError,
        /// Moveto no resource error
        MovetoNoResourceError,
        /// The self address of a module the transaction is publishing is not the sender address
        ModuleAddressDoesNotMatchSender,
        /// The module does not have any module handles. Each module or script must have at least one module handle.
        NoModuleHandles,
        /// Positive stack size at block end
        PositiveStackSizeAtBlockEnd,
        /// Missing acquires resource annotation error
        MissingAcquiresResourceAnnotationError,
        /// Extraneous acquires resource annotation error
        ExtraneousAcquiresResourceAnnotationError,
        /// Duplicate acquires resource annotation error
        DuplicateAcquiresResourceAnnotationError,
        /// Invalid acquires resource annotation error
        InvalidAcquiresResourceAnnotationError,
        /// Global reference error
        GlobalReferenceError,
        /// Constraint kind mismatch
        ConstraintKindMismatch,
        /// Number of type arguments mismatch
        NumberOfTypeArgumentsMismatch,
        /// Loop in instantiation graph
        LoopInInstantiationGraph,
        /// Zero sized struct.
        ZeroSizedStruct,
        /// Linker error
        LinkerError,
        /// Invalid constant type
        InvalidConstantType,
        /// Malformed constant data
        MalformedConstantData,
        /// Empty code unit
        EmptyCodeUnit,
        /// Invalid loop split
        InvalidLoopSplit,
        /// Invalid loop break
        InvalidLoopBreak,
        /// Invalid loop continue
        InvalidLoopContinue,
        /// Unsafe fet unused resources
        UnsafeRetUnusedResources,
        /// Too many locals
        TooManyLocals,
        /// Generic member opcode mismatch
        GenericMemberOpcodeMismatch,
        /// Function resolution failure
        FunctionResolutionFailure,
        /// Invalid operation in script
        InvalidOperationInScript,
        /// The sender is trying to publish a module named `M`, but the sender's account already contains a module with this name.
        DuplicateModuleName,
        /// Unknown invariant violation error
        UnknownInvariantViolationError,
        /// Empty value stack
        EmptyValueStack,
        /// Pc overflow
        PcOverflow,
        /// Verification error
        VerificationError,
        /// Storage error
        StorageError,
        /// Internal type error
        InternalTypeError,
        /// Event key mismatch
        EventKeyMismatch,
        /// Unreachable
        Unreachable,
        /// vm startup failure
        VmStartupFailure,
        /// Unexpected error from known move function
        UnexpectedErrorFromKnownMoveFunction,
        /// Verifier invariant violation
        VerifierInvariantViolation,
        /// Unexpected verifier error
        UnexpectedVerifierError,
        /// Unexpected deserialization error
        UnexpectedDeserializationError,
        /// Failed to serialize write set changes
        FailedToSerializeWriteSetChanges,
        /// Failed to deserialize resource
        FailedToDeserializeResource,
        /// Failed to resolve type due to linking being broken after verification
        TypeResolutionFailure,
        /// Unknown binary error
        UnknownBinaryError,
        /// Malformed
        Malformed,
        /// Bad magic
        BadMagic,
        /// Unknown version
        UnknownVersion,
        /// Unknown table type
        UnknownTableType,
        /// Unknown signature type
        UnknownSignatureType,
        /// Unknown serialized type
        UnknownSerializedType,
        /// Unknown opcode
        UnknownOpcode,
        /// BadHeader table
        BadHeaderTable,
        /// Unexpected signature type
        UnexpectedSignatureType,
        /// Duplicate table
        DuplicateTable,
        /// Unknown nominal resource
        UnknownNominalResource,
        /// Unknown kind
        UnknownKind,
        /// Unknown native struct flag
        UnknownNativeStructFlag,
        /// Bad U64
        BadU64,
        /// Bad U128
        BadU128,
        /// Value serialization error
        ValueSerializationError,
        /// Value deserialization error
        ValueDeserializationError,
        /// Code deserialization error
        CodeDeserializationError,
        /// Unknown runtime status
        UnknownRuntimeStatus,
        /// Out of gas
        OutOfGas,
        /// We tried to access a resource that does not exist under the account.
        ResourceDoesNotExist,
        /// We tried to create a resource under an account where that resource already exists.
        ResourceAlreadyExists,
        /// Missing data
        MissingData,
        /// Data format error
        DataFormatError,
        /// Aborted
        Aborted,
        /// Arithmetic error
        ArithmeticError,
        /// Execution stack overflow
        ExecutionStackOverflow,
        /// Call stack overflow
        CallStackOverflow,
        /// Vm max type depth reached
        VmMaxTypeDepthReached,
        /// Vm max value depth reached
        VmMaxValueDepthReached,
        /// Unknown status.
        UnknownStatus,
    }
}
