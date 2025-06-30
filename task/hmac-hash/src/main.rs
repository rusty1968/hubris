// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! HMAC+Hash task demonstrating usage of the AST1060 HACE hardware.
//!
//! This task showcases how to use the HMAC+hash driver for various cryptographic
//! operations including SHA-256, HMAC-SHA256, and other supported algorithms.

#![no_std]
#![no_main]

use drv_hmac_hash_api::{HmacHash, HashAlgorithm, HmacHashError};
use userlib::*;

task_slot!(HMAC_HASH_DRIVER, hmac_hash_driver);

#[export_name = "main"]
fn main() -> ! {
    let hmac_hash = HmacHash::from(HMAC_HASH_DRIVER.get_task_id());

    // Example usage of the HMAC+hash driver
    loop {
        // Test SHA-256 hashing
        test_sha256(&hmac_hash);
        
        // Test HMAC-SHA256
        test_hmac_sha256(&hmac_hash);
        
        // Test other algorithms if supported
        test_other_algorithms(&hmac_hash);

        // Sleep for a while before running tests again
        hl::sleep_for(1000);
    }
}

fn test_sha256(driver: &HmacHash) {
    let test_data = b"The quick brown fox jumps over the lazy dog";
    
    match driver.digest(HashAlgorithm::Sha256, test_data.len() as u32, test_data) {
        Ok(result) => {
            // Expected SHA-256: d7a8fbb307d7809469ca9abcb0082e4f8d5651e46d3cdb762d02d0bf37c9e592
            ringbuf_entry!(Trace::Sha256Success(result.length as usize));
        }
        Err(e) => {
            ringbuf_entry!(Trace::Sha256Error(e));
        }
    }
}

fn test_hmac_sha256(driver: &HmacHash) {
    let test_data = b"The quick brown fox jumps over the lazy dog";
    let test_key = b"secret_key";
    
    match driver.digest_hmac(
        HashAlgorithm::Sha256, 
        test_data.len() as u32,
        test_key.len() as u32,
        test_data,
        test_key
    ) {
        Ok(result) => {
            ringbuf_entry!(Trace::HmacSha256Success(result.length as usize));
        }
        Err(e) => {
            ringbuf_entry!(Trace::HmacSha256Error(e));
        }
    }
}

fn test_other_algorithms(driver: &HmacHash) {
    let test_data = b"Hello, World!";
    
    // Test MD5 if supported
    match driver.digest(HashAlgorithm::Md5, test_data.len() as u32, test_data) {
        Ok(result) => {
            ringbuf_entry!(Trace::Md5Success(result.length as usize));
        }
        Err(e) => {
            ringbuf_entry!(Trace::Md5Error(e));
        }
    }
    
    // Test SHA-1 if supported
    match driver.digest(HashAlgorithm::Sha1, test_data.len() as u32, test_data) {
        Ok(result) => {
            ringbuf_entry!(Trace::Sha1Success(result.length as usize));
        }
        Err(e) => {
            ringbuf_entry!(Trace::Sha1Error(e));
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
enum Trace {
    Sha256Success(usize),
    Sha256Error(HmacHashError),
    HmacSha256Success(usize),
    HmacSha256Error(HmacHashError),
    Md5Success(usize),
    Md5Error(HmacHashError),
    Sha1Success(usize),
    Sha1Error(HmacHashError),
}

ringbuf!(Trace, 16, Trace::Sha256Success(0));
