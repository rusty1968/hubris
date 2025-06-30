// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! AST1060 HMAC+Hash server.
//!
//! This server provides IPC access to the HACE controller for HMAC and hash operations.

#![no_std]
#![no_main]

use userlib::*;

use drv_ast1060_hmac_hash_simple::{HmacHashDriver, Algorithm};
use drv_hmac_hash_api::{HmacHashError, MAX_DIGEST_SZ, SHA256_SZ,
    ALGO_SHA1, ALGO_SHA256, ALGO_SHA384, ALGO_SHA512,
    SUPPORT_ALL};
use idol_runtime::{
    ClientError, Leased, LenLimit, NotificationHandler, RequestError, R,
};

#[export_name = "main"]
fn main() -> ! {
    let driver = HmacHashDriver::new().unwrap_lite();
    
    let mut buffer = [0; idl::INCOMING_SIZE];
    let mut server = ServerImpl {
        driver,
        block: [0; 512],
    };

    loop {
        idol_runtime::dispatch(&mut buffer, &mut server);
    }
}

struct ServerImpl {
    driver: HmacHashDriver,
    block: [u8; 512],
}

impl idl::InOrderHmacHashImpl for ServerImpl {
    fn get_supported_algorithms(
        &mut self,
        _: &RecvMessage,
    ) -> Result<u32, RequestError<HmacHashError>> {
        Ok(SUPPORT_ALL)
    }

    fn init_hash(
        &mut self,
        _: &RecvMessage,
        algorithm: u32,
    ) -> Result<(), RequestError<HmacHashError>> {
        let algo = algorithm_from_u32(algorithm)?;
        self.driver.init_hash(algo)
            .map_err(|e| RequestError::Runtime(map_error(e)))
    }

    fn init_hmac(
        &mut self,
        _: &RecvMessage,
        algorithm: u32,
        key_len: u32,
        key: LenLimit<Leased<R, [u8]>, 64>,
    ) -> Result<(), RequestError<HmacHashError>> {
        if key_len == 0 || key.len() < key_len as usize {
            return Err(HmacHashError::InvalidKeySize.into());
        }
        
        let algo = algorithm_from_u32(algorithm)?;
        
        key.read_range(0..key_len as usize, &mut self.block[..key_len as usize])
            .map_err(|_| RequestError::Fail(ClientError::WentAway))?;
            
        self.driver.init_hmac(algo, &self.block[..key_len as usize])
            .map_err(|e| RequestError::Runtime(map_error(e)))
    }

    fn init_sha256(
        &mut self,
        _: &RecvMessage,
    ) -> Result<(), RequestError<HmacHashError>> {
        self.driver.init_sha256()
            .map_err(|e| RequestError::Runtime(map_error(e)))
    }

    fn init_hmac_sha256(
        &mut self,
        _: &RecvMessage,
        key_len: u32,
        key: LenLimit<Leased<R, [u8]>, 64>,
    ) -> Result<(), RequestError<HmacHashError>> {
        if key_len == 0 || key.len() < key_len as usize {
            return Err(HmacHashError::InvalidKeySize.into());
        }
        
        key.read_range(0..key_len as usize, &mut self.block[..key_len as usize])
            .map_err(|_| RequestError::Fail(ClientError::WentAway))?;
            
        self.driver.init_hmac_sha256(&self.block[..key_len as usize])
            .map_err(|e| RequestError::Runtime(map_error(e)))
    }

    fn update(
        &mut self,
        _: &RecvMessage,
        len: u32,
        data: LenLimit<Leased<R, [u8]>, 512>,
    ) -> Result<(), RequestError<HmacHashError>> {
        if len == 0 || data.len() < len as usize {
            return Err(HmacHashError::NoData.into());
        }
        
        data.read_range(0..len as usize, &mut self.block[..len as usize])
            .map_err(|_| RequestError::Fail(ClientError::WentAway))?;
            
        self.driver.update(&self.block[..len as usize])
            .map_err(|e| RequestError::Runtime(map_error(e)))
    }

    fn finalize(
        &mut self,
        _: &RecvMessage,
        algorithm: u32,
    ) -> Result<[u8; MAX_DIGEST_SZ], RequestError<HmacHashError>> {
        let algo = algorithm_from_u32(algorithm)?;
        let result = self.driver.finalize(algo)
            .map_err(|e| RequestError::Runtime(map_error(e)))?;
            
        let mut digest = [0u8; MAX_DIGEST_SZ];
        digest[..result.len].copy_from_slice(result.as_slice());
        Ok(digest)
    }

    fn finalize_sha256(
        &mut self,
        _: &RecvMessage,
    ) -> Result<[u8; SHA256_SZ], RequestError<HmacHashError>> {
        self.driver.finalize_sha256()
            .map_err(|e| RequestError::Runtime(map_error(e)))
    }

