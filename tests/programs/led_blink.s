; led_blink.s - Blink LED D2 five times, printing 'L' each toggle
; LED I/O at 0xFF0000 (bit 0 = LED D2)
; UART data at 0xFF0100
; Reads button S2 state, toggles LED via XOR, prints 'L' to UART
; After 5 blinks, halts by spinning forever
; Expected output: "LLLLL"

_main:
	push	fp
	mov	fp,sp
	add	sp,-3		; local at -3(fp) = blink counter
	lc	r0,5
	sw	r0,-3(fp)	; blink counter = 5

_blink:
	; Read current LED/button register
	la	r0,-65536	; r0 = 0xFF0000 LED/button I/O
	lb	r1,0(r0)	; r1 = current register value

	; Toggle LED D2 by XORing with 1
	lc	r2,1		; r2 = toggle mask
	xor	r1,r2		; r1 ^= 1 (toggle bit 0)

	; Write new LED state back
	sb	r1,0(r0)	; store to LED register

	; Print 'L' to UART
	la	r2,-65280	; r2 = 0xFF0100 UART data
	lc	r0,76		; r0 = 'L' = 0x4C
	sb	r0,0(r2)	; write to UART

	; Simple delay loop: count down from 100
	lc	r0,100		; r0 = delay counter
_delay:
	add	r0,-1		; r0 -= 1
	ceq	r0,z		; compare r0 to zero
	brf	_delay		; if not zero, keep looping

	; Decrement blink counter
	lw	r0,-3(fp)	; load blink counter
	add	r0,-1		; r0 -= 1
	sw	r0,-3(fp)	; store blink counter
	ceq	r0,z		; compare to zero
	brf	_blink		; if not zero, blink again

	mov	sp,fp
	pop	fp
_halt:
	bra	_halt		; spin forever
