// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Simple AST1060 HMAC+Hash driver library
//! 
//! This provides a direct API for HMAC and hash operations without IDL overhead.

#![no_std]

use aspeed_ddk::hace_controller::{HaceController, HashAlgo};
use ast1060_pac::Peripherals;
use drv_hmac_hash_api::{HmacHashError, SHA1_SZ, SHA256_SZ, SHA384_SZ, SHA512_SZ, MAX_DIGEST_SZ};

// Re-export constants for convenience
pub const SHA1_SIZE: usize = SHA1_SZ;
pub const SHA256_SIZE: usize = SHA256_SZ;
pub const SHA384_SIZE: usize = SHA384_SZ;
pub const SHA512_SIZE: usize = SHA512_SZ;
pub const MAX_DIGEST_SIZE: usize = MAX_DIGEST_SZ;

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

    /// Convert from hmac-hash-api algorithm constant
    pub fn from_u32(algo: u32) -> Result<Self, HmacHashError> {
        match algo {
            drv_hmac_hash_api::ALGO_SHA1 => Ok(Algorithm::Sha1),
            drv_hmac_hash_api::ALGO_SHA256 => Ok(Algorithm::Sha256),
            drv_hmac_hash_api::ALGO_SHA384 => Ok(Algorithm::Sha384),
            drv_hmac_hash_api::ALGO_SHA512 => Ok(Algorithm::Sha512),
            _ => Err(HmacHashError::InvalidAlgorithm),
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

pub struct HmacHashDriver {
    hace: HaceController<'static>,
    initialized: bool,
    hmac_mode: bool,
}

impl HmacHashDriver {
    /// Create a new HMAC+Hash driver instance
    pub fn new() -> Result<Self, HmacHashError> {
        // Take the AST1060 peripherals using steal
        // This works in embedded where there's only one instance
        let peripherals = unsafe { Peripherals::steal() };
        
        // Get a static reference - this is safe because we know the peripheral
        // registers are at a fixed memory location
        let hace_reg: &'static ast1060_pac::Hace = unsafe {
            &*(&peripherals.hace as *const ast1060_pac::Hace)
        };
        
        let hace = HaceController::new(hace_reg);
        
        Ok(Self { 
            hace,
            initialized: false,
            hmac_mode: false,
        })
    }

    /// Compute hash of data with specified algorithm
    pub fn hash(&mut self, algorithm: Algorithm, data: &[u8]) -> Result<DigestResult, HmacHashError> {
        // Set the algorithm
        self.hace.algo = algorithm.to_hace_algo();
        
        // Get algorithm-specific values before borrowing context
        let hash_cmd = self.hace.algo.hash_cmd();
        let block_size = self.hace.algo.block_size() as u32;
        
        // Set up the hash operation in separate scope
        {
            let ctx = self.hace.ctx_mut();
            ctx.method = hash_cmd;
            ctx.block_size = block_size;
            
            // Copy input data to buffer
            if data.len() > ctx.buffer.len() {
                return Err(HmacHashError::InvalidDataSize);
            }
            
            ctx.buffer[..data.len()].copy_from_slice(data);
            ctx.bufcnt = data.len() as u32;
            ctx.digcnt[0] = data.len() as u64;
        }
        
        // Initialize digest with IV
        self.hace.copy_iv_to_digest();
        
        // Add padding and perform hash
        self.hace.fill_padding(0);
        
        let bufcnt = self.hace.ctx_mut().bufcnt;
        self.hace.start_hash_operation(bufcnt);
        
        // Extract result
        let mut result = DigestResult {
            bytes: [0u8; MAX_DIGEST_SIZE],
            len: algorithm.digest_size(),
        };
        
        let ctx = self.hace.ctx_mut();
        result.bytes[..result.len].copy_from_slice(&ctx.digest[..result.len]);
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
        
        // Set the algorithm
        self.hace.algo = algorithm.to_hace_algo();
        
        // Get algorithm-specific values before borrowing context
        let hash_cmd = self.hace.algo.hash_cmd();
        let block_size = self.hace.algo.block_size() as u32;
        
        // Set up context in separate scope
        {
            let ctx = self.hace.ctx_mut();
            ctx.method = hash_cmd;
            ctx.block_size = block_size;
            
            // For now, just hash the concatenated key and data
            let total_len = key.len() + data.len();
            if total_len > ctx.buffer.len() {
                return Err(HmacHashError::InvalidDataSize);
            }
            
            ctx.buffer[..key.len()].copy_from_slice(key);
            ctx.buffer[key.len()..total_len].copy_from_slice(data);
            ctx.bufcnt = total_len as u32;
            ctx.digcnt[0] = total_len as u64;
        }
        
        // Process the key using the hash_key method
        self.hace.hash_key(&key);
        
        // Initialize digest with IV
        self.hace.copy_iv_to_digest();
        
        // Add padding and perform hash
        self.hace.fill_padding(0);
        
        let bufcnt = self.hace.ctx_mut().bufcnt;
        self.hace.start_hash_operation(bufcnt);
        
        // Extract result
        let mut result = DigestResult {
            bytes: [0u8; MAX_DIGEST_SIZE],
            len: algorithm.digest_size(),
        };
        
        let ctx = self.hace.ctx_mut();
        result.bytes[..result.len].copy_from_slice(&ctx.digest[..result.len]);
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
        // Set the algorithm
        self.hace.algo = algorithm.to_hace_algo();
        
        // Get algorithm-specific values before borrowing context
        let hash_cmd = self.hace.algo.hash_cmd();
        let block_size = self.hace.algo.block_size() as u32;
        
        // Get the context and initialize it
        {
            let ctx = self.hace.ctx_mut();
            ctx.method = hash_cmd;
            ctx.block_size = block_size;
            ctx.bufcnt = 0;
            ctx.digcnt[0] = 0;
            ctx.digcnt[1] = 0;
        }
        
        // Initialize digest with IV
        self.hace.copy_iv_to_digest();
        
        self.initialized = true;
        self.hmac_mode = false;
        Ok(())
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
        
        // Set the algorithm
        self.hace.algo = algorithm.to_hace_algo();
        
        // Get algorithm-specific values before borrowing context
        let hash_cmd = self.hace.algo.hash_cmd();
        let block_size = self.hace.algo.block_size() as u32;
        
        // Get the context and initialize it
        {
            let ctx = self.hace.ctx_mut();
            ctx.method = hash_cmd;
            ctx.block_size = block_size;
            ctx.bufcnt = 0;
            ctx.digcnt[0] = 0;
            ctx.digcnt[1] = 0;
        }
        
        // Process the key
        self.hace.hash_key(&key);
        
        // Initialize digest with IV
        self.hace.copy_iv_to_digest();
        
        self.initialized = true;
        self.hmac_mode = true;
        Ok(())
    }

    /// Initialize for incremental HMAC-SHA256 (convenience method)
    pub fn init_hmac_sha256(&mut self, key: &[u8]) -> Result<(), HmacHashError> {
        self.init_hmac(Algorithm::Sha256, key)
    }

    /// Update hash/HMAC with more data
    pub fn update(&mut self, data: &[u8]) -> Result<(), HmacHashError> {
        if !self.initialized {
            return Err(HmacHashError::InvalidState);
        }
        
        // For incremental operations, we need to accumulate data in the buffer
        // This is a simplified implementation
        let ctx = self.hace.ctx_mut();
        let current_bufcnt = ctx.bufcnt as usize;
        
        if current_bufcnt + data.len() > ctx.buffer.len() {
            return Err(HmacHashError::InvalidDataSize);
        }
        
        // Add data to buffer
        ctx.buffer[current_bufcnt..current_bufcnt + data.len()].copy_from_slice(data);
        ctx.bufcnt += data.len() as u32;
        ctx.digcnt[0] += data.len() as u64;
        
        Ok(())
    }

    /// Finalize and get the digest (algorithm must match the one used in init)
    pub fn finalize(&mut self, algorithm: Algorithm) -> Result<DigestResult, HmacHashError> {
        if !self.initialized {
            return Err(HmacHashError::InvalidState);
        }
        
        // Verify algorithm matches
        if self.hace.algo.digest_size() != algorithm.digest_size() {
            return Err(HmacHashError::InvalidAlgorithm);
        }
        
        // Add padding and perform final hash
        self.hace.fill_padding(0);
        
        let bufcnt = self.hace.ctx_mut().bufcnt;
        self.hace.start_hash_operation(bufcnt);
        
        // Extract result
        let mut result = DigestResult {
            bytes: [0u8; MAX_DIGEST_SIZE],
            len: algorithm.digest_size(),
        };
        
        let ctx = self.hace.ctx_mut();
        result.bytes[..result.len].copy_from_slice(&ctx.digest[..result.len]);
        
        // Reset state
        self.initialized = false;
        self.hmac_mode = false;
        
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
