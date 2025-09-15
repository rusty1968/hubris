# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

This is **Hubris**, a microcontroller operating environment designed for deeply-embedded systems with reliability requirements. It's developed by Oxide Computer Company primarily for their server firmware, but is designed to be a general-purpose embedded OS.

## Build System and Commands

Hubris uses a custom build system called `xtask`. All builds are cross-compiled for ARM targets.

### Core Commands

```bash
# Build a complete system image for a specific board/application
cargo xtask dist app/gimletlet/app.toml

# Build individual tasks without building the entire image
cargo xtask build app/gimletlet/app.toml <task-name>

# Run clippy on specific tasks
cargo xtask clippy app/gimletlet/app.toml <task-name>

# Configure LSP/rust-analyzer for a specific Rust file
cargo xtask lsp path/to/file.rs

# Generate task dependency graphs
cargo xtask graph app/gimletlet/app.toml

# Flash firmware to hardware
cargo xtask flash app/gimletlet/app.toml

# Print memory sizes and analysis
cargo xtask sizes app/gimletlet/app.toml
```

### Important Build Notes

- **No standard `cargo build`**: This won't work due to cross-compilation requirements
- **Target-specific**: Most operations require specifying an app.toml configuration file
- **ARM cross-compilation**: Builds are for ARM Cortex-M targets, not host systems
- **Separate compilation**: Each task compiles independently and gets combined into a final image

## Architecture Overview

### High-Level Design Principles

1. **Microkernel approach**: Minimal kernel with functionality in separate tasks
2. **Memory safety**: Heavy use of Rust's safety features and minimal unsafe code
3. **Static configuration**: System shape determined at compile time
4. **Separate compilation**: Tasks compiled independently, then linked together
5. **IPC via Idol**: Interface Definition Language for type-safe inter-task communication

### Directory Structure

- `app/` - Top-level firmware applications for specific boards (e.g., gimletlet, oxide-rot-1)
- `sys/kern/` - The Hubris microkernel 
- `sys/userlib/` - User-space library for tasks
- `sys/abi/` - System ABI definitions
- `task/` - Reusable tasks (not hardware drivers)
- `drv/` - Hardware drivers and driver servers
- `lib/` - General utility libraries
- `build/xtask/` - Custom build system implementation
- `idl/` - Interface definitions in Idol format

### Task Architecture

Hubris applications are composed of **tasks** that communicate via **IPC**:

- **Tasks**: Independent units of execution with separate memory spaces
- **IPC**: Type-safe message passing defined by Idol interfaces
- **Notifications**: Lightweight signaling mechanism
- **Memory regions**: Tasks can have access to specific memory regions
- **Interrupts**: Hardware interrupts routed to appropriate tasks

### Key Components

#### Kernel (`sys/kern/`)
- Microkernel providing memory protection, scheduling, and IPC
- Runs in privileged mode
- Minimal functionality - most services in userspace tasks

#### Drivers (`drv/`)
- Hardware abstraction layers for specific SoCs/peripherals
- Often structured as `drv/SYSTEM-DEVICE` for drivers and `drv/SYSTEM-DEVICE-server` for servers
- **I2C example**: `drv/stm32xx-i2c` (driver) + `drv/stm32xx-i2c-server` (server task)

#### Applications (`app/`)
- Board-specific firmware configurations
- Each contains an `app.toml` describing task layout, memory regions, and features
- Examples: `gimletlet`, `oxide-rot-1`, `medusa`

## I2C Subsystem Specifics

This repository has undergone recent refactoring of I2C multiplexer drivers:

### I2C Architecture
- `drv/i2c-api/` - Common I2C client API
- `drv/i2c-types/` - Shared type definitions  
- `drv/stm32xx-i2c/` - STM32 I2C controller driver
- `drv/stm32xx-i2c-server/` - I2C server task
- `drv/i2c-mux-core/` - **NEW**: Generic I2C mux implementations
- `drv/i2c-devices/` - Higher-level device drivers

### Recent Refactoring
Generic I2C mux drivers have been factored out:
- **Before**: Platform-specific mux drivers embedded in `drv/stm32xx-i2c/`
- **After**: Generic implementations in `drv/i2c-mux-core/` with platform adapters

This enables:
- Cross-platform reuse (STM32, ESP32, Linux, etc.)
- Single implementation per mux device type
- Better testability via generic traits

## Development Workflow

### Setting Up for Development

1. **LSP Configuration**: Use `cargo xtask lsp <file.rs>` to get rust-analyzer configuration
2. **Choose Target**: Pick an appropriate app.toml (e.g., `app/gimletlet/app.toml`)
3. **Incremental Development**: Use `cargo xtask build` for individual tasks during development

### Common Patterns

#### Adding New Tasks
1. Create task in `task/` directory
2. Add to `app/*/app.toml` configuration
3. Define any needed Idol interfaces in `idl/`
4. Add necessary memory regions and permissions

#### Creating Hardware Drivers
1. Driver library in `drv/SYSTEM-DEVICE/` 
2. Server task in `drv/SYSTEM-DEVICE-server/`
3. API definitions in `drv/DEVICE-api/` if reusable
4. Update app.toml to include the driver server task

#### IPC Between Tasks
1. Define interface in `idl/` using Idol syntax
2. Generate client/server code via build process
3. Configure caller permissions in app.toml
4. Use generated client API in calling tasks

## Key Constraints

- **No std library**: Embedded no_std environment
- **Static allocation**: No dynamic memory allocation
- **Cross-compilation only**: Cannot build for host targets
- **Task isolation**: Tasks cannot directly share memory (IPC only)
- **Compile-time configuration**: System layout fixed at build time

## Debugging and Diagnostics

- **Humility**: Primary debugger tool (`cargo install humility-bin`)
- **Kernel dumps**: Built-in crash dump functionality
- **Task introspection**: Runtime task state examination
- **Memory analysis**: `cargo xtask sizes` for memory usage

## Target Hardware

Primary targets are ARM Cortex-M microcontrollers, specifically:
- STM32H7 series (gimletlet, etc.)
- STM32G0 series  
- LPC55 series
- Custom Oxide hardware (gimlet, sidecar, etc.)

The system is architected to be portable but currently optimized for these ARM targets.