//! Demo: Panic Handler
//! Demonstrates the panic handler output on UART.
//! Normal code runs first (turns on LED), then triggers a panic
//! which prints "PANIC\n" to UART and halts.
//! Note: panic!() macro pulls in core::panicking which is too expensive
//! for our pipeline (no linker), so we trigger the handler directly.
//! Pipeline: this file -> rustc (msp430) -> .msp430.s -> msp430-to-cor24 -> .cor24.s -> assembler -> emulator

#![no_std]

const LED_ADDR: u16 = 0xFF00;
const UART_DATA: u16 = 0xFF01;

#[inline(never)]
#[no_mangle]
pub unsafe fn mmio_write(addr: u16, val: u16) {
    core::ptr::write_volatile(addr as *mut u16, val);
}

#[inline(never)]
#[no_mangle]
pub unsafe fn uart_putc(ch: u16) {
    mmio_write(UART_DATA, ch);
}

/// Print "PANIC\n" to UART and halt — same as the panic handler.
/// Separated out so it can be called from demo code without needing
/// the full core::panicking machinery.
#[inline(never)]
#[no_mangle]
pub unsafe fn emit_panic() -> ! {
    uart_putc(b'P' as u16);
    uart_putc(b'A' as u16);
    uart_putc(b'N' as u16);
    uart_putc(b'I' as u16);
    uart_putc(b'C' as u16);
    uart_putc(b'\n' as u16);
    loop {}
}

/// Normal code runs, writes to LED, then triggers panic output.
#[inline(never)]
#[no_mangle]
pub unsafe fn demo_panic() -> ! {
    mmio_write(LED_ADDR, 1);     // Turn on LED D2 (bit 0)
    emit_panic();                 // trigger panic output on UART
}

/// Entry point
#[inline(never)]
#[no_mangle]
pub unsafe fn start() -> ! {
    demo_panic();
    loop {}
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    unsafe { emit_panic() }
}
