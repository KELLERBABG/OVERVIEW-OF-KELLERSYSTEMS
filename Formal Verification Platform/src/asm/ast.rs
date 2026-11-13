//! AST types for x86_64 and AArch64 assembly instructions.

use std::fmt;

/// Target architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Arch {
    X86_64,
    AArch64,
}

impl fmt::Display for Arch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Arch::X86_64 => write!(f, "x86_64"),
            Arch::AArch64 => write!(f, "aarch64"),
        }
    }
}

/// Register class for clobber tracking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RegClass {
    General,
    StackPointer,
    FramePointer,
    ReturnAddress,
    Zero,
    Simd,
    Flags,
    InstructionPointer,
}

/// A register in the target ISA.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Reg {
    pub arch: Arch,
    /// Index into the architecture's register file (stable identity).
    pub idx: u8,
    pub class: RegClass,
}

impl Reg {
    // x86_64 general & pointer registers
    pub const RAX: Self = Self { arch: Arch::X86_64, idx: 0,  class: RegClass::General };
    pub const RBX: Self = Self { arch: Arch::X86_64, idx: 1,  class: RegClass::General };
    pub const RCX: Self = Self { arch: Arch::X86_64, idx: 2,  class: RegClass::General };
    pub const RDX: Self = Self { arch: Arch::X86_64, idx: 3,  class: RegClass::General };
    pub const RSI: Self = Self { arch: Arch::X86_64, idx: 4,  class: RegClass::General };
    pub const RDI: Self = Self { arch: Arch::X86_64, idx: 5,  class: RegClass::General };
    pub const RBP: Self = Self { arch: Arch::X86_64, idx: 6,  class: RegClass::FramePointer };
    pub const RSP: Self = Self { arch: Arch::X86_64, idx: 7,  class: RegClass::StackPointer };
    pub const R8:  Self = Self { arch: Arch::X86_64, idx: 8,  class: RegClass::General };
    pub const R9:  Self = Self { arch: Arch::X86_64, idx: 9,  class: RegClass::General };
    pub const R10: Self = Self { arch: Arch::X86_64, idx: 10, class: RegClass::General };
    pub const R11: Self = Self { arch: Arch::X86_64, idx: 11, class: RegClass::General };
    pub const R12: Self = Self { arch: Arch::X86_64, idx: 12, class: RegClass::General };
    pub const R13: Self = Self { arch: Arch::X86_64, idx: 13, class: RegClass::General };
    pub const R14: Self = Self { arch: Arch::X86_64, idx: 14, class: RegClass::General };
    pub const R15: Self = Self { arch: Arch::X86_64, idx: 15, class: RegClass::General };
    pub const RIP: Self = Self { arch: Arch::X86_64, idx: 65, class: RegClass::InstructionPointer };

    // AArch64 (x0..x30, sp=31, xzr=32)
    pub const X0:  Self = Self { arch: Arch::AArch64, idx: 0,  class: RegClass::General };
    pub const X1:  Self = Self { arch: Arch::AArch64, idx: 1,  class: RegClass::General };
    pub const X2:  Self = Self { arch: Arch::AArch64, idx: 2,  class: RegClass::General };
    pub const X3:  Self = Self { arch: Arch::AArch64, idx: 3,  class: RegClass::General };
    pub const X29: Self = Self { arch: Arch::AArch64, idx: 29, class: RegClass::FramePointer };
    pub const X30: Self = Self { arch: Arch::AArch64, idx: 30, class: RegClass::ReturnAddress };
    pub const SP:  Self = Self { arch: Arch::AArch64, idx: 31, class: RegClass::StackPointer };
    pub const XZR: Self = Self { arch: Arch::AArch64, idx: 32, class: RegClass::Zero };

    /// Returns whether this register is callee-saved (non-volatile) across procedure calls.
    /// x86_64: rbx, rbp, r12, r13, r14, r15
    /// AArch64: x19..x28
    pub fn is_callee_saved(&self) -> bool {
        match self.arch {
            Arch::X86_64 => matches!(self.idx, 1 | 6 | 12 | 13 | 14 | 15),
            Arch::AArch64 => (19..=28).contains(&self.idx),
        }
    }

