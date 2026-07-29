	.text
_putchr:
	push	fp
	push	r2
	push	r1
	mov	fp,sp
; line 7, file "sieve.c"
	la	r2,-65280
L14:
; line 10, file "sieve.c"
	lb	r0,1(r2)
	lc	r1,2
	and	r0,r1
	clu	z,r0
	brf	L14
L16:
; line 13, file "sieve.c"
	lb	r0,1(r2)
	cls	r0,z
	brt	L16
; line 16, file "sieve.c"
	lb	r0,9(fp)
	sb	r0,(r2)
	mov	sp,fp
	pop	r1
	pop	r2
	pop	fp
	jmp	(r1)
_printn:
	push	fp
	push	r2
	push	r1
	mov	fp,sp
	add	sp,-9
	lw	r2,9(fp)
; line 23, file "sieve.c"
	lc	r0,0
	sw	r0,-6(fp)
L20:
; line 24, file "sieve.c"
	lw	r0,-6(fp)
	lc	r1,3
	cls	r0,r1
	brf	L21
; line 25, file "sieve.c"
	lc	r0,0
	sw	r0,-9(fp)
; line 26, file "sieve.c"
	lw	r0,-6(fp)
	mov	r1,r0
	add	r1,r1
	add	r0,r1
	la	r1,_divdec
	add	r0,r1
	lw	r0,(r0)
	sw	r0,-3(fp)
L22:
; line 27, file "sieve.c"
	lw	r0,-3(fp)
	cls	r2,r0
	brt	L23
; line 28, file "sieve.c"
	lw	r0,-3(fp)
	sub	r2,r0
; line 29, file "sieve.c"
	lw	r0,-9(fp)
	add	r0,1
	sw	r0,-9(fp)
	bra	L22
L23:
; line 31, file "sieve.c"
	lw	r0,-9(fp)
	add	r0,48
	push	r0
	la	r0,_putchr
	jal	r1,(r0)
	add	sp,3
; line 32, file "sieve.c"
	lw	r0,-6(fp)
	add	r0,1
	sw	r0,-6(fp)
	bra	L20
L21:
; line 34, file "sieve.c"
	mov	r0,r2
	add	r0,48
	push	r0
	la	r0,_putchr
	jal	r1,(r0)
	mov	sp,fp
	pop	r1
	pop	r2
	pop	fp
	jmp	(r1)
_putstr:
	push	fp
	push	r2
	push	r1
	mov	fp,sp
	lw	r2,9(fp)
L26:
; line 39, file "sieve.c"
	lb	r0,(r2)
	clu	z,r0
	brf	L27
; line 40, file "sieve.c"
	lb	r0,(r2)
	push	r0
	la	r0,_putchr
	jal	r1,(r0)
	add	sp,3
; line 41, file "sieve.c"
	add	r2,1
	bra	L26
L27:
	mov	sp,fp
	pop	r1
	pop	r2
	pop	fp
	jmp	(r1)

	.globl	_main
_main:
	push	fp
	push	r2
	push	r1
	mov	fp,sp
	add	sp,-12
; line 51, file "sieve.c"
	la	r0,L31
	push	r0
	la	r0,_putstr
	jal	r1,(r0)
	add	sp,3
; line 53, file "sieve.c"
	lc	r0,1
	sw	r0,-12(fp)
L34:
; line 53, file "sieve.c"
	lw	r0,-12(fp)
	la	r1,1000
	cls	r1,r0
	brt	L33
; line 54, file "sieve.c"
	lc	r0,0
	sw	r0,-9(fp)
; line 55, file "sieve.c"
	lc	r2,0
L37:
; line 55, file "sieve.c"
	la	r0,8190
	cls	r0,r2
	brt	L36
; line 56, file "sieve.c"
	la	r0,_flags
	add	r0,r2
	lc	r1,1
	sb	r1,(r0)
; line 56, file "sieve.c"
	add	r2,1
	bra	L37
L36:
; line 57, file "sieve.c"
	lc	r2,0
L40:
; line 57, file "sieve.c"
	la	r0,8190
	cls	r0,r2
	brt	L39
; line 58, file "sieve.c"
	la	r0,_flags
	add	r0,r2
	lb	r0,(r0)
	clu	z,r0
	brf	L41
; line 59, file "sieve.c"
	mov	r0,r2
	add	r0,r2
	add	r0,3
	sw	r0,-3(fp)
; line 60, file "sieve.c"
	lw	r0,-3(fp)
	add	r0,r2
	sw	r0,-6(fp)
L44:
; line 60, file "sieve.c"
	lw	r0,-6(fp)
	la	r1,8190
	cls	r1,r0
	brt	L43
; line 61, file "sieve.c"
	lw	r0,-6(fp)
	la	r1,_flags
	add	r0,r1
	lc	r1,0
	sb	r1,(r0)
; line 61, file "sieve.c"
	lw	r0,-6(fp)
	lw	r1,-3(fp)
	add	r0,r1
	sw	r0,-6(fp)
	bra	L44
L43:
; line 62, file "sieve.c"
	lw	r0,-9(fp)
	add	r0,1
	sw	r0,-9(fp)
L41:
; line 64, file "sieve.c"
	add	r2,1
	bra	L40
L39:
; line 64, file "sieve.c"
	lw	r0,-12(fp)
	add	r0,1
	sw	r0,-12(fp)
	bra	L34
L33:
; line 66, file "sieve.c"
	lw	r0,-9(fp)
	push	r0
	la	r0,_printn
	jal	r1,(r0)
	add	sp,3
; line 67, file "sieve.c"
	la	r0,L45
	push	r0
	la	r0,_putstr
	jal	r1,(r0)
	add	sp,3
; line 69, file "sieve.c"
	lc	r0,0
	mov	sp,fp
	pop	r1
	pop	r2
	pop	fp
	jmp	(r1)

	.data
_divdec:
; line 3, file "sieve.c"
	.word	1000
; line 3, file "sieve.c"
	.word	100
; line 3, file "sieve.c"
	.word	10
	.comm	_flags,8191
L31:
	.byte	49,48,48,48,32,105,116,101
	.byte	114,97,116,105,111,110,115,10
	.byte	0
L45:
	.byte	32,112,114,105,109,101,115,46
	.byte	10,0
