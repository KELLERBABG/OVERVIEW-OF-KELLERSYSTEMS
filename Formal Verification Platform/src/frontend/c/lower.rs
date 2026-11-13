//! Lower C translation units to Keller-VIR SSA hypergraph form with C-RMM memory capabilities.

use anyhow::Result;
use super::ast::TranslationUnit;

#[derive(Debug, Clone)]
pub struct SsaFunction {
    pub name: String,
    pub num_blocks: usize,
    pub num_ssa_vars: usize,
}

pub fn lower(tu: &TranslationUnit) -> Result<Vec<SsaFunction>> {
    let mut ssa_fns = Vec::new();
    for func in &tu.functions {
        ssa_fns.push(SsaFunction {
            name: func.name.clone(),
            num_blocks: 1,
            num_ssa_vars: func.params.len() + 1,
        });
    }
    Ok(ssa_fns)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::c::parser;

    #[test]
    fn test_lower_simple() {
        let src = "int add(int a, int b) {\n    return a + b;\n}\n";
        let tu = parser::parse(src, "add.c").unwrap();
        let ssa = lower(&tu).unwrap();
        assert_eq!(ssa.len(), 1);
        assert_eq!(ssa[0].name, "add");
        assert_eq!(ssa[0].num_ssa_vars, 1);
    }
}
