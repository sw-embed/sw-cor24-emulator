; LED Counter Demo with Spin Loop Delay
; Counts 0-255 on LEDs, loops forever
;
; Source for the cor24-emu --demo built-in. Pre-assembled into demo.lgo
; via cor24-asm; the binary embeds demo.lgo at build time. Regenerate
; with:
;
;   cor24-asm cli/src/demo.s -o cli/src/demo.lgo

        push    fp
        mov     fp, sp
        add     sp, -3

        la      r1, -65536
        lc      r0, 0
        sw      r0, 0(fp)

main_loop:
        lw      r0, 0(fp)
        sb      r0, 0(r1)

        la      r2, 15000
delay:
        lc      r0, 1
        sub     r2, r0
        brt     delay

        lw      r0, 0(fp)
        lc      r2, 1
        add     r0, r2
        sw      r0, 0(fp)

        bra     main_loop
