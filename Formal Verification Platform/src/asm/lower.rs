//! Lowering & Verification Engine for Assembly (x86_64 & AArch64).
//!
//! Evaluates safety obligations under C-RMM (Reduced Memory Model) without source mutation:
//! - `asm-stack-smash`: Stack buffer writes that can exceed frame boundaries or overwrite return address
//! - `asm-unaligned-simd`: SIMD instructions (movaps, movdqa) on unaligned stack offsets (must be 16-byte aligned)
//! - `asm-callee-saved-clobber`: Clobbering non-volatile registers without saving/restoring
//! - `asm-stack-imbalance`: Stack pointer rsp / sp modified without matching adjustment before ret
//! - `asm-div-by-zero`: idiv / div / sdiv / udiv with potentially zero divisor

use std::collections::{HashMap, HashSet};
use crate::asm::ast::{Arch, Function, Instruction, Operand, Reg, RegClass};
use crate::langaudit::Finding;

/// Kind of micro-op produced during lowering.
#[derive(Debug, Clone, PartialEq)]
pub enum OpKind {
    Compute,
    Load,
    Store,
    Push,
    Pop,
    Call,
    Ret,
    Branch,
    Prologue,
    Epilogue,
    Div,
}

/// A lowered micro-op representation for analysis.
#[derive(Debug, Clone)]
pub struct MicroOp {
    pub kind: OpKind,
    pub dst: Option<Operand>,
    pub srcs: Vec<Operand>,
    pub clobbers: Vec<Reg>,
    pub sp_delta: i64,
    pub src_offset: u64,
    pub line: usize,
}

/// Lowering error.
#[derive(Debug, Clone, PartialEq)]
pub enum LowerError {
    UnsupportedMnemonic(String),
    BadOperandCount { expected: usize, got: usize },
}

impl std::fmt::Display for LowerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LowerError::UnsupportedMnemonic(m) => write!(f, "unsupported mnemonic: {}", m),
            LowerError::BadOperandCount { expected, got } => {
                write!(f, "expected {} operands, got {}", expected, got)
            }
        }
    }
}

impl std::error::Error for LowerError {}

/// Verification result for a function or file.
#[derive(Debug, Clone, Default)]
pub struct LoweringResult {
    pub micro_ops: Vec<MicroOp>,
    pub functions_lowered: usize,
    pub obligations_total: usize,
    pub obligations_proved_safe: usize,
    pub findings: Vec<Finding>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RegVal {
    Zero,
    NonZero,
    Unknown,
}

/// The Assembly verifier engine.
pub struct Verifier {
    pub arch: Arch,
}

impl Verifier {
    pub fn new(arch: Arch) -> Self {
        Self { arch }
    }

    /// Strip instruction suffixes to get the base operation mnemonic.
    pub fn base_mnemonic(mnemonic: &str) -> &str {
        if mnemonic.eq_ignore_ascii_case("movaps") || mnemonic.eq_ignore_ascii_case("movaps64") || mnemonic.eq_ignore_ascii_case("vmovaps") {
            return "movaps";
        }
        if mnemonic.eq_ignore_ascii_case("movdqa") || mnemonic.eq_ignore_ascii_case("movdqa64") || mnemonic.eq_ignore_ascii_case("vmovdqa") {
            return "movdqa";
        }
        if mnemonic.eq_ignore_ascii_case("movups") || mnemonic.eq_ignore_ascii_case("movdqu") || mnemonic.eq_ignore_ascii_case("vmovups") || mnemonic.eq_ignore_ascii_case("vmovdqu") {
            return "movups";
        }
        if mnemonic.eq_ignore_ascii_case("ret") || mnemonic.eq_ignore_ascii_case("retq") {
            return "ret";
        }
        if mnemonic.eq_ignore_ascii_case("leave") || mnemonic.eq_ignore_ascii_case("leaveq") {
            return "leave";
        }
        if mnemonic.eq_ignore_ascii_case("call") || mnemonic.eq_ignore_ascii_case("callq") {
            return "call";
        }
        if mnemonic.eq_ignore_ascii_case("nop") {
            return "nop";
        }
        if mnemonic.eq_ignore_ascii_case("pxor") || mnemonic.eq_ignore_ascii_case("xorps") || mnemonic.eq_ignore_ascii_case("xorpd") || mnemonic.eq_ignore_ascii_case("vpxor") {
            return "xor";
        }

        for prefix in &["mov", "add", "sub", "xor", "and", "or", "cmp", "test", "push", "pop", "imul", "idiv", "div", "lea"] {
            if mnemonic.eq_ignore_ascii_case(prefix) {
                return prefix;
            }
            if mnemonic.len() == prefix.len() + 1 && mnemonic[..prefix.len()].eq_ignore_ascii_case(prefix) {
                let suffix = &mnemonic[prefix.len()..];
                if matches!(suffix, "b" | "B" | "w" | "W" | "l" | "L" | "q" | "Q" | "z" | "s") {
                    return prefix;
                }
            }
        }

        mnemonic
    }

