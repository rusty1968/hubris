//! I2C Client Task for AST1060 Platform
//!
//! This task demonstrates how to use the AST1060 I2C server to communicate
//! with I2C devices. It showcases various I2C operations and can be used
//! for testing and validation of the I2C subsystem.
//!
//! Features:
//! - Basic I2C read/write operations
//! - Controller enumeration and discovery
//! - Error handling and recovery
//! - MCTP slave mode demonstrations (when available)
//! - Device scanning and identification

#![no_std]
#![no_main]

use cortex_m::asm;
use drv_i2c_api::{I2cDevice, ResponseCode};
use drv_i2c_types::{Controller, PortIndex};
use ringbuf::{ringbuf, ringbuf_entry};
use userlib::task_slot;

#[derive(Copy, Clone, PartialEq)]
enum Trace {
    None,
    ClientStart,
    ControllerScan { controller: u8 },
    DeviceFound { controller: u8, address: u8 },
    ReadOperation { controller: u8, address: u8, register: u8, success: bool },
    WriteOperation { controller: u8, address: u8, register: u8, success: bool },
    ScanComplete { controllers_tested: u8, devices_found: u8 },
    SlaveConfigured { controller: u8, address: u8, success: bool },
    SlaveMessageReceived { controller: u8, length: u8 },
    SlaveOperationStart { controller: u8 },
    SlaveOperationComplete { controller: u8, messages_received: u8 },
    Error { controller: u8, address: u8, error: u8 },
}

ringbuf!(Trace, 64, Trace::None);

// Task slot for I2C server
task_slot!(I2C, i2c);

/// Create an I2C device handle for a specific controller and address
fn create_i2c_device(controller: Controller, address: u8) -> I2cDevice {
    I2cDevice::new(
        I2C.get_task_id(),
        controller,
        PortIndex(0), // Use port 0 (main port)
        None,         // No mux configuration
        address,
    )
}

/// Scan a specific I2C controller for connected devices
fn scan_controller(controller: Controller, controller_num: u8) -> u8 {
    ringbuf_entry!(Trace::ControllerScan { controller: controller_num });
    
    let mut devices_found = 0;
    
    // Scan common I2C addresses (7-bit addressing)
    // Skip reserved addresses: 0x00-0x07 and 0x78-0x7F
    for addr in 0x08..=0x77 {
        let device = create_i2c_device(controller, addr);
        
        // Try a simple read to detect device presence
        match device.read_reg::<u8, u8>(0x00) {
            Ok(_) => {
                ringbuf_entry!(Trace::DeviceFound { 
                    controller: controller_num, 
                    address: addr 
                });
                devices_found += 1;
            }
            Err(ResponseCode::NoDevice) => {
                // Expected for most addresses - device not present
            }
            Err(ResponseCode::BadController) => {
                // Controller not available in this system configuration
                ringbuf_entry!(Trace::Error { 
                    controller: controller_num, 
                    address: addr, 
                    error: ResponseCode::BadController as u8 
                });
                return 0; // Skip this controller entirely
            }
            Err(error) => {
                ringbuf_entry!(Trace::Error { 
                    controller: controller_num, 
                    address: addr, 
                    error: error as u8 
                });
            }
        }
        
        // Small delay between operations to avoid overwhelming the bus
        for _ in 0..1000 {
            asm::nop();
        }
    }
    
    devices_found
}

/// Test basic I2C operations on a device
fn test_device_operations(controller: Controller, controller_num: u8, address: u8) {
    let device = create_i2c_device(controller, address);
    
    // Test read operation - try to read a common register
    match device.read_reg::<u8, u8>(0x00) {
        Ok(_data) => {
            ringbuf_entry!(Trace::ReadOperation { 
                controller: controller_num, 
                address, 
                register: 0x00, 
                success: true 
            });
        }
        Err(error) => {
            ringbuf_entry!(Trace::ReadOperation { 
                controller: controller_num, 
                address, 
                register: 0x00, 
                success: false 
            });
            ringbuf_entry!(Trace::Error { 
                controller: controller_num, 
                address, 
                error: error as u8 
            });
        }
    }
    
    // Test write operation - try to write to a safe register
    // Note: This is commented out to avoid accidentally modifying device state
    /*
    let test_data = [0x00]; // Safe write to register 0x00
    match device.write(&test_data) {
        Ok(()) => {
            ringbuf_entry!(Trace::WriteOperation { 
                controller: controller_num, 
                address, 
                register: 0x00, 
                success: true 
            });
        }
        Err(error) => {
            ringbuf_entry!(Trace::WriteOperation { 
                controller: controller_num, 
                address, 
                register: 0x00, 
                success: false 
            });
            ringbuf_entry!(Trace::Error { 
                controller: controller_num, 
                address, 
                error: error as u8 
            });
        }
    }
    */
}

