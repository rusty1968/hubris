# AST1060 DDK Integration Guide for Hubris

This guide explains how to integrate and use the `aspeed-ddk` (Device Driver Kit) within the Hubris operating system, specifically for the AST1060 platform.

## Overview

The `aspeed-ddk` provides hardware abstraction layers for ASPEED SoCs, including the AST1060. This guide covers:

- Setting up dependencies
- Peripheral access patterns
- Hardware controller integration
- Best practices for Hubris drivers
- Example implementation (HMAC+Hash driver)

## Prerequisites

- Hubris development environment
- AST1060 target board configuration
- Basic understanding of embedded Rust and Hubris architecture

## Setting Up Dependencies

### 1. Add aspeed-ddk to Cargo.toml

For a driver using the aspeed-ddk, add it to your `Cargo.toml`:

```toml
[dependencies]
aspeed-ddk = { git = "https://github.com/oxidecomputer/aspeed-ddk", rev = "main" }
ast1060-pac = { path = "../../chips/ast1060" }
```

### 2. Feature Flags

The aspeed-ddk may have feature flags for different SoC variants:

```toml
[dependencies]
aspeed-ddk = { 
    git = "https://github.com/oxidecomputer/aspeed-ddk", 
    rev = "main",
    features = ["ast1060"] 
}
```

## Peripheral Access Patterns

### Safe Peripheral Access in Hubris

Hubris drivers need to safely access hardware peripherals. The recommended pattern is:

```rust
use ast1060_pac::Peripherals;
use aspeed_ddk::hace_controller::HaceController;

pub struct MyDriver {
    controller: HaceController<'static>,
}

impl MyDriver {
    pub fn new() -> Result<Self, MyError> {
        // Take the AST1060 peripherals using steal
        // This works in embedded where there's only one instance
        let peripherals = unsafe { Peripherals::steal() };
        
        // Get a static reference - this is safe because we know the peripheral
        // registers are at a fixed memory location
        let peripheral_reg: &'static ast1060_pac::PeripheralName = unsafe {
            &*(&peripherals.peripheral_name as *const ast1060_pac::PeripheralName)
        };
        
        let controller = HaceController::new(peripheral_reg);
        
        Ok(Self { controller })
    }
}
```

### Key Points

1. **Use `Peripherals::steal()`**: In embedded contexts, this is safe when you know there's only one instance
2. **Static references**: Convert peripheral references to `'static` lifetime for use in drivers
3. **Unsafe blocks**: Properly document why unsafe code is safe in your context

## Hardware Controller Integration

### Controller Initialization

Most aspeed-ddk controllers follow this pattern:

```rust
use aspeed_ddk::controller_name::ControllerName;

// Initialize with a reference to the peripheral registers
let controller = ControllerName::new(peripheral_registers);

// Configure the controller
controller.configure()?;

// Use controller methods
controller.perform_operation(data)?;
```

### Context Management

Some controllers use context structures for stateful operations:

```rust
// Get mutable context for configuration
let ctx = controller.ctx_mut();
ctx.field = value;
ctx.buffer[..data.len()].copy_from_slice(data);

// Use the controller with the configured context
controller.start_operation();
```

### Error Handling

Map aspeed-ddk errors to your driver's error types:

```rust
use aspeed_ddk::controller_name::ControllerError;

#[derive(Debug, Clone, Copy)]
pub enum MyDriverError {
    HardwareError,
    InvalidInput,
    Timeout,
}

impl From<ControllerError> for MyDriverError {
    fn from(err: ControllerError) -> Self {
        match err {
            ControllerError::Timeout => MyDriverError::Timeout,
            ControllerError::InvalidConfig => MyDriverError::InvalidInput,
            _ => MyDriverError::HardwareError,
        }
    }
}
```

## Example: HMAC+Hash Driver Implementation

Here's a complete example using the HACE (Hash and Crypto Engine) controller:

### Driver Structure

