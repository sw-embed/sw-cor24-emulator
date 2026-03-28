; Button Echo: LED follows button state
; LED D2 lights when button S2 is pressed
; Click S2 button in I/O panel while running
;
; Both S2 and LED D2 are active-low:
;   S2: 0=pressed, 1=released
;   LED: 0=ON, 1=OFF
; Direct copy works (no inversion needed)

        la      r1,-65536   ; I/O address (LEDSWDAT)

loop:
        lb      r0,0(r1)    ; Read button S2 (bit 0: 0=pressed, 1=released)
        sb      r0,0(r1)    ; Write to LED D2 (bit 0: 0=ON, 1=OFF)

        bra     loop        ; Keep polling

halt:
        bra     halt        ; Never reached
