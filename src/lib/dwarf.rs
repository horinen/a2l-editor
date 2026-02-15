use crate::types::{StructMember, TypeEncoding, TypeInfo, TypeKind, Variable};
use anyhow::{Context, Result};
use gimli::{EndianSlice, LittleEndian};
use object::{Object, ObjectSection};
use std::collections::HashMap;

type DwarfReader = EndianSlice<'static, LittleEndian>;

pub struct DwarfParser {
    type_cache: HashMap<u64, TypeInfo>,
    struct_map: HashMap<String, TypeInfo>,
    variable_types: HashMap<String, u64>,
    array_elem_offsets: HashMap<u64, u64>,
    type_refs: HashMap<u64, u64>,
    stats: DwarfStats,
}

#[derive(Default, Clone)]
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

impl DwarfParser {
    pub fn new() -> Self {
        Self {
            type_cache: HashMap::new(),
            struct_map: HashMap::new(),
            variable_types: HashMap::new(),
            array_elem_offsets: HashMap::new(),
            type_refs: HashMap::new(),
            stats: DwarfStats::default(),
        }
    }

    pub fn parse(elf_data: &[u8]) -> Result<Self> {
        let mut parser = Self::new();

        let obj = object::File::parse(elf_data).context("无法解析 ELF 文件")?;

        let debug_info = obj.section_by_name(".debug_info");
        let debug_abbrev = obj.section_by_name(".debug_abbrev");

        if debug_info.is_none() || debug_abbrev.is_none() {
            return Ok(parser);
        }

        let debug_info_data = Self::get_section_bytes(&debug_info.unwrap());
        let debug_abbrev_data = Self::get_section_bytes(&debug_abbrev.unwrap());

        if debug_info_data.is_empty() || debug_abbrev_data.is_empty() {
            return Ok(parser);
        }

        let endian = LittleEndian;
        let debug_info: DwarfReader = EndianSlice::new(debug_info_data, endian);
        let debug_abbrev: DwarfReader = EndianSlice::new(debug_abbrev_data, endian);

        parser.parse_dwarf_sections(debug_info, debug_abbrev)?;

        Ok(parser)
    }

    pub fn parse_from_file(path: &std::path::Path) -> Result<Self> {
        use memmap2::Mmap;
        use std::fs::File;

        let file = File::open(path).context("无法打开 ELF 文件")?;
        let mmap = unsafe { Mmap::map(&file).context("无法创建内存映射")? };

        Self::parse(&mmap)
    }

