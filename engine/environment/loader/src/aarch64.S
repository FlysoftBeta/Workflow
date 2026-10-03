.text
.globl wf_finish
.hidden wf_finish
.type wf_finish,%function
wf_finish:
	mov sp, x0
	mov x4, x0
	mov x5, x1
	lsr x6, x2, #3
1:	cbz x6, 2f
	ldr x7, [x5], #8
	str x7, [x4], #8
	sub x6, x6, #1
	b 1b
2:	mov x0, x1
	mov x1, x2
	mov x8, #215
	svc #0
	mov x0, #0
	mov x1, #0
	mov x2, #0
	mov x4, #0
	mov x5, #0
	mov x6, #0
	mov x7, #0
	mov x8, #0
	mov x29, #0
	mov x30, #0
	br x3
.globl _start
.type _start,%function
_start:
	mov x29, #0
	mov x30, #0
	mov x0, sp
	bl loader_main
	brk #0
