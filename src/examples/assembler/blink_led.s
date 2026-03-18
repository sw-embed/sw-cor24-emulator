; Blink LED: Toggle LED D2 on and off
; Watch the LED circle change color as you Step or Run
; LED D2 at address -65536 (write bit 0)

        la      r1,-65536

loop:
        lc      r0,1
        sb      r0,0(r1)    ; LED on

        lc      r0,0
        sb      r0,0(r1)    ; LED off

        bra     loop

halt:
        bra     halt
