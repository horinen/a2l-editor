use crate::dwarf::{analyze_variables_with_dwarf, DwarfParser};
use crate::types::{
    infer_a2l_type_from_encoding, A2lEntry, A2lEntryStore, TypeInfo, TypeKind, Variable,
    MAX_ARRAY_EXPAND, MAX_NESTING_DEPTH,
};
use anyhow::{Context, Result};
use memmap2::Mmap;
use object::{Object, ObjectSection, ObjectSymbol};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::path::Path;

pub struct ElfParser {
    variables: Vec<Variable>,
    file_size: u64,
    has_dwarf: bool,
    dwarf_stats: Option<DwarfStats>,
    a2l_entries: Option<A2lEntryStore>,
    #[allow(dead_code)]
    type_cache: Option<HashMap<u64, TypeInfo>>,
}

#[derive(Clone)]
pub struct DwarfStats {
    pub base_types: usize,
    pub structs: usize,
    pub unions: usize,
    pub enums: usize,
    pub arrays: usize,
    pub pointers: usize,
    pub typedefs: usize,
    pub variables: usize,
    pub struct_members: usize,
    pub enum_values: usize,
}

impl ElfParser {
    pub fn parse(path: &Path) -> Result<Self> {
        Self::parse_with_depth(path, false)
    }

    pub fn parse_deep(path: &Path) -> Result<Self> {
        Self::parse_with_depth(path, true)
    }

    pub fn parse_with_depth(path: &Path, deep: bool) -> Result<Self> {
        let file = File::open(path).context("无法打开 ELF 文件")?;
        let metadata = file.metadata().context("无法读取文件元数据")?;
        let file_size = metadata.len();

        let mmap = unsafe { Mmap::map(&file).context("无法创建内存映射")? };

        let obj = object::File::parse(&*mmap).context("无法解析 ELF 文件")?;

        let mut variables = Vec::new();
        let mut seen = HashSet::new();

        let sections: Vec<_> = obj.sections().collect();
        let section_map: std::collections::HashMap<_, _> = sections
            .iter()
            .map(|s| (s.index(), s.name().unwrap_or("")))
            .collect();

        for symbol in obj.symbols() {
            let name = match symbol.name() {
                Ok(n) if !n.is_empty() => n,
                _ => continue,
            };

            if name.starts_with('.') {
                continue;
            }

            if seen.contains(name) {
                continue;
            }

            let address = symbol.address();
            let size = symbol.size() as usize;

            if address == 0 && size == 0 {
                continue;
            }

            if size == 0 {
                continue;
            }

            let section_name = symbol
                .section_index()
                .and_then(|idx| section_map.get(&idx))
                .unwrap_or(&"");

            let is_data = section_name.contains("data")
                || section_name.contains("bss")
                || section_name.contains("rodata")
                || section_name.starts_with(".");

            if !is_data {
                continue;
            }

            let type_name = Self::infer_type_name(size);

            variables.push(Variable::new(
                name.to_string(),
                address,
                size,
                type_name,
                section_name.to_string(),
            ));

            seen.insert(name.to_string());
        }

        variables.sort_by(|a, b| a.name.cmp(&b.name));

        let (has_dwarf, dwarf_stats, type_cache, a2l_entries) = if deep {
            let parser = DwarfParser::parse(&mmap).unwrap_or_else(|_| DwarfParser::new());
            let has_dwarf = parser.has_dwarf_info();

            let stats = if has_dwarf {
                let (
                    base_types,
                    structs,
                    unions,
                    enums,
                    arrays,
                    pointers,
                    typedefs,
                    vars,
                    members,
                    values,
                ) = parser.get_stats();
                Some(DwarfStats {
                    base_types,
                    structs,
                    unions,
                    enums,
                    arrays,
                    pointers,
                    typedefs,
                    variables: vars,
                    struct_members: members,
                    enum_values: values,
                })
            } else {
                None
            };

            analyze_variables_with_dwarf(&mut variables, &mmap).ok();

            let tc = parser.type_cache().clone();
            let entries = Self::expand_all_entries(&variables, &tc);

            (has_dwarf, stats, Some(tc), Some(entries))
        } else {
            (false, None, None, None)
        };

        Ok(Self {
            variables,
            file_size,
            has_dwarf,
            dwarf_stats,
            type_cache,
            a2l_entries,
        })
    }

    fn infer_type_name(size: usize) -> String {
        match size {
            1 => "uint8_t".to_string(),
            2 => "uint16_t".to_string(),
            4 => "uint32_t".to_string(),
            8 => "uint64_t".to_string(),
            _ => format!("uint8_t[{}]", size),
        }
    }

