//! Demo kernel for AST1060 platform
//!
//! This demonstrates basic Hubris functionality on the AST1060 SoC.

#![no_std]
#![no_main]

// We have to do this if we don't otherwise use it to ensure its vector table
// gets linked in.
extern crate ast1060_pac;

use cortex_m_rt::entry;

// Re-export the kernel
pub use kern;

#[entry]
fn main() -> ! {
    // AST1060 SoC typically runs at 400MHz like other high-performance ARM Cortex-M7 systems
    const CYCLES_PER_MS: u32 = 400_000;
    
    unsafe { kern::startup::start_kernel(CYCLES_PER_MS) }
}
