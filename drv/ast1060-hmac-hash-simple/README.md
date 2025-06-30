# AST1060 HMAC+Hash Driver Library

This crate provides a simple, direct API for HMAC and hash operations on the AST1060 
using the hardware HACE controller via the aspeed-ddk crate.

## Features

- Direct library interface (no IPC/IDL overhead)
- Support for multiple hash algorithms: SHA1, SHA256, SHA384, SHA512
- HMAC support for all algorithms
- Both one-shot and incremental operations
- Built on aspeed-ddk's HaceController
- Hubris-compatible error handling
- Uses AST1060 PAC for proper peripheral access

## Usage

```rust
use drv_ast1060_hmac_hash_simple::{HmacHashDriver, Algorithm};

// Initialize the driver
let mut driver = HmacHashDriver::new().unwrap();

// One-shot operations with algorithm selection
let data = b"hello world";
let sha256_digest = driver.hash(Algorithm::Sha256, data).unwrap();
let sha512_digest = driver.hash(Algorithm::Sha512, data).unwrap();

// HMAC operations
let key = b"secret_key";
let hmac_sha256 = driver.hmac(Algorithm::Sha256, key, data).unwrap();
let hmac_sha512 = driver.hmac(Algorithm::Sha512, key, data).unwrap();

// Convenience methods for SHA256
let digest = driver.hash_sha256(data).unwrap();
let hmac = driver.hmac_sha256(key, data).unwrap();

// Incremental operations
driver.init_hash(Algorithm::Sha384).unwrap();
driver.update(b"part1").unwrap();
driver.update(b"part2").unwrap();
let final_digest = driver.finalize(Algorithm::Sha384).unwrap();

// Incremental HMAC
driver.init_hmac(Algorithm::Sha256, key).unwrap();
driver.update(b"data_chunk1").unwrap();
driver.update(b"data_chunk2").unwrap();
let final_hmac = driver.finalize(Algorithm::Sha256).unwrap();
```

## Algorithms Supported

- **SHA1**: 160-bit (20 bytes) digest
- **SHA256**: 256-bit (32 bytes) digest  
- **SHA384**: 384-bit (48 bytes) digest
- **SHA512**: 512-bit (64 bytes) digest

All algorithms support both hashing and HMAC operations.
