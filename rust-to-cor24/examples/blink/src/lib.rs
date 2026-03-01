//! LED Blink - Minimal no_std Rust for COR24
//!
//! This compiles to WASM, then translates to COR24 assembly.

#![no_std]

use core::panic::PanicInfo;

/// Memory-mapped LED register (COR24 I/O address)
const LEDS: *mut u8 = 0xFF0000 as *mut u8;

/// Panic handler - required for no_std
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

/// Main entry point - exported for WASM
#[no_mangle]
pub extern "C" fn main() {
    let mut counter: u8 = 0;

    loop {
        // Write counter value to LEDs
        unsafe {
            core::ptr::write_volatile(LEDS, counter);
        }

        // Increment (wraps at 256)
        counter = counter.wrapping_add(1);

        // Stop after 16 iterations for demo
        if counter >= 16 {
            break;
        }
    }
}

/// Simple add function - for testing basic translation
#[no_mangle]
pub extern "C" fn add(a: i32, b: i32) -> i32 {
    a + b
}
