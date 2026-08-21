use crate::dwarf::DwarfParser;
use crate::types::{
    infer_a2l_type_from_encoding, A2lEntry, A2lEntryStore, BitfieldGroup, TypeInfo, TypeKind,
    Variable, MAX_ARRAY_EXPAND, MAX_NESTING_DEPTH,
};
use anyhow::{Context, Result};
use memmap2::Mmap;
use object::{Object, ObjectSection, ObjectSymbol};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::path::Path;
use std::time::{Duration, Instant};

pub struct ElfParser {
    variables: Vec<Variable>,
    file_size: u64,
    has_dwarf: bool,
    dwarf_stats: Option<DwarfStats>,
    profile: Option<GenerationProfile>,
    a2l_entries: Option<A2lEntryStore>,
    #[allow(dead_code)]
    type_cache: Option<HashMap<u64, TypeInfo>>,
}

#[derive(Clone, Default)]
pub struct GenerationProfile {
    pub elf_object_parse: Duration,
    pub dwarf_parse_total: Duration,
    pub dwarf_die_traversal: Duration,
    pub dwarf_units: usize,
    pub dwarf_dies: usize,
    pub dwarf_members: usize,
    pub dwarf_variables: usize,
    pub dwarf_other_tags: usize,
    pub dwarf_composites_saved: usize,
    pub dwarf_name_attrs: usize,
    pub dwarf_inline_name_cache_hits: usize,
    pub dwarf_inline_name_cache_misses: usize,
    pub dwarf_debug_str_refs: usize,
    pub dwarf_debug_str_cache_hits: usize,
    pub dwarf_debug_str_cache_misses: usize,
    pub type_resolver: Duration,
    pub resolver_array_elements: Duration,
    pub resolver_alias_chains: Duration,
    pub resolver_member_types: Duration,
    pub resolver_bitfields: Duration,
    pub resolver_alias_count: usize,
    pub resolver_alias_resolved: usize,
    pub resolver_alias_zero_size: usize,
    pub resolver_alias_same_resolved: usize,
    pub resolver_alias_updates: usize,
    pub resolver_member_count: usize,
    pub resolver_member_unique_type_offsets: usize,
    pub resolver_member_updates: usize,
    pub variable_extraction: Duration,
    pub type_cache_clone: Duration,
    pub a2l_entry_expansion: Duration,
    pub type_count: usize,
    pub dwarf_variable_count: usize,
    pub variable_count: usize,
    pub entry_count: usize,
}

impl GenerationProfile {
    pub fn total_parse_time(&self) -> Duration {
        self.elf_object_parse
            + self.dwarf_parse_total
            + self.variable_extraction
            + self.type_cache_clone
            + self.a2l_entry_expansion
    }
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

struct ExpandContext<'a> {
    type_cache: &'a HashMap<u64, TypeInfo>,
    store: &'a mut A2lEntryStore,
    visited: HashSet<u64>,
    root_symbol: &'a str,
    root_addr: u64,
}

impl ElfParser {
    pub fn parse(path: &Path) -> Result<Self> {
        Self::parse_with_depth(path, false)
    }

    pub fn parse_deep(path: &Path) -> Result<Self> {
        Self::parse_with_depth(path, true)
    }