/// Get the list of available I2C controllers from the system configuration
/// In a real implementation, this could query the I2C server for available controllers
fn get_available_controllers() -> &'static [(Controller, u8)] {
    // Based on AST1060 configuration - all 14 controllers are potentially available
    // The actual availability is determined by the I2C server's pin configuration
    // in pins.rs, but that's not accessible from client tasks due to build isolation.
    // 
    // Better approaches would be:
    // 1. Adding a "GetControllers" IPC call to the I2C server
    // 2. Creating a shared configuration crate (drv-ast1060-config)
    // 3. Just trying all controllers and handling BadController errors gracefully
    // 4. Using a compile-time generated list shared between server and client
    &[
        (Controller::I2C0, 0),   (Controller::I2C1, 1),
        (Controller::I2C2, 2),   (Controller::I2C3, 3),
        (Controller::I2C4, 4),   (Controller::I2C5, 5),
        (Controller::I2C6, 6),   (Controller::I2C7, 7),
        (Controller::I2C8, 8),   (Controller::I2C9, 9),
        (Controller::I2C10, 10), (Controller::I2C11, 11),
        (Controller::I2C12, 12), (Controller::I2C13, 13),
    ]
}

/// Demonstrate I2C controller enumeration and device discovery
fn demonstrate_i2c_scanning() {
    ringbuf_entry!(Trace::ClientStart);
    
    // Get the list of configured controllers from the system
    let controllers = get_available_controllers();
    
    let mut total_devices = 0;
    
    // Scan each controller for connected devices
    for (controller, controller_num) in controllers.iter() {
        let devices_found = scan_controller(*controller, *controller_num);
        total_devices += devices_found;
        
        // Small delay between controllers
        for _ in 0..10000 {
            asm::nop();
        }
    }
    
    ringbuf_entry!(Trace::ScanComplete { 
        controllers_tested: controllers.len() as u8, 
        devices_found: total_devices 
    });
}

/// Demonstrate specific device communication
fn demonstrate_device_communication() {
    // Example: Communicate with a hypothetical device on I2C0 at address 0x50
    // This is a common address for EEPROMs
    let controller = Controller::I2C0;
    let device_address = 0x50;
    
    test_device_operations(controller, 0, device_address);
    
    // Example: Try to communicate with a device on I2C1 at address 0x48
    // This is a common address for temperature sensors
    let controller = Controller::I2C1;
    let device_address = 0x48;
    
    test_device_operations(controller, 1, device_address);
}

/// Demonstrate I2C slave mode operations (MCTP slave functionality)
fn demonstrate_slave_operations() {
    // Test slave mode on I2C2 (using a different controller to avoid conflicts)
    let controller = Controller::I2C2;
    let controller_num = 2;
    let slave_address = 0x42; // Our slave address for MCTP
    
    ringbuf_entry!(Trace::SlaveOperationStart { controller: controller_num });
    
    // Create device handle for slave operations
    let device = create_i2c_device(controller, slave_address);
    
    // Note: In a real implementation, these would use the slave-specific APIs
    // that we defined in the I2cHardware trait. For now, we'll demonstrate
    // the concept using the existing device API structure.
    
    // Simulate configuring slave mode
    // In actual implementation: device.configure_slave_mode(slave_address)
    match device.read_reg::<u8, u8>(0x00) {
        Ok(_) => {
            ringbuf_entry!(Trace::SlaveConfigured { 
                controller: controller_num, 
                address: slave_address, 
                success: true 
            });
        }
        Err(error) => {
            ringbuf_entry!(Trace::SlaveConfigured { 
                controller: controller_num, 
                address: slave_address, 
                success: false 
            });
            ringbuf_entry!(Trace::Error { 
                controller: controller_num, 
                address: slave_address, 
                error: error as u8 
            });
            return; // Exit if configuration failed
        }
    }
    
    let mut messages_received = 0;
    
    // Simulate polling for slave messages (in a real scenario, this would be interrupt-driven)
    for _poll_cycle in 0..10 {
        // Simulate checking for incoming slave messages
        // In actual implementation: device.poll_slave_messages()
        match device.read_reg::<u8, u8>(0x01) {
            Ok(status) => {
                if status != 0 {
                    // Simulate message received
                    let message_length = status; // In real implementation, get actual length
                    
                    ringbuf_entry!(Trace::SlaveMessageReceived { 
                        controller: controller_num, 
                        length: message_length 
                    });
                    
                    messages_received += 1;
                    
                    // In a real implementation, we would:
                    // 1. Read the complete message from the slave buffer
                    // 2. Process the MCTP packet
                    // 3. Generate appropriate response
                    // 4. Clear the slave buffer for next message
                }
            }
            Err(_) => {
                // No message or error - continue polling
            }
        }
        
        // Delay between polls
        for _ in 0..10000 {
            asm::nop();
        }
    }
    
    ringbuf_entry!(Trace::SlaveOperationComplete { 
        controller: controller_num, 
        messages_received 
    });
}

/// Main task entry point
#[export_name = "main"]
fn main() -> ! {
    // Wait a bit for system initialization
    for _ in 0..100000 {
        asm::nop();
    }
    
    // Demonstrate I2C functionality
    demonstrate_i2c_scanning();
    
    // Small delay between demonstrations
    for _ in 0..50000 {
        asm::nop();
    }
    
    demonstrate_device_communication();
    
    // Small delay before slave operations
    for _ in 0..50000 {
        asm::nop();
    }
    
    // Demonstrate slave mode operations
    demonstrate_slave_operations();
    
    // Main task loop - could be extended to handle IPC messages
    // or perform periodic I2C operations
    loop {
        // For now, just idle - in a real application this could:
        // - Handle incoming IPC requests for I2C operations
        // - Perform periodic device polling
        // - Implement higher-level protocols (MCTP, etc.)
        // - Continue monitoring slave mode operations
        
        for _ in 0..1000000 {
            asm::nop();
        }
    }
}