//! Demo: Nested Calls
//! Chain of function calls: start → demo_nested → level_a → level_b → level_c
//! At halt, all stack frames are live and visible in memory.
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
pub unsafe fn level_c(x: u16, y: u16) -> u16 {
    mmio_write(LED_ADDR, x);
    uart_putc(y);
    loop {}  // halt with three stack frames live
}

#[inline(never)]
#[no_mangle]
pub unsafe fn level_b(x: u16) -> u16 {
    let doubled = x + x;
    let offset = doubled + 3;
    level_c(offset, x)
}

#[inline(never)]
#[no_mangle]
pub unsafe fn level_a(x: u16) -> u16 {
    let y = x + 10;
    level_b(y)
}

#[inline(never)]
#[no_mangle]
pub unsafe fn demo_nested() {
    let btn = mmio_read(LED_ADDR);  // runtime value prevents constant folding
    level_a(btn + 5);
}

/// Entry point
#[inline(never)]
#[no_mangle]
pub unsafe fn start() -> ! {
    demo_nested();
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