    pub fn parse_with_depth(path: &Path, deep: bool) -> Result<Self> {
        let object_start = Instant::now();
        let file = File::open(path).context("无法打开 ELF 文件")?;
        let metadata = file.metadata().context("无法读取文件元数据")?;
        let file_size = metadata.len();

        let mmap = unsafe { Mmap::map(&file).context("无法创建内存映射")? };

        let obj = object::File::parse(&*mmap).context("无法解析 ELF 文件")?;
        let elf_object_parse = object_start.elapsed();

        let (variables, has_dwarf, dwarf_stats, profile, type_cache, a2l_entries) = if deep {
            let dwarf_start = Instant::now();
            let parser = DwarfParser::parse(&mmap).context("DWARF 解析失败")?;
            let dwarf_parse_total = dwarf_start.elapsed();

            if !parser.has_dwarf_info() {
                anyhow::bail!("ELF 文件不包含 DWARF 调试信息，深度解析需要 DWARF 数据");
            }

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
            let stats = Some(DwarfStats {
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
            });

            if parser.global_variable_count() == 0 {
                anyhow::bail!("DWARF 中未找到全局变量");
            }

            let variable_start = Instant::now();
            let variables = Self::extract_variables_from_dwarf(&parser);
            let variable_extraction = variable_start.elapsed();

            let type_count = parser.type_cache().len();
            let expansion_start = Instant::now();
            let entries = Self::expand_all_entries(&variables, parser.type_cache());
            let a2l_entry_expansion = expansion_start.elapsed();

            let profile = GenerationProfile {
                elf_object_parse,
                dwarf_parse_total,
                dwarf_die_traversal: parser.timings().die_traversal,
                dwarf_units: parser.timings().traversal_stats.units,
                dwarf_dies: parser.timings().traversal_stats.dies,
                dwarf_members: parser.timings().traversal_stats.members,
                dwarf_variables: parser.timings().traversal_stats.variables,
                dwarf_other_tags: parser.timings().traversal_stats.other_tags,
                dwarf_composites_saved: parser.timings().traversal_stats.composites_saved,
                dwarf_name_attrs: parser.timings().traversal_stats.name_attrs,
                dwarf_inline_name_cache_hits: parser
                    .timings()
                    .traversal_stats
                    .inline_name_cache_hits,
                dwarf_inline_name_cache_misses: parser
                    .timings()
                    .traversal_stats
                    .inline_name_cache_misses,
                dwarf_debug_str_refs: parser.timings().traversal_stats.debug_str_refs,
                dwarf_debug_str_cache_hits: parser.timings().traversal_stats.debug_str_cache_hits,
                dwarf_debug_str_cache_misses: parser
                    .timings()
                    .traversal_stats
                    .debug_str_cache_misses,
                type_resolver: parser.timings().type_resolver,
                resolver_array_elements: parser.timings().resolver_detail.array_elements,
                resolver_alias_chains: parser.timings().resolver_detail.alias_chains,
                resolver_member_types: parser.timings().resolver_detail.member_types,
                resolver_bitfields: parser.timings().resolver_detail.bitfields,
                resolver_alias_count: parser.timings().resolver_detail.alias_stats.aliases,
                resolver_alias_resolved: parser.timings().resolver_detail.alias_stats.resolved,
                resolver_alias_zero_size: parser.timings().resolver_detail.alias_stats.zero_size,
                resolver_alias_same_resolved: parser
                    .timings()
                    .resolver_detail
                    .alias_stats
                    .same_resolved,
                resolver_alias_updates: parser.timings().resolver_detail.alias_stats.updates,
                resolver_member_count: parser
                    .timings()
                    .resolver_detail
                    .member_stats
                    .members_with_type_offset,
                resolver_member_unique_type_offsets: parser
                    .timings()
                    .resolver_detail
                    .member_stats
                    .unique_type_offsets,
                resolver_member_updates: parser
                    .timings()
                    .resolver_detail
                    .member_stats
                    .member_updates,
                variable_extraction,
                type_cache_clone: Duration::ZERO,
                a2l_entry_expansion,
                type_count,
                dwarf_variable_count: parser.global_variable_count(),
                variable_count: variables.len(),
                entry_count: entries.len(),
            };

            (variables, true, stats, Some(profile), None, Some(entries))
        } else {
            let variables = Self::extract_variables_from_elf(&obj);
            (variables, false, None, None, None, None)
        };

        Ok(Self {
            variables,
            file_size,
            has_dwarf,
            dwarf_stats,
            profile,
            type_cache,
            a2l_entries,
        })
    }

