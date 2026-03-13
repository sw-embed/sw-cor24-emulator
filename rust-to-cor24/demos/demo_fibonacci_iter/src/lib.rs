//! Demo: Fibonacci (iterative)
//! Computes fib(10) = 89 using a simple loop with only 3 variables.
//! Iterative style avoids deep recursion and needs minimal registers.
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

/// Iterative fibonacci: fib(0)=1, fib(1)=1, fib(n)=fib(n-1)+fib(n-2)
/// Uses only 3 live variables (a, b, temp) — fits in COR24's 3 GP registers.
#[inline(never)]
#[no_mangle]
pub fn fibonacci_iter(n: u16) -> u16 {
    let mut a: u16 = 1;
    let mut b: u16 = 1;
    let mut i: u16 = 0;
    while i < n {
        let temp = a + b;
        a = b;
        b = temp;
        i += 1;
    }
    a
}

#[inline(never)]
#[no_mangle]
pub unsafe fn demo_fibonacci_iter() {
    let result = fibonacci_iter(10);  // Should be 89
    mmio_write(LED_ADDR, result);
    loop {}
}

/// Entry point
#[inline(never)]
#[no_mangle]
pub unsafe fn start() -> ! {
    demo_fibonacci_iter();
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
