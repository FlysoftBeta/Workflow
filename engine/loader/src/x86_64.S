.text
.globl wf_finish
.hidden wf_finish
.type wf_finish,@function
wf_finish:
	mov %rdi, %rsp
	mov %rcx, %r8
	mov %rsi, %r9
	mov %rdx, %r10
	mov %rdx, %rcx
	shr $3, %rcx
	cld
	rep movsq
	mov %r9, %rdi
	mov %r10, %rsi
	mov $11, %eax
	syscall
	xor %eax,%eax
	xor %ebx,%ebx
	xor %ecx,%ecx
	xor %edx,%edx
	xor %esi,%esi
	xor %edi,%edi
	xor %ebp,%ebp
	xor %r9d,%r9d
	xor %r10d,%r10d
	xor %r11d,%r11d
	xor %r12d,%r12d
	xor %r13d,%r13d
	xor %r14d,%r14d
	xor %r15d,%r15d
	jmp *%r8
.globl _start
.type _start,@function
_start:
	xor %rbp,%rbp
	mov %rsp,%rdi
	and $-16,%rsp
	call loader_main
	ud2
