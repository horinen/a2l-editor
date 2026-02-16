pub mod a2l;
pub mod cache;
pub mod data_package;
pub mod dwarf;
pub mod elf;
pub mod hash;
pub mod types;

pub use a2l::{
    A2lEntryInfo, A2lGenerator, A2lParser, A2lVariable, AppendResult, ExportKind, SaveResult,
    VariableChanges, VariableEdit,
};
pub use cache::Cache;
pub use data_package::{DataPackage, PackageMeta};
pub use dwarf::{analyze_variables_with_dwarf, DwarfParser};
pub use elf::{DwarfStats, ElfParser};
pub use hash::{compute_file_hash, format_file_size};
pub use types::{
    infer_a2l_type, infer_a2l_type_from_encoding, A2lEntry, A2lEntryStore, CacheEntry, Endianness,
    EnumVariant, StructMember, TypeEncoding, TypeInfo, TypeKind, Variable, MAX_ARRAY_EXPAND,
    MAX_NESTING_DEPTH,
};