```rust
use aspeed_ddk::hace_controller::{HaceController, HashAlgo};
use ast1060_pac::Peripherals;
use drv_hmac_hash_api::HmacHashError;

pub struct HmacHashDriver {
    hace: HaceController<'static>,
    initialized: bool,
}

impl HmacHashDriver {
    pub fn new() -> Result<Self, HmacHashError> {
        let peripherals = unsafe { Peripherals::steal() };
        
        let hace_reg: &'static ast1060_pac::Hace = unsafe {
            &*(&peripherals.hace as *const ast1060_pac::Hace)
        };
        
        let hace = HaceController::new(hace_reg);
        
        Ok(Self { 
            hace,
            initialized: false,
        })
    }
}
```

### Implementing Operations

```rust
impl HmacHashDriver {
    pub fn hash(&mut self, algorithm: Algorithm, data: &[u8]) -> Result<DigestResult, HmacHashError> {
        // Set the algorithm
        self.hace.algo = algorithm.to_hace_algo();
        
        // Configure context in separate scope to satisfy borrow checker
        {
            let ctx = self.hace.ctx_mut();
            ctx.method = self.hace.algo.hash_cmd();
            ctx.block_size = self.hace.algo.block_size() as u32;
            
            // Copy input data
            if data.len() > ctx.buffer.len() {
                return Err(HmacHashError::InvalidDataSize);
            }
            
            ctx.buffer[..data.len()].copy_from_slice(data);
            ctx.bufcnt = data.len() as u32;
            ctx.digcnt[0] = data.len() as u64;
        }
        
        // Perform the hash operation
        self.hace.copy_iv_to_digest();
        self.hace.fill_padding(0);
        
        let bufcnt = self.hace.ctx_mut().bufcnt;
        self.hace.start_hash_operation(bufcnt);
        
        // Extract result
        let ctx = self.hace.ctx_mut();
        let mut result = DigestResult::new(algorithm.digest_size());
        result.bytes[..result.len].copy_from_slice(&ctx.digest[..result.len]);
        
        Ok(result)
    }
}
```

## Best Practices

### 1. Borrow Checker Management

When working with controllers that have mutable contexts, be careful about borrowing:

```rust
// Good: Separate scopes for mutable borrows
{
    let ctx = controller.ctx_mut();
    ctx.configure_something();
}

// Now we can use controller methods again
controller.start_operation();

// Bad: Overlapping mutable borrows
let ctx = controller.ctx_mut();
controller.start_operation(); // Error: controller already borrowed
```

### 2. Error Propagation

Create a clear error mapping hierarchy:

```rust
// Driver-specific errors
#[derive(Debug, Clone, Copy)]
pub enum DriverError {
    HardwareFailure,
    InvalidConfiguration,
    Timeout,
}

// Map from aspeed-ddk errors
impl From<aspeed_ddk::ErrorType> for DriverError {
    fn from(err: aspeed_ddk::ErrorType) -> Self {
        // Map appropriately
    }
}

// Map to Hubris API errors
impl From<DriverError> for RequestError<ApiError> {
    fn from(err: DriverError) -> Self {
        RequestError::Runtime(err.into())
    }
}
```

### 3. Resource Management

Ensure proper cleanup and resource management:

```rust
impl HmacHashDriver {
    pub fn reset(&mut self) -> Result<(), DriverError> {
        // Reset controller state
        self.controller.reset()?;
        self.initialized = false;
        Ok(())
    }
}

impl Drop for HmacHashDriver {
    fn drop(&mut self) {
        // Cleanup if needed
        let _ = self.reset();
    }
}
```

### 4. Testing Strategies

Structure your code to enable testing:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    // Mock the hardware interface for unit tests
    #[test]
    fn test_hash_operation() {
        // Test logic without hardware
    }
}

// Integration tests can use actual hardware or simulators
```

## Integration with Hubris IDL

### Server Implementation

When creating a Hubris server using your driver:

```rust
use idol_runtime::*;

struct ServerImpl {
    driver: HmacHashDriver,
    buffer: [u8; 512], // Working buffer
}

