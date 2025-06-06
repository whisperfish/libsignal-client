//
// Copyright 2024 Signal Messenger, LLC.
// SPDX-License-Identifier: AGPL-3.0-only
//

#[cfg(not(any(feature = "ffi", feature = "jni", feature = "node")))]
compile_error!("Feature \"ffi\", \"jni\", or \"node\" must be enabled for this crate.");

use std::collections::BTreeMap;

use libsignal_bridge_macros::bridge_fn;
#[cfg(feature = "node")]
pub use libsignal_bridge_types::node;
use libsignal_bridge_types::support::*;
use libsignal_bridge_types::*;

pub mod convert;
mod keytrans;
pub mod message_backup;
pub mod net;
#[cfg(feature = "node")]
pub mod net_env;
pub mod protocol;
pub mod types;

#[bridge_fn]
pub fn test_only_fn_returns_123() -> u32 {
    123
}

#[bridge_fn]
pub fn TESTING_BridgedStringMap_dump_to_json(map: &BridgedStringMap) -> String {
    // Convert to a BTreeMap for sorted (deterministic) output.
    serde_json::to_string_pretty(&BTreeMap::from_iter(map.iter()))
        .expect("map of string -> string is always valid JSON")
}
