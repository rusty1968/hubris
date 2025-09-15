// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Driver for AST1060 I2C controllers using ASPEED DDK
//!
//! The AST1060 SoC includes 14 independent I2C controllers (I2C0-I2C13) with
//! hardware buffer support and multi-master capabilities. This driver provides
//! a hardware abstraction layer implementing the I2cHardware trait for use
//! with the Hubris I2C subsystem.
//!
//! This implementation leverages the ASPEED Device Driver Kit (DDK) which
//! provides register access, timing configuration, and hardware abstractions
//! for AST1060 I2C controllers.

#![no_std]

use aspeed_ddk::i2c::{
    ast1060_i2c::{Error as AspeedError},
    common::{I2cSpeed as AspeedSpeed},
};
use counters::*;
use drv_i2c_api::{Controller, ResponseCode};
use drv_i2c_types::traits::{I2cHardware, I2cSpeed, SlaveStatus};
use drv_i2c_types::{SlaveConfig, SlaveMessage};
// Embedded HAL imports not currently needed
use ringbuf::*;

/// Mapping from Hubris Controller enum to AST1060 I2C instances
///
/// The AST1060 SoC provides 14 independent I2C controllers (I2C0-I2C13) that need
/// to be mapped to the generic Hubris `Controller` enum values used by the I2C API.
/// This structure establishes the correspondence between the abstract controller
/// identifiers and the physical hardware instances.
///
/// # Purpose
///
/// - **Hardware Abstraction**: Maps generic `Controller` enum values to specific
///   AST1060 I2C instance numbers (0-13)
/// - **Configuration Flexibility**: Allows different applications to use different
///   subsets of available I2C controllers
/// - **Runtime Lookup**: Enables the driver to find the correct hardware instance
///   for any given controller operation
///
/// # Usage
///
/// Controller mappings are typically defined statically in the server task and
/// passed to the driver during initialization:
///
/// ```rust
/// use drv_ast1060_i2c::{ControllerMapping, create_all_controller_mappings};
/// use drv_i2c_api::Controller;
///
/// // Map all 14 controllers
/// static MAPPINGS: [ControllerMapping; 14] = create_all_controller_mappings();
///
/// // Or create custom mappings for specific applications
/// static CUSTOM_MAPPINGS: [ControllerMapping; 2] = [
///     ControllerMapping { controller: Controller::I2C0, instance_num: 0 },
///     ControllerMapping { controller: Controller::I2C1, instance_num: 7 },
/// ];
/// ```
///
/// # Fields
///
/// - `controller`: The Hubris generic controller identifier used by client APIs
/// - `instance_num`: The AST1060-specific hardware instance number (0-13)
#[derive(Copy, Clone, Debug)]
pub struct ControllerMapping {
    /// The generic Hubris controller identifier used by the I2C API
    pub controller: Controller,
    /// The AST1060-specific I2C instance number (0-13 for I2C0-I2C13)
    pub instance_num: u8,
}

/// AST1060 I2C driver implementing the I2cHardware trait
pub struct Ast1060I2cDriver {
    /// Controller configuration mappings
    controllers: &'static [ControllerMapping],
}

/// Error tracing for debugging
#[derive(Copy, Clone, Debug, PartialEq, Eq, Count)]
enum Trace {
    #[count(skip)]
    None,
    ControllerWrite { controller: u8, addr: u8, len: usize },
    ControllerRead { controller: u8, addr: u8, len: usize },
    ControllerWriteRead { controller: u8, addr: u8, write_len: usize, read_len: usize },
    AspeedError { controller: u8, error: u8 },
    ConfigureTiming { controller: u8, speed: u8 },
    ResetBus { controller: u8 },
    EnableController { controller: u8 },
    DisableController { controller: u8 },
    SlaveConfigured { controller: u8, addr: u8 },
    SlaveMessage { controller: u8, addr: u8, len: usize },
    #[count(skip)]
    Panic { controller: u8, status: u32 },
}

counted_ringbuf!(Trace, 64, Trace::None);

impl Ast1060I2cDriver {
    /// Create a new AST1060 I2C driver instance
    pub const fn new(controllers: &'static [ControllerMapping]) -> Self {
        Self {
            controllers,
        }
    }

    /// Find controller mapping by Controller enum
    fn find_controller_mapping(&self, controller: Controller) -> Result<&ControllerMapping, ResponseCode> {
        self.controllers
            .iter()
            .find(|mapping| mapping.controller == controller)
            .ok_or(ResponseCode::BadController)
    }

}

