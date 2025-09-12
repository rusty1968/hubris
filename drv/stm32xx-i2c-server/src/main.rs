// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! A driver for the STM32H7 I2C interface

#![no_std]
#![no_main]

use drv_i2c_api::*;
use drv_stm32xx_i2c::*;
use drv_stm32xx_sys_api::{Mode, OutputType, PinSet, Pull, Speed, Sys};

use fixedmap::*;
use ringbuf::*;
use userlib::*;

// Import our service layer
mod service;
mod traits;

use service::Stm32I2cService;

task_slot!(SYS, sys);

include!(concat!(env!("OUT_DIR"), "/i2c_config.rs"));

type PortMap = FixedMap<Controller, PortIndex, { i2c_config::NCONTROLLERS }>;

#[derive(Copy, Clone, Debug)]
enum MuxState {
    /// a mux+segment have been explicitly enabled
    Enabled(Mux, Segment),

    /// state is unknown: zero, one, or more mux+segment(s) may be enabled
    Unknown,
}

///
/// Contains the mux state on a per-bus basis.  If no mux+segment is enabled
/// for a bus (that is, if any/all muxes on a bus have been explicitly had
/// all segments disabled), there will not be an entry for the bus in this
/// map.
///
type MuxMap =
    FixedMap<(Controller, PortIndex), MuxState, { i2c_config::NMUXEDBUSES }>;

#[export_name = "main"]
fn main() -> ! {
    let controllers = i2c_config::controllers();
    let pins = i2c_config::pins();
    let muxes = i2c_config::muxes();

    // Create the service layer
    let mut service = Stm32I2cService::new(&controllers, &pins, &muxes);

    // Initialize the service
    if let Err(_) = service.initialize() {
        panic!("Failed to initialize I2C service");
    }

    // IPC message buffer
    let mut buffer = [0; 4];

    loop {
        hl::recv_without_notification(&mut buffer, |op, msg| match op {
            Op::WriteRead | Op::WriteReadBlock => {
                let lease_count = msg.lease_count();

                let (payload, caller) = msg
                    .fixed::<[u8; 4], usize>()
                    .ok_or(ResponseCode::BadArg)?;

                if lease_count < 2 || !lease_count.is_multiple_of(2) {
                    return Err(ResponseCode::IllegalLeaseCount);
                }

                let (addr, controller, port, mux) =
                    Marshal::unmarshal(payload)?;

                // Validate reserved addresses
                if ReservedAddress::from_u8(addr).is_some() {
                    return Err(ResponseCode::ReservedAddress);
                }

                // Validate lease count requirements
                if lease_count < 2 || !lease_count.is_multiple_of(2) {
                    return Err(ResponseCode::IllegalLeaseCount);
                }

                let mut total = 0;

                // Process write/read pairs using the service layer
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

                    // Extract write data
                    let mut write_data = [0u8; 256];
                    let write_len = winfo.len.min(256);
                    for j in 0..write_len {
                        write_data[j] = wbuf.read_at(j).unwrap_or(0);
                    }

                    // Extract read buffer length
                    let read_len = rinfo.len.min(256);
                    let mut read_data = [0u8; 256];

                    // Determine read type for block operations
                    let is_block_read = op == Op::WriteReadBlock && i == lease_count - 2;

                    // Call the appropriate service method
                    let result = if is_block_read {
                        service.handle_write_read_block(
                            controller,
                            port,
                            mux,
                            addr,
                            &write_data[..write_len],
                            &mut read_data[..read_len],
                        )
                    } else {
                        service.handle_write_read(
                            controller,
                            port,
                            mux,
                            addr,
                            &write_data[..write_len],
                            &mut read_data[..read_len],
                        )
                    };

                    match result {
                        Ok(bytes_read) => {
                            // Write the read data back to the caller's buffer
                            for j in 0..bytes_read.min(read_len) {
                                let _ = rbuf.write_at(j, read_data[j]);
                            }
                            total += bytes_read;
                        }
                        Err(service_error) => {
                            // Map service error to ResponseCode
                            let response_code = match service_error {
                                service::Stm32I2cError::BadController => ResponseCode::BadController,
                                service::Stm32I2cError::BadPort => ResponseCode::BadPort,
                                service::Stm32I2cError::MuxNotFound => ResponseCode::MuxNotFound,
                                service::Stm32I2cError::NoDevice => ResponseCode::NoDevice,
                                service::Stm32I2cError::BusError => ResponseCode::BusError,
                                _ => ResponseCode::BadDeviceState,
                            };
                            return Err(response_code);
                        }
                    }
                }

                caller.reply(total);
                Ok(())
            }
        });
    }
}

include!(concat!(env!("OUT_DIR"), "/notifications.rs"));