    fn extract_variables_from_dwarf(parser: &DwarfParser) -> Vec<Variable> {
        let type_cache = parser.type_cache();
        let mut variables = Vec::new();
        let mut seen = HashSet::new();

        for dv in parser.global_variables() {
            if seen.contains(&dv.name) {
                continue;
            }

            let type_info = if dv.type_offset > 0 {
                type_cache.get(&dv.type_offset).cloned().unwrap_or_else(|| {
                    panic!(
                        "type_offset 0x{:x} 不在 type_cache 中（变量 {}）",
                        dv.type_offset, dv.name
                    )
                })
            } else {
                continue;
            };

            if type_info.size == 0 {
                continue;
            }

            variables.push(Variable {
                name: dv.name.clone(),
                address: dv.address,
                size: type_info.size,
                type_name: type_info.name.clone(),
                type_info,
            });

            seen.insert(dv.name.clone());
        }

        variables.sort_by(|a, b| a.name.cmp(&b.name));
        variables
    }

    fn extract_variables_from_elf(obj: &object::File) -> Vec<Variable> {
        let mut variables = Vec::new();
        let mut seen = HashSet::new();

        let sections: Vec<_> = obj.sections().collect();
        let section_map: HashMap<_, _> = sections
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

            let type_name = match size {
                1 => "uint8_t".to_string(),
                2 => "uint16_t".to_string(),
                4 => "uint32_t".to_string(),
                8 => "uint64_t".to_string(),
                _ => format!("uint8_t[{}]", size),
            };

            variables.push(Variable::new(
                name.to_string(),
                address,
                size,
                type_name.clone(),
                TypeInfo::primitive(type_name, size, Default::default()),
            ));

            seen.insert(name.to_string());
        }

        variables.sort_by(|a, b| a.name.cmp(&b.name));
        variables
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

    pub fn generation_profile(&self) -> Option<&GenerationProfile> {
        self.profile.as_ref()
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
            let mut ctx = ExpandContext {
                type_cache,
                store: &mut store,
                visited: HashSet::new(),
                root_symbol: &var.name,
                root_addr: var.address,
            };
            Self::expand_entry(&var.name, var.address, &var.type_info, 0, &mut ctx);
        }

