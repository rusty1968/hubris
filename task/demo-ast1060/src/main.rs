//! AST1060 Demo Task
//!
//! This task demonstrates the capabilities of the AST1060 platform.
//! This is a simplified version that focuses on basic functionality.

#![no_std]
#![no_main]

use userlib::*;

// Define task slots
task_slot!(HMAC_HASH, hmac_hash);

struct Counters {
    timer_ticks: u32,
    demo_runs: u32,
}

static mut COUNTERS: Counters = Counters {
    timer_ticks: 0,
    demo_runs: 0,
};

#[export_name = "main"]
fn main() -> ! {
    // Wait a bit for the system to stabilize
    hl::sleep_for(1000);
    
    // Main loop - basic demonstration
    loop {
        // Wait for timer notification
        let _ = sys_recv_notification(notifications::TIMER_MASK);
        
        unsafe {
            COUNTERS.timer_ticks += 1;
        }
        
        // Run basic demo every 10 ticks
        if unsafe { COUNTERS.timer_ticks % 10 == 0 } {
            demo_basic_functionality();
            
            unsafe {
                COUNTERS.demo_runs += 1;
            }
        }
        
        // Request timer notification after 10ms
        sys_set_timer(Some(10), notifications::TIMER_MASK);
    }
}

fn demo_basic_functionality() {
    // Demonstrate basic calculations
    let test_value = unsafe { COUNTERS.timer_ticks };
    let _result = test_value.wrapping_mul(123).wrapping_add(456);
    
    // Demonstrate memory operations
    let mut buffer = [0u8; 32];
    for i in 0..buffer.len() {
        buffer[i] = (i as u8).wrapping_add(test_value as u8);
    }
    
    // Simple checksum calculation (software-based for demo)
    let _checksum = buffer.iter().fold(0u32, |acc, &x| acc.wrapping_add(x as u32));
    
    // Task communication test (if HMAC_HASH is available)
    let _hmac_hash_task = HMAC_HASH.get_task_id();
    
    // Demo completed - in a real implementation, you might:
    // - Send results via IPC
    // - Update LEDs or other hardware indicators
    // - Log to a debug interface
    // - Store results in shared memory
}

// Simple notification handling
mod notifications {
    pub const TIMER_MASK: u32 = 1 << 0;
}

// Task notification handling
#[no_mangle]
extern "C" fn handle_timer_irq() {
    // Timer interrupt handling - kept simple for now
}
