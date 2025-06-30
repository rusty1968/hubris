# AST1060 Demo App - Complete Implementation Summary

This document summarizes the complete AST1060 demo application implementation for Hubris, including all components and configurations.

## Overview

The AST1060 demo showcases:
- Complete HMAC+Hash driver and server implementation using aspeed-ddk
- Hardware-accelerated cryptographic operations
- Multi-task Hubris application architecture
- Proper memory layout configuration for AST1060
- Integration patterns for aspeed-ddk in Hubris

## Components Implemented

### 1. Core HMAC+Hash Implementation

#### `/drv/hmac-hash-api/` - Common API Crate
- **Purpose**: Shared types, constants, and error definitions
- **Key Features**:
  - Algorithm constants (SHA1, SHA256, SHA384, SHA512)
  - Digest size constants (SHA256_SZ = 32, etc.)
  - `HmacHashError` enum for unified error handling
  - Maximum digest size constant for variable-length operations

#### `/drv/ast1060-hmac-hash-simple/` - Hardware Driver
- **Purpose**: Direct aspeed-ddk HACE controller interface
- **Key Features**:
  - Safe peripheral access using `Peripherals::steal()`
  - Hardware-accelerated SHA and HMAC operations
  - Both one-shot and incremental (streaming) operations
  - Proper context management and buffer handling
  - Static lifetime management for embedded use

#### `/drv/ast1060-hmac-hash-server/` - Hubris Server
- **Purpose**: IPC server exposing HMAC+Hash functionality
- **Key Features**:
  - IDL-based API for inter-task communication
  - Error mapping from driver to IPC layer
  - Lease-based memory management for data transfer
  - Support for both streaming and one-shot operations

### 2. Demo Application

#### `/app/demo-ast1060/` - Main Application
- **Structure**:
  - `app.toml` - Task configuration and dependencies
  - `Cargo.toml` - Build dependencies
  - `src/main.rs` - Kernel entry point
  - `README.md` - Usage documentation

#### `/task/demo-ast1060/` - Demo Task
- **Purpose**: Demonstrates AST1060 capabilities
- **Features**:
  - Timer-based periodic operations
  - Basic functionality testing
  - Task communication examples
  - Performance counters and monitoring

### 3. Hardware Configuration

#### `/chips/ast1060/chip.toml` - SoC Configuration
- **Memory Layout**:
  - Main RAM: `0x0002_0000` - `0x0003_FFFF` (128K)
  - Non-cacheable RAM: `0x000A_0000` - `0x000B_FFFF` (128K)
- **Peripherals**:
  - HACE (Hash & Crypto Engine): `0x1e6d0000`
  - GPIO, UART, I2C, SPI controllers
  - Timers, Watchdog, System Control Unit
  - Additional peripherals (RNG, ADC, PWM, etc.)

#### `/boards/ast1060-dev.toml` - Board Configuration
- **Purpose**: Development board specific settings
- **Features**: Same as chip configuration with board-specific overrides

### 4. Integration Files

#### `/idl/hmac-hash.idol` - IPC Interface Definition
- **Operations**:
  - `get_supported_algorithms()` - Query available algorithms
  - `init_hash()`, `init_hmac()` - Initialize streaming operations
  - `update()` - Add data to streaming operations
  - `finalize()`, `finalize_sha256()` - Complete operations
  - `digest_sha256()`, `hmac_sha256()` - One-shot operations

## Key Implementation Patterns

### 1. Safe Peripheral Access

```rust
// Pattern used throughout the implementation
let peripherals = unsafe { Peripherals::steal() };
let hace_reg: &'static ast1060_pac::Hace = unsafe {
    &*(&peripherals.hace as *const ast1060_pac::Hace)
};
let hace = HaceController::new(hace_reg);
```

### 2. Borrow Checker Management

```rust
// Separate scopes for mutable borrows
{
    let ctx = self.hace.ctx_mut();
    ctx.configure_something();
}
// Now controller methods can be called again
self.hace.start_operation();
```

### 3. Error Handling Chain

```
Hardware Error → Driver Error → Server Error → Client Error
     ↓              ↓              ↓              ↓
 aspeed-ddk  → HmacHashError → RequestError → ClientError
```

### 4. Memory Management

```rust
// Efficient data transfer using leases
data.read_range(0..len as usize, &mut self.block[..len as usize])
    .map_err(|_| RequestError::Fail(ClientError::WentAway))?;
```

## Build and Usage

### Building the Demo

```bash
# Build all HMAC+Hash components
cargo check -p drv-hmac-hash-api -p drv-ast1060-hmac-hash-simple -p drv-ast1060-hmac-hash-server --target thumbv7m-none-eabi

# Build the demo task
cargo check -p task-demo-ast1060 --target thumbv7m-none-eabi

# Build the complete demo app (when xtask is configured)
cargo xtask build --bin demo-ast1060 --image a
```

### Expected Behavior

1. **System Startup**: Tasks initialize in priority order
2. **HMAC+Hash Server**: Starts and initializes HACE controller
3. **Demo Task**: Begins periodic testing and demonstrations
4. **Timer Operations**: 10ms timer drives periodic activities
5. **Task Communication**: Demo task communicates with HMAC+Hash server

## Development Notes

### Performance Characteristics

- **Hardware Acceleration**: HACE controller provides significant speedup over software
- **Memory Efficiency**: Zero-copy operations where possible using leases
- **Task Overhead**: Minimal IPC overhead with efficient IDL implementation

### Testing Strategy

1. **Unit Tests**: Individual component testing
2. **Integration Tests**: Full driver + server testing
3. **Hardware-in-the-Loop**: Real AST1060 testing
4. **Performance Benchmarks**: Hardware vs software comparison

### Extension Points

1. **Additional Algorithms**: Extend to support more hash algorithms
2. **DMA Integration**: Add DMA support for large data transfers
3. **Power Management**: Add low-power mode support
4. **Security Features**: Add secure key storage integration

## File Structure Summary

```
hubris/
├── app/demo-ast1060/           # Demo application
├── task/demo-ast1060/          # Demo task implementation
├── drv/hmac-hash-api/          # Common API definitions
├── drv/ast1060-hmac-hash-simple/  # Hardware driver
├── drv/ast1060-hmac-hash-server/  # IPC server
├── idl/hmac-hash.idol          # IPC interface definition
├── chips/ast1060/              # SoC configuration
│   ├── chip.toml               # Peripheral definitions
│   └── pac/                    # Peripheral access crate
└── boards/ast1060-dev.toml     # Board configuration
```

## Integration with aspeed-ddk

The implementation demonstrates proper integration patterns:

1. **Controller Usage**: Correct aspeed-ddk API usage
2. **Context Management**: Proper handling of controller contexts
3. **Error Handling**: Appropriate error mapping and propagation
4. **Lifetime Management**: Safe static lifetime handling
5. **Buffer Management**: Efficient data buffer handling

This complete implementation serves as a reference for integrating other aspeed-ddk controllers into Hubris applications.
