# AST1060 HMAC+Hash Driver for Hubris

This implementation provides a Hubris-compatible wrapper around the existing AST1060 HACE (Hash and Crypto Engine) controller from the [aspeed-rust](https://github.com/rusty1968/aspeed-rust) repository.

## Architecture

Instead of reimplementing the HACE hardware interface, this driver leverages the existing, well-tested implementation:

```
┌─────────────────┐    ┌──────────────────────┐    ┌─────────────────────┐
│   Client Task   │───▶│  HMAC+Hash Server    │───▶│  aspeed-rust        │
│  (task-hmac-    │    │ (drv-ast1060-hmac-   │    │  HaceController     │
│   hash)         │    │  hash-server)        │    │                     │
└─────────────────┘    └──────────────────────┘    └─────────────────────┘
                                │                            │
                                │                            │
                                ▼                            ▼
                       ┌─────────────────┐         ┌─────────────────┐
                       │  Hubris IPC     │         │  AST1060 HACE   │
                       │  API Types      │         │  Hardware       │
                       └─────────────────┘         └─────────────────┘
```

## Implementation Details

### Correct Usage of aspeed-rust

The `HaceController` requires a reference to the `Hace` peripheral:

```rust
// In the server main function
let hace_peripheral = Hace::new();
let hash = HmacHash::new(&hace_peripheral, notifications::HACE_IRQ_MASK);

// The wrapper struct
pub struct HmacHash<'a> {
    hace: HaceController<'a>,
    interrupt: u32,
}

impl<'a> HmacHash<'a> {
    pub fn new(hace_peripheral: &'a Hace, interrupt: u32) -> Self {
        Self {
            hace: HaceController::new(hace_peripheral),
            interrupt,
        }
    }
}
```

### Type Conversion Layer

The wrapper handles conversion between Hubris and aspeed-rust types:

```rust
fn convert_algorithm(&self, algorithm: HashAlgorithm) -> Result<HaceHashAlgorithm, HmacHashError> {
    match algorithm {
        HashAlgorithm::Md5 => Ok(HaceHashAlgorithm::Md5),
        HashAlgorithm::Sha1 => Ok(HaceHashAlgorithm::Sha1),
        HashAlgorithm::Sha256 => Ok(HaceHashAlgorithm::Sha256),
        HashAlgorithm::Sha384 => Ok(HaceHashAlgorithm::Sha384),
        HashAlgorithm::Sha512 => Ok(HaceHashAlgorithm::Sha512),
    }
}

fn convert_error(&self, error: HaceError) -> HmacHashError {
    match error {
        HaceError::NotInitialized => HmacHashError::NotInitialized,
        // ... other conversions
    }
}
```

## Components

### 1. `drv-hmac-hash-api`
- **Purpose**: Hubris IPC API definitions
- **Features**: 
  - Algorithm selection (MD5, SHA-1, SHA-256, SHA-384, SHA-512)
  - Flexible `DigestResult` type for variable-length outputs
  - Comprehensive error handling

### 2. `drv-ast1060-hmac-hash`  
- **Purpose**: Thin wrapper around `aspeed-rust::hace_controller::HaceController`
- **Features**:
  - Lifetime-aware wrapper with proper peripheral reference
  - Type conversion between Hubris and aspeed-rust APIs
  - Error translation
  - No hardware-specific code duplication

### 3. `drv-ast1060-hmac-hash-server`
- **Purpose**: Hubris IPC server
- **Features**:
  - Owns the `hace_controller` hardware resource
  - Handles interrupt notifications
  - Provides safe concurrent access via IPC
  - Manages peripheral lifetime correctly

### 4. `task-hmac-hash`
- **Purpose**: Example usage task
- **Features**:
  - Demonstrates SHA-256, HMAC-SHA256, MD5, and SHA-1 usage
  - Shows both one-shot and streaming APIs
  - Includes performance tracing

## Key Benefits

1. **Code Reuse**: Leverages existing, tested HACE implementation
2. **Maintainability**: No duplicate hardware-specific code
3. **Flexibility**: Support for multiple algorithms via enum selection
4. **Safety**: Hubris memory isolation and IPC boundaries
5. **Performance**: Direct hardware access with interrupt support
6. **Correct Lifetimes**: Proper handling of peripheral references

## Usage Example

```rust
use drv_hmac_hash_api::{HmacHash, HashAlgorithm};

// Get handle to HMAC+hash server
let hmac_hash = HmacHash::from(HMAC_HASH_DRIVER.get_task_id());

// One-shot SHA-256
let data = b"Hello, World!";
let result = hmac_hash.digest(HashAlgorithm::Sha256, data.len() as u32, data)?;

// One-shot HMAC-SHA256
let key = b"secret_key";
let hmac_result = hmac_hash.digest_hmac(
    HashAlgorithm::Sha256,
    data.len() as u32,
    key.len() as u32,
    data,
    key
)?;

// Streaming API
hmac_hash.init_hash(HashAlgorithm::Sha256)?;
hmac_hash.update(data)?;
let final_result = hmac_hash.finalize()?;
```

## Configuration

The driver is configured in `app.toml`:

```toml
[tasks.hmac_hash_driver]
name = "drv-ast1060-hmac-hash-server"
priority = 1
max-sizes = {flash = 16384, ram = 4096}
stacksize = 2048
start = true
notifications = ["hace-irq"]
interrupts = {"hace-irq" = "hace_controller"}
```

And the peripheral ownership in `boards/ast1060-rot.toml`:

```toml
[tasks.hmac_hash_driver]
uses = ["hace_controller"]
```

## Hardware Resource

The driver owns the `hace_controller` peripheral defined in `chips/ast1060/chip.toml`:

```toml
[peripherals.hace_controller]
address = 0x1e6d0000
size = 0x400
interrupts = ["hace-irq"]
```

This ensures exclusive access to the HACE hardware and proper interrupt routing.
