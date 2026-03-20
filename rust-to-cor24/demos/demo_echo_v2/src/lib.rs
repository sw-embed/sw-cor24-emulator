//! Demo: Echo v2 (Rust Logic + Interrupt Plumbing)
//! Same behavior as Echo: letters→uppercase, '!'→halt, others as-is.
//!
//! Difference from Echo: application logic is in Rust (to_upper, halt check,
//! UART I/O). Only the interrupt-specific plumbing uses asm!() passthrough:
//!   - ISR prologue: save registers + condition flag
//!   - ISR epilogue: restore + jmp (ir)
//!   - Setting iv register and enabling interrupts
//!   - Halt (self-branch)
//!
//! Pipeline: this file → rustc (msp430) → .msp430.s → msp430-to-cor24 → .cor24.s

#![no_std]
#![feature(asm_experimental_arch)]

const UART_DATA: u16 = 0xFF01;
/// Halt flag address — ISR writes 1 here on '!', main loop checks it
const HALT_FLAG: u16 = 0x0100;

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

/// Convert character to uppercase. Pure Rust — no asm!.
#[inline(never)]
#[no_mangle]
pub fn to_upper(ch: u16) -> u16 {
    if ch >= 0x61 && ch <= 0x7A {
        ch & 0xDF // clear bit 5
    } else {
        ch
    }
}

/// Handle one received character. Pure Rust — reads UART, processes, writes.
/// Sets halt flag at HALT_FLAG address if '!' received.
#[inline(never)]
#[no_mangle]
pub unsafe fn handle_rx() {
    let ch = mmio_read(UART_DATA); // read UART RX (acknowledges interrupt)
    if ch == 0x21 {
        // '!' — signal halt via memory flag
        mmio_write(HALT_FLAG, 1);
    } else {
        uart_putc(to_upper(ch));
    }
}

/// ISR: thin asm!() wrapper that saves/restores COR24 state around handle_rx().
/// Only the interrupt plumbing is in asm! — the logic is in Rust above.
#[no_mangle]
pub unsafe fn isr_handler() {
    // Save COR24 registers and condition flag (asm! — no Rust equivalent)
    core::arch::asm!(
        "; @cor24: push r0",
        "; @cor24: push r1",
        "; @cor24: push r2",
        "; @cor24: mov r2, c",
        "; @cor24: push r2",
    );

    // Application logic — pure Rust, translated through normal pipeline
    handle_rx();

    // Restore COR24 state and return from interrupt (asm! — no Rust equivalent)
    core::arch::asm!(
        "; @cor24: pop r2",
        "; @cor24: clu z, r2",
        "; @cor24: pop r2",
        "; @cor24: pop r1",
        "; @cor24: pop r0",
        "; @cor24: jmp (ir)",
        options(noreturn)
    );
}

#[no_mangle]
pub unsafe fn start() -> ! {
    // Print prompt
    uart_putc(b'?' as u16);

    // Set interrupt vector and enable UART RX interrupt (asm! — COR24 specific)
    core::arch::asm!(
        "; @cor24: la r0, isr_handler",
        "; @cor24: mov iv, r0",
        "; @cor24: lc r0, 1",
        "; @cor24: la r1, -65520",
        "; @cor24: sb r0, 0(r1)",
    );

    // Main loop: check halt flag, otherwise idle
    loop {
        if mmio_read(HALT_FLAG) != 0 {
            core::arch::asm!(
                "; @cor24: halted:",
                "; @cor24: bra halted",
                options(noreturn)
            );
        }
        core::arch::asm!("nop");
    }
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
