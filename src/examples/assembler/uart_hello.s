; UART Hello: Send "Hello\n" via UART
; UART data at 0xFF0100, status at 0xFF0101
; Poll TX busy (bit 7) before each byte

        lc      r0,72           ; 'H'
        la      r2,putc
        jal     r1,(r2)
        lc      r0,101          ; 'e'
        la      r2,putc
        jal     r1,(r2)
        lc      r0,108          ; 'l'
        la      r2,putc
        jal     r1,(r2)
        lc      r0,108          ; 'l'
        la      r2,putc
        jal     r1,(r2)
        lc      r0,111          ; 'o'
        la      r2,putc
        jal     r1,(r2)
        lc      r0,10           ; '\n'
        la      r2,putc
        jal     r1,(r2)

halt:   bra     halt

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
