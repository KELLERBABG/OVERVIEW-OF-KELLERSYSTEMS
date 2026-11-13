//! C source parser.

use anyhow::Result;
use super::ast::{CType, Function, TranslationUnit};

pub fn parse(source: &str, filename: &str) -> Result<TranslationUnit> {
    let mut functions = Vec::new();

    for line in source.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty()
            && !trimmed.starts_with('#')
            && !trimmed.starts_with("//")
            && trimmed.contains('(')
            && trimmed.contains(')')
        {
            let paren_pos = trimmed.find('(').unwrap_or(0);
            let before = &trimmed[..paren_pos];
            let name = before
                .split_whitespace()
                .last()
                .unwrap_or("")
                .to_string();

            if !name.is_empty() && !name.starts_with("return") {
                functions.push(Function {
                    name,
                    return_type: CType::Int,
                    params: Vec::new(),
                    body_offset: 0,
                });
            }
        }
    }

    Ok(TranslationUnit {
        filename: filename.to_string(),
        functions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let src = "int main(void) {\n    return 0;\n}\n";
        let tu = parse(src, "main.c").unwrap();
        assert_eq!(tu.functions.len(), 1);
        assert_eq!(tu.functions[0].name, "main");
    }
}
