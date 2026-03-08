//! Challenge system for COR24 emulator

use crate::cpu::CpuState;

/// A challenge for the user to complete
#[derive(Clone)]
pub struct Challenge {
    pub id: usize,
    pub name: String,
    pub description: String,
    pub initial_code: String,
    pub hint: String,
    pub validator: fn(&CpuState) -> bool,
}

/// Get all available challenges
pub fn get_challenges() -> Vec<Challenge> {
    vec![
        Challenge {
            id: 1,
            name: "Load and Add".to_string(),
            description: "Load the value 10 into r0, then add 5 to it. Result should be 15 in r0."
                .to_string(),
            initial_code: "; Load 10 into r0, add 5\n; Result: r0 = 15\n\n".to_string(),
            hint: "Use 'lc r0,10' to load 10, then 'add r0,5' to add 5".to_string(),
            validator: |cpu| cpu.get_reg(0) == 15,
        },
        Challenge {
            id: 2,
            name: "Compare and Branch".to_string(),
            description: "Set r0 to 1 if 5 < 10 (signed), otherwise 0. Use cls and brt/brf."
                .to_string(),
            initial_code: "; Compare 5 < 10 and set r0 accordingly\n; Result: r0 = 1\n\n"
                .to_string(),
            hint: "Load values, use cls to compare, then mov r0,c to get the result".to_string(),
            validator: |cpu| cpu.get_reg(0) == 1,
        },
        Challenge {
            id: 3,
            name: "Stack Operations".to_string(),
            description: "Push values 1, 2, 3 onto the stack, then pop them into r0, r1, r2."
                .to_string(),
            initial_code: "; Push 1, 2, 3 then pop into r0, r1, r2\n; Result: r0=3, r1=2, r2=1\n\n"
                .to_string(),
            hint: "Remember LIFO order - last pushed is first popped".to_string(),
            validator: |cpu| cpu.get_reg(0) == 3 && cpu.get_reg(1) == 2 && cpu.get_reg(2) == 1,
        },
        Challenge {
            id: 4,
            name: "Max of Two".to_string(),
            description: "Set r0 to the maximum of r0=7 and r1=12 (without branching). Use mov ra,c!"
                .to_string(),
            initial_code: "; Find max of r0=7 and r1=12, store result in r0\n; Hint: Use COR24's mov ra,c feature\n; Result: r0 = 12\n\n        lc      r0,7\n        lc      r1,12\n\n        ; Your code here\n\nhalt:   bra     halt\n"
                .to_string(),
            hint: "cls sets C if r0 < r1. If true, you want r1. Use sub/add with C flag.".to_string(),
            validator: |cpu| cpu.get_reg(0) == 12,
        },
        Challenge {
            id: 5,
            name: "Byte Sign Extension".to_string(),
            description: "Load -50 (0xCE) as unsigned into r0, then sign-extend it. Result should be 0xFFFFCE."
                .to_string(),
            initial_code: "; Load 0xCE unsigned, then sign extend\n; Result: r0 = 0xFFFFCE (-50)\n\n"
                .to_string(),
            hint: "Use lcu to load unsigned, then sxt to sign extend".to_string(),
            validator: |cpu| cpu.get_reg(0) == 0xFFFFCE,
        },
    ]
}

