// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! AST1060 HMAC+Hash server.
//!
//! This server is responsible for managing access to the HACE (Hash and Crypto Engine).

#![no_std]
#![no_main]

use userlib::*;
use drv_ast1060_hmac_hash::HmacHash;
use idol_runtime::{
    ClientError, Leased, LenLimit, RequestError, R,
};
use drv_hmac_hash_api::{HmacHashError, HashAlgorithm, DigestResult};

fn hace_hw_reset() {
    // Reset the HACE block - this is now handled by the HaceController
    // No need for explicit hardware reset here
}

struct ServerImpl {
    hash: HmacHash,
    block: [u8; 512], // Buffer for incoming data
}

impl idl::InOrderHmacHashImpl for ServerImpl {
    fn init_hash(
        &mut self,
        _: &RecvMessage,
        algo: HashAlgorithm,
    ) -> Result<(), RequestError<HmacHashError>> {
        self.hash.init_hash(algo).map_err(|e| e.into())
    }

    fn init_hmac(
        &mut self,
        _: &RecvMessage,
        algo: HashAlgorithm,
        key_len: u32,
        key: LenLimit<Leased<R, [u8]>, 64>,
    ) -> Result<(), RequestError<HmacHashError>> {
        let key_len = key_len as usize;
        let key_lease = key.into_inner();
        
        if key_len > key_lease.len() {
            return Err(HmacHashError::InvalidKeySize.into());
        }

        // Read the key from the lease
        let mut key_buf = [0u8; 64];
        key_lease.read_range(0..key_len, &mut key_buf[..key_len])
            .map_err(|_| RequestError::Fail(ClientError::WentAway))?;

        self.hash.init_hmac(algo, &key_buf[..key_len]).map_err(|e| e.into())
    }

    fn update(
        &mut self,
        _: &RecvMessage,
        len: u32,
        data: LenLimit<Leased<R, [u8]>, 512>,
    ) -> Result<(), RequestError<HmacHashError>> {
        let len = len as usize;
        let data_lease = data.into_inner();
        
        if len > data_lease.len() {
            return Err(HmacHashError::NoData.into());
        }

        // Read data from the lease
        data_lease.read_range(0..len, &mut self.block[..len])
            .map_err(|_| RequestError::Fail(ClientError::WentAway))?;

        self.hash.update(&self.block[..len]).map_err(|e| e.into())
    }

    fn finalize(
        &mut self,
        _: &RecvMessage,
    ) -> Result<DigestResult, RequestError<HmacHashError>> {
        self.hash.finalize().map_err(|e| e.into())
    }

    fn digest(
        &mut self,
        _: &RecvMessage,
        algo: HashAlgorithm,
        len: u32,
        data: LenLimit<Leased<R, [u8]>, 512>,
    ) -> Result<DigestResult, RequestError<HmacHashError>> {
        let len = len as usize;
        let data_lease = data.into_inner();
        
        if len > data_lease.len() {
            return Err(HmacHashError::NoData.into());
        }

        // Read data from the lease
        data_lease.read_range(0..len, &mut self.block[..len])
            .map_err(|_| RequestError::Fail(ClientError::WentAway))?;

        self.hash.digest(algo, &self.block[..len]).map_err(|e| e.into())
    }

    fn digest_hmac(
        &mut self,
        _: &RecvMessage,
        algo: HashAlgorithm,
        data_len: u32,
        key_len: u32,
        data: LenLimit<Leased<R, [u8]>, 512>,
        key: LenLimit<Leased<R, [u8]>, 64>,
    ) -> Result<DigestResult, RequestError<HmacHashError>> {
        let data_len = data_len as usize;
        let key_len = key_len as usize;
        let data_lease = data.into_inner();
        let key_lease = key.into_inner();
        
        if data_len > data_lease.len() || key_len > key_lease.len() {
            return Err(HmacHashError::NoData.into());
        }

        // Read data and key from leases
        data_lease.read_range(0..data_len, &mut self.block[..data_len])
            .map_err(|_| RequestError::Fail(ClientError::WentAway))?;

        let mut key_buf = [0u8; 64];
        key_lease.read_range(0..key_len, &mut key_buf[..key_len])
            .map_err(|_| RequestError::Fail(ClientError::WentAway))?;

        self.hash.digest_hmac(algo, &self.block[..data_len], &key_buf[..key_len])
            .map_err(|e| e.into())
    }
}

#[export_name = "main"]
fn main() -> ! {
    hace_hw_reset();

    // Initialize the HACE driver using Peripheral::take()
    let hash = HmacHash::new()
        .expect("Failed to initialize HACE controller");

    let mut buffer = [0; idl::INCOMING_SIZE];
    let mut server = ServerImpl {
        hash,
        block: [0; 512],
    };

    loop {
        idol_runtime::dispatch(&mut buffer, &mut server);
    }
}

mod idl {
    use super::*;

    include!(concat!(env!("OUT_DIR"), "/server_stub.rs"));
}