    /// Check whether this assembly file/function appears to be in AT&T syntax.
    pub fn is_att_syntax(func: &Function) -> bool {
        for instr in &func.instructions {
            for op in &instr.operands {
                match op {
                    Operand::Reg(_) => {
                        if instr.raw.contains('%') {
                            return true;
                        }
                    }
                    Operand::Mem { .. } => {
                        if instr.raw.contains('(') && instr.raw.contains(')') {
                            return true;
                        }
                    }
                    Operand::Imm(_) => {
                        if instr.raw.contains('$') {
                            return true;
                        }
                    }
                    _ => {}
                }
            }
        }
        false
    }

    /// Verify a single function and return its LoweringResult.
    pub fn verify_function(&self, func: &Function, file_path: &str) -> LoweringResult {
        let mut result = LoweringResult {
            functions_lowered: 1,
            ..Default::default()
        };

        let is_att = self.arch == Arch::X86_64 && Self::is_att_syntax(func);

        let mut sp_delta: i64 = 0;
        let mut allocated_frame_size: i64 = 0;
        let mut has_frame_pointer = false;
        let mut saved_regs: HashSet<String> = HashSet::new();
        let mut restored_regs: HashSet<String> = HashSet::new();
        let mut modified_callee_saved: HashMap<String, usize> = HashMap::new();
        let mut reg_values: HashMap<String, RegVal> = HashMap::new();
        let mut guarded_regs: HashSet<String> = HashSet::new();

        for instr in &func.instructions {
            let base_m = Self::base_mnemonic(&instr.mnemonic);

            // 1. Check for stack frame setup / teardown instructions
            match (self.arch, base_m) {
                (Arch::X86_64, "push") => {
                    sp_delta -= 8;
                    if let Some(Operand::Reg(r)) = instr.operands.first() {
                        let cname = r.canonical_name().to_string();
                        saved_regs.insert(cname);
                    }
                }
                (Arch::X86_64, "pop") => {
                    sp_delta += 8;
                    if let Some(Operand::Reg(r)) = instr.operands.first() {
                        let cname = r.canonical_name().to_string();
                        restored_regs.insert(cname.clone());
                        reg_values.remove(&cname);
                    }
                }
                (Arch::X86_64, "leave") => {
                    // leave: mov rsp, rbp; pop rbp
                    sp_delta = 0;
                    restored_regs.insert("rbp".to_string());
                    has_frame_pointer = false;
                }
                (Arch::X86_64, "sub") => {
                    // sub rsp, imm  or  subq $imm, %rsp
                    if instr.operands.len() >= 2 {
                        let (dst, src) = if is_att {
                            (&instr.operands[1], &instr.operands[0])
                        } else {
                            (&instr.operands[0], &instr.operands[1])
                        };

                        if let (Operand::Reg(r), Operand::Imm(imm)) = (dst, src) {
                            if r.canonical_name() == "rsp" {
                                sp_delta -= *imm;
                                allocated_frame_size += *imm;
                            }
                        }
                    }
                }
                (Arch::X86_64, "add") => {
                    // add rsp, imm  or  addq $imm, %rsp
                    if instr.operands.len() >= 2 {
                        let (dst, src) = if is_att {
                            (&instr.operands[1], &instr.operands[0])
                        } else {
                            (&instr.operands[0], &instr.operands[1])
                        };

                        if let (Operand::Reg(r), Operand::Imm(imm)) = (dst, src) {
                            if r.canonical_name() == "rsp" {
                                sp_delta += *imm;
                            }
                        }
                    }
                }
                (Arch::AArch64, "sub") => {
                    // sub sp, sp, #imm
                    if instr.operands.len() >= 3 {
                        if let (Operand::Reg(dst), Operand::Reg(src), Operand::Imm(imm)) =
                            (&instr.operands[0], &instr.operands[1], &instr.operands[2])
                        {
                            if dst.canonical_name() == "sp" && src.canonical_name() == "sp" {
                                sp_delta -= *imm;
                                allocated_frame_size += *imm;
                            }
                        }
                    }
                }
                (Arch::AArch64, "add") => {
                    // add sp, sp, #imm
                    if instr.operands.len() >= 3 {
                        if let (Operand::Reg(dst), Operand::Reg(src), Operand::Imm(imm)) =
                            (&instr.operands[0], &instr.operands[1], &instr.operands[2])
                        {
                            if dst.canonical_name() == "sp" && src.canonical_name() == "sp" {
                                sp_delta += *imm;
                            }
                        }
                    }
                }
                (Arch::AArch64, "stp") => {
                    // stp x29, x30, [sp, -imm]!  or  stp x19, x20, [sp, -imm]!
                    if instr.operands.len() >= 3 {
                        if let (Operand::Reg(r1), Operand::Reg(r2), Operand::Mem { base, disp, .. }) =
                            (&instr.operands[0], &instr.operands[1], &instr.operands[2])
                        {
                            if base.canonical_name() == "sp" && *disp < 0 {
                                sp_delta += *disp;
                                allocated_frame_size += -*disp;
                            }
                            if r1.is_callee_saved() || r1.canonical_name() == "x29" {
                                saved_regs.insert(r1.canonical_name().to_string());
                            }
                            if r2.is_callee_saved() || r2.canonical_name() == "x30" {
                                saved_regs.insert(r2.canonical_name().to_string());
                            }
                        }
                    }
                }
                (Arch::AArch64, "ldp") => {
                    // ldp x29, x30, [sp], imm  or  ldp x19, x20, [sp, #16]
                    if instr.operands.len() >= 3 {
                        if let (Operand::Reg(r1), Operand::Reg(r2), Operand::Mem { base, disp, .. }) =
                            (&instr.operands[0], &instr.operands[1], &instr.operands[2])
                        {
                            if r1.is_callee_saved() || r1.canonical_name() == "x29" {
                                restored_regs.insert(r1.canonical_name().to_string());
                            }
                            if r2.is_callee_saved() || r2.canonical_name() == "x30" {
                                restored_regs.insert(r2.canonical_name().to_string());
                            }
                            let post_delta = if let Some(Operand::Imm(imm)) = instr.operands.get(3) {
                                *imm
                            } else if *disp > 0 && instr.raw.ends_with('!') {
                                *disp
                            } else {
                                0
                            };
                            if base.canonical_name() == "sp" {
                                sp_delta += post_delta;
                            }
                        }
                    }
                }
                _ => {}
            }

            // 2. Track Frame Pointer setup
            if base_m == "mov" {
                if is_att && instr.operands.len() >= 2 {
                    if let (Operand::Reg(src), Operand::Reg(dst)) = (&instr.operands[0], &instr.operands[1]) {
                        if src.canonical_name() == "rsp" && dst.canonical_name() == "rbp" {
                            has_frame_pointer = true;
                        }
                        if src.canonical_name() == "rbp" && dst.canonical_name() == "rsp" {
                            // mov rbp, rsp (restoring stack before pop rbp)
                            sp_delta = -8;
                        }
                    }
                } else if !is_att && instr.operands.len() >= 2 {
                    if let (Operand::Reg(dst), Operand::Reg(src)) = (&instr.operands[0], &instr.operands[1]) {
                        if src.canonical_name() == "rsp" && dst.canonical_name() == "rbp" {
                            has_frame_pointer = true;
                        }
                        if src.canonical_name() == "rbp" && dst.canonical_name() == "rsp" {
                            sp_delta = -8;
                        }
                        if self.arch == Arch::AArch64 && src.canonical_name() == "sp" && dst.canonical_name() == "x29" {
                            has_frame_pointer = true;
                        }
                    }
                }
            }

            // 3. Track Register Values (Zero, NonZero) & Guarding
            match base_m {
                "xor" => {
                    // xor reg, reg -> reg is 0
                    if instr.operands.len() >= 2 {
                        if let (Operand::Reg(r1), Operand::Reg(r2)) = (&instr.operands[0], &instr.operands[1]) {
                            if r1.canonical_name() == r2.canonical_name() {
                                reg_values.insert(r1.canonical_name().to_string(), RegVal::Zero);
                            }
                        }
                    }
                }
                "mov" => {
                    if instr.operands.len() >= 2 {
                        let (dst, src) = if is_att {
                            (&instr.operands[1], &instr.operands[0])
                        } else {
                            (&instr.operands[0], &instr.operands[1])
                        };

                        if let Operand::Reg(dreg) = dst {
                            match src {
                                Operand::Imm(0) => {
                                    reg_values.insert(dreg.canonical_name().to_string(), RegVal::Zero);
                                }
                                Operand::Imm(_) => {
                                    reg_values.insert(dreg.canonical_name().to_string(), RegVal::NonZero);
                                }
                                Operand::Reg(sreg) if sreg.canonical_name() == "xzr" || sreg.canonical_name() == "wzr" => {
                                    reg_values.insert(dreg.canonical_name().to_string(), RegVal::Zero);
                                }
                                _ => {
                                    reg_values.remove(dreg.canonical_name());
                                }
                            }
                        }
                    }
                }
                "test" | "cmp" => {
                    // test reg, reg  or  cmp $0, reg
                    for op in &instr.operands {
                        if let Operand::Reg(r) = op {
                            guarded_regs.insert(r.canonical_name().to_string());
                        }
                    }
                }
                _ => {}
            }

            // 4. Determine Destination and Memory Targets
            let (dst_op, _src_ops) = Self::get_destination_and_sources(instr, is_att, self.arch);

            // ==========================================
            // Check 1: asm-stack-smash
            // ==========================================
            if let Some(Operand::Mem { base, disp, .. }) = &dst_op {
                if base.class == RegClass::StackPointer || base.class == RegClass::FramePointer {
                    result.obligations_total += 1;
                    let is_sp = base.class == RegClass::StackPointer;

                    if is_sp {
                        if (base_m == "stp" || base_m == "str") && instr.raw.ends_with('!') {
                            // Pre-indexed stack frame allocation write: allocates frame and writes within it
                            result.obligations_proved_safe += 1;
                        } else if allocated_frame_size == 0 {
                            if *disp == 0 {
                                result.findings.push(Finding {
                                    file: file_path.to_string(),
                                    line: instr.line,
                                    class: "asm-stack-smash".into(),
                                    verdict: "proved-defect".into(),
                                    message: format!(
                                        "write to [{}] at function entry overwrites return address slot",
                                        base.display_name()
                                    ),
                                });
                            } else if *disp < 0 {
                                result.findings.push(Finding {
                                    file: file_path.to_string(),
                                    line: instr.line,
                                    class: "asm-stack-smash".into(),
                                    verdict: "possible-defect".into(),
                                    message: format!(
                                        "stack write at negative offset {} below stack pointer",
                                        disp
                                    ),
                                });
                            } else {
                                result.obligations_proved_safe += 1;
                            }
                        } else {
                            // Stack frame allocated of size `allocated_frame_size`
                            if *disp == allocated_frame_size {
                                result.findings.push(Finding {
                                    file: file_path.to_string(),
                                    line: instr.line,
                                    class: "asm-stack-smash".into(),
                                    verdict: "proved-defect".into(),
                                    message: format!(
                                        "stack buffer write at offset {} overwrites saved return address slot",
                                        disp
                                    ),
                                });
                            } else if *disp > allocated_frame_size {
                                result.findings.push(Finding {
                                    file: file_path.to_string(),
                                    line: instr.line,
                                    class: "asm-stack-smash".into(),
                                    verdict: "proved-defect".into(),
                                    message: format!(
                                        "stack buffer write at offset {} exceeds frame size {} (caller stack corruption)",
                                        disp, allocated_frame_size
                                    ),
                                });
                            } else if *disp < 0 {
                                result.findings.push(Finding {
                                    file: file_path.to_string(),
                                    line: instr.line,
                                    class: "asm-stack-smash".into(),
                                    verdict: "possible-defect".into(),
                                    message: format!(
                                        "stack buffer write at negative offset {} below current stack pointer",
                                        disp
                                    ),
                                });
                            } else {
                                result.obligations_proved_safe += 1;
                            }
                        }
                    } else {
                        // Base is Frame Pointer (rbp in x86_64, x29 in AArch64)
                        if self.arch == Arch::X86_64 {
                            if *disp == 8 {
                                result.findings.push(Finding {
                                    file: file_path.to_string(),
                                    line: instr.line,
                                    class: "asm-stack-smash".into(),
                                    verdict: "proved-defect".into(),
                                    message: "write to 8(%rbp) overwrites saved return address".into(),
                                });
                            } else if allocated_frame_size > 0 && *disp < -allocated_frame_size {
                                result.findings.push(Finding {
                                    file: file_path.to_string(),
                                    line: instr.line,
                                    class: "asm-stack-smash".into(),
                                    verdict: "possible-defect".into(),
                                    message: format!(
                                        "stack write at offset {}(%rbp) exceeds allocated frame size {}",
                                        disp, allocated_frame_size
                                    ),
                                });
                            } else {
                                result.obligations_proved_safe += 1;
                            }
                        } else {
                            // AArch64 X29: [x29, #8] holds stored link register x30
                            if *disp == 8 {
                                result.findings.push(Finding {
                                    file: file_path.to_string(),
                                    line: instr.line,
                                    class: "asm-stack-smash".into(),
                                    verdict: "proved-defect".into(),
                                    message: "write to [x29, #8] overwrites stored link register (return address)".into(),
                                });
                            } else {
                                result.obligations_proved_safe += 1;
                            }
                        }
                    }
                }
            }

            // ==========================================
            // Check 2: asm-unaligned-simd
            // ==========================================
            if matches!(base_m, "movaps" | "movdqa") {
                for op in &instr.operands {
                    if let Operand::Mem { base, disp, .. } = op {
                        result.obligations_total += 1;
                        if base.class == RegClass::StackPointer || base.class == RegClass::FramePointer {
                            if *disp % 16 != 0 {
                                result.findings.push(Finding {
                                    file: file_path.to_string(),
                                    line: instr.line,
                                    class: "asm-unaligned-simd".into(),
                                    verdict: "proved-defect".into(),
                                    message: format!(
                                        "SIMD instruction '{}' accesses unaligned stack offset {} (must be 16-byte aligned)",
                                        instr.mnemonic, disp
                                    ),
                                });
                            } else if base.class == RegClass::StackPointer {
                                // In System V ABI, RSP at function entry is 8 mod 16
                                let rsp_alignment = ((sp_delta + 8).rem_euclid(16)) as i64;
                                if (rsp_alignment + disp) % 16 != 0 {
                                    result.findings.push(Finding {
                                        file: file_path.to_string(),
                                        line: instr.line,
                                        class: "asm-unaligned-simd".into(),
                                        verdict: "proved-defect".into(),
                                        message: format!(
                                            "SIMD instruction '{}' executed when stack pointer is not 16-byte aligned",
                                            instr.mnemonic
                                        ),
                                    });
                                } else {
                                    result.obligations_proved_safe += 1;
                                }
                            } else {
                                result.obligations_proved_safe += 1;
                            }
                        } else {
                            if *disp % 16 != 0 {
                                result.findings.push(Finding {
                                    file: file_path.to_string(),
                                    line: instr.line,
                                    class: "asm-unaligned-simd".into(),
                                    verdict: "possible-defect".into(),
                                    message: format!(
                                        "SIMD instruction '{}' accesses unaligned memory offset {}",
                                        instr.mnemonic, disp
                                    ),
                                });
                            } else {
                                result.obligations_proved_safe += 1;
                            }
                        }
                    }
                }
            }

            // ==========================================
            // Check 3: asm-callee-saved-clobber (Track writes)
            // ==========================================
            if let Some(Operand::Reg(dst_reg)) = &dst_op {
                if dst_reg.is_callee_saved() {
                    // Frame pointer setup: mov rsp, rbp after push rbp is standard, not a clobber
                    let is_standard_fp_setup = dst_reg.canonical_name() == "rbp"
                        && base_m == "mov"
                        && has_frame_pointer;

                    if !is_standard_fp_setup {
                        modified_callee_saved
                            .entry(dst_reg.canonical_name().to_string())
                            .or_insert(instr.line);
                    }
                }
            }

            // ==========================================
            // Check 4: asm-div-by-zero
            // ==========================================
            if matches!(base_m, "idiv" | "div" | "sdiv" | "udiv") {
                result.obligations_total += 1;

                let divisor_op = if self.arch == Arch::X86_64 {
                    instr.operands.first()
                } else {
                    instr.operands.last()
                };

                match divisor_op {
                    Some(Operand::Imm(0)) => {
                        result.findings.push(Finding {
                            file: file_path.to_string(),
                            line: instr.line,
                            class: "asm-div-by-zero".into(),
                            verdict: "proved-defect".into(),
                            message: format!("division by constant zero in '{}'", instr.mnemonic),
                        });
                    }
                    Some(Operand::Reg(r)) if r.canonical_name() == "xzr" || r.canonical_name() == "wzr" => {
                        result.findings.push(Finding {
                            file: file_path.to_string(),
                            line: instr.line,
                            class: "asm-div-by-zero".into(),
                            verdict: "proved-defect".into(),
                            message: format!("division by zero register '{}' in '{}'", r.display_name(), instr.mnemonic),
                        });
                    }
                    Some(Operand::Reg(r)) if reg_values.get(r.canonical_name()) == Some(&RegVal::Zero) => {
                        result.findings.push(Finding {
                            file: file_path.to_string(),
                            line: instr.line,
                            class: "asm-div-by-zero".into(),
                            verdict: "proved-defect".into(),
                            message: format!(
                                "division by register '{}' known to be zero in '{}'",
                                r.display_name(), instr.mnemonic
                            ),
                        });
                    }
                    Some(Operand::Reg(r)) if reg_values.get(r.canonical_name()) == Some(&RegVal::NonZero)
                        || guarded_regs.contains(r.canonical_name()) =>
                    {
                        result.obligations_proved_safe += 1;
                    }
                    Some(Operand::Reg(r)) => {
                        result.findings.push(Finding {
                            file: file_path.to_string(),
                            line: instr.line,
                            class: "asm-div-by-zero".into(),
                            verdict: "possible-defect".into(),
                            message: format!(
                                "division by register '{}' with potentially zero divisor (unguarded) in '{}'",
                                r.display_name(), instr.mnemonic
                            ),
                        });
                    }
                    Some(Operand::Imm(_)) => {
                        result.obligations_proved_safe += 1;
                    }
                    _ => {
                        result.findings.push(Finding {
                            file: file_path.to_string(),
                            line: instr.line,
                            class: "asm-div-by-zero".into(),
                            verdict: "possible-defect".into(),
                            message: format!(
                                "division with unverified divisor in '{}'",
                                instr.mnemonic
                            ),
                        });
                    }
                }
            }

            // ==========================================
            // Check 5: asm-stack-imbalance
            // ==========================================
            if base_m == "ret" {
                result.obligations_total += 1;
                if sp_delta != 0 {
                    result.findings.push(Finding {
                        file: file_path.to_string(),
                        line: instr.line,
                        class: "asm-stack-imbalance".into(),
                        verdict: "proved-defect".into(),
                        message: format!(
                            "stack pointer modified with net delta {} before return in '{}'",
                            sp_delta, func.name
                        ),
                    });
                } else {
                    result.obligations_proved_safe += 1;
                }
            }
        }

        // Post-function check for asm-callee-saved-clobber
        for (reg, line) in &modified_callee_saved {
            result.obligations_total += 1;
            let is_saved = saved_regs.contains(reg);
            let is_restored = restored_regs.contains(reg);

            if !is_saved || !is_restored {
                let reason = match (!is_saved, !is_restored) {
                    (true, true) => format!("callee-saved register '{}' modified without being saved or restored", reg),
                    (true, false) => format!("callee-saved register '{}' modified without being saved at entry", reg),
                    (false, true) => format!("callee-saved register '{}' was saved but not restored before return", reg),
                    _ => unreachable!(),
                };
                result.findings.push(Finding {
                    file: file_path.to_string(),
                    line: *line,
                    class: "asm-callee-saved-clobber".into(),
                    verdict: "proved-defect".into(),
                    message: format!("{} in function '{}'", reason, func.name),
                });
            } else {
                result.obligations_proved_safe += 1;
            }
        }

        result
    }

