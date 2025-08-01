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

task_slot!(UART, uart_driver);

#[export_name = "main"]
fn main() -> ! {
    loop {
        // NOTE: you need to put code here before running this! Otherwise LLVM
        // will turn this into a single undefined instruction.
        uart_send(b"Hello, world!\r\n");
        hl::sleep_for(1000);
    }
}

fn uart_send(text: &[u8]) {
    let peer = UART.get_task_id();

    const OP_WRITE: u16 = 1;
    let (code, _) =
        sys_send(peer, OP_WRITE, &[], &mut [], &[Lease::from(text)]);
    assert_eq!(0, code);
}
