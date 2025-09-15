// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Driver for the PCA9548 I2C mux

use crate::{GpioPin, I2cMuxConfig, I2cMuxDriver};
use bitfield::bitfield;
use drv_i2c_api::{ResponseCode, Segment};
use drv_i2c_types::traits::I2cHardware;

pub struct Pca9548;

bitfield! {
    #[derive(Copy, Clone, Eq, PartialEq)]
    pub struct ControlRegister(u8);
    channel7_enabled, set_channel7_enabled: 7;
    channel6_enabled, set_channel6_enabled: 6;
    channel5_enabled, set_channel5_enabled: 5;
    channel4_enabled, set_channel4_enabled: 4;
    channel3_enabled, set_channel3_enabled: 3;
    channel2_enabled, set_channel2_enabled: 2;
    channel1_enabled, set_channel1_enabled: 1;
    channel0_enabled, set_channel0_enabled: 0;
}

impl<I2C, GPIO> I2cMuxDriver<I2C, GPIO> for Pca9548
where
    I2C: I2cHardware,
    GPIO: GpioPin,
{
    fn configure(&self, config: &mut I2cMuxConfig<GPIO>) -> Result<(), ResponseCode> {
        if let Some(ref mut pin) = config.reset_pin {
            // Set the pin high before configuring as output to avoid glitching
            pin.set_high();
            pin.configure_as_output();
        }
        Ok(())
    }

    fn reset(&self, config: &mut I2cMuxConfig<GPIO>) -> Result<(), ResponseCode> {
        if let Some(ref mut pin) = config.reset_pin {
            pin.set_low();
            pin.set_high();
        }
        Ok(())
    }

    fn enable_segment(
        &self,
        i2c: &mut I2C,
        config: &I2cMuxConfig<GPIO>,
        segment: Option<Segment>,
    ) -> Result<(), ResponseCode> {
        let mut reg = ControlRegister(0);

        if let Some(segment) = segment {
            match segment {
                Segment::S1 => reg.set_channel0_enabled(true),
                Segment::S2 => reg.set_channel1_enabled(true),
                Segment::S3 => reg.set_channel2_enabled(true),
                Segment::S4 => reg.set_channel3_enabled(true),
                Segment::S5 => reg.set_channel4_enabled(true),
                Segment::S6 => reg.set_channel5_enabled(true),
                Segment::S7 => reg.set_channel6_enabled(true),
                Segment::S8 => reg.set_channel7_enabled(true),
                _ => return Err(ResponseCode::SegmentNotFound),
            }
        }

        // PCA9548 has only one register - any write is to the control register
        let write_data = [reg.0];
        let mut read_buf = [0u8; 0];

        let _count = i2c.write_read(config.controller, config.address, &write_data, &mut read_buf)
            .map_err(|e| e.into())?;
        Ok(())
    }
}