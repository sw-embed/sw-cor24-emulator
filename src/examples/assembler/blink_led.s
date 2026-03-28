; Blink LED: Toggle LED D2
; Hover D2 to see duty cycle (~50%)
; Use Step to watch each instruction
; Use Run speed slider to control rate
;
; LED D2 is active-low: write 0=ON, 1=OFF
; Try editing nop count to change duty:
;   more ON nops = higher duty cycle
;   more OFF nops = lower duty cycle

        la      r1,-65536   ; LED I/O address

loop:
        lc      r0,0
        sb      r0,0(r1)    ; LED on (active-low: 0=ON)
        ; --- on-time: 5 instructions ---
        nop
        nop
        nop
        nop
        nop

        lc      r0,1
        sb      r0,0(r1)    ; LED off (active-low: 1=OFF)
        ; --- off-time: 4 instructions + bra ---
        nop
        nop
        nop
        nop
        bra     loop