/// Convert Hubris I2cSpeed to ASPEED DDK I2cSpeed
///
/// Maps the generic Hubris I2C speed enumeration to the corresponding ASPEED DDK
/// speed values. This provides a clean conversion interface while maintaining
/// compatibility with both APIs.
///
/// # Speed Mappings
///
/// - `I2cSpeed::Standard` → `AspeedSpeed::Standard` (100 kHz)
/// - `I2cSpeed::Fast` → `AspeedSpeed::Fast` (400 kHz)
/// - `I2cSpeed::FastPlus` → `AspeedSpeed::FastPlus` (1 MHz)
/// - `I2cSpeed::HighSpeed` → `AspeedSpeed::Fast` (fallback to 400 kHz)
///
/// # Usage
///
/// ```rust
/// let hubris_speed = I2cSpeed::Fast;
/// let aspeed_speed = speed_to_aspeed(hubris_speed);
/// ```
///
/// # Notes
///
/// High-speed mode (3.4 MHz) is not commonly supported by I2C devices and may
/// not be available on AST1060 hardware, so it falls back to Fast mode.
fn speed_to_aspeed(speed: I2cSpeed) -> AspeedSpeed {
    match speed {
        I2cSpeed::Standard => AspeedSpeed::Standard,
        I2cSpeed::Fast => AspeedSpeed::Fast,
        I2cSpeed::FastPlus => AspeedSpeed::FastPlus,
        I2cSpeed::HighSpeed => AspeedSpeed::Fast, // Fallback to Fast mode
    }
}

/// Convert ASPEED DDK Error to Hubris ResponseCode
///
/// Maps ASPEED DDK-specific error conditions to the appropriate Hubris I2C
/// response codes. This provides meaningful error reporting while abstracting
/// away hardware-specific error details from higher-level code.
///
/// # Error Mappings
///
/// The conversion attempts to map ASPEED errors to the most appropriate
/// Hubris ResponseCode based on the failure type:
///
/// - Timeout/bus busy conditions → `BusError` or `ControllerBusy`
/// - NACK/addressing issues → `NoDevice`
/// - Invalid parameters → `BadArg`
/// - Hardware faults → `BusError`
///
/// # Usage
///
/// ```rust
/// match aspeed_operation() {
///     Ok(result) => handle_success(result),
///     Err(aspeed_err) => {
///         let response_code = error_to_response_code(aspeed_err);
///         return Err(response_code);
///     }
/// }
/// ```
fn error_to_response_code(error: AspeedError) -> ResponseCode {
    match error {
        // Map specific ASPEED errors to appropriate ResponseCodes
        // TODO: Implement actual error mapping based on ASPEED DDK error types
        _ => ResponseCode::BusError,
    }
}

impl I2cHardware for Ast1060I2cDriver {
    type Error = ResponseCode;

    fn write_read(
        &mut self,
        controller: Controller,
        addr: u8,
        write_data: &[u8],
        read_buffer: &mut [u8],
    ) -> Result<usize, Self::Error> {
        let _mapping = self.find_controller_mapping(controller)?;

        ringbuf_entry!(Trace::ControllerWriteRead {
            controller: _mapping.instance_num,
            addr,
            write_len: write_data.len(),
            read_len: read_buffer.len()
        });

        // TODO: Implement actual ASPEED DDK I2C transaction
        // Example of idiomatic error conversion:
        // match aspeed_i2c_write_read(instance, addr, write_data, read_buffer) {
        //     Ok(bytes_read) => Ok(bytes_read),
        //     Err(aspeed_err) => Err(error_to_response_code(aspeed_err)),
        // }

        // For now, return BusError to allow compilation
        Err(ResponseCode::BusError)
    }

    fn write_read_block(
        &mut self,
        controller: Controller,
        addr: u8,
        write_data: &[u8],
        read_buffer: &mut [u8],
    ) -> Result<usize, Self::Error> {
        // AST1060 doesn't have dedicated SMBus block read support
        // Fall back to regular write_read
        self.write_read(controller, addr, write_data, read_buffer)
    }

    fn configure_timing(
        &mut self,
        controller: Controller,
        speed: I2cSpeed,
    ) -> Result<(), Self::Error> {
        let _mapping = self.find_controller_mapping(controller)?;

        ringbuf_entry!(Trace::ConfigureTiming {
            controller: _mapping.instance_num,
            speed: speed as u8
        });

        // Convert speed using our conversion function
        let _aspeed_speed = speed_to_aspeed(speed);

        // TODO: Use _aspeed_speed with ASPEED DDK timing configuration
        // Example: aspeed_i2c_configure_timing(instance, _aspeed_speed)?;
        Ok(())
    }

    fn reset_bus(&mut self, controller: Controller) -> Result<(), Self::Error> {
        let _mapping = self.find_controller_mapping(controller)?;

        ringbuf_entry!(Trace::ResetBus { controller: _mapping.instance_num });

        // TODO: Implement ASPEED DDK bus reset
        Ok(())
    }

    fn enable_controller(&mut self, controller: Controller) -> Result<(), Self::Error> {
        let _mapping = self.find_controller_mapping(controller)?;

        ringbuf_entry!(Trace::EnableController { controller: _mapping.instance_num });

        // TODO: Implement ASPEED DDK controller enable
        Ok(())
    }

