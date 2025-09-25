// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![no_std]
#![no_main]

// NOTE: you will probably want to remove this when you write your actual code;
// we need to import userlib to get this to compile, but it throws a warning
// because we're not actually using it yet!
#[allow(unused_imports)]
use userlib::*;

// Test SPDM API import
use drv_spdm_responder_api::{SpdmResponder, SpdmError, SpdmVersion};

task_slot!(UART, uart_driver);
task_slot!(SPDM_RESPONDER, spdm_responder);

#[export_name = "main"]
fn main() -> ! {
    uart_send(b"Hello, world!\r\n");

    // Test SPDM client IPC
    let spdm_client = SpdmResponder::from(SPDM_RESPONDER.get_task_id());
    match spdm_client.get_version() {
        Ok(version_response) => {
            uart_send(b"SPDM IPC Success!\r\n");
            // Simple test to show the data is valid
            if version_response.version_count > 0 {
                uart_send(b"Got version data\r\n");
            }
        }
        Err(_) => {
            uart_send(b"SPDM IPC Failed!\r\n");
        }
    }

    loop {
        let mut buf = [0u8; 128];
        // NOTE: you need to put code here before running this! Otherwise LLVM
        // will turn this into a single undefined instruction.
        hl::sleep_for(1);
        if uart_read(&mut buf) {
            uart_send(b"Received: ");
            uart_send(&buf);
            uart_send(b"\r\n");
        }
    }
    uart_send(b"Goodbye!\r\n");
}

fn uart_send(text: &[u8]) {
    let peer = UART.get_task_id();

    const OP_WRITE: u16 = 1;
    let (code, _) =
        sys_send(peer, OP_WRITE, &[], &mut [], &[Lease::from(text)]);
    assert_eq!(0, code);
}

fn uart_read(text: &mut [u8]) -> bool {
    let peer = UART.get_task_id();
    const OP_READ: u16 = 2;

    /*
    let (code, _) =
        sys_send(peer, OP_READ, &[], text, &mut []);
    */
    let mut response = [0u8; 4];
    let (code, _) = sys_send(peer, OP_READ, &[], &mut response, &mut [Lease::from(text)]);

    code == 0
}
