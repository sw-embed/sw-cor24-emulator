//! Demo: Fibonacci (recursive)
//! Computes fib(10) = 55 using recursion, matching the reference C implementation.
//! Recursive style uses stack frames naturally — no register spill issues.
//! Pipeline: this file → rustc (msp430) → .msp430.s → msp430-to-cor24 → .cor24.s → assembler → emulator

#![no_std]

const LED_ADDR: u16 = 0xFF00;
const UART_DATA: u16 = 0xFF01;

#[inline(never)]
#[no_mangle]
pub unsafe fn mmio_write(addr: u16, val: u16) {
    core::ptr::write_volatile(addr as *mut u8, val as u8);
}

#[inline(never)]
#[no_mangle]
pub unsafe fn uart_putc(ch: u16) {
    mmio_write(UART_DATA, ch);
}

/// Recursive fibonacci — matches the reference COR24 C implementation (fib.c).
/// Uses stack frames for each recursive call, so only r0-r2 are needed at any point.
#[inline(never)]
#[no_mangle]
pub fn fibonacci(n: u16) -> u16 {
    if n < 2 {
        return 1;
    }
    fibonacci(n - 1) + fibonacci(n - 2)
}

#[inline(never)]
#[no_mangle]
pub unsafe fn demo_fibonacci() {
    let result = fibonacci(10);  // Should be 89
    mmio_write(LED_ADDR, result);
    loop {}
}

/// Entry point
#[inline(never)]
#[no_mangle]
pub unsafe fn start() -> ! {
    demo_fibonacci();
    loop {}
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    unsafe {
        uart_putc(b'P' as u16);
        uart_putc(b'A' as u16);
        uart_putc(b'N' as u16);
        uart_putc(b'I' as u16);
        uart_putc(b'C' as u16);
        uart_putc(b'\n' as u16);
    }
    loop {}
}
