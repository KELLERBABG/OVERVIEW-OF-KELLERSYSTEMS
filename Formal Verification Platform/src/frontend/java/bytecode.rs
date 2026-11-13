//! JVM Class File format parser and bytecode ingestion.

use std::path::Path;

/// A parsed JVM class file.
#[derive(Debug, Clone)]
pub struct ClassFile {
    pub class_name: String,
    pub super_class: String,
    pub interfaces: Vec<String>,
    pub methods: Vec<Method>,
}

/// A method in the class file.
#[derive(Debug, Clone)]
pub struct Method {
    pub name: String,
    pub descriptor: String,
    pub code: CodeAttribute,
}

/// The Code attribute of a method (bytecode + stack map).
#[derive(Debug, Clone)]
pub struct CodeAttribute {
    pub max_stack: u16,
    pub max_locals: u16,
    pub bytecode: Vec<u8>,
    pub exceptions: Vec<ExceptionTableEntry>,
}

/// An entry in the exception table.
#[derive(Debug, Clone)]
pub struct ExceptionTableEntry {
    pub start_pc: u16,
    pub end_pc: u16,
    pub handler_pc: u16,
    pub catch_type: String,
}

/// A constant pool entry (simplified).
#[derive(Debug, Clone)]
pub enum ConstantPoolEntry {
    Utf8(String),
    Class(String),
    Integer(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    MethodRef(u16, u16),
    FieldRef(u16, u16),
    NameAndType(u16, u16),
}

/// Parse a .class file from disk.
pub fn parse_class_file(path: &Path) -> anyhow::Result<ClassFile> {
    let data = std::fs::read(path)
        .map_err(|e| anyhow::anyhow!("Failed to read {}: {}", path.display(), e))?;

    if data.len() < 8 {
        return Err(anyhow::anyhow!(
            "File too small for class file: {}",
            path.display()
        ));
    }

    // Verify magic number 0xCAFEBABE
    let magic = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
    if magic != 0xCAFEBABE {
        return Err(anyhow::anyhow!(
            "Invalid class file magic: {:08X} (expected CAFEBABE)",
            magic
        ));
    }

    let minor = u16::from_be_bytes([data[4], data[5]]);
    let major = u16::from_be_bytes([data[6], data[7]]);

    if major > 65 {
        return Err(anyhow::anyhow!(
            "Unsupported class file version {}.{} (max supported: 65)",
            major,
            minor
        ));
    }

    let mut reader = ByteReader::new(&data[8..]);
    let cp_count = reader.read_u16()? as usize;

    // Read constant pool (index 1 is unused)
    let mut cp: Vec<Option<ConstantPoolEntry>> = vec![None];
    for _ in 0..cp_count.saturating_sub(1) {
        let tag = reader.read_u8()?;
        let entry = match tag {
            1 => ConstantPoolEntry::Utf8(reader.read_utf8()?),
            3 => ConstantPoolEntry::Integer(reader.read_i32()?),
            4 => ConstantPoolEntry::Long(reader.read_i64()?),
            5 => ConstantPoolEntry::Float(reader.read_f32()?),
            6 => ConstantPoolEntry::Double(reader.read_f64()?),
            7 => {
                let idx = reader.read_u16()?;
                let name_idx = cp.get(idx as usize)
                    .and_then(|e| e.as_ref())
                    .and_then(|e| if let ConstantPoolEntry::Utf8(s) = e { Some(s.clone()) } else { None })
                    .unwrap_or_default();
                ConstantPoolEntry::Class(name_idx)
            }
            9 => {
                let class_idx = reader.read_u16()?;
                let nat_idx = reader.read_u16()?;
                ConstantPoolEntry::MethodRef(class_idx, nat_idx)
            }
            8 => {
                let class_idx = reader.read_u16()?;
                let nat_idx = reader.read_u16()?;
                ConstantPoolEntry::FieldRef(class_idx, nat_idx)
            }
            12 => {
                let name_idx = reader.read_u16()?;
                let desc_idx = reader.read_u16()?;
                ConstantPoolEntry::NameAndType(name_idx, desc_idx)
            }
            _ => {
                // Skip unknown tags (skip 4 bytes for most)
                reader.skip(4)?;
                ConstantPoolEntry::Utf8(format!("<unknown_tag_{}>", tag))
            }
        };
        cp.push(Some(entry));

        // Long and Double take two constant pool entries
        if tag == 4 || tag == 6 {
            cp.push(None);
        }
    }

    let _access_flags = reader.read_u16()?;
    let this_class_idx = reader.read_u16()? as usize;
    let super_class_idx = reader.read_u16()? as usize;

    let class_name = cp.get(this_class_idx)
        .and_then(|e| e.as_ref())
        .and_then(|e| if let ConstantPoolEntry::Class(n) = e { Some(n.clone()) } else { None })
        .unwrap_or_default();

    let super_class = if super_class_idx > 0 {
        cp.get(super_class_idx)
            .and_then(|e| e.as_ref())
            .and_then(|e| if let ConstantPoolEntry::Class(n) = e { Some(n.clone()) } else { None })
            .unwrap_or_default()
    } else {
        String::new()
    };

    // Interfaces
    let iface_count = reader.read_u16()? as usize;
    let mut interfaces = Vec::new();
    for _ in 0..iface_count {
        let idx = reader.read_u16()? as usize;
        let name = cp.get(idx)
            .and_then(|e| e.as_ref())
            .and_then(|e| if let ConstantPoolEntry::Class(n) = e { Some(n.clone()) } else { None })
            .unwrap_or_default();
        interfaces.push(name);
    }

    // Fields (skip for now - we focus on methods)
    let field_count = reader.read_u16()? as usize;
    for _ in 0..field_count {
        reader.skip(2)?; // access_flags
        reader.skip(2)?; // name_index
        reader.skip(2)?; // descriptor_index
        let attr_count = reader.read_u16()? as usize;
        for _ in 0..attr_count {
            reader.skip(2)?; // attribute_name_index
            let attr_len = reader.read_u32()? as usize;
            reader.skip(attr_len)?;
        }
    }

    // Methods
    let method_count = reader.read_u16()? as usize;
    let mut methods = Vec::new();
    for _ in 0..method_count {
        reader.skip(2)?; // access_flags
        let name_idx = reader.read_u16()? as usize;
        let desc_idx = reader.read_u16()? as usize;

        let method_name = cp.get(name_idx)
            .and_then(|e| e.as_ref())
            .and_then(|e| if let ConstantPoolEntry::Utf8(s) = e { Some(s.clone()) } else { None })
            .unwrap_or_default();

        let descriptor = cp.get(desc_idx)
            .and_then(|e| e.as_ref())
            .and_then(|e| if let ConstantPoolEntry::Utf8(s) = e { Some(s.clone()) } else { None })
            .unwrap_or_default();

        // Method attributes
        let attr_count = reader.read_u16()? as usize;
        let mut code = CodeAttribute {
            max_stack: 0,
            max_locals: 0,
            bytecode: Vec::new(),
            exceptions: Vec::new(),
        };

        for _ in 0..attr_count {
            let attr_name_idx = reader.read_u16()? as usize;
            let attr_len = reader.read_u32()? as usize;

            let attr_name = cp.get(attr_name_idx)
                .and_then(|e| e.as_ref())
                .and_then(|e| if let ConstantPoolEntry::Utf8(s) = e { Some(s.clone()) } else { None })
                .unwrap_or_default();

            if attr_name == "Code" {
                let max_stack = reader.read_u16()?;
                let max_locals = reader.read_u16()?;
                let code_len = reader.read_u32()? as usize;
                let bytecode = reader.read_bytes(code_len)?;

                // Exception table
                let exc_count = reader.read_u16()? as usize;
                let mut exceptions = Vec::new();
                for _ in 0..exc_count {
                    let start_pc = reader.read_u16()?;
                    let end_pc = reader.read_u16()?;
                    let handler_pc = reader.read_u16()?;
                    let catch_idx = reader.read_u16()? as usize;
                    let catch_type = cp.get(catch_idx)
                        .and_then(|e| e.as_ref())
                        .and_then(|e| if let ConstantPoolEntry::Class(n) = e { Some(n.clone()) } else { None })
                        .unwrap_or_default();
                    exceptions.push(ExceptionTableEntry {
                        start_pc,
                        end_pc,
                        handler_pc,
                        catch_type,
                    });
                }

                // StackMapTable (skip)
                let remaining = attr_len as usize - 8 - code_len - (4 * exc_count);
                if remaining > 0 {
                    reader.skip(remaining)?;
                }

                code = CodeAttribute {
                    max_stack,
                    max_locals,
                    bytecode,
                    exceptions,
                };
            } else {
                reader.skip(attr_len as usize)?;
            }
        }

        methods.push(Method {
            name: method_name,
            descriptor,
            code,
        });
    }

    Ok(ClassFile {
        class_name,
        super_class,
        interfaces,
        methods,
    })
}

/// Low-level byte reader for the class file format.
struct ByteReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> ByteReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn read_u8(&mut self) -> anyhow::Result<u8> {
        if self.pos >= self.data.len() {
            return Err(anyhow::anyhow!("Unexpected end of class file"));
        }
        let v = self.data[self.pos];
        self.pos += 1;
        Ok(v)
    }

