/* Safe Assembly test cases for Keller Verify (TASK-05) */

    .text
    .globl safe_stack_frame
    .type safe_stack_frame, @function
safe_stack_frame:
    subq $32, %rsp
    movq %rax, 8(%rsp)
    movq %rcx, 16(%rsp)
    addq $32, %rsp
    ret

    .globl safe_frame_pointer
    .type safe_frame_pointer, @function
safe_frame_pointer:
    pushq %rbp
    movq %rsp, %rbp
    subq $32, %rsp
    movq %rdi, -8(%rbp)
    movq %rsi, -16(%rbp)
    leave
    ret

    .globl safe_aligned_simd
    .type safe_aligned_simd, @function
safe_aligned_simd:
    pushq %rbp
    movq %rsp, %rbp
    subq $32, %rsp
    movaps -16(%rbp), %xmm0
    movaps %xmm0, -32(%rbp)
    leave
    ret

    .globl safe_callee_saved
    .type safe_callee_saved, @function
safe_callee_saved:
    pushq %rbx
    pushq %r12
    movq $100, %rbx
    movq $200, %r12
    popq %r12
    popq %rbx
    ret

    .globl safe_guarded_div
    .type safe_guarded_div, @function
safe_guarded_div:
    testq %rsi, %rsi
    jz 1f
    movq %rdi, %rax
    cqo
    idivq %rsi
1:
    ret

    .globl safe_aarch64_function
    .type safe_aarch64_function, %function
safe_aarch64_function:
    stp x29, x30, [sp, #-32]!
    mov x29, sp
    stp x19, x20, [sp, #16]
    mov x19, #42
    mov x20, #84
    ldp x19, x20, [sp, #16]
    ldp x29, x30, [sp], #32
    ret
