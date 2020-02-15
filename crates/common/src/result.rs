use holochain_json_api::{error::JsonError, json::JsonString};
use holochain_json_derive::DefaultJson;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::str::FromStr;

const UNSPECIFIED: &str = "Unspecified";
const ARGUMENT_DESERIALIZATION_FAILED: &str = "ArgumentDeserializationFailed";
const OUT_OF_MEMORY: &str = "OutOfMemory";
const RECEIVED_WRONG_ACTION_RESULT: &str = "ReceivedWrongActionResult";
const CALLBACK_FAILED: &str = "CallbackFailed";
const RECURSIVE_CALL_FORBIDDEN: &str = "RecursiveCallForbidden";
const RESPONSE_SERIALIZATION_FAILED: &str = "ResponseSerializationFailed";
const NOT_AN_ALLOCATION: &str = "NotAnAllocation";
const ZERO_SIZED_ALLOCATION: &str = "ZeroSizedAllocation";
const UNKNOWN_ENTRY_TYPE: &str = "UnknownEntryType";
const MISMATCH_WASM_CALL_DATA_TYPE: &str = "MismatchWasmCallDataType";
const ENTRY_NOT_FOUND: &str = "EntryNotFound";
const WORKFLOW_FAILED: &str = "WorkflowFailed";

/// Enum of all possible ERROR codes that a Zome API Function could return.
#[derive(Clone, Debug, PartialEq, Eq, Hash, DefaultJson, PartialOrd, Ord)]
#[rustfmt::skip]
pub enum WasmError {
    Unspecified,
    ArgumentDeserializationFailed,
    OutOfMemory,
    ReceivedWrongActionResult,
    CallbackFailed,
    RecursiveCallForbidden,
    ResponseSerializationFailed,
    NotAnAllocation,
    ZeroSizedAllocation,
    UnknownEntryType,
    MismatchWasmCallDataType,
    EntryNotFound,
    WorkflowFailed,
    // something to do with zome logic that we don't know about
    Zome(String),
}

#[derive(Debug, Serialize, Deserialize, DefaultJson)]
pub enum WasmResult {
    Ok(JsonString),
    Err(WasmError),
}

impl ToString for WasmError {
    fn to_string(&self) -> String {
        match self {
            WasmError::Unspecified => UNSPECIFIED,
            WasmError::ArgumentDeserializationFailed => ARGUMENT_DESERIALIZATION_FAILED,
            WasmError::OutOfMemory => OUT_OF_MEMORY,
            WasmError::ReceivedWrongActionResult => RECEIVED_WRONG_ACTION_RESULT,
            WasmError::CallbackFailed => CALLBACK_FAILED,
            WasmError::RecursiveCallForbidden => RECURSIVE_CALL_FORBIDDEN,
            WasmError::ResponseSerializationFailed => RESPONSE_SERIALIZATION_FAILED,
            WasmError::NotAnAllocation => NOT_AN_ALLOCATION,
            WasmError::ZeroSizedAllocation => ZERO_SIZED_ALLOCATION,
            WasmError::UnknownEntryType => UNKNOWN_ENTRY_TYPE,
            WasmError::MismatchWasmCallDataType => MISMATCH_WASM_CALL_DATA_TYPE,
            WasmError::EntryNotFound => ENTRY_NOT_FOUND,
            WasmError::WorkflowFailed => WORKFLOW_FAILED,
            WasmError::Zome(s) => s,
        }
        .into()
    }
}

impl FromStr for WasmError {
    // this type doesn't matter because from_str is infallible as we always fallback to Zome error
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            UNSPECIFIED => WasmError::Unspecified,
            ARGUMENT_DESERIALIZATION_FAILED => WasmError::ArgumentDeserializationFailed,
            OUT_OF_MEMORY => WasmError::OutOfMemory,
            RECEIVED_WRONG_ACTION_RESULT => WasmError::ReceivedWrongActionResult,
            CALLBACK_FAILED => WasmError::CallbackFailed,
            RECURSIVE_CALL_FORBIDDEN => WasmError::RecursiveCallForbidden,
            RESPONSE_SERIALIZATION_FAILED => WasmError::ResponseSerializationFailed,
            NOT_AN_ALLOCATION => WasmError::NotAnAllocation,
            ZERO_SIZED_ALLOCATION => WasmError::ZeroSizedAllocation,
            UNKNOWN_ENTRY_TYPE => WasmError::UnknownEntryType,
            MISMATCH_WASM_CALL_DATA_TYPE => WasmError::MismatchWasmCallDataType,
            ENTRY_NOT_FOUND => WasmError::EntryNotFound,
            WORKFLOW_FAILED => WasmError::WorkflowFailed,
            // the fallback is simply to wrap whatever we got in a zome error
            _ => WasmError::Zome(s.into()),
        })
    }
}

impl From<WasmError> for String {
    fn from(ribosome_error_code: WasmError) -> Self {
        ribosome_error_code.to_string()
    }
}

// @TODO review this serialization, can it be an i32 instead of a full string?
// @see https://github.com/holochain/holochain-rust/issues/591
impl Serialize for WasmError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for WasmError {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(WasmError::from_str(&s).expect("could not deserialize WasmError"))
    }
}

#[cfg(test)]
pub mod tests {
    // use super::*;

    // #[test]
    // fn ribosome_error_code_round_trip() {
    //     let oom =
    //         WasmError::from_code_int(((WasmError::OutOfMemory as u64) >> 32) as RibosomeCodeBits);
    //     assert_eq!(WasmError::OutOfMemory, oom);
    //     assert_eq!(WasmError::OutOfMemory.to_string(), oom.to_string());
    // }

    // #[test]
    // fn error_conversion() {
    //     // TODO could use strum crate to iteratively
    //     // gather all known codes.
    //     for code in 1..=13 {
    //         let mut err = WasmError::from_code_int(code);
    //
    //         let err_str = err.as_str().to_owned();
    //
    //         err = err_str.parse().expect("unable to parse error");
    //
    //         let inner_code = RibosomeReturnValue::from_error(err);
    //
    //         let _one_int: u64 = inner_code.clone().into();
    //         let _another_int: u64 = inner_code.clone().into();
    //     }
    // }

    // #[test]
    // #[should_panic]
    // fn code_zero() {
    //     WasmError::from_code_int(0);
    // }
}