    fn hash(
        &mut self,
        _: &RecvMessage,
        algorithm: u32,
        data_len: u32,
        data: LenLimit<Leased<R, [u8]>, 512>,
    ) -> Result<[u8; MAX_DIGEST_SZ], RequestError<HmacHashError>> {
        if data_len == 0 || data.len() < data_len as usize {
            return Err(HmacHashError::NoData.into());
        }

        let algo = algorithm_from_u32(algorithm)?;
        
        data.read_range(0..data_len as usize, &mut self.block[..data_len as usize])
            .map_err(|_| RequestError::Fail(ClientError::WentAway))?;
            
        let result = self.driver.hash(algo, &self.block[..data_len as usize])
            .map_err(|e| RequestError::Runtime(map_error(e)))?;
            
        let mut digest = [0u8; MAX_DIGEST_SZ];
        digest[..result.len].copy_from_slice(result.as_slice());
        Ok(digest)
    }

    fn hmac(
        &mut self,
        _: &RecvMessage,
        algorithm: u32,
        data_len: u32,
        key_len: u32,
        data: LenLimit<Leased<R, [u8]>, 512>,
        key: LenLimit<Leased<R, [u8]>, 64>,
    ) -> Result<[u8; MAX_DIGEST_SZ], RequestError<HmacHashError>> {
        if data_len == 0 || data.len() < data_len as usize {
            return Err(HmacHashError::NoData.into());
        }
        if key_len == 0 || key.len() < key_len as usize {
            return Err(HmacHashError::InvalidKeySize.into());
        }

        let algo = algorithm_from_u32(algorithm)?;
        
        data.read_range(0..data_len as usize, &mut self.block[..data_len as usize])
            .map_err(|_| RequestError::Fail(ClientError::WentAway))?;
            
        let mut key_buf = [0u8; 64];
        key.read_range(0..key_len as usize, &mut key_buf[..key_len as usize])
            .map_err(|_| RequestError::Fail(ClientError::WentAway))?;
            
        let result = self.driver.hmac(algo, &key_buf[..key_len as usize], &self.block[..data_len as usize])
            .map_err(|e| RequestError::Runtime(map_error(e)))?;
            
        let mut digest = [0u8; MAX_DIGEST_SZ];
        digest[..result.len].copy_from_slice(result.as_slice());
        Ok(digest)
    }

    fn digest_sha256(
        &mut self,
        _: &RecvMessage,
        len: u32,
        data: LenLimit<Leased<R, [u8]>, 512>,
    ) -> Result<[u8; SHA256_SZ], RequestError<HmacHashError>> {
        if len == 0 || data.len() < len as usize {
            return Err(HmacHashError::NoData.into());
        }

        data.read_range(0..len as usize, &mut self.block[..len as usize])
            .map_err(|_| RequestError::Fail(ClientError::WentAway))?;
            
        self.driver.hash_sha256(&self.block[..len as usize])
            .map_err(|e| RequestError::Runtime(map_error(e)))
    }

    fn digest_hmac_sha256(
        &mut self,
        _: &RecvMessage,
        data_len: u32,
        key_len: u32,
        data: LenLimit<Leased<R, [u8]>, 512>,
        key: LenLimit<Leased<R, [u8]>, 64>,
    ) -> Result<[u8; SHA256_SZ], RequestError<HmacHashError>> {
        if data_len == 0 || data.len() < data_len as usize {
            return Err(HmacHashError::NoData.into());
        }
        if key_len == 0 || key.len() < key_len as usize {
            return Err(HmacHashError::InvalidKeySize.into());
        }

        data.read_range(0..data_len as usize, &mut self.block[..data_len as usize])
            .map_err(|_| RequestError::Fail(ClientError::WentAway))?;
            
        let mut key_buf = [0u8; 64];
        key.read_range(0..key_len as usize, &mut key_buf[..key_len as usize])
            .map_err(|_| RequestError::Fail(ClientError::WentAway))?;
            
        self.driver.hmac_sha256(&key_buf[..key_len as usize], &self.block[..data_len as usize])
            .map_err(|e| RequestError::Runtime(map_error(e)))
    }

    fn get_digest_size(
        &mut self,
        _: &RecvMessage,
        algorithm: u32,
    ) -> Result<u32, RequestError<HmacHashError>> {
        let algo = algorithm_from_u32(algorithm)?;
        Ok(algo.digest_size() as u32)
    }
}

impl NotificationHandler for ServerImpl {
    fn current_notification_mask(&self) -> u32 {
        // We don't use notifications
        0
    }

    fn handle_notification(&mut self, _bits: u32) {
        unreachable!()
    }
}

// Helper functions
fn algorithm_from_u32(algo: u32) -> Result<Algorithm, RequestError<HmacHashError>> {
    match algo {
        ALGO_SHA1 => Ok(Algorithm::Sha1),
        ALGO_SHA256 => Ok(Algorithm::Sha256),
        ALGO_SHA384 => Ok(Algorithm::Sha384),
        ALGO_SHA512 => Ok(Algorithm::Sha512),
        _ => Err(HmacHashError::InvalidAlgorithm.into()),
    }
}

fn map_error(e: HmacHashError) -> HmacHashError {
    // Since we're using the same error type, no mapping needed
    e
}

mod idl {
    use drv_hmac_hash_api::HmacHashError;

    include!(concat!(env!("OUT_DIR"), "/server_stub.rs"));
}

include!(concat!(env!("OUT_DIR"), "/notifications.rs"));
