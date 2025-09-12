//! Generic I2C Service Layer
//!
//! This module provides a platform-agnostic I2C service implementation
//! that can be used across different embedded platforms.

use drv_i2c_api::*;
use drv_stm32xx_i2c::*;
use drv_stm32xx_sys_api::{Mode, OutputType, PinSet, Pull, Speed, Sys};

use fixedmap::*;
use ringbuf::*;
use userlib::*;

/// Platform-specific controller identifier
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct Stm32ControllerId(pub drv_i2c_api::Controller);

/// Platform-specific mux address type
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Stm32MuxAddress(pub u8);

/// Platform-specific segment type
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Stm32Segment(pub drv_i2c_api::Segment);

/// Platform-specific error type
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Stm32I2cError {
    BadController,
    BadPort,
    MuxNotFound,
    SegmentNotFound,
    MuxMissing,
    BadMuxRegister,
    BusReset,
    BusResetMux,
    BusError,
    ControllerBusy,
    BusLocked,
    BusLockedMux,
    NoDevice,
    BadArg,
    IllegalLeaseCount,
    TooMuchData,
    ReservedAddress,
    BadMux,
    BadSegment,
    OperationNotSupported,
    BadDeviceState,
    BadResponse,
}

/// Platform-specific recovery statistics
#[derive(Clone, Debug, Default)]
pub struct Stm32RecoveryStats {
    pub recovery_count: u32,
    pub successful_recoveries: u32,
    pub failed_recoveries: u32,
    pub last_recovery_time: Option<u64>,
}

/// Platform-specific transaction statistics
#[derive(Clone, Debug, Default)]
pub struct Stm32TransactionStats {
    pub total_transactions: u32,
    pub successful_transactions: u32,
    pub failed_transactions: u32,
    pub average_duration_us: u32,
    pub max_duration_us: u32,
}

/// Generic I2C service layer implementation for STM32
pub struct Stm32I2cService<'a> {
    controllers: &'a [I2cController<'a>],
    pins: &'a [I2cPins],
    muxes: &'a [I2cMux<'a>],
    portmap: PortMap,
    muxmap: MuxMap,
    sys: Sys,
}

