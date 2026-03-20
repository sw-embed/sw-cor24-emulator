; UART Echo: Interrupt-driven character echo
; Letters (a-z, A-Z): echoes uppercase (a->A, B->B)
; '!' halts the program
; Everything else: echoes as-is (1->1, ?->?)
;
; Usage: Assemble & Run, then type in the UART RX input.
; Each keystroke triggers an interrupt that echoes to UART TX.

; --- Setup interrupt vector ---
        la      r0, isr
        mov     iv, r0          ; iv = ISR address

; --- Enable UART RX interrupt ---
        la      r1, -65520
        lc      r0, 1
        sb      r0, 0(r1)       ; enable UART RX interrupt

; --- Print prompt ---
        la      r1, -65280
.txp:
        lb      r2, 1(r1)       ; poll TX busy
        cls     r2, z
        brt     .txp
        lc      r0, 63          ; '?'
        sb      r0, 0(r1)       ; transmit prompt

; --- Main loop: spin forever (two instructions to avoid halt detection) ---
idle:
        lc      r0, 0
        bra     idle

; --- Interrupt Service Routine ---
isr:
        push    r0
        push    r1
        push    r2
        mov     r2, c
        push    r2

        ; Read UART RX byte (acknowledges interrupt)
        la      r1, -65280
        lb      r0, 0(r1)      ; r0 = received character

        ; Check for '!' (33) -> halt
        mov     r2, r0
        lc      r0, 33
        ceq     r0, r2
        brt     do_halt

        ; Check if lowercase letter: 'a'-'z' (97-122)
        lc      r0, 97          ; 'a'
        clu     r2, r0          ; char < 'a'?
        brt     not_lower       ; yes -> not lowercase
        lc      r0, 123         ; 'z'+1
        clu     r2, r0          ; char < 'z'+1?
        brf     not_lower       ; no -> not lowercase

        ; Lowercase letter: convert to uppercase (clear bit 5)
        mov     r0, r2
        lcu     r1, 223         ; mask to clear bit 5
        and     r0, r1          ; r0 = uppercase version
        la      r1, -65280
.tx1:
        lb      r2, 1(r1)      ; poll TX busy
        cls     r2, z
        brt     .tx1
        sb      r0, 0(r1)      ; transmit uppercase
        bra     isr_done

not_lower:
        ; Not lowercase — echo as-is (already uppercase or non-letter)
        la      r1, -65280
.tx2:
        lb      r0, 1(r1)      ; poll TX busy
        cls     r0, z
        brt     .tx2
        sb      r2, 0(r1)      ; transmit original character

isr_done:
        ; Restore registers
        pop     r2
        clu     z, r2           ; restore condition flag
        pop     r2
        pop     r1
        pop     r0
        jmp     (ir)            ; return from interrupt

do_halt:
        bra     do_halt         ; halt on '!'