    fn get_section_bytes(section: &object::Section) -> &'static [u8] {
        match section.data() {
            Ok(data) => {
                let slice = data.as_ref();
                unsafe { std::slice::from_raw_parts(slice.as_ptr(), slice.len()) }
            }
            Err(_) => &[],
        }
    }

    fn parse_dwarf_sections(
        &mut self,
        debug_info: DwarfReader,
        debug_abbrev: DwarfReader,
    ) -> Result<()> {
        let debug_info = gimli::DebugInfo::from(debug_info);
        let debug_abbrev = gimli::DebugAbbrev::from(debug_abbrev);

        let mut iter = debug_info.units();

        while let Some(header) = iter.next().context("遍历 DWARF 单元失败")? {
            match header.abbreviations(&debug_abbrev) {
                Ok(abbrevs) => {
                    self.parse_unit_types(&header, &abbrevs)?;
                }
                Err(_) => continue,
            }
        }

        self.resolve_all_member_types();
        self.resolve_type_refs();
        self.resolve_array_element_types();

        Ok(())
    }

    fn resolve_type_refs(&mut self) {
        let refs: Vec<(u64, u64)> = self.type_refs.iter().map(|(k, v)| (*k, *v)).collect();

        for (from_offset, to_offset) in refs {
            if to_offset > 0 {
                if let Some(target_type) = self.type_cache.get(&to_offset).cloned() {
                    if let Some(type_info) = self.type_cache.get_mut(&from_offset) {
                        type_info.size = target_type.size;
                        type_info.encoding = target_type.encoding;
                        type_info.kind = target_type.kind;
                        type_info.members = target_type.members.clone();
                        type_info.variants = target_type.variants.clone();
                        type_info.array_dims = target_type.array_dims.clone();
                        type_info.pointer_target = target_type.pointer_target.clone();
                    }
                }
            }
        }
    }

    fn resolve_array_element_types(&mut self) {
        let array_offsets: Vec<u64> = self
            .type_cache
            .iter()
            .filter(|(_, t)| t.kind == TypeKind::Array)
            .map(|(offset, _)| *offset)
            .collect();

        let elem_type_offsets: std::collections::HashMap<u64, u64> = self
            .array_elem_offsets
            .iter()
            .map(|(k, v)| (*k, *v))
            .collect();

        for array_offset in array_offsets {
            if let Some(&elem_offset) = elem_type_offsets.get(&array_offset) {
                if elem_offset > 0 {
                    if let Some(elem_type) = self.type_cache.get(&elem_offset).cloned() {
                        if let Some(array_type) = self.type_cache.get_mut(&array_offset) {
                            array_type.pointer_target = Some(Box::new(elem_type));
                            array_type.encoding = array_type
                                .pointer_target
                                .as_ref()
                                .map(|e| e.encoding)
                                .unwrap_or(TypeEncoding::Unsigned);
                        }
                    }
                }
            }
        }
    }

    fn resolve_all_member_types(&mut self) {
        for type_info in self.struct_map.values_mut() {
            for member in &mut type_info.members {
                if let Some(type_offset) = member.type_offset {
                    if type_offset > 0 {
                        if let Some(resolved) = self.type_cache.get(&type_offset) {
                            member.type_name = resolved.name.clone();
                            if member.type_size == 0 {
                                member.type_size = resolved.size;
                            }
                        }
                    }
                }
            }
        }
    }

    fn parse_unit_types(
        &mut self,
        header: &gimli::UnitHeader<DwarfReader>,
        abbrevs: &gimli::Abbreviations,
    ) -> Result<()> {
        let unit_offset = header
            .offset()
            .as_debug_info_offset()
            .map(|o| o.0)
            .unwrap_or(0);

        let mut cursor = header.entries(abbrevs);

        while let Some((_, entry)) = cursor.next_dfs().context("遍历 DIE 失败")? {
            let global_offset = unit_offset + entry.offset().0;

            match entry.tag() {
                gimli::constants::DW_TAG_base_type => {
                    self.parse_base_type_with_offset(entry, global_offset);
                }
                gimli::constants::DW_TAG_structure_type => {
                    self.parse_struct_type_with_offset(header, abbrevs, entry, unit_offset);
                }
                gimli::constants::DW_TAG_union_type => {
                    self.parse_union_type_with_offset(header, abbrevs, entry, unit_offset);
                }
                gimli::constants::DW_TAG_enumeration_type => {
                    self.parse_enum_type_with_offset(header, abbrevs, entry, unit_offset);
                }
                gimli::constants::DW_TAG_array_type => {
                    self.parse_array_type_with_offset(header, abbrevs, entry, unit_offset);
                }
                gimli::constants::DW_TAG_pointer_type => {
                    self.parse_pointer_type_with_offset(entry, global_offset);
                }
                gimli::constants::DW_TAG_typedef => {
                    self.parse_typedef_with_offset(entry, global_offset);
                }
                gimli::constants::DW_TAG_const_type => {
                    self.parse_const_type_with_offset(entry, global_offset);
                }
                gimli::constants::DW_TAG_volatile_type => {
                    self.parse_volatile_type_with_offset(entry, global_offset);
                }
                gimli::constants::DW_TAG_variable => {
                    self.parse_variable(entry);
                }
                gimli::constants::DW_TAG_member => {
                    self.stats.struct_members += 1;
                }
                gimli::constants::DW_TAG_enumerator => {
                    self.stats.enum_values += 1;
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn parse_base_type_with_offset(
        &mut self,
        entry: &gimli::DebuggingInformationEntry<DwarfReader>,
        global_offset: usize,
    ) {
        let name = Self::get_name_static(entry);
        let size = Self::get_size_static(entry);
        let encoding = Self::get_encoding_static(entry);

        if let Some(type_name) = name {
            let mut type_info = TypeInfo::primitive(type_name.clone(), size, encoding);
            type_info.offset = global_offset as u64;
            self.type_cache.insert(global_offset as u64, type_info);
            self.stats.base_types += 1;
        }
    }

    fn parse_struct_type_with_offset(
        &mut self,
        header: &gimli::UnitHeader<DwarfReader>,
        abbrevs: &gimli::Abbreviations,
        entry: &gimli::DebuggingInformationEntry<DwarfReader>,
        unit_offset: usize,
    ) {
        let local_offset = entry.offset().0;
        let global_offset = unit_offset + local_offset;
        let name = Self::get_name_static(entry);
        let size = Self::get_size_static(entry);

        let members =
            Self::parse_struct_members_static_with_unit_offset(header, abbrevs, entry, unit_offset);

        let type_name = name
            .clone()
            .unwrap_or_else(|| format!("<anonymous@0x{:x}>", global_offset));

        let mut type_info =
            TypeInfo::struct_type(type_name.clone(), size, members, global_offset as u64);
        type_info.offset = global_offset as u64;

        self.type_cache
            .insert(global_offset as u64, type_info.clone());

        if let Some(named) = name {
            self.struct_map.insert(named, type_info);
        }

        self.stats.structs += 1;
    }

    fn parse_union_type_with_offset(
        &mut self,
        header: &gimli::UnitHeader<DwarfReader>,
        abbrevs: &gimli::Abbreviations,
        entry: &gimli::DebuggingInformationEntry<DwarfReader>,
        unit_offset: usize,
    ) {
        let local_offset = entry.offset().0;
        let global_offset = unit_offset + local_offset;
        let name = Self::get_name_static(entry);
        let size = Self::get_size_static(entry);

        let members =
            Self::parse_union_members_static_with_unit_offset(header, abbrevs, entry, unit_offset);

        let type_name = name
            .clone()
            .unwrap_or_else(|| format!("<anonymous_union@0x{:x}>", global_offset));

        let mut type_info =
            TypeInfo::union_type(type_name.clone(), size, members, global_offset as u64);
        type_info.offset = global_offset as u64;

        self.type_cache
            .insert(global_offset as u64, type_info.clone());

        if let Some(named) = name {
            self.struct_map.insert(named, type_info);
        }

        self.stats.unions += 1;
    }

    fn parse_union_members_static_with_unit_offset(
        header: &gimli::UnitHeader<DwarfReader>,
        abbrevs: &gimli::Abbreviations,
        parent_entry: &gimli::DebuggingInformationEntry<DwarfReader>,
        unit_offset: usize,
    ) -> Vec<StructMember> {
        let mut members = Vec::new();
        let parent_offset = parent_entry.offset();

        let mut cursor = header.entries(abbrevs);
        let mut found_parent = false;
        let mut parent_depth: isize = 0;
        let mut current_depth: isize = 0;

        loop {
            match cursor.next_dfs() {
                Ok(Some((delta, entry))) => {
                    current_depth += delta;

                    if entry.offset() == parent_offset {
                        found_parent = true;
                        parent_depth = current_depth;
                        continue;
                    }

                    if !found_parent {
                        continue;
                    }

                    if current_depth <= parent_depth {
                        break;
                    }

                    if entry.tag() == gimli::constants::DW_TAG_member {
                        if let Some(name) = Self::get_name_static(entry) {
                            let size = Self::get_size_static(entry);
                            let (type_offset, is_unit_ref) =
                                Self::get_type_offset_info_static(entry);
                            let global_type_offset = if type_offset > 0 {
                                if is_unit_ref {
                                    unit_offset + type_offset as usize
                                } else {
                                    type_offset as usize
                                }
                            } else {
                                0
                            };

                            let bitfield_info = Self::get_bitfield_info_static(entry);

                            let mut member =
                                StructMember::new(name, 0, "unknown".to_string(), size)
                                    .with_type_offset(global_type_offset as u64);

                            if let Some((bit_offset, bit_size)) = bitfield_info {
                                member = member.with_bitfield(bit_offset, bit_size);
                            }

                            members.push(member);
                        }
                    }
                }
                Ok(None) => break,
                Err(_) => continue,
            }
        }

        members
    }

    fn parse_struct_members_static_with_unit_offset(
        header: &gimli::UnitHeader<DwarfReader>,
        abbrevs: &gimli::Abbreviations,
        parent_entry: &gimli::DebuggingInformationEntry<DwarfReader>,
        unit_offset: usize,
    ) -> Vec<StructMember> {
        let mut members = Vec::new();
        let parent_offset = parent_entry.offset();

        let mut cursor = header.entries(abbrevs);
        let mut found_parent = false;
        let mut parent_depth: isize = 0;
        let mut current_depth: isize = 0;

        loop {
            match cursor.next_dfs() {
                Ok(Some((delta, entry))) => {
                    current_depth += delta;

                    if entry.offset() == parent_offset {
                        found_parent = true;
                        parent_depth = current_depth;
                        continue;
                    }

                    if !found_parent {
                        continue;
                    }

                    if current_depth <= parent_depth {
                        break;
                    }

                    if entry.tag() == gimli::constants::DW_TAG_member {
                        if let Some(name) = Self::get_name_static(entry) {
                            let offset = Self::get_member_location_static(entry);
                            let size = Self::get_size_static(entry);
                            let (type_offset, is_unit_ref) =
                                Self::get_type_offset_info_static(entry);
                            let global_type_offset = if type_offset > 0 {
                                if is_unit_ref {
                                    unit_offset + type_offset as usize
                                } else {
                                    type_offset as usize
                                }
                            } else {
                                0
                            };

                            let bitfield_info = Self::get_bitfield_info_static(entry);

                            let mut member =
                                StructMember::new(name, offset, "unknown".to_string(), size)
                                    .with_type_offset(global_type_offset as u64);

                            if let Some((bit_offset, bit_size)) = bitfield_info {
                                member = member.with_bitfield(bit_offset, bit_size);
                            }

                            members.push(member);
                        }
                    }
                }
                Ok(None) => break,
                Err(_) => continue,
            }
        }

        members
    }

    fn parse_enum_type_with_offset(
        &mut self,
        header: &gimli::UnitHeader<DwarfReader>,
        abbrevs: &gimli::Abbreviations,
        entry: &gimli::DebuggingInformationEntry<DwarfReader>,
        unit_offset: usize,
    ) {
        let local_offset = entry.offset().0;
        let global_offset = unit_offset + local_offset;
        let name = Self::get_name_static(entry);
        let size = Self::get_size_static(entry);
        let encoding = Self::get_encoding_static(entry);

        let variants = Self::parse_enum_variants(header, abbrevs, entry);

        if let Some(type_name) = name {
            let mut type_info =
                TypeInfo::enum_type(type_name, size, encoding, variants, global_offset as u64);
            type_info.offset = global_offset as u64;
            self.type_cache.insert(global_offset as u64, type_info);
            self.stats.enums += 1;
        }
    }

    fn parse_enum_variants(
        header: &gimli::UnitHeader<DwarfReader>,
        abbrevs: &gimli::Abbreviations,
        parent_entry: &gimli::DebuggingInformationEntry<DwarfReader>,
    ) -> Vec<crate::types::EnumVariant> {
        use crate::types::EnumVariant;

        let mut variants = Vec::new();
        let parent_offset = parent_entry.offset();

        let mut cursor = header.entries(abbrevs);
        let mut found_parent = false;
        let mut parent_depth: isize = 0;
        let mut current_depth: isize = 0;

        loop {
            match cursor.next_dfs() {
                Ok(Some((delta, entry))) => {
                    current_depth += delta;

                    if entry.offset() == parent_offset {
                        found_parent = true;
                        parent_depth = current_depth;
                        continue;
                    }

                    if !found_parent {
                        continue;
                    }

                    if current_depth <= parent_depth {
                        break;
                    }

                    if entry.tag() == gimli::constants::DW_TAG_enumerator {
                        if let Some(name) = Self::get_name_static(entry) {
                            if let Some(value) = Self::get_enum_value(entry) {
                                variants.push(EnumVariant::new(name, value));
                            }
                        }
                    }
                }
                Ok(None) => break,
                Err(_) => continue,
            }
        }

        variants
    }

    fn get_enum_value(entry: &gimli::DebuggingInformationEntry<DwarfReader>) -> Option<i64> {
        if let Some(attr) = entry
            .attr(gimli::constants::DW_AT_const_value)
            .ok()
            .flatten()
        {
            match attr.value() {
                gimli::AttributeValue::Sdata(v) => return Some(v),
                gimli::AttributeValue::Udata(v) => return Some(v as i64),
                gimli::AttributeValue::Data1(v) => return Some(v as i64),
                gimli::AttributeValue::Data2(v) => return Some(v as i64),
                gimli::AttributeValue::Data4(v) => return Some(v as i64),
                gimli::AttributeValue::Data8(v) => return Some(v as i64),
                _ => {}
            }
        }
        None
    }

    fn parse_array_type_with_offset(
        &mut self,
        header: &gimli::UnitHeader<DwarfReader>,
        abbrevs: &gimli::Abbreviations,
        entry: &gimli::DebuggingInformationEntry<DwarfReader>,
        unit_offset: usize,
    ) {
        let local_offset = entry.offset().0;
        let global_offset = unit_offset + local_offset;
        let size = Self::get_size_static(entry);

        let dims = Self::parse_array_dimensions(header, abbrevs, entry);
        let elem_type_offset = Self::get_type_offset_with_unit(entry, unit_offset);

        if elem_type_offset > 0 {
            self.array_elem_offsets
                .insert(global_offset as u64, elem_type_offset);
        }

        let mut type_info = TypeInfo::array_type(
            Self::format_array_name(&dims),
            size,
            TypeInfo::primitive("unknown".to_string(), 0, TypeEncoding::Unsigned),
            dims.clone(),
            global_offset as u64,
        );
        type_info.offset = global_offset as u64;
        self.type_cache.insert(global_offset as u64, type_info);
        self.stats.arrays += 1;
    }

    fn parse_array_dimensions(
        header: &gimli::UnitHeader<DwarfReader>,
        abbrevs: &gimli::Abbreviations,
        parent_entry: &gimli::DebuggingInformationEntry<DwarfReader>,
    ) -> Vec<usize> {
        let mut dims = Vec::new();
        let parent_offset = parent_entry.offset();

        let mut cursor = header.entries(abbrevs);
        let mut found_parent = false;
        let mut parent_depth: isize = 0;
        let mut current_depth: isize = 0;

        loop {
            match cursor.next_dfs() {
                Ok(Some((delta, entry))) => {
                    current_depth += delta;

                    if entry.offset() == parent_offset {
                        found_parent = true;
                        parent_depth = current_depth;
                        continue;
                    }

                    if !found_parent {
                        continue;
                    }

                    if current_depth <= parent_depth {
                        break;
                    }

                    if entry.tag() == gimli::constants::DW_TAG_subrange_type {
                        if let Some(dim) = Self::get_array_dimension(entry) {
                            dims.push(dim);
                        }
                    }
                }
                Ok(None) => break,
                Err(_) => continue,
            }
        }

        dims
    }

    fn get_array_dimension(entry: &gimli::DebuggingInformationEntry<DwarfReader>) -> Option<usize> {
        if let Some(attr) = entry
            .attr(gimli::constants::DW_AT_upper_bound)
            .ok()
            .flatten()
        {
            match attr.value() {
                gimli::AttributeValue::Udata(v) => return Some(v as usize + 1),
                gimli::AttributeValue::Data1(v) => return Some(v as usize + 1),
                gimli::AttributeValue::Data2(v) => return Some(v as usize + 1),
                gimli::AttributeValue::Data4(v) => return Some(v as usize + 1),
                gimli::AttributeValue::Data8(v) => return Some(v as usize + 1),
                gimli::AttributeValue::Sdata(v) => return Some((v as usize) + 1),
                _ => {}
            }
        }

        if let Some(attr) = entry.attr(gimli::constants::DW_AT_count).ok().flatten() {
            match attr.value() {
                gimli::AttributeValue::Udata(v) => return Some(v as usize),
                gimli::AttributeValue::Data1(v) => return Some(v as usize),
                gimli::AttributeValue::Data2(v) => return Some(v as usize),
                gimli::AttributeValue::Data4(v) => return Some(v as usize),
                gimli::AttributeValue::Data8(v) => return Some(v as usize),
                gimli::AttributeValue::Sdata(v) => return Some(v as usize),
                _ => {}
            }
        }

        None
    }

    fn format_array_name(dims: &[usize]) -> String {
        if dims.is_empty() {
            return "array".to_string();
        }
        let dims_str: Vec<String> = dims.iter().map(|d| d.to_string()).collect();
        format!("array[{}]", dims_str.join("]["))
    }

    fn parse_pointer_type_with_offset(
        &mut self,
        entry: &gimli::DebuggingInformationEntry<DwarfReader>,
        global_offset: usize,
    ) {
        let size = Self::get_size_static(entry);

        let mut type_info = TypeInfo::pointer_type(
            "pointer".to_string(),
            size,
            TypeInfo::primitive("void".to_string(), 0, TypeEncoding::Unsigned),
            global_offset as u64,
        );
        type_info.offset = global_offset as u64;
        self.type_cache.insert(global_offset as u64, type_info);
        self.stats.pointers += 1;
    }

    fn parse_typedef_with_offset(
        &mut self,
        entry: &gimli::DebuggingInformationEntry<DwarfReader>,
        global_offset: usize,
    ) {
        let name = Self::get_name_static(entry);
        let target_offset = Self::get_type_offset_static(entry);

        if let Some(type_name) = name {
            let mut type_info = TypeInfo::primitive(type_name.clone(), 0, TypeEncoding::Unsigned);
            type_info.kind = TypeKind::Typedef;
            type_info.offset = global_offset as u64;

            if target_offset > 0 {
                self.type_refs
                    .insert(global_offset as u64, target_offset as u64);
            }

            self.type_cache.insert(global_offset as u64, type_info);
            self.stats.typedefs += 1;
        }
    }

    fn parse_const_type_with_offset(
        &mut self,
        entry: &gimli::DebuggingInformationEntry<DwarfReader>,
        global_offset: usize,
    ) {
        let name = Self::get_name_static(entry);
        let target_offset = Self::get_type_offset_static(entry);

        let type_name = name.unwrap_or_else(|| "const".to_string());
        let mut type_info = TypeInfo::primitive(type_name, 0, TypeEncoding::Unsigned);
        type_info.offset = global_offset as u64;

        if target_offset > 0 {
            self.type_refs
                .insert(global_offset as u64, target_offset as u64);
        }

        self.type_cache.insert(global_offset as u64, type_info);
    }

    fn parse_volatile_type_with_offset(
        &mut self,
        entry: &gimli::DebuggingInformationEntry<DwarfReader>,
        global_offset: usize,
    ) {
        let name = Self::get_name_static(entry);
        let target_offset = Self::get_type_offset_static(entry);

        let type_name = name.unwrap_or_else(|| "volatile".to_string());
        let mut type_info = TypeInfo::primitive(type_name, 0, TypeEncoding::Unsigned);
        type_info.offset = global_offset as u64;

        if target_offset > 0 {
            self.type_refs
                .insert(global_offset as u64, target_offset as u64);
        }

        self.type_cache.insert(global_offset as u64, type_info);
    }

    fn parse_variable(&mut self, entry: &gimli::DebuggingInformationEntry<DwarfReader>) {
        self.stats.variables += 1;

        if let Some(name) = Self::get_name_static(entry) {
            let type_offset = Self::get_type_offset_static(entry);
            if type_offset > 0 {
                self.variable_types.insert(name, type_offset);
            }
        }
    }

    pub fn get_variable_count(&self) -> usize {
        self.variable_types.len()
    }

    pub fn get_type_cache_size(&self) -> usize {
        self.type_cache.len()
    }

    fn get_name_static(entry: &gimli::DebuggingInformationEntry<DwarfReader>) -> Option<String> {
        entry
            .attr(gimli::constants::DW_AT_name)
            .ok()
            .flatten()
            .and_then(|attr| match attr.value() {
                gimli::AttributeValue::String(s) => Some(String::from_utf8_lossy(&s).to_string()),
                _ => None,
            })
    }

    fn get_size_static(entry: &gimli::DebuggingInformationEntry<DwarfReader>) -> usize {
        entry
            .attr(gimli::constants::DW_AT_byte_size)
            .ok()
            .flatten()
            .and_then(|attr| match attr.value() {
                gimli::AttributeValue::Udata(v) => Some(v as usize),
                gimli::AttributeValue::Data1(v) => Some(v as usize),
                gimli::AttributeValue::Data2(v) => Some(v as usize),
                gimli::AttributeValue::Data4(v) => Some(v as usize),
                gimli::AttributeValue::Data8(v) => Some(v as usize),
                _ => None,
            })
            .unwrap_or(0)
    }

    fn get_encoding_static(entry: &gimli::DebuggingInformationEntry<DwarfReader>) -> TypeEncoding {
        entry
            .attr(gimli::constants::DW_AT_encoding)
            .ok()
            .flatten()
            .and_then(|attr| match attr.value() {
                gimli::AttributeValue::Encoding(gimli::constants::DW_ATE_unsigned)
                | gimli::AttributeValue::Encoding(gimli::constants::DW_ATE_unsigned_char) => {
                    Some(TypeEncoding::Unsigned)
                }
                gimli::AttributeValue::Encoding(gimli::constants::DW_ATE_signed)
                | gimli::AttributeValue::Encoding(gimli::constants::DW_ATE_signed_char) => {
                    Some(TypeEncoding::Signed)
                }
                gimli::AttributeValue::Encoding(gimli::constants::DW_ATE_float) => {
                    Some(TypeEncoding::Float)
                }
                _ => None,
            })
            .unwrap_or(TypeEncoding::Unsigned)
    }

    fn get_member_location_static(entry: &gimli::DebuggingInformationEntry<DwarfReader>) -> usize {
        entry
            .attr(gimli::constants::DW_AT_data_member_location)
            .ok()
            .flatten()
            .and_then(|attr| match attr.value() {
                gimli::AttributeValue::Udata(v) => Some(v as usize),
                gimli::AttributeValue::Data1(v) => Some(v as usize),
                gimli::AttributeValue::Data2(v) => Some(v as usize),
                gimli::AttributeValue::Data4(v) => Some(v as usize),
                gimli::AttributeValue::Data8(v) => Some(v as usize),
                gimli::AttributeValue::Sdata(v) => Some(v as usize),
                gimli::AttributeValue::Block(block) => {
                    if block.len() >= 2 && block[0] == 0x23 {
                        Some(block[1] as usize)
                    } else if block.len() >= 3 && block[0] == 0x23 {
                        Some(block[1] as usize | ((block[2] as usize) << 8))
                    } else {
                        None
                    }
                }
                gimli::AttributeValue::Exprloc(expr) => {
                    let data = &expr.0;
                    if data.len() >= 2 && data[0] == 0x23 {
                        Some(data[1] as usize)
                    } else if data.len() >= 3 && data[0] == 0x23 {
                        Some(data[1] as usize | ((data[2] as usize) << 8))
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .unwrap_or(0)
    }

    fn get_type_offset_static(entry: &gimli::DebuggingInformationEntry<DwarfReader>) -> u64 {
        entry
            .attr(gimli::constants::DW_AT_type)
            .ok()
            .flatten()
            .and_then(|attr| match attr.value() {
                gimli::AttributeValue::UnitRef(r) => Some(r.0 as u64),
                gimli::AttributeValue::DebugInfoRef(r) => Some(r.0 as u64),
                _ => None,
            })
            .unwrap_or(0)
    }

    fn get_type_offset_with_unit(
        entry: &gimli::DebuggingInformationEntry<DwarfReader>,
        unit_offset: usize,
    ) -> u64 {
        entry
            .attr(gimli::constants::DW_AT_type)
            .ok()
            .flatten()
            .and_then(|attr| match attr.value() {
                gimli::AttributeValue::UnitRef(r) => Some((unit_offset + r.0) as u64),
                gimli::AttributeValue::DebugInfoRef(r) => Some(r.0 as u64),
                _ => None,
            })
            .unwrap_or(0)
    }

    fn get_type_offset_info_static(
        entry: &gimli::DebuggingInformationEntry<DwarfReader>,
    ) -> (u64, bool) {
        if let Some(attr) = entry.attr(gimli::constants::DW_AT_type).ok().flatten() {
            match attr.value() {
                gimli::AttributeValue::UnitRef(r) => return (r.0 as u64, true),
                gimli::AttributeValue::DebugInfoRef(r) => return (r.0 as u64, false),
                _ => {}
            }
        }
        (0, false)
    }

    fn get_bitfield_info_static(
        entry: &gimli::DebuggingInformationEntry<DwarfReader>,
    ) -> Option<(usize, usize)> {
        let bit_size = entry
            .attr(gimli::constants::DW_AT_bit_size)
            .ok()
            .flatten()
            .and_then(|attr| match attr.value() {
                gimli::AttributeValue::Udata(v) => Some(v as usize),
                gimli::AttributeValue::Data1(v) => Some(v as usize),
                gimli::AttributeValue::Data2(v) => Some(v as usize),
                gimli::AttributeValue::Data4(v) => Some(v as usize),
                gimli::AttributeValue::Data8(v) => Some(v as usize),
                gimli::AttributeValue::Sdata(v) => Some(v as usize),
                _ => None,
            })?;

        let bit_offset = entry
            .attr(gimli::constants::DW_AT_bit_offset)
            .ok()
            .flatten()
            .and_then(|attr| match attr.value() {
                gimli::AttributeValue::Udata(v) => Some(v as usize),
                gimli::AttributeValue::Data1(v) => Some(v as usize),
                gimli::AttributeValue::Data2(v) => Some(v as usize),
                gimli::AttributeValue::Data4(v) => Some(v as usize),
                gimli::AttributeValue::Data8(v) => Some(v as usize),
                gimli::AttributeValue::Sdata(v) => Some(v as usize),
                _ => None,
            });

        Some((bit_offset.unwrap_or(0), bit_size))
    }

    pub fn debug_member_type(&self, struct_name: &str) {
        if let Some(type_info) = self.struct_map.get(struct_name) {
            println!("结构体: {}", struct_name);
            for member in &type_info.members {
                println!(
                    "  成员: {} type_offset=0x{:x}",
                    member.name,
                    member.type_offset.unwrap_or(0)
                );
                if let Some(offset) = member.type_offset {
                    if offset > 0 {
                        if let Some(resolved) = self.type_cache.get(&offset) {
                            println!("    -> 解析为: {} ({})", resolved.name, resolved.kind);
                        } else {
                            println!("    -> 未找到类型，检查 type_cache 中是否存在相近偏移...");
                            let target = offset as i64;
                            for k in self.type_cache.keys() {
                                if (*k as i64 - target).abs() < 10 {
                                    if let Some(t) = self.type_cache.get(k) {
                                        println!("      0x{:x}: {} ({})", k, t.name, t.kind);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn check_type_at_offset(&self, offset: u64) {
        println!("检查偏移 0x{:x}:", offset);
        if let Some(t) = self.type_cache.get(&offset) {
            println!("  找到: {} ({})", t.name, t.kind);
        } else {
            println!("  未找到");
            let target = offset as i64;
            let mut found = false;
            for k in self.type_cache.keys() {
                if (*k as i64 - target).abs() < 100 {
                    found = true;
                    if let Some(t) = self.type_cache.get(k) {
                        println!("  相近: 0x{:x}: {} ({})", k, t.name, t.kind);
                    }
                }
            }
            if !found {
                println!("  没有相近的偏移");
            }
        }
    }

    pub fn has_dwarf_info(&self) -> bool {
        !self.struct_map.is_empty()
    }

    pub fn type_cache(&self) -> &HashMap<u64, TypeInfo> {
        &self.type_cache
    }

    pub fn type_count(&self) -> usize {
        self.type_cache.len()
    }

    pub fn get_stats(
        &self,
    ) -> (
        usize,
        usize,
        usize,
        usize,
        usize,
        usize,
        usize,
        usize,
        usize,
        usize,
    ) {
        (
            self.stats.base_types,
            self.stats.structs,
            self.stats.unions,
            self.stats.enums,
            self.stats.arrays,
            self.stats.pointers,
            self.stats.typedefs,
            self.stats.variables,
            self.stats.struct_members,
            self.stats.enum_values,
        )
    }

    pub fn get_type_by_offset(&self, offset: u64) -> Option<&TypeInfo> {
        self.type_cache.get(&offset)
    }

    pub fn all_types(&self) -> Vec<&TypeInfo> {
        self.type_cache.values().collect()
    }

    pub fn find_struct_by_name(&self, name: &str) -> Option<&TypeInfo> {
        self.struct_map.get(name)
    }

    pub fn find_structs_containing_member(
        &self,
        member_name: &str,
    ) -> Vec<(&TypeInfo, &StructMember)> {
        let mut results = Vec::new();
        let search_lower = member_name.to_lowercase();

        // 首先尝试精确匹配
        for type_info in self.struct_map.values() {
            if !type_info.members.is_empty() {
                for member in &type_info.members {
                    let member_lower = member.name.to_lowercase();
                    if member_lower == search_lower {
                        results.push((type_info, member));
                    }
                }
            }
        }

        // 如果精确匹配没有结果，尝试包含匹配
        if results.is_empty() {
            for type_info in self.struct_map.values() {
                if !type_info.members.is_empty() {
                    for member in &type_info.members {
                        let member_lower = member.name.to_lowercase();
                        if member_lower.contains(&search_lower) {
                            results.push((type_info, member));
                        }
                    }
                }
            }
        }

        results
    }

    pub fn list_structs(&self) -> Vec<&TypeInfo> {
        self.struct_map.values().collect()
    }

    pub fn list_variables_with_types(&self) -> Vec<(String, String)> {
        self.variable_types
            .iter()
            .filter_map(|(name, &type_offset)| {
                if let Some(type_info) = self.type_cache.get(&type_offset) {
                    Some((name.clone(), type_info.name.clone()))
                } else {
                    Some((
                        name.clone(),
                        format!("unknown (offset: 0x{:x})", type_offset),
                    ))
                }
            })
            .collect()
    }

    pub fn list_struct_instance_variables(&self) -> Vec<(String, &TypeInfo)> {
        self.variable_types
            .iter()
            .filter_map(|(name, &type_offset)| {
                if let Some(type_info) = self.type_cache.get(&type_offset) {
                    if type_info.kind == crate::types::TypeKind::Struct {
                        return Some((name.clone(), type_info));
                    }
                }
                None
            })
            .collect()
    }

    pub fn debug_type_resolution(&self) -> (usize, usize, usize) {
        let total = self.variable_types.len();
        let mut resolved = 0;
        let mut unresolved_offsets = std::collections::HashSet::new();

        for &offset in self.variable_types.values() {
            if self.type_cache.contains_key(&offset) {
                resolved += 1;
            } else {
                unresolved_offsets.insert(offset);
            }
        }

        (total, resolved, unresolved_offsets.len())
    }
}

pub fn analyze_variables_with_dwarf(variables: &mut [Variable], elf_data: &[u8]) -> Result<bool> {
    let parser = DwarfParser::parse(elf_data)?;

    let has_dwarf = parser.has_dwarf_info();

    for var in variables.iter_mut() {
        if let Some(&type_offset) = parser.variable_types.get(&var.name) {
            if let Some(type_info) = parser.type_cache.get(&type_offset) {
                var.type_info = Some(type_info.clone());
                continue;
            }
        }

        let type_info = infer_type_from_name(&var.name, var.size);
        var.type_info = Some(type_info);
    }

    Ok(has_dwarf)
}

fn infer_type_from_name(name: &str, size: usize) -> TypeInfo {
    let lower = name.to_lowercase();

    let (encoding, type_name) = if lower.contains("_u8")
        || lower.contains("_uint8")
        || lower.ends_with("_u8")
        || lower.contains("uint8_t")
    {
        (TypeEncoding::Unsigned, "uint8_t".to_string())
    } else if lower.contains("_u16")
        || lower.contains("_uint16")
        || lower.ends_with("_u16")
        || lower.contains("uint16_t")
    {
        (TypeEncoding::Unsigned, "uint16_t".to_string())
    } else if lower.contains("_u32")
        || lower.contains("_uint32")
        || lower.ends_with("_u32")
        || lower.contains("uint32_t")
    {
        (TypeEncoding::Unsigned, "uint32_t".to_string())
    } else if lower.contains("_u64")
        || lower.contains("_uint64")
        || lower.ends_with("_u64")
        || lower.contains("uint64_t")
    {
        (TypeEncoding::Unsigned, "uint64_t".to_string())
    } else if lower.contains("_s8")
        || lower.contains("_int8")
        || lower.ends_with("_s8")
        || lower.contains("int8_t")
    {
        (TypeEncoding::Signed, "int8_t".to_string())
    } else if lower.contains("_s16")
        || lower.contains("_int16")
        || lower.ends_with("_s16")
        || lower.contains("int16_t")
    {
        (TypeEncoding::Signed, "int16_t".to_string())
    } else if lower.contains("_s32")
        || lower.contains("_int32")
        || lower.ends_with("_s32")
        || lower.contains("int32_t")
        || lower.contains("_int")
        || lower.ends_with("_i")
    {
        (TypeEncoding::Signed, "int32_t".to_string())
    } else if lower.contains("_s64")
        || lower.contains("_int64")
        || lower.ends_with("_s64")
        || lower.contains("int64_t")
    {
        (TypeEncoding::Signed, "int64_t".to_string())
    } else if lower.contains("_f32")
        || lower.contains("_float")
        || lower.ends_with("_f32")
        || lower.contains("float32")
    {
        (TypeEncoding::Float, "float".to_string())
    } else if lower.contains("_f64")
        || lower.contains("_double")
        || lower.ends_with("_f64")
        || lower.contains("float64")
    {
        (TypeEncoding::Float, "double".to_string())
    } else if lower.contains("_bool") || lower.ends_with("_b") || lower.contains("boolean") {
        (TypeEncoding::Unsigned, "bool".to_string())
    } else {
        match size {
            1 => (TypeEncoding::Unsigned, "uint8_t".to_string()),
            2 => (TypeEncoding::Unsigned, "uint16_t".to_string()),
            4 => (TypeEncoding::Unsigned, "uint32_t".to_string()),
            8 => (TypeEncoding::Unsigned, "uint64_t".to_string()),
            _ => (TypeEncoding::Unsigned, format!("uint8_t[{}]", size)),
        }
    };

    TypeInfo::primitive(type_name, size, encoding)
}
