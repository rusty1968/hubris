# AST1060 I2C Driver Implementation TODO

## Overview

The AST1060 I2C driver skeleton has been successfully created and compiles without errors. The basic structure is in place, but the actual ASPEED DDK integration needs to be implemented to make it functional.

## Completed Work

✅ **Build Issues Fixed**
- Fixed missing imports (`LeaseAttributes`, `FromPrimitive`, `I2cHardware` trait)
- Fixed API compatibility issues (FixedMap, Option handling, caller assignment)
- Removed references to non-existent Op variants (`ConfigureController`, `ResetBus`)
- Fixed lease read/write operations to handle Option returns
- Cleaned up unused imports and control flow issues

✅ **Driver Structure**
- Created `Ast1060I2cDriver` struct implementing `I2cHardware` trait
- Added controller mapping system for all 14 AST1060 I2C controllers
- Implemented trace logging framework
- Created server task with mux management and message handling

✅ **Interrupt Configuration**
- Fixed AST1060 chip interrupt definitions for I2C controllers
- Added proper interrupt mapping using dotted notation (`i2c0.irq`, etc.)
- Configured notifications for all 14 I2C interrupt handlers
- Resolved interrupt handling in the build system

✅ **Code Quality Improvements**
- Replaced manual conversion functions with idiomatic wrapper functions
- Added comprehensive rustdoc documentation for controller mapping
- Cleaned up unnecessary build script code (removed unused register generation)
- Implemented proper speed/error conversion patterns

## Pending Implementation Tasks

### Core I2C Operations

1. **Implement actual ASPEED DDK I2C transaction in write_read method**
   - Replace placeholder `Err(ResponseCode::BusError)` with real DDK calls
   - Handle write-then-read transactions properly
   - Add proper error handling and timeout management

2. **Implement ASPEED DDK timing configuration in configure_timing method**
   - Use DDK APIs to set I2C clock speeds
   - Support Standard (100kHz), Fast (400kHz), and Fast+ (1MHz) modes
   - Configure proper timing parameters for AST1060 hardware

3. **Implement ASPEED DDK bus reset in reset_bus method**
   - Use DDK bus recovery mechanisms
   - Handle stuck bus conditions
   - Reset controller state properly

4. **Implement ASPEED DDK controller enable/disable methods**
   - Enable/disable I2C controllers through DDK
   - Manage power states and clock gating
   - Handle controller initialization and cleanup

### Support Functions

5. **Implement ASPEED DDK slave mode configuration**
   - Configure slave addresses for MCTP support
   - Enable/disable slave receive functionality
   - Implement slave message polling and buffer management

6. **Implement proper speed conversion from I2cSpeed to AspeedSpeed**
   - Map Hubris I2cSpeed enum values to ASPEED DDK equivalents
   - Handle unsupported speed requests gracefully
   - Add validation for speed capabilities

7. **Implement proper error conversion from AspeedError to ResponseCode**
   - Map ASPEED DDK error codes to Hubris ResponseCode values
   - Provide meaningful error reporting
   - Handle DDK-specific error conditions

8. **Add proper error handling and trace logging throughout driver**
   - Use trace variants for debugging (ControllerWrite, ControllerRead, etc.)
   - Add error trace entries for failures
   - Implement comprehensive logging for troubleshooting

### Infrastructure

~~9. **Fix AST1060 chip interrupt definitions for I2C controllers** ✅ COMPLETED~~
   - ~~Add I2C interrupt definitions to AST1060 chip configuration~~ ✅
   - ~~Map interrupt names (i2c0-irq through i2c13-irq) to hardware IRQs~~ ✅
   - ~~Ensure proper interrupt handling in the build system~~ ✅

## Current Status

The driver framework compiles successfully and integrates with the Hubris I2C subsystem. The main missing piece is the actual ASPEED DDK integration, which requires:

- Understanding the ASPEED DDK API for AST1060 I2C controllers
- Implementing hardware-specific register access and timing
- Adding proper interrupt handling and error management

## Files Modified

- `drv/ast1060-i2c/src/lib.rs` - Core driver implementation
- `drv/ast1060-i2c-server/src/main.rs` - Server task with message handling
- `app/ast1060-starter/app.toml` - Application configuration

## Next Steps

The highest priority items are:
1. Core I2C transaction implementation (write_read)
2. Basic controller enable/disable functionality
3. Error and speed conversion utilities
4. Interrupt definitions for proper system integration

Once these are complete, the driver will be functional for basic I2C operations, with slave mode support following for MCTP protocols.