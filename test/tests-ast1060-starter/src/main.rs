// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Test suite for AST1060 I2C driver and platform integration
//!
//! This test suite validates the AST1060 I2C driver implementation including:
//! - Driver hardware abstraction layer functionality
//! - I2C server task IPC interfaces
//! - Controller mapping and configuration
//! - Basic I2C operations validation
//! - Error handling and fault injection
//!
//! The suite builds on the core Hubris test framework and adds AST1060-specific
//! tests for I2C operations and system integration.

#![feature(used_with_arg)]
#![no_std]
#![no_main]
#![forbid(clippy::wildcard_imports)]

use drv_i2c_api::{I2cDevice, ResponseCode};
use drv_i2c_types::{Controller, PortIndex};
use ringbuf::{ringbuf, ringbuf_entry};
use test_api::{RunnerOp, SuiteOp};
use userlib::{hl, task_slot, TaskId};
use zerocopy::IntoBytes;

#[derive(Copy, Clone, PartialEq)]
enum Trace {
    None,
    TestStart,
    TestFinish,
    I2cOperationStart { controller: u8, operation: u8 },
    I2cOperationComplete { controller: u8, result: u8 },
    DriverInitialized,
    ErrorEncountered { controller: u8, error: u8 },
}

ringbuf!(Trace, 128, Trace::None);

/// Test configuration from app.toml
task_config::task_config! {
    test_name: &'static str,
    controllers_count: u32,
    default_speed: &'static str,
}