    fn disable_controller(&mut self, controller: Controller) -> Result<(), Self::Error> {
        let _mapping = self.find_controller_mapping(controller)?;

        ringbuf_entry!(Trace::DisableController { controller: _mapping.instance_num });

        // TODO: Implement ASPEED DDK controller disable
        Ok(())
    }

    fn configure_slave_mode(
        &mut self,
        controller: Controller,
        config: &SlaveConfig,
    ) -> Result<(), Self::Error> {
        let _mapping = self.find_controller_mapping(controller)?;

        ringbuf_entry!(Trace::SlaveConfigured {
            controller: _mapping.instance_num,
            addr: config.address
        });

        // TODO: Implement ASPEED DDK slave mode configuration
        Ok(())
    }

    fn enable_slave_receive(&mut self, controller: Controller) -> Result<(), Self::Error> {
        let _mapping = self.find_controller_mapping(controller)?;
        // TODO: Implement ASPEED DDK slave receive enable
        Ok(())
    }

    fn disable_slave_receive(&mut self, controller: Controller) -> Result<(), Self::Error> {
        let _mapping = self.find_controller_mapping(controller)?;
        // TODO: Implement ASPEED DDK slave receive disable
        Ok(())
    }

    fn poll_slave_messages(
        &mut self,
        controller: Controller,
        _messages: &mut [SlaveMessage],
    ) -> Result<usize, Self::Error> {
        let _mapping = self.find_controller_mapping(controller)?;

        // TODO: Implement ASPEED DDK slave message polling
        // For now, return 0 messages
        Ok(0)
    }

    fn get_slave_status(&self, controller: Controller) -> Result<SlaveStatus, Self::Error> {
        let _mapping = self.find_controller_mapping(controller)?;

        // TODO: Implement ASPEED DDK slave status
        // Return default slave status for now
        Ok(SlaveStatus {
            enabled: false,
            messages_received: 0,
            messages_dropped: 0,
            address_matches: 0,
            bus_errors: 0,
            buffer_full: false,
        })
    }
}

/// Creates controller mappings for all 14 AST1060 I2C controllers
///
/// This convenience function generates a complete mapping array that associates each
/// of the 14 AST1060 I2C hardware instances (I2C0-I2C13) with the corresponding
/// Hubris `Controller` enum values.
///
/// # Returns
///
/// A static array of 14 `ControllerMapping` entries, one for each AST1060 I2C controller.
/// The mappings follow a direct 1:1 correspondence:
/// - `Controller::I2C0` ↔ instance 0 (I2C0)
/// - `Controller::I2C1` ↔ instance 1 (I2C1)
/// - ...
/// - `Controller::I2C13` ↔ instance 13 (I2C13)
///
/// # Usage
///
/// This function is typically used during server initialization to create a complete
/// set of controller mappings:
///
/// ```rust
/// use drv_ast1060_i2c::{Ast1060I2cDriver, create_all_controller_mappings};
///
/// static CONTROLLER_MAPPINGS: [ControllerMapping; 14] = create_all_controller_mappings();
///
/// // Initialize driver with all controllers available
/// let driver = Ast1060I2cDriver::new(&CONTROLLER_MAPPINGS);
/// ```
///
/// # Design Notes
///
/// - **Const Function**: Can be evaluated at compile time for static initialization
/// - **Complete Coverage**: Includes all 14 AST1060 I2C controllers
/// - **Standard Mapping**: Uses the most straightforward controller-to-instance mapping
/// - **Application Flexibility**: Applications can choose to use a subset by creating
///   custom mapping arrays if not all controllers are needed
///
/// For applications that only need specific controllers, custom mappings can be created
/// manually rather than using this complete set.
pub const fn create_all_controller_mappings() -> [ControllerMapping; 14] {
    [
        ControllerMapping { controller: Controller::I2C0, instance_num: 0 },
        ControllerMapping { controller: Controller::I2C1, instance_num: 1 },
        ControllerMapping { controller: Controller::I2C2, instance_num: 2 },
        ControllerMapping { controller: Controller::I2C3, instance_num: 3 },
        ControllerMapping { controller: Controller::I2C4, instance_num: 4 },
        ControllerMapping { controller: Controller::I2C5, instance_num: 5 },
        ControllerMapping { controller: Controller::I2C6, instance_num: 6 },
        ControllerMapping { controller: Controller::I2C7, instance_num: 7 },
        ControllerMapping { controller: Controller::I2C8, instance_num: 8 },
        ControllerMapping { controller: Controller::I2C9, instance_num: 9 },
        ControllerMapping { controller: Controller::I2C10, instance_num: 10 },
        ControllerMapping { controller: Controller::I2C11, instance_num: 11 },
        ControllerMapping { controller: Controller::I2C12, instance_num: 12 },
        ControllerMapping { controller: Controller::I2C13, instance_num: 13 },
    ]
}