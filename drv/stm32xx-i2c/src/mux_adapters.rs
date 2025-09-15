// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Platform adapters for generic I2C mux drivers

use drv_i2c_mux_core::GpioPin;
use drv_stm32xx_sys_api as sys_api;

/// STM32 GPIO pin adapter that implements the generic GpioPin trait
pub struct Stm32GpioPin<'a> {
    pub pins: sys_api::PinSet,
    pub sys: &'a sys_api::Sys,
}

impl<'a> GpioPin for Stm32GpioPin<'a> {
    fn set_high(&mut self) {
        self.sys.gpio_set(self.pins);
    }
    
    fn set_low(&mut self) {
        self.sys.gpio_reset(self.pins);
    }
    
    fn configure_as_output(&mut self) {
        self.sys.gpio_configure_output(
            self.pins,
            sys_api::OutputType::PushPull,
            sys_api::Speed::Low,
            sys_api::Pull::None,
        );
    }
}

/// Convert our I2cGpio to the generic adapter
impl<'a> Stm32GpioPin<'a> {
    pub fn from_i2c_gpio(gpio: &crate::I2cGpio, sys: &'a sys_api::Sys) -> Self {
        Self {
            pins: gpio.gpio_pins,
            sys,
        }
    }
}