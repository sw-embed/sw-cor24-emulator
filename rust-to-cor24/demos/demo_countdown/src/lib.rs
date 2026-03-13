//! Demo: Countdown
//! Counts down from 10 to 0, writing each value to LED register.
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
pub fn delay(mut n: u16) {
    while n != 0 {
        unsafe { core::ptr::write_volatile(&mut n as *mut u16, n - 1); }
    }
}

#[inline(never)]
#[no_mangle]
pub unsafe fn uart_putc(ch: u16) {
    mmio_write(UART_DATA, ch);
}

#[no_mangle]
pub unsafe fn demo_countdown() {
    let mut count: u16 = 10;
    while count != 0 {
        mmio_write(LED_ADDR, count);
        delay(1000);
        count -= 1;
    }
    mmio_write(LED_ADDR, 0);
    loop {}
}

/// Entry point
#[inline(never)]
#[no_mangle]
pub unsafe fn start() -> ! {
    demo_countdown();
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
