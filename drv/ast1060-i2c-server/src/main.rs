// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! A driver server for AST1060 I2C controllers using ASPEED DDK

#![no_std]
#![no_main]

use drv_ast1060_i2c::{Ast1060I2cDriver, ControllerMapping, create_all_controller_mappings};
use drv_i2c_api::*;
use drv_i2c_types::{traits::I2cHardware, Op, ResponseCode};

use fixedmap::*;
use userlib::{hl, LeaseAttributes, FromPrimitive};

include!(concat!(env!("OUT_DIR"), "/pins.rs"));
include!(concat!(env!("OUT_DIR"), "/notifications.rs"));

/// Map type for tracking mux states across controllers and ports
type MuxMap = FixedMap<(Controller, PortIndex), Option<(Mux, Segment)>, 32>;

/// Global static controller mappings for all 14 AST1060 I2C controllers
static CONTROLLER_MAPPINGS: [ControllerMapping; 14] = create_all_controller_mappings();

/// Global AST1060 I2C driver instance
static mut I2C_DRIVER: Option<Ast1060I2cDriver> = None;

fn lookup_controller(controller: Controller) -> Result<&'static I2cController, ResponseCode> {
    CONTROLLERS
        .iter()
        .find(|c| c.controller == controller)
        .ok_or(ResponseCode::BadController)
}

fn validate_port(
    controller: Controller,
    port: PortIndex,
) -> Result<(), ResponseCode> {
    PINS.iter()
        .find(|pin| pin.controller == controller && pin.port == port)
        .ok_or(ResponseCode::BadPort)?;

    Ok(())
}