    fn read_u16(&mut self) -> anyhow::Result<u16> {
        if self.pos + 2 > self.data.len() {
            return Err(anyhow::anyhow!("Unexpected end of class file (u16)"));
        }
        let v = u16::from_be_bytes([self.data[self.pos], self.data[self.pos + 1]]);
        self.pos += 2;
        Ok(v)
    }

    fn read_u32(&mut self) -> anyhow::Result<u32> {
        if self.pos + 4 > self.data.len() {
            return Err(anyhow::anyhow!("Unexpected end of class file (u32)"));
        }
        let v = u32::from_be_bytes([
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
        ]);
        self.pos += 4;
        Ok(v)
    }

    fn read_i32(&mut self) -> anyhow::Result<i32> {
        let v = self.read_u32()? as i32;
        Ok(v)
    }

    fn read_i64(&mut self) -> anyhow::Result<i64> {
        let next_pos = self.pos.checked_add(8).ok_or_else(|| anyhow::anyhow!("Overflow calculating end of i64"))?;
        if next_pos > self.data.len() {
            return Err(anyhow::anyhow!("Unexpected end of class file (i64)"));
        }
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&self.data[self.pos..next_pos]);
        self.pos = next_pos;
        Ok(i64::from_be_bytes(bytes))
    }

    fn read_f32(&mut self) -> anyhow::Result<f32> {
        let bits = self.read_u32()?;
        Ok(f32::from_bits(bits))
    }

    fn read_f64(&mut self) -> anyhow::Result<f64> {
        let bits = self.read_i64()? as u64;
        Ok(f64::from_bits(bits))
    }

    fn read_utf8(&mut self) -> anyhow::Result<String> {
        let len = self.read_u16()? as usize;
        let next_pos = self.pos.checked_add(len).ok_or_else(|| anyhow::anyhow!("Overflow calculating end of utf8"))?;
        if next_pos > self.data.len() {
            return Err(anyhow::anyhow!("Unexpected end of class file (utf8)"));
        }
        let bytes = &self.data[self.pos..next_pos];
        self.pos = next_pos;
        Ok(String::from_utf8_lossy(bytes).to_string())
    }

    fn read_bytes(&mut self, len: usize) -> anyhow::Result<Vec<u8>> {
        let next_pos = self.pos.checked_add(len).ok_or_else(|| anyhow::anyhow!("Overflow calculating end of bytes"))?;
        if next_pos > self.data.len() {
            return Err(anyhow::anyhow!("Unexpected end of class file (bytes)"));
        }
        let bytes = self.data[self.pos..next_pos].to_vec();
        self.pos = next_pos;
        Ok(bytes)
    }

    fn skip(&mut self, n: usize) -> anyhow::Result<()> {
        if self.pos + n > self.data.len() {
            return Err(anyhow::anyhow!("Unexpected end of class file (skip {})", n));
        }
        self.pos += n;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal valid .class file in memory for testing.
    fn build_minimal_class() -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::new();

        // Magic + version
        buf.extend_from_slice(&[0xCA, 0xFE, 0xBA, 0xBE]);
        buf.extend_from_slice(&[0x00, 0x00]); // minor 0
        buf.extend_from_slice(&[0x00, 0x34]); // major 52 (Java 8)

        // Constant pool count = 5 (indices 1..=4)
        buf.extend_from_slice(&[0x00, 0x05]);

        // CP[1]: Utf8 "java/lang/Object"
        buf.push(1);
        let s = b"java/lang/Object";
        buf.extend_from_slice(&(s.len() as u16).to_be_bytes());
        buf.extend_from_slice(s);

        // CP[2]: Class -> 1
        buf.push(7);
        buf.extend_from_slice(&[0x00, 0x01]);

        // CP[3]: Utf8 "Test"
        buf.push(1);
        let s = b"Test";
        buf.extend_from_slice(&(s.len() as u16).to_be_bytes());
        buf.extend_from_slice(s);

        // CP[4]: Class -> 3
        buf.push(7);
        buf.extend_from_slice(&[0x00, 0x03]);

        // access_flags = 0x0001 (public)
        buf.extend_from_slice(&[0x00, 0x01]);
        // this_class = 4
        buf.extend_from_slice(&[0x00, 0x04]);
        // super_class = 2
        buf.extend_from_slice(&[0x00, 0x02]);

        // interfaces_count = 0
        buf.extend_from_slice(&[0x00, 0x00]);

        // fields_count = 0
        buf.extend_from_slice(&[0x00, 0x00]);

        // methods_count = 1
        buf.extend_from_slice(&[0x00, 0x01]);

        // Method: access=1 (public), name_idx=1, desc_idx=1
        buf.extend_from_slice(&[0x00, 0x01]);
        buf.extend_from_slice(&[0x00, 0x01]);
        buf.extend_from_slice(&[0x00, 0x01]);

        // attributes_count = 0
        buf.extend_from_slice(&[0x00, 0x00]);

        buf
    }

    #[test]
    fn test_parse_minimal_class() {
        let dir = std::env::temp_dir();
        let path = dir.join("keller_test_minimal.class");
        std::fs::write(&path, build_minimal_class()).unwrap();

        let cf = parse_class_file(&path).unwrap();
        assert_eq!(cf.class_name, "Test");
        assert_eq!(cf.super_class, "java/lang/Object");
        assert_eq!(cf.methods.len(), 1);

        let _ = std::fs::remove_file(&path);
    }
}