/// Get example programs
pub fn get_examples() -> Vec<(String, String, String)> {
    vec![
        (
            "Basic Arithmetic".to_string(),
            "Load values and perform arithmetic operations.".to_string(),
            r#"; Example 1: Basic Arithmetic
; Load constants and add them

        lc      r0,10       ; r0 = 10
        lc      r1,20       ; r1 = 20
        add     r0,r1       ; r0 = r0 + r1 = 30

        lc      r2,5        ; r2 = 5
        add     r0,r2       ; r0 = 35

halt:   bra     halt        ; Stop execution
"#
            .to_string(),
        ),
        (
            "Compare and Branch".to_string(),
            "Compare values and branch based on results.".to_string(),
            r#"; Example 2: Compare and Branch
; Compare two values and branch if less than

        lc      r0,5        ; r0 = 5
        lc      r1,10       ; r1 = 10

        cls     r0,r1       ; C = (r0 < r1) = true
        brt     less        ; Branch if true

        lc      r2,0        ; Not taken
        bra     done

less:   lc      r2,1        ; r2 = 1 (5 < 10)

done:   bra     done
"#
            .to_string(),
        ),
        (
            "Stack Frame".to_string(),
            "Set up a stack frame like a C function.".to_string(),
            r#"; Example 3: Stack Frame
; Simulate a C function prologue/epilogue

        ; Function entry
        push    fp          ; Save frame pointer
        push    r2          ; Save callee-saved reg
        push    r1          ; Save return address
        mov     fp,sp       ; Set up frame pointer

        ; Function body
        lc      r0,42       ; Return value

        ; Function exit
        mov     sp,fp       ; Restore stack
        pop     r1          ; Restore r1
        pop     r2          ; Restore r2
        pop     fp          ; Restore fp

halt:   bra     halt        ; (would be jmp (r1))
"#
            .to_string(),
        ),
        (
            "Loop Counter".to_string(),
            "Count from 0 to 5 using a loop.".to_string(),
            r#"; Example 4: Loop Counter
; Count from 0 to 5

        lc      r0,0        ; r0 = counter = 0
        lc      r1,5        ; r1 = limit = 5

loop:   add     r0,1        ; counter++
        cls     r0,r1       ; C = (counter < limit)
        brt     loop        ; Continue if less

        ; r0 = 5 when done
halt:   bra     halt
"#
            .to_string(),
        ),
        (
            "Memory Access".to_string(),
            "Store and load values from memory.".to_string(),
            r#"; Example 5: Memory Access
; Store values to memory and read them back

        lc      r0,100      ; Value to store
        la      r1,0x1000   ; Load 24-bit address (la, not lc!)

        ; Store byte
        sb      r0,0(r1)    ; mem[0x1000] = 100

        ; Load it back
        lb      r2,0(r1)    ; r2 = mem[0x1000]

        ; r2 should be 100
halt:   bra     halt
"#
            .to_string(),
        ),
        (
            "Condition Flag".to_string(),
            "COR24's unique mov ra,c to capture comparison result.".to_string(),
            r#"; Example 6: Condition Flag
; COR24 can move the C flag directly to a register
; This is useful for branchless comparisons

        lc      r0,5        ; r0 = 5
        lc      r1,10       ; r1 = 10

        cls     r0,r1       ; C = (5 < 10) = 1
        mov     r2,c        ; r2 = C = 1 (no branch needed!)

        lc      r0,20       ; r0 = 20
        cls     r0,r1       ; C = (20 < 10) = 0
        mov     r0,c        ; r0 = C = 0

halt:   bra     halt
"#
            .to_string(),
        ),
        (
            "Sign Extension".to_string(),
            "Demonstrate sxt/zxt for byte-to-word conversion.".to_string(),
            r#"; Example 7: Sign Extension
; COR24 is 24-bit but loads 8-bit values
; sxt/zxt extend bytes to full 24-bit words

        lc      r0,127      ; r0 = 0x00007F (positive)
        lc      r1,-1       ; r1 = 0xFFFFFF (sign extended)

        lcu     r2,255      ; r2 = 0x0000FF (unsigned)

        ; Store byte and reload with different extension
        la      r0,0x100
        lc      r1,0x80     ; -128 signed, 128 unsigned
        sb      r1,0(r0)    ; Store byte

        lb      r1,0(r0)    ; r1 = 0xFFFF80 (sign extended)
        lbu     r2,0(r0)    ; r2 = 0x000080 (zero extended)

halt:   bra     halt
"#
            .to_string(),
        ),
        (
            "24-bit Arithmetic".to_string(),
            "Working with COR24's 24-bit word size.".to_string(),
            r#"; Example 8: 24-bit Arithmetic
; COR24 uses 24-bit (3-byte) words
; Max unsigned: 0xFFFFFF = 16,777,215
; Max signed: 0x7FFFFF = 8,388,607

        la      r0,0x7FFFFF ; Max positive signed
        lc      r1,1
        add     r0,r1       ; Overflow to 0x800000 (negative)

        la      r0,0xFFFFFF ; Max unsigned
        add     r0,r1       ; Wraps to 0x000000

        ; 24-bit multiplication
        la      r0,0x100    ; 256
        lc      r1,16
        mul     r0,r1       ; r0 = 4096 (0x1000)

halt:   bra     halt
"#
            .to_string(),
        ),
        (
            "LED Blink".to_string(),
            "Memory-mapped I/O: Control LEDs at address 0xFF0000.".to_string(),
            r#"; Example 9: LED Blink
; COR24 uses memory-mapped I/O
; LEDs at 0xFF0000 (write), Switches at 0xFF0000 (read)
;
; This program cycles through LED patterns.
; Click Step repeatedly to watch LEDs change!
; Toggle switches in the I/O panel to see input.

        la      r1,0xFF0000 ; I/O address (LEDSWDAT)
        lc      r0,1        ; Start with LED 0

loop:
        sb      r0,0(r1)    ; Write to LEDs

        ; Shift LED pattern left
        lc      r2,1
        shl     r0,r2

        ; Check if we've shifted past bit 7
        lcu     r2,128      ; 0x80
        clu     r2,r0       ; C = (128 < pattern)?
        brf     loop        ; If not overflowed, continue

        ; Reset to LED 0
        lc      r0,1
        bra     loop

halt:   bra     halt        ; Never reached
"#
            .to_string(),
        ),
        (
            "Button Echo (blinky)".to_string(),
            "LED D2 follows button S2 - matches COR24-TB hardware demo.".to_string(),
            r#"; Example 10: Button Echo (blinky)
; LED D2 follows button S2
; Click S2 button in I/O panel while running
;
; Matches reference blinky.c: *ledsw = *ledsw
; Hardware: S2 = bit 0 read, D2 = bit 0 write

        la      r1,0xFF0000 ; I/O address (LEDSWDAT)

loop:
        lb      r0,0(r1)    ; Read button S2 (bit 0)
        sb      r0,0(r1)    ; Write to LED D2 (bit 0)

        bra     loop        ; Keep polling

halt:   bra     halt        ; Never reached
"#
            .to_string(),
        ),
    ]
}
