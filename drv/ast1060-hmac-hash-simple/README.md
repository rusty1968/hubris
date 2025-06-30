# Simple AST1060 HMAC+Hash Driver Library

This crate provides a simple, direct API for HMAC and hash operations on the AST1060 
using the hardware HACE controller via the aspeed-ddk crate.

## Features

- Direct library interface (no IPC/IDL overhead)
- Support for SHA256 and HMAC-SHA256
- Built on aspeed-ddk's HaceController
- Hubris-compatible error handling

## Usage

```rust
use drv_ast1060_hmac_hash_simple::{HmacHashDriver, HashAlgorithm};

// Initialize the driver
let mut driver = HmacHashDriver::new().unwrap();

// Hash some data
let data = b"hello world";
let digest = driver.hash_sha256(data).unwrap();

// Or use HMAC
let key = b"secret_key";
let hmac = driver.hmac_sha256(key, data).unwrap();
```
