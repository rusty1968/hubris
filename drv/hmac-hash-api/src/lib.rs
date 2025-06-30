// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! API crate for HMAC+Hash server.

#![no_std]

use derive_idol_err::IdolError;
use userlib::{sys_send, FromPrimitive};

pub const SHA1_SZ: usize = 20;
pub const SHA256_SZ: usize = 32;
pub const SHA384_SZ: usize = 48;
pub const SHA512_SZ: usize = 64;
pub const MAX_DIGEST_SZ: usize = SHA512_SZ;

// Algorithm constants for IDL interface
pub const ALGO_SHA1: u32 = 1;
pub const ALGO_SHA256: u32 = 2;
pub const ALGO_SHA384: u32 = 3;
pub const ALGO_SHA512: u32 = 4;

// Algorithm support bitmask constants
pub const SUPPORT_SHA1: u32 = 1 << 0;
pub const SUPPORT_SHA256: u32 = 1 << 1;
pub const SUPPORT_SHA384: u32 = 1 << 2;
pub const SUPPORT_SHA512: u32 = 1 << 3;
pub const SUPPORT_ALL: u32 = SUPPORT_SHA1 | SUPPORT_SHA256 | SUPPORT_SHA384 | SUPPORT_SHA512;

/// Errors that can be produced from the HMAC+hash server API.
///
/// This enumeration doesn't include errors that result from configuration
/// issues, like sending messages to the wrong task.
#[derive(
    Copy, Clone, Debug, FromPrimitive, Eq, PartialEq, IdolError, counters::Count,
)]
pub enum HmacHashError {
    NotInitialized = 1,
    InvalidState,
    Busy, // Some other owner is using the Hash block
    NoData,
    InvalidKeySize,
    InvalidAlgorithm,
    UnsupportedAlgorithm,
    HardwareError,
    InvalidDataSize,

    #[idol(server_death)]
    ServerRestarted,
}

include!(concat!(env!("OUT_DIR"), "/client_stub.rs"));