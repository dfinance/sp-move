use super::{Config, Error};
use crate::gas::GasWeightMapping;
use frame_support::dispatch::DispatchErrorWithPostInfo;
use frame_support::dispatch::DispatchResultWithPostInfo;
use frame_support::dispatch::PostDispatchInfo;
use frame_support::dispatch::Weight;
use frame_support::weights::Pays;
use move_vm::types::VmResult;
use move_core_types::vm_status::StatusCode;

pub fn is_ok(vm_result: &VmResult) -> bool {
    matches!(vm_result.status_code, StatusCode::EXECUTED)
}

pub fn from_status_code<T: Config>(code: StatusCode) -> Result<(), Error<T>> {
    match code {
        StatusCode::EXECUTED => Ok(()),
        _ => Err(Error::<T>::from(code)),
    }
}

pub fn from_status_code_with_gas<T: Config>(
    code: StatusCode,
    gas: Weight,
) -> DispatchResultWithPostInfo {
    let gas = PostDispatchInfo {
        actual_weight: Some(gas),
        pays_fee: Pays::Yes,
    };

    match code {
        StatusCode::EXECUTED => Ok(gas),
        _ => Err({
            DispatchErrorWithPostInfo {
                post_info: gas,
                error: Error::<T>::from(code).into(),
            }
        }),
    }
}

pub fn from_vm_result<T: Config>(vm_result: VmResult) -> DispatchResultWithPostInfo {
    let gas = PostDispatchInfo {
        actual_weight: Some(T::GasWeightMapping::gas_to_weight(vm_result.gas_used)),
        pays_fee: Pays::Yes,
    };

    match vm_result.status_code {
        StatusCode::EXECUTED => Ok(gas),
        status_code => Err({
            DispatchErrorWithPostInfo {
                post_info: gas,
                error: Error::<T>::from(status_code).into(),
            }
        }),
    }
}

pub fn from_vm_results<T: Config>(vm_results: &[VmResult]) -> DispatchResultWithPostInfo {
    let mut gas_total = 0;
    for vm_result in vm_results {
        gas_total += vm_result.gas_used;

        match vm_result.status_code {
            StatusCode::EXECUTED => {}
            status_code => {
                let gas = PostDispatchInfo {
                    actual_weight: Some(T::GasWeightMapping::gas_to_weight(gas_total)),
                    pays_fee: Pays::Yes,
                };
                return Err({
                    DispatchErrorWithPostInfo {
                        post_info: gas,
                        error: Error::<T>::from(status_code).into(),
                    }
                });
            }
        }
    }

    let gas = PostDispatchInfo {
        actual_weight: Some(T::GasWeightMapping::gas_to_weight(gas_total)),
        pays_fee: Pays::Yes,
    };

    Ok(gas)
}

