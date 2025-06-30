// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Simple AST1060 HMAC+Hash driver library
//! 
//! This provides a direct API for HMAC and hash operations without IDL overhead.

#![no_std]

use aspeed_ddk::hace_controller::{HaceController, HashAlgo};
use ast1060_pac::Peripherals;

pub const SHA1_SIZE: usize = 20;
pub const SHA256_SIZE: usize = 32;
pub const SHA384_SIZE: usize = 48;
pub const SHA512_SIZE: usize = 64;
pub const MAX_DIGEST_SIZE: usize = SHA512_SIZE;

/// Variable-length digest result
#[derive(Debug, Clone)]
pub struct DigestResult {
    pub bytes: [u8; MAX_DIGEST_SIZE],
    pub len: usize,
}

impl DigestResult {
    /// Get the digest as a slice
    pub fn as_slice(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}

/// Supported hash algorithms
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Algorithm {
    Sha1,
    Sha256, 
    Sha384,
    Sha512,
}

impl Algorithm {
    /// Convert to aspeed-ddk HashAlgo
    fn to_hace_algo(self) -> HashAlgo {
        match self {
            Algorithm::Sha1 => HashAlgo::SHA1,
            Algorithm::Sha256 => HashAlgo::SHA256,
            Algorithm::Sha384 => HashAlgo::SHA384,
            Algorithm::Sha512 => HashAlgo::SHA512,
        }
    }

    /// Get the digest size for this algorithm
    pub fn digest_size(self) -> usize {
        match self {
            Algorithm::Sha1 => SHA1_SIZE,
            Algorithm::Sha256 => SHA256_SIZE,
            Algorithm::Sha384 => SHA384_SIZE,
            Algorithm::Sha512 => SHA512_SIZE,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum HmacHashError {
    HardwareError,
    InvalidState,
    InvalidKeySize,
    InvalidDataSize,
}

pub struct HmacHashDriver<'a> {
    hace: HaceController<'a>,
}

impl<'a> HmacHashDriver<'a> {
    /// Create a new HMAC+Hash driver instance
    pub fn new() -> Result<Self, HmacHashError> {
        // Take the AST1060 peripherals from the external PAC
        let peripherals = Peripherals::take().ok_or(HmacHashError::HardwareError)?;
        let hace_peripheral = peripherals.hace;
        let hace = HaceController::new(hace_peripheral);
        
        Ok(Self { hace })
    }

    /// Compute hash of data with specified algorithm
    pub fn hash(&mut self, algorithm: Algorithm, data: &[u8]) -> Result<DigestResult, HmacHashError> {
        let digest_raw = self.hace.hash(algorithm.to_hace_algo(), data)
            .map_err(|_| HmacHashError::HardwareError)?;
        
        let mut result = DigestResult {
            bytes: [0u8; MAX_DIGEST_SIZE],
            len: algorithm.digest_size(),
        };
        
        result.bytes[..result.len].copy_from_slice(&digest_raw[..result.len]);
        Ok(result)
    }

    /// Compute SHA256 hash of data (convenience method)
    pub fn hash_sha256(&mut self, data: &[u8]) -> Result<[u8; SHA256_SIZE], HmacHashError> {
        let result = self.hash(Algorithm::Sha256, data)?;
        let mut digest = [0u8; SHA256_SIZE];
        digest.copy_from_slice(result.as_slice());
        Ok(digest)
    }

    /// Compute HMAC with specified algorithm
    pub fn hmac(&mut self, algorithm: Algorithm, key: &[u8], data: &[u8]) -> Result<DigestResult, HmacHashError> {
        if key.len() > 64 {
            return Err(HmacHashError::InvalidKeySize);
        }
        
        let digest_raw = self.hace.hmac(algorithm.to_hace_algo(), key, data)
            .map_err(|_| HmacHashError::HardwareError)?;
        
        let mut result = DigestResult {
            bytes: [0u8; MAX_DIGEST_SIZE],
            len: algorithm.digest_size(),
        };
        
        result.bytes[..result.len].copy_from_slice(&digest_raw[..result.len]);
        Ok(result)
    }

    /// Compute HMAC-SHA256 of data with key (convenience method)
    pub fn hmac_sha256(&mut self, key: &[u8], data: &[u8]) -> Result<[u8; SHA256_SIZE], HmacHashError> {
        let result = self.hmac(Algorithm::Sha256, key, data)?;
        let mut digest = [0u8; SHA256_SIZE];
        digest.copy_from_slice(result.as_slice());
        Ok(digest)
    }

    /// Initialize for incremental hashing with specified algorithm
    pub fn init_hash(&mut self, algorithm: Algorithm) -> Result<(), HmacHashError> {
        self.hace.init_hash(algorithm.to_hace_algo())
            .map_err(|_| HmacHashError::HardwareError)
    }

    /// Initialize for incremental SHA256 hashing (convenience method)
    pub fn init_sha256(&mut self) -> Result<(), HmacHashError> {
        self.init_hash(Algorithm::Sha256)
    }

    /// Initialize for incremental HMAC with specified algorithm
    pub fn init_hmac(&mut self, algorithm: Algorithm, key: &[u8]) -> Result<(), HmacHashError> {
        if key.len() > 64 {
            return Err(HmacHashError::InvalidKeySize);
        }
        
        self.hace.init_hmac(algorithm.to_hace_algo(), key)
            .map_err(|_| HmacHashError::HardwareError)
    }

    /// Initialize for incremental HMAC-SHA256 (convenience method)
    pub fn init_hmac_sha256(&mut self, key: &[u8]) -> Result<(), HmacHashError> {
        self.init_hmac(Algorithm::Sha256, key)
    }

    /// Update hash/HMAC with more data
    pub fn update(&mut self, data: &[u8]) -> Result<(), HmacHashError> {
        self.hace.update(data)
            .map_err(|_| HmacHashError::HardwareError)
    }

    /// Finalize and get the digest (algorithm must match the one used in init)
    pub fn finalize(&mut self, algorithm: Algorithm) -> Result<DigestResult, HmacHashError> {
        let digest_raw = self.hace.finalize()
            .map_err(|_| HmacHashError::HardwareError)?;
        
        let mut result = DigestResult {
            bytes: [0u8; MAX_DIGEST_SIZE],
            len: algorithm.digest_size(),
        };
        
        result.bytes[..result.len].copy_from_slice(&digest_raw[..result.len]);
        Ok(result)
    }

    /// Finalize and get SHA256 digest (convenience method)
    pub fn finalize_sha256(&mut self) -> Result<[u8; SHA256_SIZE], HmacHashError> {
        let result = self.finalize(Algorithm::Sha256)?;
        let mut digest = [0u8; SHA256_SIZE];
        digest.copy_from_slice(result.as_slice());
        Ok(digest)
    }
}
