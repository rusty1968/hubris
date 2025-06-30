// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! AST1060 HMAC+Hash driver wrapper.
//!
//! This driver wraps the existing HACE controller implementation from
//! the aspeed-rust crate to provide Hubris-compatible APIs.

#![no_std]

//use drv_hmac_hash_api::{HmacHashError, HashAlgorithm, DigestResult};
use aspeed_ddk::hace_controller::{HaceController, HashAlgo as HaceHashAlgo};
use ast1060_pac::Hace;
use ast1060_pac::Peripherals as Ast1060Peripherals;
use userlib::*;

/// Wrapper around the HACE controller for Hubris integration
pub struct HmacHash<'a> {
    hace: HaceController<'a>,
}

impl<'a> HmacHash<'a> {
    /// Create a new HACE driver instance by taking the peripheral
    pub fn new() -> Result<Self, HmacHashError> {
        // Take the HACE peripheral from the PAC
        let peripherals = unsafe { Ast1060Peripherals::steal() };
        let hace_peripheral = peripherals.hace;
        
        // Convert to static reference (safe because we own the peripheral)
        let hace_ref: &'static Hace = unsafe { 
            core::mem::transmute(&hace_peripheral)
        };
        
        Ok(Self {
            hace: HaceController::new(hace_ref),
        })
    }

    /// Initialize for hash operation with specified algorithm
    pub fn init_hash(&mut self, algorithm: HashAlgorithm) -> Result<(), HmacHashError> {
        let hace_algo = self.convert_algorithm(algorithm)?;
        self.hace.algo = hace_algo;
        
        // Initialize the context for hashing
        let ctx = self.hace.ctx_mut();
        ctx.method = hace_algo.hash_cmd();
        ctx.block_size = hace_algo.block_size() as u32;
        ctx.digcnt = [0; 2];
        ctx.bufcnt = 0;
        
        // Copy initial vector to digest
        self.hace.copy_iv_to_digest();
        
        Ok(())
    }

    /// Initialize for HMAC operation with specified algorithm
    pub fn init_hmac(&mut self, algorithm: HashAlgorithm, key: &[u8]) -> Result<(), HmacHashError> {
        if key.is_empty() || key.len() > 128 {
            return Err(HmacHashError::InvalidKeySize);
        }

        let hace_algo = self.convert_algorithm(algorithm)?;
        self.hace.algo = hace_algo;
        
        // Initialize the context for HMAC
        let ctx = self.hace.ctx_mut();
        ctx.method = hace_algo.hash_cmd();
        ctx.block_size = hace_algo.block_size() as u32;
        ctx.digcnt = [0; 2];
        ctx.bufcnt = 0;
        
        // Handle key processing
        if key.len() > hace_algo.block_size() {
            // Hash the key if it's too long
            self.hace.hash_key(&key);
        } else {
            // Pad the key to block size
            ctx.key[..key.len()].copy_from_slice(key);
            ctx.key[key.len()..hace_algo.block_size()].fill(0);
            ctx.key_len = hace_algo.block_size() as u32;
            
            // Prepare HMAC pads
            for i in 0..hace_algo.block_size() {
                ctx.ipad[i] = ctx.key[i] ^ 0x36;
                ctx.opad[i] = ctx.key[i] ^ 0x5c;
            }
        }
        
        // Copy initial vector and start with inner pad
        self.hace.copy_iv_to_digest();
        
        Ok(())
    }

    /// Update with more data
    pub fn update(&mut self, data: &[u8]) -> Result<(), HmacHashError> {
        let ctx = self.hace.ctx_mut();
        let block_size = ctx.block_size as usize;
        
        for &byte in data {
            ctx.buffer[ctx.bufcnt as usize] = byte;
            ctx.bufcnt += 1;
            ctx.digcnt[0] += 1;
            
            // Check for buffer overflow and handle block size overflow
            if ctx.digcnt[0] == 0 {
                ctx.digcnt[1] += 1;
            }
            
            // Process block when buffer is full
            if ctx.bufcnt as usize == block_size {
                ctx.method &= !aspeed_ddk::hace_controller::HACE_SG_EN; // Disable SG mode
                self.hace.start_hash_operation(ctx.bufcnt);
                ctx.bufcnt = 0;
            }
        }
        
        Ok(())
    }

    /// Finalize and return digest
    pub fn finalize(&mut self) -> Result<DigestResult, HmacHashError> {
        let algorithm = self.hace.algo;
        
        // Fill padding and finalize
        self.hace.fill_padding(0);
        let bufcnt = self.hace.ctx_mut().bufcnt;
        self.hace.start_hash_operation(bufcnt);
        
        // Get the digest
        let digest_size = algorithm.digest_size();
        let digest = &self.hace.ctx_mut().digest[..digest_size];
        
        // Convert back to Hubris types
        let hubris_algo = self.convert_algorithm_back(algorithm)?;
        Ok(DigestResult::new(hubris_algo, digest))
    }

    /// One-shot hash digest
    pub fn digest(&mut self, algorithm: HashAlgorithm, data: &[u8]) -> Result<DigestResult, HmacHashError> {
        self.init_hash(algorithm)?;
        self.update(data)?;
        self.finalize()
    }

    /// One-shot HMAC digest
    pub fn digest_hmac(&mut self, algorithm: HashAlgorithm, data: &[u8], key: &[u8]) -> Result<DigestResult, HmacHashError> {
        self.init_hmac(algorithm, key)?;
        
        // Process inner hash: H(K XOR ipad, message)
        let block_size = self.hace.algo.block_size();
        self.update(&self.hace.ctx_mut().ipad[..block_size])?;
        self.update(data)?;
        let inner_result = self.finalize()?;
        
        // Process outer hash: H(K XOR opad, inner_result)
        self.init_hash(algorithm)?;
        self.update(&self.hace.ctx_mut().opad[..block_size])?;
        self.update(inner_result.as_slice())?;
        self.finalize()
    }

    // Helper methods for converting between API types

    fn convert_algorithm(&self, algorithm: HashAlgorithm) -> Result<HaceHashAlgo, HmacHashError> {
        match algorithm {
            HashAlgorithm::Md5 => Err(HmacHashError::UnsupportedAlgorithm), // MD5 not in the enum
            HashAlgorithm::Sha1 => Ok(HaceHashAlgo::SHA1),
            HashAlgorithm::Sha256 => Ok(HaceHashAlgo::SHA256),
            HashAlgorithm::Sha384 => Ok(HaceHashAlgo::SHA384),
            HashAlgorithm::Sha512 => Ok(HaceHashAlgo::SHA512),
        }
    }

    fn convert_algorithm_back(&self, algorithm: HaceHashAlgo) -> Result<HashAlgorithm, HmacHashError> {
        match algorithm {
            HaceHashAlgo::SHA1 => Ok(HashAlgorithm::Sha1),
            HaceHashAlgo::SHA224 => Err(HmacHashError::UnsupportedAlgorithm), // Not in Hubris enum
            HaceHashAlgo::SHA256 => Ok(HashAlgorithm::Sha256),
            HaceHashAlgo::SHA384 => Ok(HashAlgorithm::Sha384),
            HaceHashAlgo::SHA512 => Ok(HashAlgorithm::Sha512),
            HaceHashAlgo::SHA512_224 => Err(HmacHashError::UnsupportedAlgorithm), // Not in Hubris enum
            HaceHashAlgo::SHA512_256 => Err(HmacHashError::UnsupportedAlgorithm), // Not in Hubris enum
        }
    }
}
