// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Generic I2C multiplexer drivers
//!
//! This crate provides hardware-agnostic implementations of common I2C
//! multiplexer devices. The drivers are parameterized over abstract traits
//! to allow reuse across different microcontroller families and I2C
//! implementations.

#![no_std]

use drv_i2c_api::{ResponseCode, Segment};
use drv_i2c_types::{Controller, traits::I2cHardware};

pub mod pca9545;
pub mod pca9548;

/// Abstraction for GPIO pin control (reset/enable lines)
pub trait GpioPin {
    /// Set the pin high
    fn set_high(&mut self);

    /// Set the pin low
    fn set_low(&mut self);

    /// Configure pin as output
    fn configure_as_output(&mut self);
}

/// Configuration for a generic I2C mux
pub struct I2cMuxConfig<G> {
    /// I2C controller to use for communication
    pub controller: Controller,
    /// I2C address of the mux device
    pub address: u8,
    /// Optional reset/enable GPIO pin
    pub reset_pin: Option<G>,
}

/// Generic trait for I2C multiplexer drivers
pub trait I2cMuxDriver<I2C, GPIO>
where
    I2C: I2cHardware,
    GPIO: GpioPin,
{
    /// Configure the mux hardware
    fn configure(&self, config: &mut I2cMuxConfig<GPIO>) -> Result<(), ResponseCode>;

    /// Reset the mux device
    fn reset(&self, config: &mut I2cMuxConfig<GPIO>) -> Result<(), ResponseCode>;

    /// Enable a specific segment on the mux (or disable all if None)
    fn enable_segment(
        &self,
        i2c: &mut I2C,
        config: &I2cMuxConfig<GPIO>,
        segment: Option<Segment>,
    ) -> Result<(), ResponseCode>;
}