impl<'a> Stm32I2cService<'a> {
    pub fn new(
        controllers: &'a [I2cController<'a>],
        pins: &'a [I2cPins],
        muxes: &'a [I2cMux<'a>],
    ) -> Self {
        let sys = Sys::from(SYS.get_task_id());
        Self {
            controllers,
            pins,
            muxes,
            portmap: PortMap::default(),
            muxmap: MuxMap::default(),
            sys,
        }
    }

    /// Initialize the service
    pub fn initialize(&mut self) -> Result<(), Stm32I2cError> {
        // Turn on I2C peripherals
        for controller in self.controllers {
            controller.enable(&self.sys);
        }

        // Configure controllers
        for controller in self.controllers {
            controller.configure();
            sys_irq_control(controller.notification, true);
        }

        // Configure pins
        self.configure_pins()?;

        // Configure muxes
        self.configure_muxes()?;

        Ok(())
    }

    /// Handle a write-read transaction
    pub fn handle_write_read(
        &mut self,
        controller: drv_i2c_api::Controller,
        port: drv_i2c_api::PortIndex,
        mux: Option<(drv_i2c_api::Mux, drv_i2c_api::Segment)>,
        addr: u8,
        write_data: &[u8],
        read_data: &mut [u8],
    ) -> Result<usize, Stm32I2cError> {
        // Validate inputs
        if ReservedAddress::from_u8(addr).is_some() {
            return Err(Stm32I2cError::ReservedAddress);
        }

        let controller = self.lookup_controller(controller)?;
        self.validate_port(controller.controller, port)?;

        // Configure port if needed
        self.configure_port(controller, port)?;

        // Configure mux if needed
        self.configure_mux(&mut self.muxmap, controller, port, mux)?;

        // Perform the transaction
        let result = controller.write_read(
            addr,
            write_data.len(),
            |pos| write_data.get(pos).copied().unwrap_or(0),
            ReadLength::Fixed(read_data.len()),
            |pos, byte| {
                if pos < read_data.len() {
                    read_data[pos] = byte;
                }
            },
        );

        match result {
            Ok(_) => Ok(read_data.len()),
            Err(code) => {
                // Handle error recovery
                self.handle_transaction_error(code, controller, port)?;
                Err(self.map_response_code(code))
            }
        }
    }

    /// Handle a block read transaction
    pub fn handle_write_read_block(
        &mut self,
        controller: drv_i2c_api::Controller,
        port: drv_i2c_api::PortIndex,
        mux: Option<(drv_i2c_api::Mux, drv_i2c_api::Segment)>,
        addr: u8,
        write_data: &[u8],
        read_data: &mut [u8],
    ) -> Result<usize, Stm32I2cError> {
        // Similar to write_read but with block read semantics
        if ReservedAddress::from_u8(addr).is_some() {
            return Err(Stm32I2cError::ReservedAddress);
        }

        let controller = self.lookup_controller(controller)?;
        self.validate_port(controller.controller, port)?;

        self.configure_port(controller, port)?;
        self.configure_mux(&mut self.muxmap, controller, port, mux)?;

        // For block read, the final read is variable length
        let result = controller.write_read(
            addr,
            write_data.len(),
            |pos| write_data.get(pos).copied().unwrap_or(0),
            ReadLength::Variable, // Block read
            |pos, byte| {
                if pos < read_data.len() {
                    read_data[pos] = byte;
                }
            },
        );

        match result {
            Ok(_) => Ok(read_data.len()),
            Err(code) => {
                self.handle_transaction_error(code, controller, port)?;
                Err(self.map_response_code(code))
            }
        }
    }
}

