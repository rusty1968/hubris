// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Simple AST1060 HMAC+Hash driver library
//! 
//! This provides a direct API for HMAC and hash operations without IDL overhead.

#![no_std]

use aspeed_ddk::hace::{HaceController, HashAlgo};
use asto::Peripheral;

pub const SHA256_SIZE: usize = 32;

#[derive(Debug, Copy, Clone)]
pub enum HmacHashError {
    HardwareError,
    InvalidState,
    InvalidKeySize,
    InvalidDataSize,
}

pub struct HmacHashDriver {
    hace: HaceController,
}

impl HmacHashDriver {
    /// Create a new HMAC+Hash driver instance
    pub fn new() -> Result<Self, HmacHashError> {
        let hace_peripheral = Peripheral::from_ptr(0x1e6d0000 as *mut u8, 0x400);
        let hace = HaceController::new(hace_peripheral);
        
        Ok(Self { hace })
    }

    /// Compute SHA256 hash of data
    pub fn hash_sha256(&mut self, data: &[u8]) -> Result<[u8; SHA256_SIZE], HmacHashError> {
        self.hace.hash(HashAlgo::Sha256, data)
            .map_err(|_| HmacHashError::HardwareError)
    }

    /// Compute HMAC-SHA256 of data with key
    pub fn hmac_sha256(&mut self, key: &[u8], data: &[u8]) -> Result<[u8; SHA256_SIZE], HmacHashError> {
        if key.len() > 64 {
            return Err(HmacHashError::InvalidKeySize);
        }
        
        self.hace.hmac(HashAlgo::Sha256, key, data)
            .map_err(|_| HmacHashError::HardwareError)
    }

    /// Initialize for incremental SHA256 hashing
    pub fn init_sha256(&mut self) -> Result<(), HmacHashError> {
        self.hace.init_hash(HashAlgo::Sha256)
            .map_err(|_| HmacHashError::HardwareError)
    }

    /// Initialize for incremental HMAC-SHA256
    pub fn init_hmac_sha256(&mut self, key: &[u8]) -> Result<(), HmacHashError> {
        if key.len() > 64 {
            return Err(HmacHashError::InvalidKeySize);
        }
        
        self.hace.init_hmac(HashAlgo::Sha256, key)
            .map_err(|_| HmacHashError::HardwareError)
    }

    /// Update hash/HMAC with more data
    pub fn update(&mut self, data: &[u8]) -> Result<(), HmacHashError> {
        self.hace.update(data)
            .map_err(|_| HmacHashError::HardwareError)
    }

    /// Finalize and get the digest
    pub fn finalize(&mut self) -> Result<[u8; SHA256_SIZE], HmacHashError> {
        self.hace.finalize()
            .map_err(|_| HmacHashError::HardwareError)
    }
}