    /// Helper to identify destination operand and source operands for an instruction.
    fn get_destination_and_sources(
        instr: &Instruction,
        is_att: bool,
        arch: Arch,
    ) -> (Option<Operand>, Vec<Operand>) {
        let base_m = Self::base_mnemonic(&instr.mnemonic);

        // AArch64 store instructions
        if arch == Arch::AArch64 {
            match base_m {
                "str" | "stur" | "strb" | "strh" => {
                    if instr.operands.len() >= 2 {
                        return (Some(instr.operands[1].clone()), vec![instr.operands[0].clone()]);
                    }
                }
                "stp" => {
                    if instr.operands.len() >= 3 {
                        return (
                            Some(instr.operands[2].clone()),
                            vec![instr.operands[0].clone(), instr.operands[1].clone()],
                        );
                    }
                }
                "cmp" | "cmn" | "tst" | "b" | "bl" | "ret" => {
                    return (None, instr.operands.clone());
                }
                _ => {
                    // Normal AArch64: destination is first operand
                    if !instr.operands.is_empty() {
                        return (Some(instr.operands[0].clone()), instr.operands[1..].to_vec());
                    }
                }
            }
        }

        // x86_64
        match base_m {
            "push" => {
                let sp = Reg::RSP;
                (Some(Operand::Mem { base: sp, disp: 0, index: None, scale: 1 }), instr.operands.clone())
            }
            "pop" => {
                if let Some(first) = instr.operands.first() {
                    (Some(first.clone()), vec![])
                } else {
                    (None, vec![])
                }
            }
            "cmp" | "test" | "ret" | "call" | "nop" => {
                (None, instr.operands.clone())
            }
            _ => {
                if is_att {
                    // AT&T syntax: 2-operand is (src, dst)
                    if instr.operands.len() >= 2 {
                        let dst = instr.operands.last().cloned();
                        let srcs = instr.operands[..instr.operands.len() - 1].to_vec();
                        (dst, srcs)
                    } else if instr.operands.len() == 1 {
                        (Some(instr.operands[0].clone()), instr.operands.clone())
                    } else {
                        (None, vec![])
                    }
                } else {
                    // Intel syntax: 2-operand is (dst, src)
                    if !instr.operands.is_empty() {
                        (Some(instr.operands[0].clone()), instr.operands[1..].to_vec())
                    } else {
                        (None, vec![])
                    }
                }
            }
        }
    }
}

impl Default for Verifier {
    fn default() -> Self {
        Self::new(Arch::X86_64)
    }
}