// Helper methods for Stm32I2cService
impl<'a> Stm32I2cService<'a> {
    fn lookup_controller(&self, controller: drv_i2c_api::Controller) -> Result<&I2cController<'a>, Stm32I2cError> {
        self.controllers
            .iter()
            .find(|c| c.controller == controller)
            .ok_or(Stm32I2cError::BadController)
    }

    fn validate_port(&self, controller: drv_i2c_api::Controller, port: drv_i2c_api::PortIndex) -> Result<(), Stm32I2cError> {
        self.pins
            .iter()
            .find(|pin| pin.controller == controller && pin.port == port)
            .ok_or(Stm32I2cError::BadPort)?;
        Ok(())
    }

    fn configure_port(&mut self, controller: &I2cController<'a>, port: drv_i2c_api::PortIndex) -> Result<(), Stm32I2cError> {
        let current = self.portmap.get(controller.controller).unwrap_or(drv_i2c_api::PortIndex(0));

        if current == port {
            return Ok(());
        }

        // Configure pins for the new port
        for pin in self.pins.iter().filter(|p| p.controller == controller.controller) {
            if pin.port == current {
                // Deconfigure old port
                for gpio_pin in &[pin.scl, pin.sda] {
                    self.sys.gpio_configure(
                        gpio_pin.port,
                        gpio_pin.pin_mask,
                        Mode::Analog,
                        OutputType::OpenDrain,
                        Speed::Low,
                        Pull::None,
                        pin.function,
                    );
                }
            } else if pin.port == port {
                // Configure new port
                for gpio_pin in &[pin.scl, pin.sda] {
                    self.sys.gpio_configure_alternate(
                        *gpio_pin,
                        OutputType::OpenDrain,
                        Speed::Low,
                        Pull::None,
                        pin.function,
                    );
                }
            }
        }

        self.portmap.insert(controller.controller, port);
        Ok(())
    }

    fn configure_mux(
        &mut self,
        muxmap: &mut MuxMap,
        controller: &I2cController<'a>,
        port: drv_i2c_api::PortIndex,
        mux: Option<(drv_i2c_api::Mux, drv_i2c_api::Segment)>,
    ) -> Result<(), Stm32I2cError> {
        let bus = (controller.controller, port);

        match muxmap.get(bus) {
            Some(MuxState::Enabled(current_id, current_segment)) => match mux {
                Some((id, segment)) if id == current_id => {
                    if segment == current_segment {
                        return Ok(());
                    }
                }
                _ => {
                    // Disable current mux
                    self.disable_mux_segment(controller, port, current_id)?;
                    muxmap.remove(bus);
                }
            },
            Some(MuxState::Unknown) => {
                // Reset all muxes on this bus
                self.reset_muxes_on_bus(controller, port)?;
                muxmap.remove(bus);
            }
            None => {}
        }

        // Enable new mux segment if specified
        if let Some((id, segment)) = mux {
            self.enable_mux_segment(controller, port, id, segment)?;
            muxmap.insert(bus, MuxState::Enabled(id, segment));
        }

        Ok(())
    }

    fn enable_mux_segment(
        &mut self,
        controller: &I2cController<'a>,
        port: drv_i2c_api::PortIndex,
        mux_id: drv_i2c_api::Mux,
        segment: drv_i2c_api::Segment,
    ) -> Result<(), Stm32I2cError> {
        for mux in self.muxes {
            if mux.controller == controller.controller && mux.port == port && mux.id == mux_id {
                return mux.driver.enable_segment(mux, controller, Some(segment))
                    .map_err(|_| Stm32I2cError::BadDeviceState);
            }
        }
        Err(Stm32I2cError::MuxNotFound)
    }

    fn disable_mux_segment(
        &mut self,
        controller: &I2cController<'a>,
        port: drv_i2c_api::PortIndex,
        mux_id: drv_i2c_api::Mux,
    ) -> Result<(), Stm32I2cError> {
        for mux in self.muxes {
            if mux.controller == controller.controller && mux.port == port && mux.id == mux_id {
                return mux.driver.enable_segment(mux, controller, None)
                    .map_err(|_| Stm32I2cError::BadDeviceState);
            }
        }
        Err(Stm32I2cError::MuxNotFound)
    }

    fn reset_muxes_on_bus(
        &mut self,
        controller: &I2cController<'a>,
        port: drv_i2c_api::PortIndex,
    ) -> Result<(), Stm32I2cError> {
        for mux in self.muxes.iter().filter(|m| m.controller == controller.controller && m.port == port) {
            let _ = mux.driver.enable_segment(mux, controller, None);
        }
        Ok(())
    }

    fn handle_transaction_error(
        &mut self,
        code: drv_i2c_api::ResponseCode,
        controller: &I2cController<'a>,
        port: drv_i2c_api::PortIndex,
    ) -> Result<(), Stm32I2cError> {
        if self.needs_reset(code) {
            self.reset_controller(controller, port)?;
        }
        Ok(())
    }

    fn needs_reset(&self, code: drv_i2c_api::ResponseCode) -> bool {
        matches!(
            code,
            drv_i2c_api::ResponseCode::BusLocked
                | drv_i2c_api::ResponseCode::BusLockedMux
                | drv_i2c_api::ResponseCode::BusReset
                | drv_i2c_api::ResponseCode::BusResetMux
                | drv_i2c_api::ResponseCode::BusError
                | drv_i2c_api::ResponseCode::ControllerBusy
        )
    }

    fn reset_controller(
        &mut self,
        controller: &I2cController<'a>,
        port: drv_i2c_api::PortIndex,
    ) -> Result<(), Stm32I2cError> {
        let bus = (controller.controller, port);

        // Reset I2C controller
        controller.reset();

        // Reset muxes on this bus
        self.reset_muxes_on_bus(controller, port)?;

        // Mark bus as unknown state
        self.muxmap.insert(bus, MuxState::Unknown);

        Ok(())
    }

    fn configure_pins(&mut self) -> Result<(), Stm32I2cError> {
        // Wiggle SCL to clear any old transactions
        for pin in self.pins {
            self.wiggle_scl(pin.scl, pin.sda);
        }
        Ok(())
    }

    fn configure_muxes(&mut self) -> Result<(), Stm32I2cError> {
        // Initial mux configuration would go here
        Ok(())
    }

    fn wiggle_scl(&self, scl: PinSet, sda: PinSet) {
        // Simplified wiggle implementation
        self.sys.gpio_set(scl);
        self.sys.gpio_configure_output(
            scl,
            OutputType::OpenDrain,
            Speed::Low,
            Pull::None,
        );

        // Wiggle sequence (simplified)
        for _ in 0..9 {
            self.sys.gpio_reset(scl);
            self.sys.gpio_set(scl);
        }
    }

    fn map_response_code(&self, code: drv_i2c_api::ResponseCode) -> Stm32I2cError {
        match code {
            drv_i2c_api::ResponseCode::BadController => Stm32I2cError::BadController,
            drv_i2c_api::ResponseCode::BadPort => Stm32I2cError::BadPort,
            drv_i2c_api::ResponseCode::MuxNotFound => Stm32I2cError::MuxNotFound,
            drv_i2c_api::ResponseCode::NoDevice => Stm32I2cError::NoDevice,
            drv_i2c_api::ResponseCode::BusError => Stm32I2cError::BusError,
            _ => Stm32I2cError::BadDeviceState,
        }
    }
}