    /// Canonical register name (e.g. eax -> rax, w0 -> x0).
    pub fn canonical_name(&self) -> &'static str {
        match self.arch {
            Arch::X86_64 => match self.idx {
                0 => "rax",
                1 => "rbx",
                2 => "rcx",
                3 => "rdx",
                4 => "rsi",
                5 => "rdi",
                6 => "rbp",
                7 => "rsp",
                8 => "r8",
                9 => "r9",
                10 => "r10",
                11 => "r11",
                12 => "r12",
                13 => "r13",
                14 => "r14",
                15 => "r15",
                65 => "rip",
                16..=47 => "simd",
                _ => "unknown",
            },
            Arch::AArch64 => match self.idx {
                0 => "x0", 1 => "x1", 2 => "x2", 3 => "x3",
                4 => "x4", 5 => "x5", 6 => "x6", 7 => "x7",
                8 => "x8", 9 => "x9", 10 => "x10", 11 => "x11",
                12 => "x12", 13 => "x13", 14 => "x14", 15 => "x15",
                16 => "x16", 17 => "x17", 18 => "x18", 19 => "x19",
                20 => "x20", 21 => "x21", 22 => "x22", 23 => "x23",
                24 => "x24", 25 => "x25", 26 => "x26", 27 => "x27",
                28 => "x28", 29 => "x29", 30 => "x30",
                31 => "sp",
                32 => "xzr",
                _ => "simd",
            },
        }
    }

    /// Parse a register name for the given architecture.
    /// Handles % prefix (AT&T syntax), subregisters (eax -> rax, w0 -> x0),
    /// SIMD registers (xmm0..xmm31, ymm, zmm, v0..v31), and aliases (lr, fp).
    pub fn parse(arch: Arch, name: &str) -> Option<Self> {
        let n = name.trim().trim_start_matches('%').to_ascii_lowercase();
        match arch {
            Arch::X86_64 => {
                match n.as_str() {
                    "rax" | "eax" | "ax" | "al" | "ah" => Some(Self::RAX),
                    "rbx" | "ebx" | "bx" | "bl" | "bh" => Some(Self::RBX),
                    "rcx" | "ecx" | "cx" | "cl" | "ch" => Some(Self::RCX),
                    "rdx" | "edx" | "dx" | "dl" | "dh" => Some(Self::RDX),
                    "rsi" | "esi" | "si" | "sil" => Some(Self::RSI),
                    "rdi" | "edi" | "di" | "dil" => Some(Self::RDI),
                    "rbp" | "ebp" | "bp" | "bpl" => Some(Self::RBP),
                    "rsp" | "esp" | "sp" | "spl" => Some(Self::RSP),
                    "r8" | "r8d" | "r8w" | "r8b" => Some(Self::R8),
                    "r9" | "r9d" | "r9w" | "r9b" => Some(Self::R9),
                    "r10" | "r10d" | "r10w" | "r10b" => Some(Self::R10),
                    "r11" | "r11d" | "r11w" | "r11b" => Some(Self::R11),
                    "r12" | "r12d" | "r12w" | "r12b" => Some(Self::R12),
                    "r13" | "r13d" | "r13w" | "r13b" => Some(Self::R13),
                    "r14" | "r14d" | "r14w" | "r14b" => Some(Self::R14),
                    "r15" | "r15d" | "r15w" | "r15b" => Some(Self::R15),
                    "rip" | "eip" => Some(Self::RIP),
                    s if s.starts_with("xmm") || s.starts_with("ymm") || s.starts_with("zmm") => {
                        let num_str = &s[3..];
                        if let Ok(num) = num_str.parse::<u8>() {
                            if num <= 31 {
                                return Some(Self { arch, idx: 16 + num, class: RegClass::Simd });
                            }
                        }
                        None
                    }
                    _ => None,
                }
            }
            Arch::AArch64 => {
                if n == "sp" || n == "wsp" { return Some(Self::SP); }
                if n == "xzr" || n == "wzr" { return Some(Self::XZR); }
                if n == "fp" { return Some(Self::X29); }
                if n == "lr" { return Some(Self::X30); }

                // x0..x30 or w0..w30
                if let Some(num_str) = n.strip_prefix('x').or_else(|| n.strip_prefix('w')) {
                    if let Ok(num) = num_str.parse::<u8>() {
                        if num <= 30 {
                            let class = match num {
                                29 => RegClass::FramePointer,
                                30 => RegClass::ReturnAddress,
                                _ => RegClass::General,
                            };
                            return Some(Self { arch, idx: num, class });
                        }
                    }
                }

                // SIMD: v0..v31, q0..q31, d0..d31, s0..s31
                let simd_prefix = ['v', 'q', 'd', 's', 'b', 'h'];
                for p in simd_prefix {
                    if let Some(rest) = n.strip_prefix(p) {
                        let bare_num = rest.split('.').next().unwrap_or(rest);
                        if let Ok(num) = bare_num.parse::<u8>() {
                            if num <= 31 {
                                return Some(Self { arch, idx: 33 + num, class: RegClass::Simd });
                            }
                        }
                    }
                }

                None
            }
        }
    }

    /// Display name for the register.
    pub fn display_name(&self) -> String {
        match self.arch {
            Arch::X86_64 => match self.idx {
                0..=15 => self.canonical_name().to_string(),
                65 => "rip".to_string(),
                16..=47 => format!("xmm{}", self.idx - 16),
                _ => format!("reg{}", self.idx),
            },
            Arch::AArch64 => match self.idx {
                31 => "sp".into(),
                32 => "xzr".into(),
                33..=64 => format!("v{}", self.idx - 33),
                _ => format!("x{}", self.idx),
            },
        }
    }
}