        store
    }

    fn expand_entry(
        name: &str,
        addr: u64,
        type_info: &TypeInfo,
        depth: usize,
        ctx: &mut ExpandContext,
    ) {
        if depth > MAX_NESTING_DEPTH {
            return;
        }

        if type_info.offset > 0 && ctx.visited.contains(&type_info.offset) {
            return;
        }
        ctx.visited.insert(type_info.offset);

        match type_info.kind {
            TypeKind::Struct | TypeKind::Union => {
                Self::expand_composite(name, addr, type_info, depth, ctx);
            }
            TypeKind::Array => {
                Self::expand_array(name, addr, type_info, depth, ctx);
            }
            TypeKind::Primitive | TypeKind::Enum | TypeKind::Pointer | TypeKind::Typedef => {
                let a2l_type = infer_a2l_type_from_encoding(type_info.size, type_info.encoding);
                ctx.store.add(A2lEntry::new(
                    name.to_string(),
                    addr,
                    type_info.size,
                    a2l_type.to_string(),
                    type_info.name.clone(),
                ));
            }
        }

        ctx.visited.remove(&type_info.offset);
    }

    fn expand_composite(
        prefix: &str,
        base_addr: u64,
        type_info: &TypeInfo,
        depth: usize,
        ctx: &mut ExpandContext,
    ) {
        let bitfield_groups = Self::compute_bitfield_groups(&type_info.members);

        for member in &type_info.members {
            let full_name = if member.name == "_" {
                prefix.to_string()
            } else {
                format!("{}.{}", prefix, member.name)
            };

            if member.is_bitfield() {
                Self::expand_bitfield(&full_name, base_addr, member, &bitfield_groups, ctx);
            } else {
                Self::expand_member(&full_name, base_addr, member, depth, ctx);
            }
        }
    }

    fn expand_bitfield(
        name: &str,
        base_addr: u64,
        member: &crate::types::StructMember,
        groups: &HashMap<usize, BitfieldGroup>,
        ctx: &mut ExpandContext,
    ) {
        let bg = match groups.get(&member.offset) {
            Some(g) => g,
            None => return,
        };

        let container_addr = base_addr + bg.container_offset as u64;
        let container_a2l_type =
            infer_a2l_type_from_encoding(bg.container_size, Default::default());
        let raw_bo = member.bit_offset.unwrap_or(0);
        let raw_bs = member.bit_size.unwrap_or(0);
        let storage_bits = if member.type_size > 0 {
            member.type_size * 8
        } else {
            bg.container_size * 8
        };
        let absolute_lsb = if member.bit_offset_is_absolute || raw_bo + raw_bs > storage_bits {
            raw_bo
        } else {
            member.offset * 8 + storage_bits.saturating_sub(raw_bo + raw_bs)
        };
        // 条目地址与 SYMBOL_LINK 均指向容器字节，BIT_MASK 需要容器内相对位偏移，
        // 直接用结构体内绝对偏移会移位越界或落到容器之外
        let bit_offset = absolute_lsb.saturating_sub(bg.container_offset * 8);
        let bit_size = raw_bs;
        let symbol_link_offset = container_addr.saturating_sub(ctx.root_addr);

        ctx.store.add(
            A2lEntry::new(
                name.to_string(),
                container_addr,
                bg.container_size,
                container_a2l_type.to_string(),
                member.type_name.clone(),
            )
            .with_bitfield(bit_offset, bit_size)
            .with_symbol_link(ctx.root_symbol.to_string(), symbol_link_offset),
        );
    }

    fn expand_member(
        name: &str,
        base_addr: u64,
        member: &crate::types::StructMember,
        depth: usize,
        ctx: &mut ExpandContext,
    ) {
        let member_addr = base_addr + member.offset as u64;
        let type_offset = match member.type_offset {
            Some(off) if off > 0 => off,
            _ => return,
        };

        if let Some(member_type) = ctx.type_cache.get(&type_offset) {
            Self::expand_entry(name, member_addr, member_type, depth + 1, ctx);
        }
    }

    fn expand_array(
        prefix: &str,
        base_addr: u64,
        type_info: &TypeInfo,
        depth: usize,
        ctx: &mut ExpandContext,
    ) {
        // 数组展开规则：
        // 1. primitive/enum 数组展开到最内层元素，保留所有维度索引。
        // 2. bitfield struct/union 数组先生成元素容器，再展开位域成员。
        // 3. 普通 struct/union 数组只递归展开成员，不额外生成元素容器。
        // 4. 元素类型缺失时按推导出的元素大小降级为标量元素。
        let (effective_dims, final_elem_type, final_elem_size) =
            Self::flatten_array_type(type_info, 0);

        let original_total: usize = type_info.array_dims.iter().product();

        if original_total > MAX_ARRAY_EXPAND || original_total == 0 {
            return;
        }

        if let Some(ref elem_type) = final_elem_type {
            Self::expand_multi_dim_array(
                prefix,
                base_addr,
                elem_type,
                &effective_dims,
                final_elem_size,
                depth,
                ctx,
            );
        } else {
            let total_elements: usize = effective_dims.iter().product();
            for i in 0..total_elements {
                let elem_name = format!("{}._{}_", prefix, i);
                let elem_addr = base_addr + (i * final_elem_size) as u64;
                let elem_a2l_type =
                    infer_a2l_type_from_encoding(final_elem_size, type_info.encoding);
                ctx.store.add(A2lEntry::new(
                    elem_name,
                    elem_addr,
                    final_elem_size,
                    elem_a2l_type.to_string(),
                    type_info.name.clone(),
                ));
            }
        }
    }

    fn compute_bitfield_groups(
        members: &[crate::types::StructMember],
    ) -> HashMap<usize, BitfieldGroup> {
        let mut groups: HashMap<usize, BitfieldGroup> = HashMap::new();
        let mut i = 0;
        while i < members.len() {
            if members[i].is_bitfield() {
                let start = i;
                i += 1;
                while i < members.len() && members[i].is_bitfield() {
                    i += 1;
                }
                let group = &members[start..i];
                let min_offset = group.iter().map(|m| m.offset).min().unwrap_or(0);
                let max_end = group
                    .iter()
                    .map(|m| m.offset + m.type_size)
                    .max()
                    .unwrap_or(0);
                let container_size = max_end.saturating_sub(min_offset);
                for member in group {
                    groups.insert(
                        member.offset,
                        BitfieldGroup {
                            container_offset: min_offset,
                            container_size,
                        },
                    );
                }
            } else {
                i += 1;
            }
        }
        groups
    }

    fn flatten_array_type(
        type_info: &TypeInfo,
        base_elem_size: usize,
    ) -> (Vec<usize>, Option<TypeInfo>, usize) {
        let mut all_dims: Vec<usize> = type_info.array_dims.iter().copied().collect();

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
                all_dims.extend(inner.array_dims.iter().copied());

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

    fn is_bitfield_composite(type_info: &TypeInfo) -> bool {
        matches!(type_info.kind, TypeKind::Struct | TypeKind::Union)
            && type_info.members.iter().any(|member| member.is_bitfield())
    }

    fn expand_multi_dim_array(
        prefix: &str,
        base_addr: u64,
        elem_type: &TypeInfo,
        dims: &[usize],
        elem_size: usize,
        depth: usize,
        ctx: &mut ExpandContext,
    ) {
        if dims.is_empty() {
            let fixed_elem_type = if elem_type.size == 0 {
                let mut t = elem_type.clone();
                t.size = elem_size;
                t
            } else {
                elem_type.clone()
            };
            if Self::is_bitfield_composite(&fixed_elem_type) {
                let a2l_type =
                    infer_a2l_type_from_encoding(fixed_elem_type.size, fixed_elem_type.encoding);
                ctx.store.add(
                    A2lEntry::new(
                        prefix.to_string(),
                        base_addr,
                        fixed_elem_type.size,
                        a2l_type.to_string(),
                        fixed_elem_type.name.clone(),
                    )
                    .with_symbol_link(
                        ctx.root_symbol.to_string(),
                        base_addr.saturating_sub(ctx.root_addr),
                    ),
                );
            }
            Self::expand_entry(prefix, base_addr, &fixed_elem_type, depth, ctx);
            return;
        }

        let current_dim = dims[0];
        let remaining_dims = &dims[1..];
        let stride: usize = remaining_dims.iter().product::<usize>() * elem_size;

        for i in 0..current_dim {
            let elem_name = format!("{}._{}_", prefix, i);
            let elem_addr = base_addr + (i * stride) as u64;
            Self::expand_multi_dim_array(
                &elem_name,
                elem_addr,
                elem_type,
                remaining_dims,
                elem_size,
                depth,
                ctx,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{StructMember, TypeEncoding};

    fn expand_single(name: &str, address: u64, type_info: TypeInfo) -> A2lEntryStore {
        let var = Variable::new(
            name.to_string(),
            address,
            type_info.size,
            type_info.name.clone(),
            type_info,
        );
        ElfParser::expand_all_entries(&[var], &HashMap::new())
    }

    #[test]
    fn expands_primitive_array_to_leaf_elements() {
        let elem = TypeInfo::primitive("uint16_t".to_string(), 2, TypeEncoding::Unsigned);
        let array = TypeInfo::array_type("array[3]".to_string(), 6, elem, vec![3], 0x10);

        let store = expand_single("Speeds", 0x1000, array);

        assert_eq!(store.len(), 3);
        assert_eq!(store.entries[0].full_name, "Speeds._0_");
        assert_eq!(store.entries[0].address, 0x1000);
        assert_eq!(store.entries[0].size, 2);
        assert_eq!(store.entries[0].a2l_type, "UWORD");
        assert_eq!(store.entries[2].full_name, "Speeds._2_");
        assert_eq!(store.entries[2].address, 0x1004);
    }

    #[test]
    fn expands_nested_enum_array_to_leaf_elements() {
        let enum_type = TypeInfo::enum_type(
            "FaultId".to_string(),
            1,
            TypeEncoding::Unsigned,
            Vec::new(),
            0x10,
        );
        let inner = TypeInfo::array_type("array[3]".to_string(), 3, enum_type, vec![3], 0x20);
        let outer = TypeInfo::array_type("array[2]".to_string(), 6, inner, vec![2], 0x30);

        let store = expand_single("Cfg", 0x1000, outer);

        assert_eq!(store.len(), 6);
        assert_eq!(store.entries[0].full_name, "Cfg._0_._0_");
        assert_eq!(store.entries[0].address, 0x1000);
        assert_eq!(store.entries[5].full_name, "Cfg._1_._2_");
        assert_eq!(store.entries[5].address, 0x1005);
    }

    #[test]
    fn expands_const_nested_enum_array_to_leaf_elements() {
        let enum_type = TypeInfo::enum_type(
            "FaultId".to_string(),
            1,
            TypeEncoding::Unsigned,
            Vec::new(),
            0x10,
        );
        let inner = TypeInfo::array_type("array[6]".to_string(), 6, enum_type, vec![6], 0x20);
        let mut const_outer =
            TypeInfo::array_type("const array[4]".to_string(), 24, inner, vec![4], 0x30);
        const_outer.kind = TypeKind::Array;

        let store = expand_single("Cdd_L9388_WssFaultIdConfigs", 0x801F18AB, const_outer);

        assert_eq!(store.len(), 24);
        assert_eq!(
            store.entries[0].full_name,
            "Cdd_L9388_WssFaultIdConfigs._0_._0_"
        );
        assert_eq!(store.entries[0].address, 0x801F18AB);
        assert_eq!(store.entries[0].size, 1);
        assert_eq!(
            store.entries[23].full_name,
            "Cdd_L9388_WssFaultIdConfigs._3_._5_"
        );
        assert_eq!(store.entries[23].address, 0x801F18C2);
    }

    #[test]
    fn expands_bitfield_struct_array_with_container_and_members() {
        let value = StructMember::new(
            "FltDebValue".to_string(),
            0,
            "unsigned short int".to_string(),
            2,
        )
        .with_bitfield(0, 14, true);
        let status = StructMember::new(
            "CurrentDebStatus".to_string(),
            1,
            "unsigned char".to_string(),
            1,
        )
        .with_bitfield(14, 2, true);
        let elem = TypeInfo::struct_type("FaultDebounce".to_string(), 2, vec![value, status], 0x40);
        let array = TypeInfo::array_type("array[2]".to_string(), 4, elem, vec![2], 0x50);

        let store = expand_single("Fault", 0x2000, array);

        assert_eq!(store.len(), 6);
        assert_eq!(store.entries[0].full_name, "Fault._0_");
        assert_eq!(store.entries[1].full_name, "Fault._0_.FltDebValue");
        assert_eq!(store.entries[1].bit_offset, Some(0));
        assert_eq!(store.entries[1].bit_size, Some(14));
        assert_eq!(store.entries[2].full_name, "Fault._0_.CurrentDebStatus");
        assert_eq!(store.entries[2].bit_offset, Some(14));
        assert_eq!(store.entries[3].full_name, "Fault._1_");
        assert_eq!(store.entries[3].address, 0x2002);
    }

    #[test]
    fn normalizes_absolute_bitfield_offset_to_container() {
        // 模拟 Dcm 配置表：13 字节普通成员后跟共享 1 字节容器的单 bit 标志位，
        // 绝对 LSB 为结构体内偏移（104~108），BIT_MASK 需换算为容器内偏移（0~4）
        let header =
            StructMember::new("Header".to_string(), 0, "unsigned char[13]".to_string(), 13);
        let flag = |name: &str, absolute_lsb: usize| {
            StructMember::new(name.to_string(), 13, "unsigned char".to_string(), 1)
                .with_bitfield(absolute_lsb, 1, true)
        };
        let members = vec![
            header,
            flag("SuppPosResponsAllowed", 104),
            flag("AllowedInDefaultSession", 105),
            flag("SecurityAccessNeeded", 108),
        ];
        let elem = TypeInfo::struct_type("DcmService".to_string(), 14, members, 0x40);
        let array = TypeInfo::array_type("array[2]".to_string(), 28, elem, vec![2], 0x50);

        let store = expand_single("Srv", 0x1000, array);

        let e = store.get_by_name("Srv._0_.SecurityAccessNeeded").unwrap();
        assert_eq!(e.address, 0x1000 + 13);
        assert_eq!(e.size, 1);
        assert_eq!(e.bit_offset, Some(108 - 13 * 8));
        assert_eq!(e.symbol_link_offset, Some(13));

        let e2 = store.get_by_name("Srv._0_.SuppPosResponsAllowed").unwrap();
        assert_eq!(e2.bit_offset, Some(0));

        let e3 = store.get_by_name("Srv._1_.SecurityAccessNeeded").unwrap();
        assert_eq!(e3.address, 0x1000 + 14 + 13);
        assert_eq!(e3.bit_offset, Some(4));
    }

    #[test]
    fn expands_nested_bitfield_struct_array_with_containers_and_members() {
        let value = StructMember::new(
            "FltDebValue".to_string(),
            0,
            "unsigned short int".to_string(),
            2,
        )
        .with_bitfield(0, 14, true);
        let status = StructMember::new(
            "CurrentDebStatus".to_string(),
            1,
            "unsigned char".to_string(),
            1,
        )
        .with_bitfield(14, 2, true);
        let elem = TypeInfo::struct_type("FaultDebounce".to_string(), 2, vec![value, status], 0x40);
        let inner = TypeInfo::array_type("array[23]".to_string(), 46, elem, vec![23], 0x50);
        let outer = TypeInfo::array_type("array[1]".to_string(), 46, inner, vec![1], 0x60);

        let store = expand_single("Cdd_TLE918X_FaultDebInfo", 0x7000, outer);

        assert_eq!(store.len(), 69);
        assert_eq!(
            store.entries[0].full_name,
            "Cdd_TLE918X_FaultDebInfo._0_._0_"
        );
        assert_eq!(store.entries[0].address, 0x7000);
        assert_eq!(
            store.entries[1].full_name,
            "Cdd_TLE918X_FaultDebInfo._0_._0_.FltDebValue"
        );
        assert_eq!(
            store.entries[2].full_name,
            "Cdd_TLE918X_FaultDebInfo._0_._0_.CurrentDebStatus"
        );
        assert_eq!(
            store.entries[66].full_name,
            "Cdd_TLE918X_FaultDebInfo._0_._22_"
        );
        assert_eq!(store.entries[66].address, 0x702C);
        assert_eq!(
            store.entries[68].full_name,
            "Cdd_TLE918X_FaultDebInfo._0_._22_.CurrentDebStatus"
        );
    }

    #[test]
    fn expands_plain_struct_array_without_container_entries() {
        let mut member = StructMember::new("Value".to_string(), 0, "uint16_t".to_string(), 2);
        member.type_offset = Some(0x60);
        let member_type = TypeInfo::primitive("uint16_t".to_string(), 2, TypeEncoding::Unsigned);
        let elem = TypeInfo::struct_type("Plain".to_string(), 2, vec![member], 0x70);
        let array = TypeInfo::array_type("array[2]".to_string(), 4, elem, vec![2], 0x80);
        let var = Variable::new(
            "PlainArray".to_string(),
            0x3000,
            4,
            "array[2]".to_string(),
            array,
        );
        let mut type_cache = HashMap::new();
        type_cache.insert(0x60, member_type);

        let store = ElfParser::expand_all_entries(&[var], &type_cache);

        assert_eq!(store.len(), 2);
        assert_eq!(store.entries[0].full_name, "PlainArray._0_.Value");
        assert_eq!(store.entries[0].address, 0x3000);
        assert_eq!(store.entries[1].full_name, "PlainArray._1_.Value");
        assert_eq!(store.entries[1].address, 0x3002);
    }

    #[test]
    fn expands_array_with_unknown_element_type_as_scalar_fallback() {
        let mut array = TypeInfo::array_type(
            "array[4]".to_string(),
            8,
            TypeInfo::primitive("unknown".to_string(), 0, TypeEncoding::Unsigned),
            vec![4],
            0x90,
        );
        array.pointer_target = None;

        let store = expand_single("UnknownArray", 0x4000, array);

        assert_eq!(store.len(), 4);
        assert_eq!(store.entries[0].full_name, "UnknownArray._0_");
        assert_eq!(store.entries[0].address, 0x4000);
        assert_eq!(store.entries[0].size, 2);
        assert_eq!(store.entries[0].a2l_type, "UWORD");
        assert_eq!(store.entries[3].full_name, "UnknownArray._3_");
        assert_eq!(store.entries[3].address, 0x4006);
    }
}
