/* Vulnerable Assembly test cases for Keller Verify (TASK-05) */

    .text
    /* 1. asm-stack-smash: write to (%rsp) overwriting return address */
    .globl vuln_stack_smash_entry
    .type vuln_stack_smash_entry, @function
vuln_stack_smash_entry:
    movq $0, (%rsp)
    ret

    /* 1. asm-stack-smash: write beyond allocated stack frame */
    .globl vuln_stack_smash_overflow
    .type vuln_stack_smash_overflow, @function
vuln_stack_smash_overflow:
    subq $32, %rsp
    movq %rax, 48(%rsp)
    addq $32, %rsp
    ret

    /* 1. asm-stack-smash: write to return address via frame pointer */
    .globl vuln_stack_smash_fp
    .type vuln_stack_smash_fp, @function
vuln_stack_smash_fp:
    pushq %rbp
    movq %rsp, %rbp
    movq $0xdeadbeef, 8(%rbp)
    leave
    ret

    /* 2. asm-unaligned-simd: unaligned offset on stack */
    .globl vuln_unaligned_simd
    .type vuln_unaligned_simd, @function
vuln_unaligned_simd:
    subq $32, %rsp
    movaps 8(%rsp), %xmm0
    addq $32, %rsp
    ret

    /* 3. asm-callee-saved-clobber: modifying rbx without saving */
    .globl vuln_clobber_rbx
    .type vuln_clobber_rbx, @function
vuln_clobber_rbx:
    movq $42, %rbx
    ret

    /* 3. asm-callee-saved-clobber: saving r12 but forgetting to restore */
    .globl vuln_clobber_r12
    .type vuln_clobber_r12, @function
vuln_clobber_r12:
    pushq %r12
    movq $99, %r12
    ret

    /* 4. asm-stack-imbalance: modifying stack pointer without matching add before ret */
    .globl vuln_stack_imbalance
    .type vuln_stack_imbalance, @function
vuln_stack_imbalance:
    subq $24, %rsp
    movq $1, %rax
    ret

    /* 5. asm-div-by-zero: division by register known to be zero */
    .globl vuln_div_zero_known
    .type vuln_div_zero_known, @function
vuln_div_zero_known:
    xorq %rcx, %rcx
    idivq %rcx
    ret

    /* 5. asm-div-by-zero: division by unguarded register */
    .globl vuln_div_zero_unguarded
    .type vuln_div_zero_unguarded, @function
vuln_div_zero_unguarded:
    idivq %rsi
    ret
