//! Demo: Echo (Interrupts)
//! Interrupt-driven UART echo with uppercase transformation.
//! Pipeline: this file -> rustc (msp430) -> .msp430.s -> msp430-to-cor24 -> .cor24.s -> assembler -> emulator
//!
//! Prints '?' prompt, then echoes typed characters:
//! - Letters (a-z, A-Z) echo as uppercase: 'a' -> "A", 'B' -> "B"
//! - '!' halts the program
//! - All other characters echo as-is: '1' -> "1", '?' -> "?"
//!
//! Uses COR24 UART RX interrupt via asm!() passthrough.
//! The ISR is pure COR24 assembly because the interrupt mechanism
//! (r6=vector, r7=return, jmp (ir) to return) is hardware-specific.

#![no_std]
#![feature(asm_experimental_arch)]

const UART_DATA: u16 = 0xFF01;
const UART_STAT: u16 = 0xFF02;

#[inline(never)]
#[no_mangle]
pub unsafe fn mmio_write(addr: u16, val: u16) {
    core::ptr::write_volatile(addr as *mut u8, val as u8);
}

#[inline(never)]
#[no_mangle]
pub unsafe fn mmio_read(addr: u16) -> u8 {
    core::ptr::read_volatile(addr as *const u8)
}

#[inline(never)]
#[no_mangle]
pub unsafe fn uart_putc(ch: u16) {
    while (mmio_read(UART_STAT) & 0x80) != 0 {}
    mmio_write(UART_DATA, ch);
}

/// ISR: read UART RX, echo uppercase for letters, as-is for others, halt on '!'.
/// Pure COR24 assembly via @cor24 passthrough — the interrupt mechanism
/// (r6/r7 registers, jmp (ir) return) has no MSP430/Rust equivalent.
#[no_mangle]
pub unsafe fn isr_handler() {
    core::arch::asm!(
        // Save registers and condition flag
        "; @cor24: push r0",
        "; @cor24: push r1",
        "; @cor24: push r2",
        "; @cor24: mov r2, c",
        "; @cor24: push r2",
        // Read UART RX byte (acknowledges interrupt)
        "; @cor24: la r1, -65280",
        "; @cor24: lb r0, 0(r1)",
        // Check for '!' (0x21) -> halt
        "; @cor24: mov r2, r0",
        "; @cor24: lc r0, 33",
        "; @cor24: ceq r0, r2",
        "; @cor24: brt do_halt",
        // Check if lowercase: 0x61 ('a') <= ch <= 0x7A ('z')
        "; @cor24: lc r0, 97",
        "; @cor24: clu r2, r0",
        "; @cor24: brt not_lower",
        "; @cor24: lc r0, 123",
        "; @cor24: clu r2, r0",
        "; @cor24: brf not_lower",
        // Lowercase: convert to uppercase (AND 0xDF) and echo
        "; @cor24: mov r0, r2",
        "; @cor24: lcu r1, 223",
        "; @cor24: and r0, r1",
        "; @cor24: la r1, -65280",
        "; @cor24: sb r0, 0(r1)",
        "; @cor24: bra isr_done",
        // Not lowercase: echo character as-is (already uppercase or non-letter)
        "; @cor24: not_lower:",
        "; @cor24: la r1, -65280",
        "; @cor24: sb r2, 0(r1)",
        // Restore registers and return from interrupt
        "; @cor24: isr_done:",
        "; @cor24: pop r2",
        "; @cor24: clu z, r2",
        "; @cor24: pop r2",
        "; @cor24: pop r1",
        "; @cor24: pop r0",
        "; @cor24: jmp (ir)",
        // Halt on '!'
        "; @cor24: do_halt:",
        "; @cor24: bra do_halt",
        options(noreturn)
    );
}

#[no_mangle]
pub unsafe fn start() -> ! {
    // Print prompt
    uart_putc(b'?' as u16);

    // Set interrupt vector to isr_handler
    core::arch::asm!(
        "; @cor24: la r0, isr_handler",
        "; @cor24: mov iv, r0",
    );

    // Enable UART RX interrupt (bit 0 of 0xFF0010)
    core::arch::asm!(
        "; @cor24: lc r0, 1",
        "; @cor24: la r1, -65520",
        "; @cor24: sb r0, 0(r1)",
    );

    // Idle loop (two instructions to avoid halt detection)
    loop {
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
