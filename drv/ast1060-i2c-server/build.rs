// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create the pin configuration from board-specific data
    build_util::expose_target_board();
    build_util::build_notifications()?;

    let out_dir = std::env::var("OUT_DIR")?;
    let dest_path = std::path::Path::new(&out_dir).join("pins.rs");

    let mut file = std::fs::File::create(dest_path)?;

    writeln!(file, "// AST1060 I2C pin configuration")?;
    writeln!(file, "// Generated automatically by build.rs")?;
    writeln!(file)?;
    writeln!(file, "use drv_i2c_api::{{Controller, PortIndex}};")?;
    writeln!(file)?;

    // AST1060 controllers don't use the same port/pin structure as STM32
    // Each controller is a separate hardware instance
    writeln!(file, "#[derive(Copy, Clone, Debug, Eq, PartialEq)]")?;
    writeln!(file, "pub struct I2cController {{")?;
    writeln!(file, "    pub controller: Controller,")?;
    writeln!(file, "}}")?;
    writeln!(file)?;

    writeln!(file, "pub const CONTROLLERS: &[I2cController] = &[")?;
    for i in 0..14 {
        writeln!(file, "    I2cController {{ controller: Controller::I2C{} }},", i)?;
    }
    writeln!(file, "];")?;
    writeln!(file)?;

    // AST1060 doesn't have pin-based ports like STM32
    // Each controller is a direct hardware interface
    writeln!(file, "#[derive(Copy, Clone, Debug, Eq, PartialEq)]")?;
    writeln!(file, "pub struct I2cPins {{")?;
    writeln!(file, "    pub controller: Controller,")?;
    writeln!(file, "    pub port: PortIndex,")?;
    writeln!(file, "}}")?;
    writeln!(file)?;

    writeln!(file, "pub const PINS: &[I2cPins] = &[")?;
    for i in 0..14 {
        writeln!(file, "    I2cPins {{ controller: Controller::I2C{}, port: PortIndex(0) }},", i)?;
    }
    writeln!(file, "];")?;
    writeln!(file)?;

    // No muxes defined by default - these would be board-specific
    writeln!(file, "use drv_i2c_api::{{Mux, Segment}};")?;
    writeln!(file)?;
    writeln!(file, "#[derive(Copy, Clone, Debug, Eq, PartialEq)]")?;
    writeln!(file, "pub struct I2cMux<'a> {{")?;
    writeln!(file, "    pub controller: Controller,")?;
    writeln!(file, "    pub port: PortIndex,")?;
    writeln!(file, "    pub id: Mux,")?;
    writeln!(file, "    pub driver: &'a str,")?;
    writeln!(file, "    pub address: u8,")?;
    writeln!(file, "}}")?;
    writeln!(file)?;
    writeln!(file, "pub const MUXES: &[I2cMux<'static>] = &[];")?;

    Ok(())
}