// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! SPDM Responder API - Minimal version for IPC testing

#![no_std]

use derive_idol_err::IdolError;
use hubpack::SerializedSize;
use serde::{Deserialize, Serialize};
use userlib::{sys_send, FromPrimitive};

pub use userlib::RecvMessage;

// Include generated client stub
include!(concat!(env!("OUT_DIR"), "/client_stub.rs"));

/// SPDM Error codes for IPC testing
#[derive(Debug, Copy, Clone, PartialEq, Eq, IdolError, Serialize, Deserialize, SerializedSize, FromPrimitive, counters::Count)]
#[repr(u32)]
pub enum SpdmError {
    InvalidRequest = 1,
    UnsupportedVersion = 2,
    UnsupportedOperation = 3,
    InvalidParameter = 4,
    CryptoOperationFailed = 5,
    CertificateNotFound = 6,
    InvalidNonce = 7,
    SessionNotEstablished = 8,
    MeasurementUnavailable = 9,
    InternalError = 10,

    #[idol(server_death)]
    ServerRestarted,
}

/// SPDM Version for testing
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, SerializedSize)]
#[repr(u8)]
pub enum SpdmVersion {
    V1_0 = 0x10,
    V1_1 = 0x11,
    V1_2 = 0x12,
}

/// SPDM Request codes for testing
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, SerializedSize)]
#[repr(u8)]
pub enum SpdmRequestCode {
    GetVersion = 0x84,
    GetCapabilities = 0xE1,
    GetCertificate = 0x82,
    ChallengeAuth = 0x83,
}

/// Simple version response for testing
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, SerializedSize)]
pub struct SpdmVersionResponse {
    pub version_count: u8,
    pub versions: [SpdmVersion; 4],
}

impl Default for SpdmVersionResponse {
    fn default() -> Self {
        Self {
            version_count: 3,
            versions: [SpdmVersion::V1_0, SpdmVersion::V1_1, SpdmVersion::V1_2, SpdmVersion::V1_0],
        }
    }
}

/// Minimal capabilities for testing
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, SerializedSize)]
pub struct SpdmCapabilities {
    pub ct_exponent: u8,
    pub flags: u32,
}

/// Minimal algorithm request/response for testing
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, SerializedSize)]
pub struct AlgorithmRequest {
    pub base_asym_algo: u32,
    pub base_hash_algo: u32,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, SerializedSize)]
pub struct AlgorithmResponse {
    pub base_asym_sel: u32,
    pub base_hash_sel: u32,
}

/// Minimal challenge auth response for testing
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, SerializedSize)]
pub struct ChallengeAuthResponse {
    pub cert_chain_hash: [u8; 32],
    pub nonce: [u8; 32],
    pub signature: [u8; 32], // Simplified for testing
}

/// Minimal measurement response for testing
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, SerializedSize)]
pub struct MeasurementResponse {
    pub measurement_index: u8,
    pub measurement_hash: [u8; 32],
}

/// Minimal key exchange response for testing
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, SerializedSize)]
pub struct KeyExchangeResponse {
    pub heartbeat_period: u8,
    pub public_key: [u8; 32], // Simplified for testing
}

/// Minimal PSK exchange response for testing
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, SerializedSize)]
pub struct PskExchangeResponse {
    pub heartbeat_period: u8,
    pub context: [u8; 32],
}