fn find_mux(
    controller: &I2cController,
    port: PortIndex,
    id: Mux,
    mut func: impl FnMut(&I2cMux<'_>) -> Result<(), ResponseCode>,
) -> Result<(), ResponseCode> {
    for mux in MUXES {
        if mux.controller == controller.controller
            && mux.port == port
            && mux.id == id
        {
            return func(mux);
        }
    }

    Err(ResponseCode::MuxNotFound)
}

fn all_muxes(
    controller: &I2cController,
    port: PortIndex,
    mut func: impl FnMut(&I2cMux<'_>) -> Result<(), ResponseCode>,
) -> Result<(), ResponseCode> {
    for mux in MUXES {
        if mux.controller == controller.controller && mux.port == port {
            func(mux)?;
        }
    }

    Ok(())
}

fn configure_mux(
    muxmap: &mut MuxMap,
    controller: &I2cController,
    port: PortIndex,
    mux: Option<(Mux, Segment)>,
) -> Result<(), ResponseCode> {
    let bus = (controller.controller, port);

    match mux {
        Some((mux_id, segment)) => {
            // Check if this mux+segment is already configured
            if let Some(current) = muxmap.get(bus) {
                if current == Some((mux_id, segment)) {
                    return Ok(());
                }
            }

            // Configure the mux to the new segment
            find_mux(controller, port, mux_id, |mux_config| {
                // Get I2C driver
                let driver = unsafe {
                    I2C_DRIVER.as_mut().ok_or(ResponseCode::BusError)?
                };

                // For AST1060, mux operations are regular I2C transactions
                let write_data = [segment as u8];
                let mut read_buffer = [];

                driver.write_read(
                    controller.controller,
                    mux_config.address,
                    &write_data,
                    &mut read_buffer,
                )?;

                Ok(())
            })?;

            muxmap.insert(bus, Some((mux_id, segment)));
        }
        None => {
            // If we previously had a mux configured on this bus, disable all segments
            if let Some(Some(_)) = muxmap.get(bus) {
                all_muxes(controller, port, |mux_config| {
                    // Get I2C driver
                    let driver = unsafe {
                        I2C_DRIVER.as_mut().ok_or(ResponseCode::BusError)?
                    };

                    // Disable all segments (typically by writing 0)
                    let write_data = [0u8];
                    let mut read_buffer = [];

                    driver.write_read(
                        controller.controller,
                        mux_config.address,
                        &write_data,
                        &mut read_buffer,
                    )?;

                    Ok(())
                })?;
            }

            muxmap.insert(bus, None);
        }
    }

    Ok(())
}

#[export_name = "main"]
fn main() -> ! {
    // Initialize the AST1060 I2C driver
    unsafe {
        I2C_DRIVER = Some(Ast1060I2cDriver::new(&CONTROLLER_MAPPINGS));
    }

    let mut muxmap = MuxMap::default();

    // Field messages
    let mut buffer = [0; 4];

    loop {
        hl::recv_without_notification(&mut buffer, |op, msg| match op {
            Op::WriteRead | Op::WriteReadBlock => {
                let lease_count = msg.lease_count();

                let (payload, caller) = msg
                    .fixed::<[u8; 4], usize>()
                    .ok_or(ResponseCode::BadArg)?;

                if lease_count < 2 || lease_count % 2 != 0 {
                    return Err(ResponseCode::IllegalLeaseCount);
                }

                let (addr, controller, port, mux) = Marshal::unmarshal(payload)?;

                if ReservedAddress::from_u8(addr).is_some() {
                    return Err(ResponseCode::ReservedAddress);
                }

                let controller_config = lookup_controller(controller)?;
                validate_port(controller, port)?;

                configure_mux(&mut muxmap, controller_config, port, mux)?;

                let driver = unsafe {
                    I2C_DRIVER.as_mut().ok_or(ResponseCode::BusError)?
                };

                let mut total = 0;

                // Iterate over write/read pairs
                for i in (0..lease_count).step_by(2) {
                    let wbuf = caller.borrow(i);
                    let winfo = wbuf.info().ok_or(ResponseCode::BadArg)?;

                    if !winfo.attributes.contains(LeaseAttributes::READ) {
                        return Err(ResponseCode::BadArg);
                    }

                    let rbuf = caller.borrow(i + 1);
                    let rinfo = rbuf.info().ok_or(ResponseCode::BadArg)?;

                    if winfo.len == 0 && rinfo.len == 0 {
                        return Err(ResponseCode::BadArg);
                    }

                    if winfo.len > 255 || rinfo.len > 255 {
                        return Err(ResponseCode::BadArg);
                    }

                    // Read write data from lease
                    let mut write_data = [0u8; 255];
                    for pos in 0..winfo.len {
                        write_data[pos] = wbuf.read_at(pos).ok_or(ResponseCode::BadArg)?;
                    }

                    // Prepare read buffer
                    let mut read_buffer = [0u8; 255];
                    let read_slice = &mut read_buffer[..rinfo.len];

                    // Perform the I2C transaction
                    let bytes_read = if op == Op::WriteReadBlock {
                        driver.write_read_block(
                            controller,
                            addr,
                            &write_data[..winfo.len],
                            read_slice,
                        )?
                    } else {
                        driver.write_read(
                            controller,
                            addr,
                            &write_data[..winfo.len],
                            read_slice,
                        )?
                    };

                    // Write read data back to lease
                    for pos in 0..bytes_read.min(rinfo.len) {
                        rbuf.write_at(pos, read_slice[pos]).ok_or(ResponseCode::BadArg)?;
                    }

                    total += bytes_read;
                }

                caller.reply(total);
                Ok(())
            }
            Op::ConfigureSlaveAddress => {
                // Slave mode configuration - not implemented yet
                Err(ResponseCode::NoDevice)
            }
            Op::EnableSlaveReceive => {
                // Slave mode - not implemented yet
                Err(ResponseCode::NoDevice)
            }
            Op::DisableSlaveReceive => {
                // Slave mode - not implemented yet
                Err(ResponseCode::NoDevice)
            }
            Op::CheckSlaveBuffer => {
                // Slave mode - not implemented yet
                Err(ResponseCode::NoDevice)
            }
        });
    }
}