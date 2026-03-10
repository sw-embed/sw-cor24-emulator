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
            "Add".to_string(),
            "Compute 100 + 200 + 42 = 342, return in r0".to_string(),
            r#"; Add: Compute 100 + 200 + 42 = 342
; Result in r0

        lc      r0,100      ; r0 = 100
        lcu     r1,200      ; r1 = 200 (unsigned, >127)
        add     r0,r1       ; r0 = 300
        lc      r1,42       ; r1 = 42
        add     r0,r1       ; r0 = 342 (0x156)

halt:   bra     halt
"#
            .to_string(),
        ),
        (
            "Blink LED".to_string(),
            "Toggle LED with delay loop".to_string(),
            r#"; Blink LED: Toggle LED D2 on and off
; LED D2 at 0xFF0000 (write bit 0)
; Click Run to watch the LED blink!

        la      r1,0xFF0000

loop:
        lc      r0,1
        sb      r0,0(r1)

        push    r1
        lc      r1,10
delay1: lc      r2,0
wait1:  lc      r0,1
        add     r2,r0
        lc      r0,127
        clu     r2,r0
        brt     wait1
        lc      r0,1
        sub     r1,r0
        ceq     r1,z
        brf     delay1
        pop     r1

        lc      r0,0
        sb      r0,0(r1)

        push    r1
        lc      r1,10
delay2: lc      r2,0
wait2:  lc      r0,1
        add     r2,r0
        lc      r0,127
        clu     r2,r0
        brt     wait2
        lc      r0,1
        sub     r1,r0
        ceq     r1,z
        brf     delay2
        pop     r1

        bra     loop

halt:   bra     halt
"#
            .to_string(),
        ),
        (
            "Button Echo".to_string(),
            "LED D2 follows button S2".to_string(),
            r#"; Button Echo: LED follows button state
; LED D2 lights when button S2 is pressed
; Click S2 button in I/O panel while running
;
; S2 is active-low (normally 1, pressed = 0)
; We invert with XOR so LED on = button pressed

        la      r1,0xFF0000 ; I/O address (LEDSWDAT)
        lc      r2,1        ; Bit mask for XOR

loop:
        lb      r0,0(r1)    ; Read button S2 (bit 0: 1=released, 0=pressed)
        xor     r0,r2       ; Invert: pressed(0)->1(LED on), released(1)->0(LED off)
        sb      r0,0(r1)    ; Write to LED D2 (bit 0)

        bra     loop        ; Keep polling

halt:   bra     halt        ; Never reached
"#
            .to_string(),
        ),
        (
            "Countdown".to_string(),
            "Count 10→0 on LED, then halt".to_string(),
            r#"; Countdown: Display 10 down to 0 on LED
; Writes count to LED register, delays, decrements

        la      r1,0xFF0000 ; LED address
        lc      r0,10       ; Start at 10

loop:   sb      r0,0(r1)    ; Write count to LED

        ; Delay loop
        push    r0
        lc      r2,0
wait:   add     r2,1
        lc      r0,127
        clu     r2,r0
        brt     wait
        pop     r0

        sub     r0,1        ; count--
        ceq     r0,z        ; count == 0?
        brf     loop        ; Continue if not zero

        ; Clear LED and halt
        lc      r0,0
        sb      r0,0(r1)
halt:   bra     halt
"#
            .to_string(),
        ),
        (
            "Fibonacci".to_string(),
            "Compute fib(10) = 55, display on LED".to_string(),
            r#"; Fibonacci: Compute fib(10) = 55
; Iterative: a=0, b=1, repeat 9 times: tmp=a+b, a=b, b=tmp
; Result (55) displayed on LED

        lc      r0,0            ; a = 0
        lc      r1,1            ; b = 1
        lc      r2,9            ; 9 iterations for fib(10)

loop:   push    r1              ; save old b
        add     r1,r0           ; b = a + b (new fib value)
        pop     r0              ; a = old b
        sub     r2,1            ; counter--
        ceq     r2,z
        brf     loop            ; continue if counter != 0

        ; r1 = fib(10) = 55, display on LED
        la      r0,0xFF0000
        sb      r1,0(r0)

halt:   bra     halt
"#
            .to_string(),
        ),
        (
            "Memory Access".to_string(),
            "Store and load values from memory".to_string(),
            r#"; Memory Access: Store and load values
; Store values to memory and read them back

        lc      r0,100      ; Value to store
        la      r1,0x0080   ; Address just past program area

        ; Store byte
        sb      r0,0(r1)    ; mem[0x0080] = 100

        ; Store word (3 bytes)
        sw      r0,4(r1)    ; mem[0x0084..87] = 100

        ; Load them back
        lb      r2,0(r1)    ; r2 = mem[0x0080] = 100
        lw      r2,4(r1)    ; r2 = mem[0x0084] = 100

halt:   bra     halt
"#
            .to_string(),
        ),
        (
            "Nested Calls".to_string(),
            "Function call chain showing stack frames".to_string(),
            r#"; Nested Calls: 3-level function call chain
; main -> level_a -> level_b, showing stack frames
; Result: r0 = ((5 + 10) * 2) + 3 = 33

        ; --- main ---
        lc      r0,5            ; arg = 5
        la      r1,ret_a        ; return address
        la      r2,level_a
        jal     r1,(r2)         ; call level_a(5)
ret_a:
halt:   bra     halt            ; r0 = 33

        ; --- level_a(x): returns level_b(x + 10) ---
level_a:
        push    fp
        push    r1              ; save return addr
        mov     fp,sp
        add     r0,10           ; x + 10 = 15
        la      r1,ret_b
        la      r2,level_b
        jal     r1,(r2)         ; call level_b(15)
ret_b:  mov     sp,fp
        pop     r1              ; restore return addr
        pop     fp
        jmp     (r1)            ; return

        ; --- level_b(x): returns x * 2 + 3 ---
level_b:
        push    fp
        push    r1              ; save return addr
        mov     fp,sp
        add     r0,r0           ; x * 2 = 30
        add     r0,3            ; + 3 = 33
        mov     sp,fp
        pop     r1
        pop     fp
        jmp     (r1)            ; return
"#
            .to_string(),
        ),
        (
            "Stack Variables".to_string(),
            "Local variables and register spilling".to_string(),
            r#"; Stack Variables: Local vars on the stack
; Demonstrates register spilling via push/pop
;
; Computes: a=seed+1, b=a+seed, c=b+a, result=a^b^c
; with seed=7: a=8, b=15, c=23, result=8^15^23=16

        lc      r0,7            ; seed = 7
        la      r1,ret_main
        la      r2,compute
        jal     r1,(r2)         ; call compute(7)
ret_main:
        ; r0 = result (16 = 0x10)
        la      r1,0xFF0000
        sb      r0,0(r1)        ; Display on LED
halt:   bra     halt

        ; --- compute(seed in r0) ---
        ; Uses r0-r2 for values, spills to stack when
        ; we run out of registers
compute:
        push    r1              ; spill return addr

        ; a = seed + 1
        mov     r1,r0           ; r1 = seed (keep copy)
        add     r0,1            ; r0 = a = 8

        ; b = a + seed
        mov     r2,r0           ; r2 = a (save)
        add     r0,r1           ; r0 = b = a + seed = 15

        ; c = b + a  (need a, but r2 has it)
        push    r0              ; spill b — out of regs
        add     r0,r2           ; r0 = c = b + a = 23

        ; result = a ^ b ^ c
        xor     r2,r0           ; r2 = a ^ c
        pop     r0              ; restore b
        xor     r2,r0           ; r2 = a ^ c ^ b = 16
        mov     r0,r2           ; r0 = result

        pop     r1              ; restore return addr
        jmp     (r1)
"#
            .to_string(),
        ),
        (
            "UART Hello".to_string(),
            "Write \"Hello\\n\" to UART output".to_string(),
            r#"; UART Hello: Send "Hello\n" via UART
; UART data register at 0xFF0100
; Write one byte at a time

        la      r1,0xFF0100     ; UART data address

        lc      r0,72           ; 'H'
        sb      r0,0(r1)
        lc      r0,101          ; 'e'
        sb      r0,0(r1)
        lc      r0,108          ; 'l'
        sb      r0,0(r1)
        lc      r0,108          ; 'l'
        sb      r0,0(r1)
        lc      r0,111          ; 'o'
        sb      r0,0(r1)
        lc      r0,10           ; '\n'
        sb      r0,0(r1)

halt:   bra     halt
"#
            .to_string(),
        ),
    ]
}
