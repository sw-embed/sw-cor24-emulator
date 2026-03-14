; Fibonacci: Print series 1 1 2 3 5 8 13 21 34 55
; UART TX: "1 1 2 3 5 8 13 21 34 55\n"

        lc      r0,0            ; a = 0
        lc      r1,1            ; b = 1
        lc      r2,10           ; 10 iterations

loop:   push    r0              ; save a
        push    r2              ; save counter
        ; print b (current fib number)
        mov     r0,r1           ; r0 = b (value to print)
        push    r1              ; save b
        la      r2,print_num
        jal     r1,(r2)         ; call print_num
        pop     r1              ; restore b
        pop     r2              ; restore counter

        ; print space (unless last iteration)
        push    r0
        lc      r0,1
        ceq     r0,r2           ; 1 == counter? (last)
        pop     r0
        brt     skip_sp
        push    r1
        push    r2
        lc      r0,32           ; ' '
        la      r2,putc
        jal     r1,(r2)         ; call putc
        pop     r2
        pop     r1
skip_sp:
        ; advance: a=old_b, b=old_a+old_b
        pop     r0              ; restore old a
        push    r1              ; save old b
        add     r1,r0           ; b = a + b (new fib)
        pop     r0              ; a = old b

        push    r0
        lc      r0,1
        sub     r2,r0           ; counter--
        pop     r0
        ceq     r2,z
        brf     loop

        ; print newline
        lc      r0,10
        la      r2,putc
        jal     r1,(r2)         ; call putc

halt:   bra     halt

; print_num: print r0 as 1-2 digit decimal
; Uses jal calling convention: r1 = return address
; Clobbers r0, r1, r2
print_num:
        push    r1              ; save return address
        lc      r1,0            ; tens = 0
.div:   lc      r2,10
        clu     r0,r2           ; r0 < 10?
        brt     .ones           ; yes, r0 = ones digit
        sub     r0,r2           ; r0 -= 10
        add     r1,1            ; tens++
        bra     .div
.ones:  push    r0              ; save ones
        ; print tens if nonzero
        ceq     r1,z
        brt     .notens
        push    r1              ; save tens
        lc      r0,48
        add     r0,r1           ; '0' + tens
        la      r2,putc
        jal     r1,(r2)         ; call putc
        pop     r1              ; restore tens (not needed, but clean stack)
.notens:
        pop     r0              ; ones
        lc      r1,48
        add     r0,r1           ; '0' + ones
        la      r2,putc
        jal     r1,(r2)         ; call putc
        pop     r1              ; restore original return address
        jmp     (r1)            ; return to caller

; putc: send byte in r0, polling TX busy first
; Uses jal calling convention: r1 = return address
putc:   push    r1              ; save return address
        push    r0              ; save char
        la      r1,0xFF0100     ; UART base
.wait:  lb      r2,1(r1)        ; read status byte
        lcu     r0,128
        and     r2,r0           ; isolate bit 7
        ceq     r2,z
        brf     .wait           ; spin while TX busy
        pop     r0              ; restore char
        sb      r0,0(r1)        ; transmit byte
        pop     r1              ; restore return address
        jmp     (r1)