// Type aliases for the generic service
type PortMap = FixedMap<drv_i2c_api::Controller, drv_i2c_api::PortIndex, 8>; // Default size
type MuxMap = FixedMap<(drv_i2c_api::Controller, drv_i2c_api::PortIndex), MuxState, 16>; // Default size

#[derive(Copy, Clone, Debug)]
enum MuxState {
    Enabled(drv_i2c_api::Mux, drv_i2c_api::Segment),
    Unknown,
}

use drv_i2c_api::*;
use drv_stm32xx_i2c::*;
use drv_stm32xx_sys_api::{Mode, OutputType, PinSet, Pull, Speed, Sys};

use fixedmap::*;
use ringbuf::*;
use userlib::*;

use core::hash::Hash;
use heapless::FnvIndexMap;

/// Platform-specific controller identifier
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct Stm32ControllerId(pub drv_i2c_api::Controller);

/// Platform-specific mux address type
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Stm32MuxAddress(pub u8);

/// Platform-specific segment type
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Stm32Segment(pub drv_i2c_api::Segment);

/// Platform-specific error type
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Stm32I2cError {
    BadController,
    BadPort,
    MuxNotFound,
    SegmentNotFound,
    MuxMissing,
    BadMuxRegister,
    BusReset,
    BusResetMux,
    BusError,
    ControllerBusy,
    BusLocked,
    BusLockedMux,
    NoDevice,
    BadArg,
    IllegalLeaseCount,
    TooMuchData,
    ReservedAddress,
    BadMux,
    BadSegment,
    OperationNotSupported,
    BadDeviceState,
    BadResponse,
}

/// Platform-specific recovery statistics
#[derive(Clone, Debug, Default)]
pub struct Stm32RecoveryStats {
    pub recovery_count: u32,
    pub successful_recoveries: u32,
    pub failed_recoveries: u32,
    pub last_recovery_time: Option<u64>,
}

/// Platform-specific transaction statistics
#[derive(Clone, Debug, Default)]
pub struct Stm32TransactionStats {
    pub total_transactions: u32,
    pub successful_transactions: u32,
    pub failed_transactions: u32,
    pub average_duration_us: u32,
    pub max_duration_us: u32,
}

