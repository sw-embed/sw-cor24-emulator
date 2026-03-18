; Blink LED: Toggle LED D2 at ~1Hz
; Hover over D2 to see duty cycle %
; LED D2 at address -65536 (write bit 0)
;
; At default Run Speed (100/s shown, ~800/s
; effective), the delay loop of 400 gives
; roughly 1 blink per second.
; Edit the delay value to change the rate.

        la      r1,-65536

loop:
        lc      r0,1
        sb      r0,0(r1)    ; LED on

        ; On-time delay
        push    r0
        la      r0,400
on_wait:
        add     r0,-1
        ceq     r0,z
        brf     on_wait
        pop     r0

        lc      r0,0
        sb      r0,0(r1)    ; LED off

        ; Off-time delay
        push    r0
        la      r0,400
off_wait:
        add     r0,-1
        ceq     r0,z
        brf     off_wait
        pop     r0

        bra     loop
