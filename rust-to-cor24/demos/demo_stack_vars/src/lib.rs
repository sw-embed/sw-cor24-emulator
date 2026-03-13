//! Demo: Stack Variables
//! Accumulates values across many variables, forcing register spills to stack.
//! At halt, spill slots contain intermediate values visible in memory dump.
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
pub unsafe fn mmio_read(addr: u16) -> u16 {
    core::ptr::read_volatile(addr as *const u8) as u16
}

#[inline(never)]
#[no_mangle]
pub unsafe fn uart_putc(ch: u16) {
    mmio_write(UART_DATA, ch);
}

#[inline(never)]
#[no_mangle]
pub unsafe fn accumulate(seed: u16) -> u16 {
    let a = seed + 1;
    let b = a + seed;
    let c = b + a;
    let d = c + b;
    let e = d + c;
    let result = a ^ b ^ c ^ d ^ e;
    mmio_write(LED_ADDR, result);
    uart_putc(a);
    uart_putc(b);
    uart_putc(c);
    uart_putc(d);
    uart_putc(e);
    loop {}  // halt with spill slots visible
}

#[inline(never)]
#[no_mangle]
pub unsafe fn demo_stack_vars() {
    let x = mmio_read(LED_ADDR);  // runtime value
    accumulate(x + 1);
}

/// Entry point
#[inline(never)]
#[no_mangle]
pub unsafe fn start() -> ! {
    demo_stack_vars();
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
