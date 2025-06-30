//! Basic test for the AST1060 HMAC+Hash driver
//! 
//! This is a compile-time test to verify that our API is correct.

#![no_std]
#![no_main]

use drv_ast1060_hmac_hash::HmacHash;
use drv_hmac_hash_api::{HashAlgorithm, DigestResult};
use ast1060_pac::Hace;
use panic_halt as _;

// Mock implementation to test compilation
fn test_api_usage() -> Result<DigestResult, drv_hmac_hash_api::HmacHashError> {
    // This is pseudo-code to verify the API compiles correctly
    // In a real system, hace_peripheral would come from the HAL
    let hace_peripheral: &Hace = unsafe { &*(0x1e6d0000 as *const Hace) };
    let mut driver = HmacHash::new(hace_peripheral, 42);
    
    // Test hash operation
    let data = b"hello world";
    let result = driver.digest(HashAlgorithm::Sha256, data)?;
    
    // Test HMAC operation
    let key = b"secret_key";
    let hmac_result = driver.digest_hmac(HashAlgorithm::Sha256, data, key)?;
    
    Ok(hmac_result)
}

#[no_mangle]
pub extern "C" fn main() -> ! {
    // This function will never be called, it's just for compilation testing
    loop {}
}