impl<T: Config> From<StatusCode> for Error<T> {
    fn from(sp: StatusCode) -> Self {
        match sp {
            StatusCode::UNKNOWN_VALIDATION_STATUS => Self::UnknownValidationStatus,
            StatusCode::INVALID_SIGNATURE => Self::InvalidSignature,
            StatusCode::INVALID_AUTH_KEY => Self::InvalidAuthKey,
            StatusCode::SEQUENCE_NUMBER_TOO_OLD => Self::SequenceNumberTooOld,
            StatusCode::SEQUENCE_NUMBER_TOO_NEW => Self::SequenceNumberTooNew,
            StatusCode::SEQUENCE_NUMBER_TOO_BIG => Self::SequenceNumberTooBig,
            StatusCode::INSUFFICIENT_BALANCE_FOR_TRANSACTION_FEE => {
                Self::InsufficientBalanceForTransactionFee
            }
            StatusCode::TRANSACTION_EXPIRED => Self::TransactionExpired,
            StatusCode::SENDING_ACCOUNT_DOES_NOT_EXIST => Self::SendingAccountDoesNotExist,
            StatusCode::REJECTED_WRITE_SET => Self::RejectedWriteSet,
            StatusCode::INVALID_WRITE_SET => Self::InvalidWriteSet,
            StatusCode::EXCEEDED_MAX_TRANSACTION_SIZE => Self::ExceededMaxTransactionSize,
            StatusCode::UNKNOWN_SCRIPT => Self::UnknownScript,
            StatusCode::UNKNOWN_MODULE => Self::UnknownModule,
            StatusCode::MAX_GAS_UNITS_EXCEEDS_MAX_GAS_UNITS_BOUND => {
                Self::MaxGasUnitsExceedsMaxGasUnitsBound
            }
            StatusCode::MAX_GAS_UNITS_BELOW_MIN_TRANSACTION_GAS_UNITS => {
                Self::MaxGasUnitsBelowMinTransactionGasUnits
            }
            StatusCode::GAS_UNIT_PRICE_BELOW_MIN_BOUND => Self::GasUnitPriceBelowMinBound,
            StatusCode::GAS_UNIT_PRICE_ABOVE_MAX_BOUND => Self::GasUnitPriceAboveMaxBound,
            StatusCode::INVALID_GAS_SPECIFIER => Self::InvalidGasSpecifier,
            StatusCode::SENDING_ACCOUNT_FROZEN => Self::SendingAccountFrozen,
            StatusCode::UNABLE_TO_DESERIALIZE_ACCOUNT => Self::UnableToDeserializeAccount,
            StatusCode::CURRENCY_INFO_DOES_NOT_EXIST => Self::CurrencyInfoDoesNotExist,
            StatusCode::INVALID_MODULE_PUBLISHER => Self::InvalidModulePublisher,
            StatusCode::NO_ACCOUNT_ROLE => Self::NoAccountRole,
            StatusCode::BAD_CHAIN_ID => Self::BadChainId,
            StatusCode::UNKNOWN_VERIFICATION_ERROR => Self::UnknownVerificationError,
            StatusCode::INDEX_OUT_OF_BOUNDS => Self::IndexOutOfBounds,
            StatusCode::INVALID_SIGNATURE_TOKEN => Self::InvalidSignatureToken,
            StatusCode::RECURSIVE_STRUCT_DEFINITION => Self::RecursiveStructDefinition,
            // StatusCode::INVALID_RESOURCE_FIELD => Self::InvalidResourceField,
            StatusCode::INVALID_FALL_THROUGH => Self::InvalidFallThrough,
            StatusCode::NEGATIVE_STACK_SIZE_WITHIN_BLOCK => Self::NegativeStackSizeWithinBlock,
            StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE => Self::InvalidMainFunctionSignature,
            StatusCode::DUPLICATE_ELEMENT => Self::DuplicateElement,
            StatusCode::INVALID_MODULE_HANDLE => Self::InvalidModuleHandle,
            StatusCode::UNIMPLEMENTED_HANDLE => Self::UnimplementedHandle,
            StatusCode::LOOKUP_FAILED => Self::LookupFailed,
            StatusCode::TYPE_MISMATCH => Self::TypeMismatch,
            StatusCode::MISSING_DEPENDENCY => Self::MissingDependency,
            // StatusCode::POP_RESOURCE_ERROR => Self::PopResourceError,
            StatusCode::BR_TYPE_MISMATCH_ERROR => Self::BrTypeMismatchError,
            StatusCode::ABORT_TYPE_MISMATCH_ERROR => Self::AbortTypeMismatchError,
            StatusCode::STLOC_TYPE_MISMATCH_ERROR => Self::StlocTypeMismatchError,
            StatusCode::STLOC_UNSAFE_TO_DESTROY_ERROR => Self::StlocUnsafeToDestroyError,
            StatusCode::UNSAFE_RET_LOCAL_OR_RESOURCE_STILL_BORROWED => {
                Self::UnsafeRetLocalOrResourceStillBorrowed
            }
            StatusCode::RET_TYPE_MISMATCH_ERROR => Self::RetTypeMismatchError,
            StatusCode::RET_BORROWED_MUTABLE_REFERENCE_ERROR => {
                Self::RetBorrowedMutableReferenceError
            }
            StatusCode::FREEZEREF_TYPE_MISMATCH_ERROR => Self::FreezerefTypeMismatchError,
            StatusCode::FREEZEREF_EXISTS_MUTABLE_BORROW_ERROR => {
                Self::FreezerefExistsMutableBorrowError
            }
            StatusCode::BORROWFIELD_TYPE_MISMATCH_ERROR => Self::BorrowfieldTypeMismatchError,
            StatusCode::BORROWFIELD_BAD_FIELD_ERROR => Self::BorrowfieldBadFieldError,
            StatusCode::BORROWFIELD_EXISTS_MUTABLE_BORROW_ERROR => {
                Self::BorrowfieldExistsMutableBorrowError
            }
            StatusCode::COPYLOC_UNAVAILABLE_ERROR => Self::CopylocUnavailableError,
            StatusCode::COPYLOC_EXISTS_BORROW_ERROR => Self::CopylocExistsBorrowError,
            StatusCode::MOVELOC_UNAVAILABLE_ERROR => Self::MovelocUnavailableError,
            StatusCode::MOVELOC_EXISTS_BORROW_ERROR => Self::MovelocExistsBorrowError,
            StatusCode::BORROWLOC_REFERENCE_ERROR => Self::BorrowlocReferenceError,
            StatusCode::BORROWLOC_UNAVAILABLE_ERROR => Self::BorrowlocUnavailableError,
            StatusCode::BORROWLOC_EXISTS_BORROW_ERROR => Self::BorrowlocExistsBorrowError,
            StatusCode::CALL_TYPE_MISMATCH_ERROR => Self::CallTypeMismatchError,
            StatusCode::CALL_BORROWED_MUTABLE_REFERENCE_ERROR => {
                Self::CallBorrowedMutableReferenceError
            }
            StatusCode::PACK_TYPE_MISMATCH_ERROR => Self::PackTypeMismatchError,
            StatusCode::UNPACK_TYPE_MISMATCH_ERROR => Self::UnpackTypeMismatchError,
            StatusCode::READREF_TYPE_MISMATCH_ERROR => Self::ReadrefTypeMismatchError,
            // StatusCode::READREF_RESOURCE_ERROR => Self::ReadrefResourceError,
            StatusCode::READREF_EXISTS_MUTABLE_BORROW_ERROR => {
                Self::ReadrefExistsMutableBorrowError
            }
            StatusCode::WRITEREF_TYPE_MISMATCH_ERROR => Self::WriterefTypeMismatchError,
            // StatusCode::WRITEREF_RESOURCE_ERROR => Self::WriterefResourceError,
            StatusCode::WRITEREF_EXISTS_BORROW_ERROR => Self::WriterefExistsBorrowError,
            StatusCode::WRITEREF_NO_MUTABLE_REFERENCE_ERROR => {
                Self::WriterefNoMutableReferenceError
            }
            StatusCode::INTEGER_OP_TYPE_MISMATCH_ERROR => Self::IntegerOpTypeMismatchError,
            StatusCode::BOOLEAN_OP_TYPE_MISMATCH_ERROR => Self::BooleanOpTypeMismatchError,
            StatusCode::EQUALITY_OP_TYPE_MISMATCH_ERROR => Self::EqualityOpTypeMismatchError,
            // StatusCode::EXISTS_RESOURCE_TYPE_MISMATCH_ERROR => {
            // Self::ExistsResourceTypeMismatchError
            // }
            StatusCode::BORROWGLOBAL_TYPE_MISMATCH_ERROR => Self::BorrowglobalTypeMismatchError,
            // StatusCode::BORROWGLOBAL_NO_RESOURCE_ERROR => Self::BorrowglobalNoResourceError,
            StatusCode::MOVEFROM_TYPE_MISMATCH_ERROR => Self::MovefromTypeMismatchError,
            // StatusCode::MOVEFROM_NO_RESOURCE_ERROR => Self::MovefromNoResourceError,
            StatusCode::MOVETO_TYPE_MISMATCH_ERROR => Self::MovetoTypeMismatchError,
            // StatusCode::MOVETO_NO_RESOURCE_ERROR => Self::MovetoNoResourceError,
            StatusCode::MODULE_ADDRESS_DOES_NOT_MATCH_SENDER => {
                Self::ModuleAddressDoesNotMatchSender
            }
            StatusCode::NO_MODULE_HANDLES => Self::NoModuleHandles,
            StatusCode::POSITIVE_STACK_SIZE_AT_BLOCK_END => Self::PositiveStackSizeAtBlockEnd,
            // StatusCode::MISSING_ACQUIRES_RESOURCE_ANNOTATION_ERROR => {
            //     Self::MissingAcquiresResourceAnnotationError
            // }
            // StatusCode::EXTRANEOUS_ACQUIRES_RESOURCE_ANNOTATION_ERROR => {
            //     Self::ExtraneousAcquiresResourceAnnotationError
            // }
            // StatusCode::DUPLICATE_ACQUIRES_RESOURCE_ANNOTATION_ERROR => {
            //     Self::DuplicateAcquiresResourceAnnotationError
            // }
            // StatusCode::INVALID_ACQUIRES_RESOURCE_ANNOTATION_ERROR => {
            //     Self::InvalidAcquiresResourceAnnotationError
            // }
            StatusCode::GLOBAL_REFERENCE_ERROR => Self::GlobalReferenceError,
            // StatusCode::CONSTRAINT_KIND_MISMATCH => Self::ConstraintKindMismatch,
            StatusCode::NUMBER_OF_TYPE_ARGUMENTS_MISMATCH => Self::NumberOfTypeArgumentsMismatch,
            StatusCode::LOOP_IN_INSTANTIATION_GRAPH => Self::LoopInInstantiationGraph,
            StatusCode::ZERO_SIZED_STRUCT => Self::ZeroSizedStruct,
            StatusCode::LINKER_ERROR => Self::LinkerError,
            StatusCode::INVALID_CONSTANT_TYPE => Self::InvalidConstantType,
            StatusCode::MALFORMED_CONSTANT_DATA => Self::MalformedConstantData,
            StatusCode::EMPTY_CODE_UNIT => Self::EmptyCodeUnit,
            StatusCode::INVALID_LOOP_SPLIT => Self::InvalidLoopSplit,
            StatusCode::INVALID_LOOP_BREAK => Self::InvalidLoopBreak,
            StatusCode::INVALID_LOOP_CONTINUE => Self::InvalidLoopContinue,
            // StatusCode::UNSAFE_RET_UNUSED_RESOURCES => Self::UnsafeRetUnusedResources,
            StatusCode::TOO_MANY_LOCALS => Self::TooManyLocals,
            StatusCode::GENERIC_MEMBER_OPCODE_MISMATCH => Self::GenericMemberOpcodeMismatch,
            StatusCode::FUNCTION_RESOLUTION_FAILURE => Self::FunctionResolutionFailure,
            StatusCode::INVALID_OPERATION_IN_SCRIPT => Self::InvalidOperationInScript,
            StatusCode::DUPLICATE_MODULE_NAME => Self::DuplicateModuleName,
            StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR => Self::UnknownInvariantViolationError,
            StatusCode::EMPTY_VALUE_STACK => Self::EmptyValueStack,
            StatusCode::PC_OVERFLOW => Self::PcOverflow,
            StatusCode::VERIFICATION_ERROR => Self::VerificationError,
            StatusCode::STORAGE_ERROR => Self::StorageError,
            StatusCode::INTERNAL_TYPE_ERROR => Self::InternalTypeError,
            StatusCode::EVENT_KEY_MISMATCH => Self::EventKeyMismatch,
            StatusCode::UNREACHABLE => Self::Unreachable,
            StatusCode::VM_STARTUP_FAILURE => Self::VmStartupFailure,
            StatusCode::UNEXPECTED_ERROR_FROM_KNOWN_MOVE_FUNCTION => {
                Self::UnexpectedErrorFromKnownMoveFunction
            }
            StatusCode::VERIFIER_INVARIANT_VIOLATION => Self::VerifierInvariantViolation,
            StatusCode::UNEXPECTED_VERIFIER_ERROR => Self::UnexpectedVerifierError,
            StatusCode::UNEXPECTED_DESERIALIZATION_ERROR => Self::UnexpectedDeserializationError,
            StatusCode::FAILED_TO_SERIALIZE_WRITE_SET_CHANGES => {
                Self::FailedToSerializeWriteSetChanges
            }
            StatusCode::FAILED_TO_DESERIALIZE_RESOURCE => Self::FailedToDeserializeResource,
            StatusCode::TYPE_RESOLUTION_FAILURE => Self::TypeResolutionFailure,
            StatusCode::UNKNOWN_BINARY_ERROR => Self::UnknownBinaryError,
            StatusCode::MALFORMED => Self::Malformed,
            StatusCode::BAD_MAGIC => Self::BadMagic,
            StatusCode::UNKNOWN_VERSION => Self::UnknownVersion,
            StatusCode::UNKNOWN_TABLE_TYPE => Self::UnknownTableType,
            StatusCode::UNKNOWN_SIGNATURE_TYPE => Self::UnknownSignatureType,
            StatusCode::UNKNOWN_SERIALIZED_TYPE => Self::UnknownSerializedType,
            StatusCode::UNKNOWN_OPCODE => Self::UnknownOpcode,
            StatusCode::BAD_HEADER_TABLE => Self::BadHeaderTable,
            StatusCode::UNEXPECTED_SIGNATURE_TYPE => Self::UnexpectedSignatureType,
            StatusCode::DUPLICATE_TABLE => Self::DuplicateTable,
            // StatusCode::UNKNOWN_NOMINAL_RESOURCE => Self::UnknownNominalResource,
            // StatusCode::UNKNOWN_KIND => Self::UnknownKind,
            StatusCode::UNKNOWN_NATIVE_STRUCT_FLAG => Self::UnknownNativeStructFlag,
            StatusCode::BAD_U64 => Self::BadU64,
            StatusCode::BAD_U128 => Self::BadU128,
            StatusCode::VALUE_SERIALIZATION_ERROR => Self::ValueSerializationError,
            StatusCode::VALUE_DESERIALIZATION_ERROR => Self::ValueDeserializationError,
            StatusCode::CODE_DESERIALIZATION_ERROR => Self::CodeDeserializationError,
            StatusCode::UNKNOWN_RUNTIME_STATUS => Self::UnknownRuntimeStatus,
            StatusCode::OUT_OF_GAS => Self::OutOfGas,
            StatusCode::RESOURCE_DOES_NOT_EXIST => Self::ResourceDoesNotExist,
            StatusCode::RESOURCE_ALREADY_EXISTS => Self::ResourceAlreadyExists,
            StatusCode::MISSING_DATA => Self::MissingData,
            StatusCode::DATA_FORMAT_ERROR => Self::DataFormatError,
            StatusCode::ABORTED => Self::Aborted,
            StatusCode::ARITHMETIC_ERROR => Self::ArithmeticError,
            StatusCode::EXECUTION_STACK_OVERFLOW => Self::ExecutionStackOverflow,
            StatusCode::CALL_STACK_OVERFLOW => Self::CallStackOverflow,
            StatusCode::VM_MAX_TYPE_DEPTH_REACHED => Self::VmMaxTypeDepthReached,
            StatusCode::VM_MAX_VALUE_DEPTH_REACHED => Self::VmMaxValueDepthReached,
            StatusCode::UNKNOWN_STATUS => Self::UnknownStatus,

            StatusCode::EXECUTED => unreachable!(),

            StatusCode::BAD_TRANSACTION_FEE_CURRENCY => Self::BadTransactionFeeCurrency,
            StatusCode::FEATURE_UNDER_GATING => Self::FeatureUnderGating,
            StatusCode::FIELD_MISSING_TYPE_ABILITY => Self::FieldMissingTypeAbility,
            StatusCode::POP_WITHOUT_DROP_ABILITY => Self::PopWithoutDropAbility,
            StatusCode::COPYLOC_WITHOUT_COPY_ABILITY => Self::CopylocWithoutCopyAbility,
            StatusCode::READREF_WITHOUT_COPY_ABILITY => Self::ReadrefWithoutCopyAbility,
            StatusCode::WRITEREF_WITHOUT_DROP_ABILITY => Self::WriterefWithoutDropAbility,
            StatusCode::EXISTS_WITHOUT_KEY_ABILITY_OR_BAD_ARGUMENT => {
                Self::ExistsWithoutKeyAbilityOrBadArgument
            }
            StatusCode::BORROWGLOBAL_WITHOUT_KEY_ABILITY => Self::BorrowglobalWithoutKeyAbility,
            StatusCode::MOVEFROM_WITHOUT_KEY_ABILITY => Self::MovefromWithoutKeyAbility,
            StatusCode::MOVETO_WITHOUT_KEY_ABILITY => Self::MovetoWithoutKeyAbility,
            StatusCode::MISSING_ACQUIRES_ANNOTATION => Self::MissingAcquiresAnnotation,
            StatusCode::EXTRANEOUS_ACQUIRES_ANNOTATION => Self::ExtraneousAcquiresAnnotation,
            StatusCode::DUPLICATE_ACQUIRES_ANNOTATION => Self::DuplicateAcquiresAnnotation,
            StatusCode::INVALID_ACQUIRES_ANNOTATION => Self::InvalidAcquiresAnnotation,
            StatusCode::CONSTRAINT_NOT_SATISFIED => Self::ConstraintNotSatisfied,
            StatusCode::UNSAFE_RET_UNUSED_VALUES_WITHOUT_DROP => {
                Self::UnsafeRetUnusedValuesWithoutDrop
            }
            StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE => {
                Self::BackwardIncompatibleModuleUpdate
            }
            StatusCode::CYCLIC_MODULE_DEPENDENCY => Self::CyclicModuleDependency,
            StatusCode::NUMBER_OF_ARGUMENTS_MISMATCH => Self::NumberOfArgumentsMismatch,
            StatusCode::INVALID_PARAM_TYPE_FOR_DESERIALIZATION => {
                Self::InvalidParamTypeForDeserialization
            }
            StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT => Self::FailedToDeserializeArgument,
            StatusCode::NUMBER_OF_SIGNER_ARGUMENTS_MISMATCH => {
                Self::NumberOfSignerArgumentsMismatch
            }
            StatusCode::CALLED_SCRIPT_VISIBLE_FROM_NON_SCRIPT_VISIBLE => {
                Self::CalledScriptVisibleFromNonScriptVisible
            }
            StatusCode::EXECUTE_SCRIPT_FUNCTION_CALLED_ON_NON_SCRIPT_VISIBLE => {
                Self::ExecuteScriptFunctionCalledOnNonScriptVisible
            }
            StatusCode::INVALID_FRIEND_DECL_WITH_SELF => Self::InvalidFriendDeclWithSelf,
            StatusCode::INVALID_FRIEND_DECL_WITH_MODULES_OUTSIDE_ACCOUNT_ADDRESS => {
                Self::InvalidFriendDeclWithModulesOutsideAccountAddress
            }
            StatusCode::INVALID_FRIEND_DECL_WITH_MODULES_IN_DEPENDENCIES => {
                Self::InvalidFriendDeclWithModulesInDependencies
            }
            StatusCode::CYCLIC_MODULE_FRIENDSHIP => Self::CyclicModuleFriendship,
            StatusCode::UNKNOWN_ABILITY => Self::UnknownAbility,
            StatusCode::INVALID_FLAG_BITS => Self::InvalidFlagBits,
        }
    }
}