/// Generic I2C service layer implementation for STM32
pub struct Stm32I2cService<'a> {
    controllers: &'a [I2cController<'a>],
    pins: &'a [I2cPins],
    muxes: &'a [I2cMux<'a>],
    portmap: PortMap,
    muxmap: MuxMap,
    sys: Sys,
}

impl<'a> Stm32I2cService<'a> {
    pub fn new(
        controllers: &'a [I2cController<'a>],
        pins: &'a [I2cPins],
        muxes: &'a [I2cMux<'a>],
    ) -> Self {
        let sys = Sys::from(SYS.get_task_id());
        Self {
            controllers,
            pins,
            muxes,
            portmap: PortMap::default(),
            muxmap: MuxMap::default(),
            sys,
        }
    }

    /// Initialize the service
    pub fn initialize(&mut self) -> Result<(), Stm32I2cError> {
        // Turn on I2C peripherals
        for controller in self.controllers {
            controller.enable(&self.sys);
        }

        // Configure controllers
        for controller in self.controllers {
            controller.configure();
            sys_irq_control(controller.notification, true);
        }

        // Configure pins
        self.configure_pins()?;

        // Configure muxes
        self.configure_muxes()?;

        Ok(())
    }

    /// Handle a write-read transaction
    pub fn handle_write_read(
        &mut self,
        controller: drv_i2c_api::Controller,
        port: drv_i2c_api::PortIndex,
        mux: Option<(drv_i2c_api::Mux, drv_i2c_api::Segment)>,
        addr: u8,
        write_data: &[u8],
        read_data: &mut [u8],
    ) -> Result<usize, Stm32I2cError> {
        // Validate inputs
        if ReservedAddress::from_u8(addr).is_some() {
            return Err(Stm32I2cError::ReservedAddress);
        }

        let controller = self.lookup_controller(controller)?;
        self.validate_port(controller.controller, port)?;

        // Configure port if needed
        self.configure_port(controller, port)?;

        // Configure mux if needed
        self.configure_mux(&mut self.muxmap, controller, port, mux)?;

        // Perform the transaction
        let result = controller.write_read(
            addr,
            write_data.len(),
            |pos| write_data.get(pos).copied().unwrap_or(0),
            ReadLength::Fixed(read_data.len()),
            |pos, byte| {
                if pos < read_data.len() {
                    read_data[pos] = byte;
                }
            },
        );

        match result {
            Ok(_) => Ok(read_data.len()),
            Err(code) => {
                // Handle error recovery
                self.handle_transaction_error(code, controller, port)?;
                Err(self.map_response_code(code))
            }
        }
    }

    /// Handle a block read transaction
    pub fn handle_write_read_block(
        &mut self,
        controller: drv_i2c_api::Controller,
        port: drv_i2c_api::PortIndex,
        mux: Option<(drv_i2c_api::Mux, drv_i2c_api::Segment)>,
        addr: u8,
        write_data: &[u8],
        read_data: &mut [u8],
    ) -> Result<usize, Stm32I2cError> {
        // Similar to write_read but with block read semantics
        if ReservedAddress::from_u8(addr).is_some() {
            return Err(Stm32I2cError::ReservedAddress);
        }

        let controller = self.lookup_controller(controller)?;
        self.validate_port(controller.controller, port)?;

        self.configure_port(controller, port)?;
        self.configure_mux(&mut self.muxmap, controller, port, mux)?;

        // For block read, the final read is variable length
        let result = controller.write_read(
            addr,
            write_data.len(),
            |pos| write_data.get(pos).copied().unwrap_or(0),
            ReadLength::Variable, // Block read
            |pos, byte| {
                if pos < read_data.len() {
                    read_data[pos] = byte;
                }
            },
        );

        match result {
            Ok(_) => Ok(read_data.len()),
            Err(code) => {
                self.handle_transaction_error(code, controller, port)?;
                Err(self.map_response_code(code))
            }
        }
    }

    // Helper methods
    fn lookup_controller(&self, controller: drv_i2c_api::Controller) -> Result<&I2cController<'a>, Stm32I2cError> {
        self.controllers
            .iter()
            .find(|c| c.controller == controller)
            .ok_or(Stm32I2cError::BadController)
    }

    fn validate_port(&self, controller: drv_i2c_api::Controller, port: drv_i2c_api::PortIndex) -> Result<(), Stm32I2cError> {
        self.pins
            .iter()
            .find(|pin| pin.controller == controller && pin.port == port)
            .ok_or(Stm32I2cError::BadPort)?;
        Ok(())
    }

    fn configure_port(&mut self, controller: &I2cController<'a>, port: drv_i2c_api::PortIndex) -> Result<(), Stm32I2cError> {
        let current = self.portmap.get(controller.controller).unwrap_or(drv_i2c_api::PortIndex(0));

        if current == port {
            return Ok(());
        }

        // Configure pins for the new port
        for pin in self.pins.iter().filter(|p| p.controller == controller.controller) {
            if pin.port == current {
                // Deconfigure old port
                for gpio_pin in &[pin.scl, pin.sda] {
                    self.sys.gpio_configure(
                        gpio_pin.port,
                        gpio_pin.pin_mask,
                        Mode::Analog,
                        OutputType::OpenDrain,
                        Speed::Low,
                        Pull::None,
                        pin.function,
                    );
                }
            } else if pin.port == port {
                // Configure new port
                for gpio_pin in &[pin.scl, pin.sda] {
                    self.sys.gpio_configure_alternate(
                        *gpio_pin,
                        OutputType::OpenDrain,
                        Speed::Low,
                        Pull::None,
                        pin.function,
                    );
                }
            }
        }

        self.portmap.insert(controller.controller, port);
        Ok(())
    }

    fn configure_mux(
        &mut self,
        muxmap: &mut MuxMap,
        controller: &I2cController<'a>,
        port: drv_i2c_api::PortIndex,
        mux: Option<(drv_i2c_api::Mux, drv_i2c_api::Segment)>,
    ) -> Result<(), Stm32I2cError> {
        let bus = (controller.controller, port);

        match muxmap.get(bus) {
            Some(MuxState::Enabled(current_id, current_segment)) => match mux {
                Some((id, segment)) if id == current_id => {
                    if segment == current_segment {
                        return Ok(());
                    }
                }
                _ => {
                    // Disable current mux
                    self.disable_mux_segment(controller, port, current_id)?;
                    muxmap.remove(bus);
                }
            },
            Some(MuxState::Unknown) => {
                // Reset all muxes on this bus
                self.reset_muxes_on_bus(controller, port)?;
                muxmap.remove(bus);
            }
            None => {}
        }

        // Enable new mux segment if specified
        if let Some((id, segment)) = mux {
            self.enable_mux_segment(controller, port, id, segment)?;
            muxmap.insert(bus, MuxState::Enabled(id, segment));
        }

        Ok(())
    }

    fn enable_mux_segment(
        &mut self,
        controller: &I2cController<'a>,
        port: drv_i2c_api::PortIndex,
        mux_id: drv_i2c_api::Mux,
        segment: drv_i2c_api::Segment,
    ) -> Result<(), Stm32I2cError> {
        for mux in self.muxes {
            if mux.controller == controller.controller && mux.port == port && mux.id == mux_id {
                return mux.driver.enable_segment(mux, controller, Some(segment))
                    .map_err(|_| Stm32I2cError::BadDeviceState);
            }
        }
        Err(Stm32I2cError::MuxNotFound)
    }

    fn disable_mux_segment(
        &mut self,
        controller: &I2cController<'a>,
        port: drv_i2c_api::PortIndex,
        mux_id: drv_i2c_api::Mux,
    ) -> Result<(), Stm32I2cError> {
        for mux in self.muxes {
            if mux.controller == controller.controller && mux.port == port && mux.id == mux_id {
                return mux.driver.enable_segment(mux, controller, None)
                    .map_err(|_| Stm32I2cError::BadDeviceState);
            }
        }
        Err(Stm32I2cError::MuxNotFound)
    }

    fn reset_muxes_on_bus(
        &mut self,
        controller: &I2cController<'a>,
        port: drv_i2c_api::PortIndex,
    ) -> Result<(), Stm32I2cError> {
        for mux in self.muxes.iter().filter(|m| m.controller == controller.controller && m.port == port) {
            let _ = mux.driver.enable_segment(mux, controller, None);
        }
        Ok(())
    }

    fn handle_transaction_error(
        &mut self,
        code: drv_i2c_api::ResponseCode,
        controller: &I2cController<'a>,
        port: drv_i2c_api::PortIndex,
    ) -> Result<(), Stm32I2cError> {
        if self.needs_reset(code) {
            self.reset_controller(controller, port)?;
        }
        Ok(())
    }

    fn needs_reset(&self, code: drv_i2c_api::ResponseCode) -> bool {
        matches!(
            code,
            drv_i2c_api::ResponseCode::BusLocked
                | drv_i2c_api::ResponseCode::BusLockedMux
                | drv_i2c_api::ResponseCode::BusReset
                | drv_i2c_api::ResponseCode::BusResetMux
                | drv_i2c_api::ResponseCode::BusError
                | drv_i2c_api::ResponseCode::ControllerBusy
        )
    }

    fn reset_controller(
        &mut self,
        controller: &I2cController<'a>,
        port: drv_i2c_api::PortIndex,
    ) -> Result<(), Stm32I2cError> {
        let bus = (controller.controller, port);

        // Reset I2C controller
        controller.reset();

        // Reset muxes on this bus
        self.reset_muxes_on_bus(controller, port)?;

        // Mark bus as unknown state
        self.muxmap.insert(bus, MuxState::Unknown);

        Ok(())
    }

    fn configure_pins(&mut self) -> Result<(), Stm32I2cError> {
        // Wiggle SCL to clear any old transactions
        for pin in self.pins {
            self.wiggle_scl(pin.scl, pin.sda);
        }
        Ok(())
    }

    fn configure_muxes(&mut self) -> Result<(), Stm32I2cError> {
        // Initial mux configuration would go here
        Ok(())
    }

    fn wiggle_scl(&self, scl: PinSet, sda: PinSet) {
        // Simplified wiggle implementation
        self.sys.gpio_set(scl);
        self.sys.gpio_configure_output(
            scl,
            OutputType::OpenDrain,
            Speed::Low,
            Pull::None,
        );

        // Wiggle sequence (simplified)
        for _ in 0..9 {
            self.sys.gpio_reset(scl);
            self.sys.gpio_set(scl);
        }
    }

    fn map_response_code(&self, code: drv_i2c_api::ResponseCode) -> Stm32I2cError {
        match code {
            drv_i2c_api::ResponseCode::BadController => Stm32I2cError::BadController,
            drv_i2c_api::ResponseCode::BadPort => Stm32I2cError::BadPort,
            drv_i2c_api::ResponseCode::MuxNotFound => Stm32I2cError::MuxNotFound,
            drv_i2c_api::ResponseCode::NoDevice => Stm32I2cError::NoDevice,
            drv_i2c_api::ResponseCode::BusError => Stm32I2cError::BusError,
            _ => Stm32I2cError::BadDeviceState,
        }
    }
}

// Type aliases for the generic service - these will be passed as parameters
type PortMap = FixedMap<drv_i2c_api::Controller, drv_i2c_api::PortIndex, 8>; // Default size
type MuxMap = FixedMap<(drv_i2c_api::Controller, drv_i2c_api::PortIndex), MuxState, 16>; // Default size

#[derive(Copy, Clone, Debug)]
enum MuxState {
    Enabled(drv_i2c_api::Mux, drv_i2c_api::Segment),
    Unknown,
}
