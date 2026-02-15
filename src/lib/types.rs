use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const MAX_ARRAY_EXPAND: usize = 1000;
pub const MAX_NESTING_DEPTH: usize = 50;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    pub name: String,
    pub address: u64,
    pub size: usize,
    pub type_name: String,
    pub section: String,
    pub type_info: Option<TypeInfo>,
}

impl Variable {
    pub fn new(
        name: String,
        address: u64,
        size: usize,
        type_name: String,
        section: String,
    ) -> Self {
        Self {
            name,
            address,
            size,
            type_name,
            section,
            type_info: None,
        }
    }

    pub fn with_type_info(mut self, type_info: TypeInfo) -> Self {
        self.type_info = Some(type_info);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeInfo {
    pub name: String,
    pub size: usize,
    pub encoding: TypeEncoding,
    pub kind: TypeKind,
    pub members: Vec<StructMember>,
    pub variants: Vec<EnumVariant>,
    pub array_dims: Vec<usize>,
    pub pointer_target: Option<Box<TypeInfo>>,
    pub offset: u64,
}

impl TypeInfo {
    pub fn new(name: String, size: usize, encoding: TypeEncoding) -> Self {
        Self {
            name,
            size,
            encoding,
            kind: TypeKind::Primitive,
            members: Vec::new(),
            variants: Vec::new(),
            array_dims: Vec::new(),
            pointer_target: None,
            offset: 0,
        }
    }

    pub fn primitive(name: String, size: usize, encoding: TypeEncoding) -> Self {
        Self {
            name,
            size,
            encoding,
            kind: TypeKind::Primitive,
            members: Vec::new(),
            variants: Vec::new(),
            array_dims: Vec::new(),
            pointer_target: None,
            offset: 0,
        }
    }

    pub fn struct_type(name: String, size: usize, members: Vec<StructMember>, offset: u64) -> Self {
        Self {
            name,
            size,
            encoding: TypeEncoding::Unsigned,
            kind: TypeKind::Struct,
            members,
            variants: Vec::new(),
            array_dims: Vec::new(),
            pointer_target: None,
            offset,
        }
    }

    pub fn union_type(name: String, size: usize, members: Vec<StructMember>, offset: u64) -> Self {
        Self {
            name,
            size,
            encoding: TypeEncoding::Unsigned,
            kind: TypeKind::Union,
            members,
            variants: Vec::new(),
            array_dims: Vec::new(),
            pointer_target: None,
            offset,
        }
    }

    pub fn enum_type(
        name: String,
        size: usize,
        encoding: TypeEncoding,
        variants: Vec<EnumVariant>,
        offset: u64,
    ) -> Self {
        Self {
            name,
            size,
            encoding,
            kind: TypeKind::Enum,
            members: Vec::new(),
            variants,
            array_dims: Vec::new(),
            pointer_target: None,
            offset,
        }
    }

    pub fn array_type(
        name: String,
        size: usize,
        element_type: TypeInfo,
        dims: Vec<usize>,
        offset: u64,
    ) -> Self {
        Self {
            name,
            size,
            encoding: element_type.encoding,
            kind: TypeKind::Array,
            members: Vec::new(),
            variants: Vec::new(),
            array_dims: dims,
            pointer_target: Some(Box::new(element_type)),
            offset,
        }
    }

    pub fn pointer_type(name: String, size: usize, target: TypeInfo, offset: u64) -> Self {
        Self {
            name,
            size,
            encoding: TypeEncoding::Unsigned,
            kind: TypeKind::Pointer,
            members: Vec::new(),
            variants: Vec::new(),
            array_dims: Vec::new(),
            pointer_target: Some(Box::new(target)),
            offset,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TypeKind {
    Primitive,
    Struct,
    Union,
    Enum,
    Array,
    Pointer,
    Typedef,
}

impl std::fmt::Display for TypeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeKind::Primitive => write!(f, "primitive"),
            TypeKind::Struct => write!(f, "struct"),
            TypeKind::Union => write!(f, "union"),
            TypeKind::Enum => write!(f, "enum"),
            TypeKind::Array => write!(f, "array"),
            TypeKind::Pointer => write!(f, "pointer"),
            TypeKind::Typedef => write!(f, "typedef"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Endianness {
    Little,
    Big,
}

impl Default for Endianness {
    fn default() -> Self {
        Endianness::Little
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TypeEncoding {
    #[default]
    Unsigned,
    Signed,
    Float,
}

impl std::fmt::Display for TypeEncoding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeEncoding::Unsigned => write!(f, "unsigned"),
            TypeEncoding::Signed => write!(f, "signed"),
            TypeEncoding::Float => write!(f, "float"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructMember {
    pub name: String,
    pub offset: usize,
    pub type_name: String,
    pub type_size: usize,
    pub type_offset: Option<u64>,
    pub bit_offset: Option<usize>,
    pub bit_size: Option<usize>,
}

impl StructMember {
    pub fn new(name: String, offset: usize, type_name: String, type_size: usize) -> Self {
        Self {
            name,
            offset,
            type_name,
            type_size,
            type_offset: None,
            bit_offset: None,
            bit_size: None,
        }
    }

    pub fn with_type_offset(mut self, type_offset: u64) -> Self {
        self.type_offset = Some(type_offset);
        self
    }

    pub fn with_bitfield(mut self, bit_offset: usize, bit_size: usize) -> Self {
        self.bit_offset = Some(bit_offset);
        self.bit_size = Some(bit_size);
        self
    }

    pub fn is_bitfield(&self) -> bool {
        self.bit_size.is_some()
    }

    pub fn get_effective_bit_offset(
        &self,
        endianness: Endianness,
        container_size_bits: usize,
    ) -> Option<usize> {
        let raw_offset = self.bit_offset?;

        let effective = match endianness {
            Endianness::Little => container_size_bits - raw_offset - self.bit_size.unwrap_or(0),
            Endianness::Big => raw_offset,
        };

        Some(effective)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumVariant {
    pub name: String,
    pub value: i64,
}

impl EnumVariant {
    pub fn new(name: String, value: i64) -> Self {
        Self { name, value }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub file_hash: String,
    pub file_path: String,
    pub file_size: u64,
    pub modified_time: u64,
    pub variable_count: usize,
    pub parse_time_ms: u64,
    pub created_at: i64,
    pub has_dwarf: bool,
}

impl CacheEntry {
    pub fn new(
        file_hash: String,
        file_path: String,
        file_size: u64,
        modified_time: u64,
        variable_count: usize,
        parse_time_ms: u64,
        has_dwarf: bool,
    ) -> Self {
        Self {
            file_hash,
            file_path,
            file_size,
            modified_time,
            variable_count,
            parse_time_ms,
            created_at: chrono::Utc::now().timestamp(),
            has_dwarf,
        }
    }
}

pub fn infer_a2l_type(size: usize, type_name: &str) -> &'static str {
    let lower = type_name.to_lowercase();

    if lower.contains("float") || lower.contains("double") {
        return match size {
            4 => "FLOAT32_IEEE",
            8 => "FLOAT64_IEEE",
            _ => "FLOAT64_IEEE",
        };
    }

    if lower.contains("u8") || lower.contains("uint8") || lower.contains("char") {
        return "UBYTE";
    }
    if lower.contains("u16") || lower.contains("uint16") || lower.contains("wchar") {
        return "UWORD";
    }
    if lower.contains("u32") || lower.contains("uint32") {
        return "ULONG";
    }
    if lower.contains("u64") || lower.contains("uint64") {
        return "A_UINT64";
    }

    if lower.contains("i8") || lower.contains("int8") || lower.contains("sbyte") {
        return "SBYTE";
    }
    if lower.contains("i16") || lower.contains("int16") || lower.contains("short") {
        return "SWORD";
    }
    if lower.contains("i32") || lower.contains("int32") || lower.contains("int") {
        return "SLONG";
    }
    if lower.contains("i64") || lower.contains("int64") {
        return "A_INT64";
    }

    match size {
        1 => "UBYTE",
        2 => "UWORD",
        4 => "ULONG",
        8 => "A_UINT64",
        _ => "UBYTE",
    }
}

pub fn infer_a2l_type_from_encoding(size: usize, encoding: TypeEncoding) -> &'static str {
    match (size, encoding) {
        (1, TypeEncoding::Unsigned) => "UBYTE",
        (1, TypeEncoding::Signed) => "SBYTE",
        (2, TypeEncoding::Unsigned) => "UWORD",
        (2, TypeEncoding::Signed) => "SWORD",
        (4, TypeEncoding::Unsigned) => "ULONG",
        (4, TypeEncoding::Signed) => "SLONG",
        (4, TypeEncoding::Float) => "FLOAT32_IEEE",
        (8, TypeEncoding::Unsigned) => "A_UINT64",
        (8, TypeEncoding::Signed) => "A_INT64",
        (8, TypeEncoding::Float) => "FLOAT64_IEEE",
        _ => "UBYTE",
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2lEntry {
    pub full_name: String,
    pub address: u64,
    pub size: usize,
    pub a2l_type: String,
    pub type_name: String,
    pub bit_offset: Option<usize>,
    pub bit_size: Option<usize>,
    pub array_index: Option<Vec<usize>>,
}

impl A2lEntry {
    pub fn new(
        full_name: String,
        address: u64,
        size: usize,
        a2l_type: String,
        type_name: String,
    ) -> Self {
        Self {
            full_name,
            address,
            size,
            a2l_type,
            type_name,
            bit_offset: None,
            bit_size: None,
            array_index: None,
        }
    }

    pub fn with_bitfield(mut self, bit_offset: usize, bit_size: usize) -> Self {
        self.bit_offset = Some(bit_offset);
        self.bit_size = Some(bit_size);
        self
    }

    pub fn with_array_index(mut self, index: Vec<usize>) -> Self {
        self.array_index = Some(index);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2lEntryStore {
    pub entries: Vec<A2lEntry>,
    pub name_index: HashMap<String, usize>,
}

impl A2lEntryStore {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            name_index: HashMap::new(),
        }
    }

    pub fn add(&mut self, entry: A2lEntry) {
        let idx = self.entries.len();
        self.name_index.insert(entry.full_name.clone(), idx);
        self.entries.push(entry);
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn get_by_name(&self, name: &str) -> Option<&A2lEntry> {
        self.name_index.get(name).map(|&idx| &self.entries[idx])
    }

    pub fn search(&self, query: &str) -> Vec<&A2lEntry> {
        let query_lower = query.to_lowercase();
        self.entries
            .iter()
            .filter(|e| e.full_name.to_lowercase().contains(&query_lower))
            .collect()
    }
}

impl Default for A2lEntryStore {
    fn default() -> Self {
        Self::new()
    }
}