    pub fn variables(&self) -> &[Variable] {
        &self.variables
    }

    pub fn variable_count(&self) -> usize {
        self.variables.len()
    }

    pub fn file_size(&self) -> u64 {
        self.file_size
    }

    pub fn has_dwarf(&self) -> bool {
        self.has_dwarf
    }

    pub fn dwarf_stats(&self) -> Option<&DwarfStats> {
        self.dwarf_stats.as_ref()
    }

    pub fn search(&self, pattern: &str) -> Vec<&Variable> {
        let pattern_lower = pattern.to_lowercase();
        self.variables
            .iter()
            .filter(|v| v.name.to_lowercase().contains(&pattern_lower))
            .collect()
    }

    pub fn get_by_name(&self, name: &str) -> Option<&Variable> {
        self.variables.iter().find(|v| v.name == name)
    }

    pub fn a2l_entries(&self) -> Option<&A2lEntryStore> {
        self.a2l_entries.as_ref()
    }

    pub fn a2l_entry_count(&self) -> usize {
        self.a2l_entries.as_ref().map(|s| s.len()).unwrap_or(0)
    }

    pub fn set_a2l_entries(&mut self, store: A2lEntryStore) {
        self.a2l_entries = Some(store);
    }

    fn expand_all_entries(
        variables: &[Variable],
        type_cache: &HashMap<u64, TypeInfo>,
    ) -> A2lEntryStore {
        let mut store = A2lEntryStore::new();

        for var in variables {
            Self::expand_variable(var, type_cache, &mut store);
        }

        store
    }

    fn expand_variable(
        var: &Variable,
        type_cache: &HashMap<u64, TypeInfo>,
        store: &mut A2lEntryStore,
    ) {
        let mut visited: HashSet<u64> = HashSet::new();

        if let Some(ref type_info) = var.type_info {
            Self::expand_recursive(
                &var.name,
                var.address,
                type_info,
                0,
                &mut visited,
                type_cache,
                store,
                None,
            );
        } else {
            let a2l_type = infer_a2l_type_from_encoding(var.size, Default::default());
            store.add(A2lEntry::new(
                var.name.clone(),
                var.address,
                var.size,
                a2l_type.to_string(),
                var.type_name.clone(),
            ));
        }
    }

    fn expand_recursive(
        prefix: &str,
        base_addr: u64,
        type_info: &TypeInfo,
        depth: usize,
        visited: &mut HashSet<u64>,
        type_cache: &HashMap<u64, TypeInfo>,
        store: &mut A2lEntryStore,
        array_index: Option<Vec<usize>>,
    ) {
        if depth > MAX_NESTING_DEPTH {
            return;
        }

        if type_info.offset > 0 && visited.contains(&type_info.offset) {
            return;
        }
        visited.insert(type_info.offset);

        let a2l_type = infer_a2l_type_from_encoding(type_info.size, type_info.encoding);
        store.add(
            A2lEntry::new(
                prefix.to_string(),
                base_addr,
                type_info.size,
                a2l_type.to_string(),
                type_info.name.clone(),
            )
            .with_array_index(array_index.clone().unwrap_or_default()),
        );

        match type_info.kind {
            TypeKind::Struct | TypeKind::Union => {
                for member in &type_info.members {
                    let member_full_name = format!("{}.{}", prefix, member.name);
                    let member_addr = base_addr + member.offset as u64;

                    if member.is_bitfield() {
                        let member_a2l_type =
                            infer_a2l_type_from_encoding(member.type_size, type_info.encoding);
                        store.add(
                            A2lEntry::new(
                                member_full_name,
                                member_addr,
                                member.type_size,
                                member_a2l_type.to_string(),
                                member.type_name.clone(),
                            )
                            .with_bitfield(
                                member.bit_offset.unwrap_or(0),
                                member.bit_size.unwrap_or(0),
                            ),
                        );
                    } else if let Some(type_offset) = member.type_offset {
                        if type_offset > 0 {
                            if let Some(member_type) = type_cache.get(&type_offset) {
                                Self::expand_recursive(
                                    &member_full_name,
                                    member_addr,
                                    member_type,
                                    depth + 1,
                                    visited,
                                    type_cache,
                                    store,
                                    None,
                                );
                            }
                        }
                    }
                }
            }
            TypeKind::Array => {
                let (effective_dims, final_elem_type, final_elem_size) =
                    Self::flatten_array_type(type_info, 0);

                let total_elements: usize = effective_dims.iter().product();
                let original_total: usize = type_info.array_dims.iter().product();

                if original_total <= MAX_ARRAY_EXPAND && original_total > 0 {
                    if let Some(ref elem_type) = final_elem_type {
                        let base_idx = array_index.clone().unwrap_or_default();
                        Self::expand_multi_dim_array(
                            prefix,
                            base_addr,
                            elem_type,
                            &effective_dims,
                            final_elem_size,
                            depth,
                            visited,
                            type_cache,
                            store,
                            &base_idx,
                        );
                    } else if total_elements > 0 {
                        for i in 0..total_elements {
                            let multi_idx = Self::flat_to_multi_index(i, &effective_dims);
                            let idx: Vec<usize> = array_index
                                .clone()
                                .unwrap_or_default()
                                .into_iter()
                                .chain(multi_idx.into_iter())
                                .collect();
                            let elem_name = Self::format_array_element_name(prefix, &idx);
                            let elem_addr = base_addr + (i * final_elem_size) as u64;
                            let elem_a2l_type =
                                infer_a2l_type_from_encoding(final_elem_size, type_info.encoding);
                            store.add(
                                A2lEntry::new(
                                    elem_name,
                                    elem_addr,
                                    final_elem_size,
                                    elem_a2l_type.to_string(),
                                    type_info.name.clone(),
                                )
                                .with_array_index(idx),
                            );
                        }
                    }
                }
            }
            _ => {}
        }

        visited.remove(&type_info.offset);
    }

