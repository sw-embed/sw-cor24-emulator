//! Demo: Blink LED
//! Toggle LED D2 on/off with a delay loop.
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
pub unsafe fn demo_blinky() -> ! {
    loop {
        mmio_write(LED_ADDR, 1);   // LED on
        delay(5000);
        mmio_write(LED_ADDR, 0);   // LED off
        delay(5000);
    }
}

/// Entry point: called via reset vector (`la r0, start` + `jmp (r0)` at address 0)
#[inline(never)]
#[no_mangle]
pub unsafe fn start() -> ! {
    demo_blinky()
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
