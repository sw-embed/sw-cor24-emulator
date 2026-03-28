//! Demo: Button Echo
//! Reads button S2 state and echoes it to LED D2 in a tight loop.
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

#[no_mangle]
pub unsafe fn demo_button_echo() -> ! {
    loop {
        let btn = mmio_read(LED_ADDR);  // S2: 0=pressed, 1=released (active-low)
        mmio_write(LED_ADDR, btn);      // LED: 0=ON, 1=OFF (active-low, direct copy)
    }
}

/// Entry point
#[inline(never)]
#[no_mangle]
pub unsafe fn start() -> ! {
    demo_button_echo()
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
