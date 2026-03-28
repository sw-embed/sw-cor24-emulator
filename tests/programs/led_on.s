; led_on.s - Turn on LED D2
; LED I/O at 0xFF0000, bit 0 controls LED D2 (active-low: 0=ON, 1=OFF)
; Expected: LED register = 0x00 (ON)

_main:
	la	r0,-65536	; 0xFF0000 = LED I/O address
	lc	r1,0		; bit 0 = 0 → LED on (active-low)
	sb	r1,0(r0)	; write to LED register
_halt:
	bra	_halt		; spin forever (test runner stops on cycle limit)