impl idl::InOrderHmacHashImpl for ServerImpl {
    fn hash_data(
        &mut self,
        _: &RecvMessage,
        algorithm: u32,
        data: LenLimit<Leased<R, [u8]>, 512>,
    ) -> Result<[u8; 32], RequestError<HmacHashError>> {
        // Convert IDL types to driver types
        let algo = Algorithm::from_u32(algorithm)?;
        
        // Read data from IPC
        data.read_range(0..data.len(), &mut self.buffer[..data.len()])
            .map_err(|_| RequestError::Fail(ClientError::WentAway))?;
        
        // Call driver
        let result = self.driver.hash(algo, &self.buffer[..data.len()])
            .map_err(|e| RequestError::Runtime(e))?;
            
        // Convert result
        let mut digest = [0u8; 32];
        digest.copy_from_slice(result.as_slice());
        Ok(digest)
    }
}
```

### Build Configuration

Add appropriate build script (`build.rs`):

```rust
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    build_util::expose_target_board();
    
    let out = build_util::out_dir();
    let mut task_config = std::fs::File::create(out.join("task_config.rs"))?;
    
    writeln!(task_config, "pub const TASK_CONFIG: &str = \"hmac-hash-server\";")?;
    
    idol::server::build_server_support(
        "../../idl/hmac-hash.idol",
        "server_stub.rs",
        idol::server::ServerStyle::InOrder,
    )?;
    
    Ok(())
}
```

## Board Configuration

Add your server to the appropriate board configuration:

```toml
# In boards/ast1060-rot.toml

[tasks.hmac_hash]
name = "drv-ast1060-hmac-hash-server"
priority = 3
max-sizes = { flash = 16384, ram = 4096 }
start = true
task-slots = ["hash_driver"]

[tasks.hmac_hash.config]
pins = []
```

## Debugging Tips

### 1. Hardware Register Access

Use the PAC to verify register states:

```rust
// Check hardware state directly
let reg_value = peripheral.some_register.read();
println!("Register value: 0x{:08x}", reg_value.bits());
```

### 2. Controller State

Most controllers provide debug methods:

```rust
// Check controller state
let state = controller.get_state();
println!("Controller state: {:?}", state);
```

### 3. Buffer Contents

Verify data buffers:

```rust
// Check buffer contents
let ctx = controller.ctx();
println!("Buffer: {:x?}", &ctx.buffer[..ctx.bufcnt as usize]);
```

## Common Pitfalls

### 1. Lifetime Management

```rust
// Avoid: Trying to store references with inadequate lifetimes
pub struct BadDriver<'a> {
    controller: HaceController<'a>, // This won't work in Hubris
}

// Good: Use 'static lifetimes for hardware
pub struct GoodDriver {
    controller: HaceController<'static>,
}
```

### 2. Borrow Checker Issues

```rust
// Avoid: Overlapping mutable borrows
let ctx = self.controller.ctx_mut();
ctx.configure();
self.controller.start(); // Error!

// Good: Separate scopes
{
    let ctx = self.controller.ctx_mut();
    ctx.configure();
}
self.controller.start(); // OK
```

### 3. Error Handling

```rust
// Avoid: Ignoring hardware errors
let _ = controller.operation(); // Don't do this

// Good: Proper error propagation
controller.operation().map_err(|e| MyError::from(e))?;
```

## Performance Considerations

### 1. Buffer Management

Reuse buffers when possible:

```rust
pub struct Driver {
    controller: Controller<'static>,
    working_buffer: [u8; 1024], // Reuse this
}
```

### 2. Hardware Optimization

Use hardware features efficiently:

```rust
// Use DMA when available
controller.configure_dma(buffer_addr, length)?;

// Use hardware acceleration features
controller.enable_acceleration()?;
```

### 3. Minimize Copies

Avoid unnecessary data copies:

```rust
// Good: Work with slices directly
fn process_data(&mut self, data: &[u8]) -> Result<(), Error> {
    self.controller.process_slice(data)
}

// Avoid: Unnecessary copying
fn process_data(&mut self, data: &[u8]) -> Result<(), Error> {
    let mut buffer = [0u8; 1024];
    buffer[..data.len()].copy_from_slice(data); // Unnecessary copy
    self.controller.process_slice(&buffer[..data.len()])
}
```

## Conclusion

The aspeed-ddk provides powerful hardware abstractions for AST1060 development in Hubris. Key points:

1. Use safe peripheral access patterns with `'static` lifetimes
2. Manage borrow checker requirements with scoped borrows
3. Implement proper error handling and propagation
4. Follow Hubris conventions for server implementation
5. Test thoroughly with both unit and integration tests

This integration approach ensures reliable, performant drivers that leverage the full capabilities of the AST1060 hardware while maintaining Hubris's safety guarantees.