impl fmt::Display for Reg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// An operand in an instruction.
#[derive(Debug, Clone, PartialEq)]
pub enum Operand {
    Reg(Reg),
    Imm(i64),
    /// Memory: base register + displacement, with optional index register and scale.
    Mem {
        base: Reg,
        disp: i64,
        index: Option<Reg>,
        scale: i64,
    },
    /// Symbolic label operand (e.g. branch targets, external symbols).
    Label(String),
}

impl Operand {
    pub fn mem(base: Reg, disp: i64) -> Self {
        Operand::Mem {
            base,
            disp,
            index: None,
            scale: 1,
        }
    }

    pub fn mem_scaled(base: Reg, disp: i64, index: Option<Reg>, scale: i64) -> Self {
        Operand::Mem {
            base,
            disp,
            index,
            scale,
        }
    }
}

impl fmt::Display for Operand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Operand::Reg(r) => write!(f, "{}", r),
            Operand::Imm(v) => write!(f, "{:#x}", v),
            Operand::Mem { base, disp, index, scale } => {
                match (index, *disp) {
                    (None, 0) => write!(f, "[{}]", base),
                    (None, d) if d > 0 => write!(f, "[{} + {:#x}]", base, d),
                    (None, d) => write!(f, "[{} - {:#x}]", base, -d),
                    (Some(idx), 0) => write!(f, "[{} + {} * {}]", base, idx, scale),
                    (Some(idx), d) if d > 0 => write!(f, "[{} + {} * {} + {:#x}]", base, idx, scale, d),
                    (Some(idx), d) => write!(f, "[{} + {} * {} - {:#x}]", base, idx, scale, -d),
                }
            }
            Operand::Label(lbl) => write!(f, "{}", lbl),
        }
    }
}

/// A single assembly instruction.
#[derive(Debug, Clone)]
pub struct Instruction {
    pub arch: Arch,
    pub mnemonic: String,
    pub operands: Vec<Operand>,
    /// Byte offset or sequence counter.
    pub offset: u64,
    /// 1-based source line number.
    pub line: usize,
    /// Raw textual line for reporting.
    pub raw: String,
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:08x}: {}", self.offset, self.mnemonic)?;
        for (i, op) in self.operands.iter().enumerate() {
            if i == 0 { write!(f, " {}", op)?; } else { write!(f, ", {}", op)?; }
        }
        Ok(())
    }
}

/// A parsed function containing a sequence of instructions.
#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub instructions: Vec<Instruction>,
    pub start_line: usize,
}