    fn format_array_element_name(prefix: &str, indices: &[usize]) -> String {
        if indices.is_empty() {
            return prefix.to_string();
        }
        let idx_str: Vec<String> = indices.iter().map(|i| format!("._{}_", i)).collect();
        format!("{}{}", prefix, idx_str.join(""))
    }

    fn flatten_array_type(
        type_info: &TypeInfo,
        base_elem_size: usize,
    ) -> (Vec<usize>, Option<TypeInfo>, usize) {
        let mut all_dims: Vec<usize> = type_info
            .array_dims
            .iter()
            .filter(|&&d| d > 1)
            .copied()
            .collect();

        let mut current_size = if type_info.size > 0 {
            let total: usize = type_info.array_dims.iter().product();
            if total > 0 {
                type_info.size / total
            } else {
                base_elem_size
            }
        } else {
            base_elem_size
        };

        let mut elem_type = type_info.pointer_target.clone();

        while let Some(ref inner) = elem_type {
            if inner.kind == TypeKind::Array {
                let inner_dims: Vec<usize> = inner
                    .array_dims
                    .iter()
                    .filter(|&&d| d > 1)
                    .copied()
                    .collect();
                all_dims.extend(inner_dims);

                let inner_total: usize = inner.array_dims.iter().product();
                if inner_total > 0 && inner.size > 0 {
                    current_size = inner.size / inner_total;
                }
                elem_type = inner.pointer_target.clone();
            } else {
                break;
            }
        }

        (all_dims, elem_type.map(|b| *b), current_size)
    }

    fn flat_to_multi_index(flat_idx: usize, dims: &[usize]) -> Vec<usize> {
        let mut result = Vec::with_capacity(dims.len());
        let mut remaining = flat_idx;
        for i in (1..dims.len()).rev() {
            let stride: usize = dims[i + 1..].iter().product();
            result.push(remaining / stride);
            remaining %= stride;
        }
        result.push(remaining);
        result
    }

    fn expand_multi_dim_array(
        prefix: &str,
        base_addr: u64,
        elem_type: &TypeInfo,
        dims: &[usize],
        elem_size: usize,
        depth: usize,
        visited: &mut HashSet<u64>,
        type_cache: &HashMap<u64, TypeInfo>,
        store: &mut A2lEntryStore,
        base_idx: &[usize],
    ) {
        if dims.is_empty() {
            let fixed_elem_type = if elem_type.size == 0 {
                let mut t = elem_type.clone();
                t.size = elem_size;
                t
            } else {
                elem_type.clone()
            };
            Self::expand_recursive(
                prefix,
                base_addr,
                &fixed_elem_type,
                depth,
                visited,
                type_cache,
                store,
                Some(base_idx.to_vec()),
            );
            return;
        }

        let current_dim = dims[0];
        let remaining_dims = &dims[1..];
        let stride: usize = remaining_dims.iter().product::<usize>() * elem_size;

        for i in 0..current_dim {
            let mut full_idx = base_idx.to_vec();
            full_idx.push(i);
            let elem_name = format!("{}._{}_", prefix, i);
            let elem_addr = base_addr + (i * stride) as u64;
            Self::expand_multi_dim_array(
                &elem_name,
                elem_addr,
                elem_type,
                remaining_dims,
                elem_size,
                depth,
                visited,
                type_cache,
                store,
                &full_idx,
            );
        }
    }
}
