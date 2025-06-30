// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! API crate for HMAC+Hash server.

#![no_std]

use derive_idol_err::IdolError;
use userlib::{sys_send, FromPrimitive};

pub const SHA256_SZ: usize = 32;

/// Errors that can be produced from the HMAC+hash server API.
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

    #[idol(server_death)]
    ServerRestarted,
}

include!(concat!(env!("OUT_DIR"), "/client_stub.rs"));