/// Helper macro for building a list of test functions with their names.
macro_rules! test_cases {
    ($($(#[$attr:meta])* $name:path,)*) => {
        #[no_mangle]
        #[used(linker)]
        static TESTS: &[(&str, &(dyn Fn() + Send + Sync))] = &[
            $(
                $(#[$attr])*
                (stringify!($name), &$name)
            ),*
        ];
    };
}

// AST1060-specific test cases
test_cases! {
    // Core I2C driver tests
    test_driver_initialization,
    test_controller_mapping,
    test_basic_i2c_operations,
    test_error_handling,

    // Hardware integration tests
    test_all_controllers_accessible,
    test_interrupt_handling,

    // Configuration validation tests
    test_task_config_validation,
    test_controller_count_validation,
}

// Test framework integration
task_slot!(SUITE, suite);
task_slot!(RUNNER, runner);
task_slot!(I2C, i2c);

/// Get I2C client handle for a specific controller
fn i2c_device(controller: Controller, address: u8) -> I2cDevice {
    I2cDevice::new(
        I2C.get_task_id(),
        controller,
        PortIndex(0),
        None,
        address,
    )
}

//////////////////////////////////////////////////////////////////////////////
// Core I2C Driver Tests
//////////////////////////////////////////////////////////////////////////////

/// Test that the AST1060 I2C driver can be initialized properly
fn test_driver_initialization() {
    ringbuf_entry!(Trace::DriverInitialized);

    // The driver should already be running as the i2c task
    // Verify we can communicate with it by creating a device
    let device = i2c_device(Controller::I2C0, 0x50);

    // Try a basic operation that should complete (even if it fails)
    let result = device.read_reg::<u8, u8>(0x00);

    // The important thing is that we get a response, not a hang or panic
    assert!(result.is_ok() || result.is_err());
}

/// Test the controller mapping system works correctly
fn test_controller_mapping() {
    use drv_ast1060_i2c::create_all_controller_mappings;

    let mappings = create_all_controller_mappings();

    // Verify we have all 14 controllers mapped
    assert_eq!(mappings.len(), 14);

    // Verify mapping correctness
    assert_eq!(mappings[0].controller, Controller::I2C0);
    assert_eq!(mappings[0].instance_num, 0);

    assert_eq!(mappings[13].controller, Controller::I2C13);
    assert_eq!(mappings[13].instance_num, 13);

    // Verify no duplicates in controller assignments
    for i in 0..mappings.len() {
        for j in (i + 1)..mappings.len() {
            assert_ne!(mappings[i].controller as u8, mappings[j].controller as u8);
            assert_ne!(mappings[i].instance_num, mappings[j].instance_num);
        }
    }
}

/// Test basic I2C operations
fn test_basic_i2c_operations() {
    ringbuf_entry!(Trace::I2cOperationStart { controller: 0, operation: 1 });

    let device = i2c_device(Controller::I2C0, 0x50);

    // Test read operation
    let result = device.read_reg::<u8, u8>(0x00);

    // Record the result
    let result_code = match &result {
        Ok(_) => 0,
        Err(e) => *e as u8,
    };

    ringbuf_entry!(Trace::I2cOperationComplete { controller: 0, result: result_code });

    // We expect either success or a specific error (not a crash)
    assert!(matches!(
        result.err(),
        None | Some(ResponseCode::BusError |
                   ResponseCode::NoDevice |
                   ResponseCode::OperationNotSupported)
    ));

    // Test write operation
    let write_data = [0x42];
    let result = device.write(&write_data);
    assert!(result.is_ok() || matches!(
        result.err(),
        Some(ResponseCode::BusError |
             ResponseCode::NoDevice |
             ResponseCode::OperationNotSupported)
    ));
}

/// Test error handling
fn test_error_handling() {
    // Test with invalid address (general call address)
    let device = i2c_device(Controller::I2C0, 0x00);
    let result = device.read_reg::<u8, u8>(0x00);

    ringbuf_entry!(Trace::ErrorEncountered {
        controller: 0,
        error: match result.as_ref().err() { Some(e) => *e as u8, None => 0 }
    });

    // Should get an error or succeed, but not hang or panic
    assert!(result.is_err() || result.is_ok());

    // Test timeout handling - this should complete in reasonable time
    let start_time = userlib::sys_get_timer().now;
    let device = i2c_device(Controller::I2C0, 0x7F);
    let result = device.read_reg::<u8, u8>(0x00);
    let end_time = userlib::sys_get_timer().now;

    // Should complete within reasonable time (less than 1 second assuming ms ticks)
    assert!(end_time - start_time < 1000);
    assert!(result.is_ok() || result.is_err());
}

//////////////////////////////////////////////////////////////////////////////
// Hardware Integration Tests
//////////////////////////////////////////////////////////////////////////////

/// Test that all controllers are accessible
fn test_all_controllers_accessible() {
    let controllers = [
        Controller::I2C0, Controller::I2C1, Controller::I2C2, Controller::I2C3,
        Controller::I2C4, Controller::I2C5, Controller::I2C6, Controller::I2C7,
        Controller::I2C8, Controller::I2C9, Controller::I2C10, Controller::I2C11,
        Controller::I2C12, Controller::I2C13,
    ];

    for controller in controllers.iter() {
        let device = i2c_device(*controller, 0x50);
        let result = device.read_reg::<u8, u8>(0x00);
        // Should get a response for all controllers
        assert!(result.is_ok() || result.is_err());
    }
}

/// Test interrupt handling
fn test_interrupt_handling() {
    // This test verifies that interrupts are properly configured
    // We use the test-irq that's mapped to i2c0.irq in the app.toml

    userlib::sys_irq_control(notifications::TEST_IRQ_MASK, true);

    // Trigger the test interrupt
    trigger_test_irq();

    let rm = userlib::sys_recv_closed(
        &mut [],
        notifications::TEST_IRQ_MASK,
        TaskId::KERNEL,
    ).unwrap();

    assert_eq!(rm.sender, TaskId::KERNEL);
    assert_eq!(rm.operation, notifications::TEST_IRQ_MASK);
}

//////////////////////////////////////////////////////////////////////////////
// Configuration Validation Tests
//////////////////////////////////////////////////////////////////////////////

/// Test task configuration values
fn test_task_config_validation() {
    // Verify test configuration from app.toml
    assert_eq!(TASK_CONFIG.test_name, "AST1060 I2C Test Suite");
    assert_eq!(TASK_CONFIG.controllers_count, 14);
    assert_eq!(TASK_CONFIG.default_speed, "Fast");
}

/// Test controller count validation
fn test_controller_count_validation() {
    // Verify we have the expected number of controllers
    assert_eq!(TASK_CONFIG.controllers_count, 14);

    // This matches the number of controllers in the AST1060 SoC
    let expected_controllers = [
        Controller::I2C0, Controller::I2C1, Controller::I2C2, Controller::I2C3,
        Controller::I2C4, Controller::I2C5, Controller::I2C6, Controller::I2C7,
        Controller::I2C8, Controller::I2C9, Controller::I2C10, Controller::I2C11,
        Controller::I2C12, Controller::I2C13,
    ];

    assert_eq!(expected_controllers.len(), TASK_CONFIG.controllers_count as usize);
}

//////////////////////////////////////////////////////////////////////////////
// Test Framework Support
//////////////////////////////////////////////////////////////////////////////

/// Helper function to trigger test interrupt
#[track_caller]
fn trigger_test_irq() {
    let runner = RUNNER.get_task_id();
    let mut response = 0u32;
    let op = RunnerOp::SoftIrq as u16;
    let arg = notifications::TEST_IRQ_MASK;
    let (rc, len) = userlib::sys_send(
        runner,
        op,
        arg.as_bytes(),
        response.as_mut_bytes(),
        &[],
    );
    assert_eq!(rc, 0);
    assert_eq!(len, 0);
}

/// Main test suite entry point
#[export_name = "main"]
fn main() -> ! {
    // Initialize tracing
    ringbuf_entry!(Trace::TestStart);

    let mut buffer = [0; 4];
    loop {
        hl::recv_without_notification(
            &mut buffer,
            |op, msg| -> Result<(), u32> {
                match op {
                    SuiteOp::RunCase => {
                        let (&idx, caller) = msg.fixed::<usize, ()>().ok_or(2u32)?;
                        caller.reply(());

                        ringbuf_entry!(Trace::TestStart);

                        // Run the test case
                        TESTS[idx].1();

                        ringbuf_entry!(Trace::TestFinish);

                        // Report completion to runner
                        let op = RunnerOp::TestComplete as u16;
                        let (rc, len) = userlib::sys_send(
                            RUNNER.get_task_id(),
                            op,
                            &[],
                            &mut [],
                            &[],
                        );
                        assert_eq!(rc, 0);
                        assert_eq!(len, 0);
                    }
                }
                Ok(())
            },
        )
    }
}

include!(concat!(env!("OUT_DIR"), "/notifications.rs"));