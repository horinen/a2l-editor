use crate::types::{EnumVariant, StructMember, TypeEncoding, TypeInfo, TypeKind, Variable};
use anyhow::{Context, Result};
use gimli::{EndianSlice, RunTimeEndian};
use object::{Object, ObjectSection};
use std::collections::{HashMap, HashSet};

type DwarfReader = EndianSlice<'static, RunTimeEndian>;

#[derive(Debug, Clone)]
pub struct DwarfVariable {
    pub name: String,
    pub address: u64,
    pub type_offset: u64,
}

#[derive(Debug)]
struct CompositeBuilder {
    kind: TypeKind,
    global_offset: u64,
    name: Option<String>,
    size: usize,
    encoding: TypeEncoding,
    depth: isize,
    members: Vec<StructMember>,
    variants: Vec<EnumVariant>,
    array_dims: Vec<usize>,
    elem_type_offset: Option<u64>,
}

impl CompositeBuilder {
    fn new(kind: TypeKind, global_offset: u64, depth: isize) -> Self {
        Self {
            kind,
            global_offset,
            name: None,
            size: 0,
            encoding: TypeEncoding::Unsigned,
            depth,
            members: Vec::new(),
            variants: Vec::new(),
            array_dims: Vec::new(),
            elem_type_offset: None,
        }
    }

    fn into_type_info(self) -> TypeInfo {
        let type_name = self.name.clone().unwrap_or_else(|| {
            format!(
                "<anonymous_{}@0x{:x}>",
                self.kind.to_string().to_lowercase(),
                self.global_offset
            )
        });

        match self.kind {
            TypeKind::Struct => {
                TypeInfo::struct_type(type_name, self.size, self.members, self.global_offset)
            }
            TypeKind::Union => {
                TypeInfo::union_type(type_name, self.size, self.members, self.global_offset)
            }
            TypeKind::Enum => TypeInfo::enum_type(
                type_name,
                self.size,
                self.encoding,
                self.variants,
                self.global_offset,
            ),
            TypeKind::Array => TypeInfo::array_type(
                Self::format_array_name(&self.array_dims),
                self.size,
                TypeInfo::primitive("unknown".to_string(), 0, TypeEncoding::Unsigned),
                self.array_dims.clone(),
                self.global_offset,
            ),
            _ => TypeInfo::primitive(type_name, self.size, self.encoding),
        }
    }

    fn format_array_name(dims: &[usize]) -> String {
        if dims.is_empty() {
            return "array".to_string();
        }
        let dims_str: Vec<String> = dims.iter().map(|d| d.to_string()).collect();
        format!("array[{}]", dims_str.join("]["))
    }
}

struct TypeResolver;

impl TypeResolver {
    fn resolve(
        type_cache: &mut HashMap<u64, TypeInfo>,
        type_refs: &HashMap<u64, u64>,
        array_elem_offsets: &HashMap<u64, u64>,
    ) {
        Self::resolve_type_graph(type_cache, type_refs);
        Self::resolve_array_element_types(type_cache, type_refs, array_elem_offsets);
        Self::resolve_alias_chains(type_cache, type_refs);
        Self::resolve_member_types(type_cache);
        Self::normalize_bitfield_offsets(type_cache);
    }

    fn resolve_type_graph(type_cache: &mut HashMap<u64, TypeInfo>, type_refs: &HashMap<u64, u64>) {
        let refs: Vec<(u64, u64)> = type_refs.iter().map(|(k, v)| (*k, *v)).collect();

        let mut in_deg: HashMap<u64, usize> = HashMap::new();
        let mut dependents: HashMap<u64, Vec<u64>> = HashMap::new();

        for &(from, to) in &refs {
            *in_deg.entry(from).or_insert(0) += 1;
            dependents.entry(to).or_default().push(from);
        }

        let mut queue: std::collections::VecDeque<u64> = std::collections::VecDeque::new();
        for &(from, to) in &refs {
            if type_cache.get(&to).map(|t| t.size > 0).unwrap_or(false)
                && in_deg.get(&from).copied().unwrap_or(0) == 1
            {
                queue.push_back(from);
            }
        }

        let mut sorted: Vec<u64> = Vec::new();
        let mut visited_sort: HashSet<u64> = HashSet::new();
        while let Some(node) = queue.pop_front() {
            if visited_sort.contains(&node) {
                continue;
            }
            visited_sort.insert(node);
            sorted.push(node);

            if let Some(deps) = dependents.get(&node) {
                for &dep in deps {
                    let deg = in_deg.get_mut(&dep).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(dep);
                    }
                }
            }
        }

        for offset in &sorted {
            if let Some(&target_offset) = type_refs.get(offset) {
                Self::copy_resolved_type(type_cache, *offset, target_offset, true);
            }
        }

        for &(from_offset, to_offset) in &refs {
            if to_offset > 0 {
                Self::copy_resolved_type(type_cache, from_offset, to_offset, true);
            }
        }
    }

    fn copy_resolved_type(
        type_cache: &mut HashMap<u64, TypeInfo>,
        from_offset: u64,
        to_offset: u64,
        only_empty: bool,
    ) {
        let Some(target_type) = type_cache.get(&to_offset).cloned() else {
            return;
        };
        if target_type.size == 0 {
            return;
        }
        if let Some(type_info) = type_cache.get_mut(&from_offset) {
            if only_empty && type_info.size > 0 {
                return;
            }
            let own_name = type_info.name.clone();
            *type_info = target_type;
            type_info.name = own_name;
            type_info.offset = from_offset;
        }
    }

    fn resolve_member_types(type_cache: &mut HashMap<u64, TypeInfo>) {
        let resolutions: Vec<(u64, Vec<(usize, String, usize)>)> = type_cache
            .iter()
            .filter(|(_, t)| t.kind == TypeKind::Struct || t.kind == TypeKind::Union)
            .map(|(offset, type_info)| {
                let member_updates: Vec<(usize, String, usize)> = type_info
                    .members
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, member)| {
                        member.type_offset.and_then(|type_offset| {
                            if type_offset > 0 {
                                type_cache
                                    .get(&type_offset)
                                    .map(|resolved| (idx, resolved.name.clone(), resolved.size))
                            } else {
                                None
                            }
                        })
                    })
                    .collect();
                (*offset, member_updates)
            })
            .collect();

        for (offset, updates) in resolutions {
            if let Some(type_info) = type_cache.get_mut(&offset) {
                for (idx, type_name, type_size) in updates {
                    let member = &mut type_info.members[idx];
                    member.type_name = type_name;
                    if member.type_size == 0 {
                        member.type_size = type_size;
                    }
                }
            }
        }
    }

    fn resolve_alias_chains(
        type_cache: &mut HashMap<u64, TypeInfo>,
        type_refs: &HashMap<u64, u64>,
    ) {
        for _ in 0..32 {
            let refs: Vec<(u64, u64)> = type_refs.iter().map(|(from, to)| (*from, *to)).collect();
            let mut updates = Vec::new();

            for (from_offset, to_offset) in refs {
                let Some(current) = type_cache.get(&from_offset) else {
                    continue;
                };
                let Some(target) = type_cache.get(&to_offset) else {
                    continue;
                };
                if target.size == 0 {
                    continue;
                }
                if current.size > 0 && Self::same_resolved_type(current, target) {
                    continue;
                }

                updates.push((from_offset, target.clone()));
            }

            if updates.is_empty() {
                break;
            }

            for (from_offset, target_type) in updates {
                if let Some(type_info) = type_cache.get_mut(&from_offset) {
                    let own_name = type_info.name.clone();
                    *type_info = target_type;
                    type_info.name = own_name;
                    type_info.offset = from_offset;
                }
            }
        }
    }

    fn resolve_array_element_types(
        type_cache: &mut HashMap<u64, TypeInfo>,
        type_refs: &HashMap<u64, u64>,
        array_elem_offsets: &HashMap<u64, u64>,
    ) {
        let updates: Vec<(u64, TypeInfo)> = type_cache
            .iter()
            .filter(|(_, t)| t.kind == TypeKind::Array)
            .filter_map(|(array_offset, _)| {
                let elem_offset = *array_elem_offsets.get(array_offset)?;
                if elem_offset == 0 {
                    return None;
                }
                let mut visiting = HashSet::new();
                Self::resolve_type_by_offset(
                    type_cache,
                    type_refs,
                    array_elem_offsets,
                    elem_offset,
                    &mut visiting,
                )
                .map(|elem_type| (*array_offset, elem_type))
            })
            .collect();

        for (array_offset, elem_type) in updates {
            if let Some(array_type) = type_cache.get_mut(&array_offset) {
                let needs_update = match &array_type.pointer_target {
                    None => true,
                    Some(current) => !Self::same_resolved_type(current, &elem_type),
                };
                if needs_update {
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

    fn resolve_type_by_offset(
        type_cache: &HashMap<u64, TypeInfo>,
        type_refs: &HashMap<u64, u64>,
        array_elem_offsets: &HashMap<u64, u64>,
        offset: u64,
        visiting: &mut HashSet<u64>,
    ) -> Option<TypeInfo> {
        if !visiting.insert(offset) {
            return None;
        }

        let mut resolved = type_cache.get(&offset)?.clone();

        if let Some(&target_offset) = type_refs.get(&offset) {
            if target_offset > 0 {
                if let Some(target) = Self::resolve_type_by_offset(
                    type_cache,
                    type_refs,
                    array_elem_offsets,
                    target_offset,
                    visiting,
                ) {
                    let own_name = resolved.name.clone();
                    resolved = target;
                    resolved.name = own_name;
                    resolved.offset = offset;
                }
            }
        }

        if resolved.kind == TypeKind::Array {
            if let Some(&elem_offset) = array_elem_offsets.get(&offset) {
                if elem_offset > 0 {
                    if let Some(elem_type) = Self::resolve_type_by_offset(
                        type_cache,
                        type_refs,
                        array_elem_offsets,
                        elem_offset,
                        visiting,
                    ) {
                        resolved.encoding = elem_type.encoding;
                        resolved.pointer_target = Some(Box::new(elem_type));
                    }
                }
            }
        }

        visiting.remove(&offset);
        Some(resolved)
    }

    fn normalize_bitfield_offsets(type_cache: &mut HashMap<u64, TypeInfo>) {
        let offsets: Vec<u64> = type_cache
            .iter()
            .filter(|(_, t)| t.kind == TypeKind::Struct || t.kind == TypeKind::Union)
            .map(|(offset, _)| *offset)
            .collect();

        for offset in offsets {
            if let Some(type_info) = type_cache.get_mut(&offset) {
                for member in &mut type_info.members {
                    if member.is_bitfield() && !member.bit_offset_is_absolute {
                        let storage_bits = member.type_size * 8;
                        let raw_bo = member.bit_offset.unwrap_or(0);
                        let raw_bs = member.bit_size.unwrap_or(0);
                        let absolute_lsb =
                            member.offset * 8 + storage_bits.saturating_sub(raw_bo + raw_bs);
                        member.bit_offset = Some(absolute_lsb);
                        member.bit_offset_is_absolute = true;
                    }
                }
            }
        }
    }

    fn same_resolved_type(left: &TypeInfo, right: &TypeInfo) -> bool {
        if left.kind != right.kind
            || left.size != right.size
            || left.array_dims != right.array_dims
            || left.members.len() != right.members.len()
            || left.variants.len() != right.variants.len()
        {
            return false;
        }

        if !left
            .members
            .iter()
            .zip(&right.members)
            .all(|(left, right)| {
                left.name == right.name
                    && left.offset == right.offset
                    && left.type_name == right.type_name
                    && left.type_size == right.type_size
                    && left.type_offset == right.type_offset
                    && left.bit_offset == right.bit_offset
                    && left.bit_size == right.bit_size
                    && left.bit_offset_is_absolute == right.bit_offset_is_absolute
            })
        {
            return false;
        }

        if !left
            .variants
            .iter()
            .zip(&right.variants)
            .all(|(left, right)| left.name == right.name && left.value == right.value)
        {
            return false;
        }

        match (
            left.pointer_target.as_deref(),
            right.pointer_target.as_deref(),
        ) {
            (Some(left_target), Some(right_target)) => {
                Self::same_resolved_type(left_target, right_target)
            }
            (None, None) => true,
            _ => false,
        }
    }
}

pub struct DwarfParser {
    type_cache: HashMap<u64, TypeInfo>,
    struct_map: HashMap<String, TypeInfo>,
    variable_types: HashMap<String, u64>,
    global_variables: Vec<DwarfVariable>,
    array_elem_offsets: HashMap<u64, u64>,
    type_refs: HashMap<u64, u64>,
    stats: DwarfStats,
    big_endian: bool,
    debug_str: Option<&'static [u8]>,
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
            global_variables: Vec::new(),
            array_elem_offsets: HashMap::new(),
            type_refs: HashMap::new(),
            stats: DwarfStats::default(),
            big_endian: false,
            debug_str: None,
        }
    }

    pub fn parse(elf_data: &[u8]) -> Result<Self> {
        let mut parser = Self::new();

        let obj = object::File::parse(elf_data).context("无法解析 ELF 文件")?;

        parser.big_endian = !obj.is_little_endian();

        let debug_info = obj.section_by_name(".debug_info");
        let debug_abbrev = obj.section_by_name(".debug_abbrev");

        if debug_info.is_none() || debug_abbrev.is_none() {
            return Ok(parser);
        }

        let debug_info_data = Self::get_section_bytes(&debug_info.unwrap());
        let debug_abbrev_data = Self::get_section_bytes(&debug_abbrev.unwrap());

        parser.debug_str = obj
            .section_by_name(".debug_str")
            .map(|s| Self::get_section_bytes(&s));

        if debug_info_data.is_empty() || debug_abbrev_data.is_empty() {
            return Ok(parser);
        }

        let endian = if obj.is_little_endian() {
            RunTimeEndian::Little
        } else {
            RunTimeEndian::Big
        };
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

        TypeResolver::resolve(
            &mut self.type_cache,
            &self.type_refs,
            &self.array_elem_offsets,
        );
        Ok(())
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

        let mut composite_stack: Vec<CompositeBuilder> = Vec::new();
        let mut cursor = header.entries(abbrevs);
        let mut current_depth: isize = 0;

        while let Some((delta, entry)) = cursor.next_dfs().context("遍历 DIE 失败")? {
            current_depth += delta;
            let global_offset = unit_offset + entry.offset().0;

            while let Some(top) = composite_stack.last() {
                if current_depth <= top.depth {
                    let completed = composite_stack.pop().unwrap();
                    self.save_composite_type(completed, unit_offset);
                } else {
                    break;
                }
            }

            match entry.tag() {
                gimli::constants::DW_TAG_base_type => {
                    self.parse_base_type_with_offset(entry, global_offset);
                }
                gimli::constants::DW_TAG_structure_type => {
                    let builder = CompositeBuilder::new(
                        TypeKind::Struct,
                        global_offset as u64,
                        current_depth,
                    );
                    composite_stack.push(builder);
                    self.pre_parse_composite(
                        entry,
                        composite_stack.last_mut().unwrap(),
                        unit_offset,
                    );
                }
                gimli::constants::DW_TAG_union_type => {
                    let builder =
                        CompositeBuilder::new(TypeKind::Union, global_offset as u64, current_depth);
                    composite_stack.push(builder);
                    self.pre_parse_composite(
                        entry,
                        composite_stack.last_mut().unwrap(),
                        unit_offset,
                    );
                }
                gimli::constants::DW_TAG_enumeration_type => {
                    let builder =
                        CompositeBuilder::new(TypeKind::Enum, global_offset as u64, current_depth);
                    composite_stack.push(builder);
                    self.pre_parse_composite(
                        entry,
                        composite_stack.last_mut().unwrap(),
                        unit_offset,
                    );
                }
                gimli::constants::DW_TAG_array_type => {
                    let builder =
                        CompositeBuilder::new(TypeKind::Array, global_offset as u64, current_depth);
                    let elem_type_offset = Self::get_type_offset_with_unit(entry, unit_offset);
                    composite_stack.push(builder);
                    if let Some(top) = composite_stack.last_mut() {
                        top.size = Self::get_size_static(entry);
                        if elem_type_offset > 0 {
                            top.elem_type_offset = Some(elem_type_offset);
                            self.array_elem_offsets
                                .insert(global_offset as u64, elem_type_offset);
                        }
                    }
                }
                gimli::constants::DW_TAG_pointer_type => {
                    self.parse_pointer_type_with_offset(entry, global_offset);
                }
                gimli::constants::DW_TAG_typedef => {
                    self.parse_typedef_with_offset(entry, global_offset, unit_offset);
                }
                gimli::constants::DW_TAG_const_type => {
                    self.parse_const_type_with_offset(entry, global_offset, unit_offset);
                }
                gimli::constants::DW_TAG_volatile_type => {
                    self.parse_volatile_type_with_offset(entry, global_offset, unit_offset);
                }
                gimli::constants::DW_TAG_variable => {
                    self.parse_variable(entry, unit_offset);
                }
                gimli::constants::DW_TAG_member => {
                    self.stats.struct_members += 1;
                    if let Some(parent) = composite_stack.last_mut() {
                        if parent.kind == TypeKind::Struct || parent.kind == TypeKind::Union {
                            if let Some(member) = self.parse_member(entry, unit_offset) {
                                parent.members.push(member);
                            }
                        }
                    }
                }
                gimli::constants::DW_TAG_enumerator => {
                    self.stats.enum_values += 1;
                    if let Some(parent) = composite_stack.last_mut() {
                        if parent.kind == TypeKind::Enum {
                            if let Some(name) = self.get_name(entry) {
                                if let Some(value) = Self::get_enum_value(entry) {
                                    parent.variants.push(EnumVariant::new(name, value));
                                }
                            }
                        }
                    }
                }
                gimli::constants::DW_TAG_subrange_type => {
                    if let Some(parent) = composite_stack.last_mut() {
                        if parent.kind == TypeKind::Array {
                            if let Some(dim) = Self::get_array_dimension(entry) {
                                parent.array_dims.push(dim);
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        while let Some(completed) = composite_stack.pop() {
            self.save_composite_type(completed, unit_offset);
        }

        Ok(())
    }

    fn pre_parse_composite(
        &mut self,
        entry: &gimli::DebuggingInformationEntry<DwarfReader>,
        builder: &mut CompositeBuilder,
        unit_offset: usize,
    ) {
        builder.name = self.get_name(entry);
        builder.size = Self::get_size_static(entry);
        if builder.kind == TypeKind::Enum {
            builder.encoding = Self::get_encoding_static(entry);
        }
        if builder.kind == TypeKind::Array {
            let elem_type_offset = Self::get_type_offset_with_unit(entry, unit_offset);
            if elem_type_offset > 0 {
                builder.elem_type_offset = Some(elem_type_offset);
            }
        }
    }

    fn parse_member(
        &mut self,
        entry: &gimli::DebuggingInformationEntry<DwarfReader>,
        unit_offset: usize,
    ) -> Option<StructMember> {
        let name = self.get_name(entry).unwrap_or_else(|| "_".to_string());
        let offset = Self::get_member_location_static(entry);
        let size = Self::get_size_static(entry);
        let (type_offset, is_unit_ref) = Self::get_type_offset_info_static(entry);
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

        let mut member = StructMember::new(name, offset, "unknown".to_string(), size)
            .with_type_offset(global_type_offset as u64);

        if let Some((bit_offset, bit_size, is_absolute)) = bitfield_info {
            member = member.with_bitfield(bit_offset, bit_size, is_absolute);
        }

        Some(member)
    }

    fn save_composite_type(&mut self, builder: CompositeBuilder, _unit_offset: usize) {
        match builder.kind {
            TypeKind::Struct => {
                self.stats.structs += 1;
            }
            TypeKind::Union => {
                self.stats.unions += 1;
            }
            TypeKind::Enum => {
                self.stats.enums += 1;
            }
            TypeKind::Array => {
                self.stats.arrays += 1;
            }
            _ => {}
        }

        let original_name = builder.name.clone();
        let type_info = builder.into_type_info();
        let offset = type_info.offset;
        let kind = type_info.kind;

        self.type_cache.insert(offset, type_info);

        if kind == TypeKind::Struct || kind == TypeKind::Union {
            if let Some(named) = original_name {
                self.struct_map
                    .insert(named, self.type_cache.get(&offset).unwrap().clone());
            }
        }
    }

    fn parse_base_type_with_offset(
        &mut self,
        entry: &gimli::DebuggingInformationEntry<DwarfReader>,
        global_offset: usize,
    ) {
        let name = self.get_name(entry);
        let size = Self::get_size_static(entry);
        let encoding = Self::get_encoding_static(entry);

        if let Some(type_name) = name {
            let mut type_info = TypeInfo::primitive(type_name.clone(), size, encoding);
            type_info.offset = global_offset as u64;
            self.type_cache.insert(global_offset as u64, type_info);
            self.stats.base_types += 1;
        }
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
                gimli::AttributeValue::Sdata(v) => {
                    if v >= 0 {
                        return Some(v as usize + 1);
                    }
                    return None;
                }
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
                gimli::AttributeValue::Sdata(v) => {
                    if v >= 0 {
                        return Some(v as usize);
                    }
                    return None;
                }
                _ => {}
            }
        }

        None
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
        unit_offset: usize,
    ) {
        let name = self.get_name(entry);
        let target_offset = Self::get_type_offset_with_unit(entry, unit_offset);

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
        unit_offset: usize,
    ) {
        let name = self.get_name(entry);
        let target_offset = Self::get_type_offset_with_unit(entry, unit_offset);

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
        unit_offset: usize,
    ) {
        let name = self.get_name(entry);
        let target_offset = Self::get_type_offset_with_unit(entry, unit_offset);

        let type_name = name.unwrap_or_else(|| "volatile".to_string());
        let mut type_info = TypeInfo::primitive(type_name, 0, TypeEncoding::Unsigned);
        type_info.offset = global_offset as u64;

        if target_offset > 0 {
            self.type_refs
                .insert(global_offset as u64, target_offset as u64);
        }

        self.type_cache.insert(global_offset as u64, type_info);
    }

    fn parse_variable(
        &mut self,
        entry: &gimli::DebuggingInformationEntry<DwarfReader>,
        unit_offset: usize,
    ) {
        self.stats.variables += 1;

        if let Some(name) = self.get_name(entry) {
            let type_offset = Self::get_type_offset_with_unit(entry, unit_offset);

            if let Some(address) = self.parse_location(entry) {
                self.global_variables.push(DwarfVariable {
                    name: name.clone(),
                    address,
                    type_offset,
                });
            }

            if type_offset > 0 {
                self.variable_types.insert(name, type_offset);
            }
        }
    }

    fn parse_location(&self, entry: &gimli::DebuggingInformationEntry<DwarfReader>) -> Option<u64> {
        let attr = entry
            .attr(gimli::constants::DW_AT_location)
            .ok()
            .flatten()?;

        let value = attr.value();
        match &value {
            gimli::AttributeValue::Exprloc(expr) => self.parse_dw_op_addr(expr.0.as_ref()),
            gimli::AttributeValue::Block(block) => self.parse_dw_op_addr(block.as_ref()),
            _ => None,
        }
    }

    fn parse_dw_op_addr(&self, data: &[u8]) -> Option<u64> {
        if data.is_empty() || data[0] != 0x03 {
            return None;
        }

        match data.len() {
            5 => {
                if self.big_endian {
                    Some(
                        ((data[1] as u64) << 24)
                            | ((data[2] as u64) << 16)
                            | ((data[3] as u64) << 8)
                            | data[4] as u64,
                    )
                } else {
                    Some(
                        data[1] as u64
                            | ((data[2] as u64) << 8)
                            | ((data[3] as u64) << 16)
                            | ((data[4] as u64) << 24),
                    )
                }
            }
            9 => {
                if self.big_endian {
                    Some(
                        ((data[1] as u64) << 56)
                            | ((data[2] as u64) << 48)
                            | ((data[3] as u64) << 40)
                            | ((data[4] as u64) << 32)
                            | ((data[5] as u64) << 24)
                            | ((data[6] as u64) << 16)
                            | ((data[7] as u64) << 8)
                            | data[8] as u64,
                    )
                } else {
                    Some(
                        data[1] as u64
                            | ((data[2] as u64) << 8)
                            | ((data[3] as u64) << 16)
                            | ((data[4] as u64) << 24)
                            | ((data[5] as u64) << 32)
                            | ((data[6] as u64) << 40)
                            | ((data[7] as u64) << 48)
                            | ((data[8] as u64) << 56),
                    )
                }
            }
            _ => None,
        }
    }

    pub fn get_variable_count(&self) -> usize {
        self.variable_types.len()
    }

    pub fn variable_types(&self) -> &HashMap<String, u64> {
        &self.variable_types
    }

    pub fn global_variables(&self) -> &[DwarfVariable] {
        &self.global_variables
    }

    pub fn global_variable_count(&self) -> usize {
        self.global_variables.len()
    }

    pub fn get_type_cache_size(&self) -> usize {
        self.type_cache.len()
    }

    fn get_name(&self, entry: &gimli::DebuggingInformationEntry<DwarfReader>) -> Option<String> {
        entry
            .attr(gimli::constants::DW_AT_name)
            .ok()
            .flatten()
            .and_then(|attr| match attr.value() {
                gimli::AttributeValue::String(s) => Some(String::from_utf8_lossy(&s).to_string()),
                gimli::AttributeValue::DebugStrRef(offset) => {
                    self.read_debug_str(offset.0 as usize)
                }
                _ => None,
            })
    }

    fn read_debug_str(&self, offset: usize) -> Option<String> {
        let data = self.debug_str?;
        if offset >= data.len() {
            return None;
        }
        let slice = &data[offset..];
        let end = slice.iter().position(|&b| b == 0).unwrap_or(slice.len());
        Some(String::from_utf8_lossy(&slice[..end]).to_string())
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

    fn read_uleb128(data: &[u8]) -> usize {
        let mut result: usize = 0;
        let mut shift: usize = 0;
        for &byte in data {
            result |= ((byte & 0x7f) as usize) << shift;
            if byte & 0x80 == 0 {
                break;
            }
            shift += 7;
        }
        result
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
                        Some(Self::read_uleb128(&block[1..]))
                    } else {
                        None
                    }
                }
                gimli::AttributeValue::Exprloc(expr) => {
                    let data = &expr.0;
                    if data.len() >= 2 && data[0] == 0x23 {
                        Some(Self::read_uleb128(&data[1..]))
                    } else {
                        None
                    }
                }
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
    ) -> Option<(usize, usize, bool)> {
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

        let read_attr = |attr_name| {
            entry
                .attr(attr_name)
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
                })
        };

        if let Some(data_bit_offset) = read_attr(gimli::constants::DW_AT_data_bit_offset) {
            return Some((data_bit_offset, bit_size, true));
        }

        let bit_offset = read_attr(gimli::constants::DW_AT_bit_offset).unwrap_or(0);
        Some((bit_offset, bit_size, false))
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
            println!("  size: {}, encoding: {:?}", t.size, t.encoding);
            if !t.array_dims.is_empty() {
                println!("  array_dims: {:?}", t.array_dims);
            }
        } else {
            println!("  未找到");
        }
        if let Some(target) = self.type_refs.get(&offset) {
            println!("  type_refs -> 0x{:x}", target);
            if let Some(t) = self.type_cache.get(target) {
                println!("    目标: {} ({}, size={})", t.name, t.kind, t.size);
            }
        }
    }

    pub fn has_dwarf_info(&self) -> bool {
        !self.type_cache.is_empty() || !self.global_variables.is_empty()
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
                var.type_info = type_info.clone();
                continue;
            }
        }

        var.type_info = infer_type_from_name(&var.name, var.size);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_array_element_alias_chain_to_struct() {
        let mut parser = DwarfParser::new();
        let struct_offset = 0x10;
        let alias_offset = 0x20;
        let const_offset = 0x30;
        let array_offset = 0x40;

        let member = StructMember::new(
            "FltDebValue".to_string(),
            0,
            "unsigned short int".to_string(),
            2,
        )
        .with_bitfield(0, 14, true);
        parser.type_cache.insert(
            struct_offset,
            TypeInfo::struct_type("FaultDebounce".to_string(), 2, vec![member], struct_offset),
        );

        let mut alias = TypeInfo::primitive("Alias".to_string(), 0, TypeEncoding::Unsigned);
        alias.offset = alias_offset;
        parser.type_cache.insert(alias_offset, alias);
        parser.type_refs.insert(alias_offset, struct_offset);

        let mut const_type = TypeInfo::primitive("const".to_string(), 0, TypeEncoding::Unsigned);
        const_type.offset = const_offset;
        parser.type_cache.insert(const_offset, const_type);
        parser.type_refs.insert(const_offset, alias_offset);

        parser.type_cache.insert(
            array_offset,
            TypeInfo::array_type(
                "array[13]".to_string(),
                26,
                TypeInfo::primitive("unknown".to_string(), 0, TypeEncoding::Unsigned),
                vec![13],
                array_offset,
            ),
        );
        parser.array_elem_offsets.insert(array_offset, const_offset);

        TypeResolver::resolve(
            &mut parser.type_cache,
            &parser.type_refs,
            &parser.array_elem_offsets,
        );

        let array_type = parser.type_cache.get(&array_offset).unwrap();
        let elem_type = array_type.pointer_target.as_deref().unwrap();
        assert_eq!(elem_type.kind, TypeKind::Struct);
        assert_eq!(elem_type.size, 2);
        assert_eq!(elem_type.members[0].name, "FltDebValue");
    }

    #[test]
    fn propagates_nested_array_element_updates() {
        let mut parser = DwarfParser::new();
        let struct_offset = 0x10;
        let inner_array_offset = 0x20;
        let outer_array_offset = 0x30;

        let member = StructMember::new(
            "CurrentDebStatus".to_string(),
            1,
            "unsigned char".to_string(),
            1,
        )
        .with_bitfield(14, 2, true);
        let struct_type =
            TypeInfo::struct_type("FaultDebounce".to_string(), 2, vec![member], struct_offset);
        parser.type_cache.insert(struct_offset, struct_type);

        let stale_inner = TypeInfo::array_type(
            "array[23]".to_string(),
            46,
            TypeInfo::primitive("unknown".to_string(), 0, TypeEncoding::Unsigned),
            vec![23],
            inner_array_offset,
        );
        parser
            .type_cache
            .insert(inner_array_offset, stale_inner.clone());
        parser
            .array_elem_offsets
            .insert(inner_array_offset, struct_offset);

        parser.type_cache.insert(
            outer_array_offset,
            TypeInfo::array_type(
                "array[1]".to_string(),
                46,
                stale_inner,
                vec![1],
                outer_array_offset,
            ),
        );
        parser
            .array_elem_offsets
            .insert(outer_array_offset, inner_array_offset);

        TypeResolver::resolve(
            &mut parser.type_cache,
            &parser.type_refs,
            &parser.array_elem_offsets,
        );

        let outer = parser.type_cache.get(&outer_array_offset).unwrap();
        let inner = outer.pointer_target.as_deref().unwrap();
        let elem = inner.pointer_target.as_deref().unwrap();
        assert_eq!(elem.kind, TypeKind::Struct);
        assert_eq!(elem.members[0].name, "CurrentDebStatus");
    }

    #[test]
    fn recursively_resolves_deep_nested_array_elements() {
        let mut parser = DwarfParser::new();
        let elem_offset = 0x10;
        parser.type_cache.insert(
            elem_offset,
            TypeInfo::primitive("uint8_t".to_string(), 1, TypeEncoding::Unsigned),
        );

        let mut child_offset = elem_offset;
        for depth in (0..12).rev() {
            let array_offset = 0x20 + depth;
            parser.type_cache.insert(
                array_offset,
                TypeInfo::array_type(
                    format!("array_depth_{}", depth),
                    1,
                    TypeInfo::primitive("unknown".to_string(), 0, TypeEncoding::Unsigned),
                    vec![1],
                    array_offset,
                ),
            );
            parser.array_elem_offsets.insert(array_offset, child_offset);
            child_offset = array_offset;
        }

        TypeResolver::resolve(
            &mut parser.type_cache,
            &parser.type_refs,
            &parser.array_elem_offsets,
        );

        let mut current = parser
            .type_cache
            .get(&0x20)
            .and_then(|t| t.pointer_target.as_deref())
            .unwrap();
        for _ in 1..12 {
            current = current.pointer_target.as_deref().unwrap();
        }
        assert_eq!(current.name, "uint8_t");
        assert_eq!(current.size, 1);
    }

    #[test]
    fn refreshes_alias_after_array_targets_are_resolved() {
        let mut parser = DwarfParser::new();
        let enum_offset = 0x10;
        let inner_array_offset = 0x20;
        let outer_array_offset = 0x30;
        let const_offset = 0x40;

        parser.type_cache.insert(
            enum_offset,
            TypeInfo::enum_type(
                "FaultId".to_string(),
                1,
                TypeEncoding::Unsigned,
                Vec::new(),
                enum_offset,
            ),
        );
        parser.type_cache.insert(
            inner_array_offset,
            TypeInfo::array_type(
                "array[6]".to_string(),
                6,
                TypeInfo::primitive("unknown".to_string(), 0, TypeEncoding::Unsigned),
                vec![6],
                inner_array_offset,
            ),
        );
        parser
            .array_elem_offsets
            .insert(inner_array_offset, enum_offset);

        parser.type_cache.insert(
            outer_array_offset,
            TypeInfo::array_type(
                "array[4]".to_string(),
                24,
                TypeInfo::primitive("unknown".to_string(), 0, TypeEncoding::Unsigned),
                vec![4],
                outer_array_offset,
            ),
        );
        parser
            .array_elem_offsets
            .insert(outer_array_offset, inner_array_offset);

        let mut const_type = TypeInfo::primitive("const".to_string(), 0, TypeEncoding::Unsigned);
        const_type.offset = const_offset;
        parser.type_cache.insert(const_offset, const_type);
        parser.type_refs.insert(const_offset, outer_array_offset);

        TypeResolver::resolve(
            &mut parser.type_cache,
            &parser.type_refs,
            &parser.array_elem_offsets,
        );

        let const_array = parser.type_cache.get(&const_offset).unwrap();
        let inner = const_array.pointer_target.as_deref().unwrap();
        assert_eq!(const_array.array_dims, vec![4]);
        assert_eq!(inner.array_dims, vec![6]);
        assert_eq!(
            inner.pointer_target.as_deref().unwrap().kind,
            TypeKind::Enum
        );
    }

    #[test]
    fn refreshes_array_element_when_member_signature_changes() {
        let mut parser = DwarfParser::new();
        let struct_offset = 0x10;
        let array_offset = 0x20;

        let stale_member =
            StructMember::new("OldStatus".to_string(), 0, "unsigned char".to_string(), 1);
        let fresh_member =
            StructMember::new("NewStatus".to_string(), 1, "unsigned char".to_string(), 1);

        parser.type_cache.insert(
            struct_offset,
            TypeInfo::struct_type(
                "StatusBits".to_string(),
                2,
                vec![fresh_member],
                struct_offset,
            ),
        );
        parser.type_cache.insert(
            array_offset,
            TypeInfo::array_type(
                "array[2]".to_string(),
                4,
                TypeInfo::struct_type(
                    "StatusBits".to_string(),
                    2,
                    vec![stale_member],
                    struct_offset,
                ),
                vec![2],
                array_offset,
            ),
        );
        parser
            .array_elem_offsets
            .insert(array_offset, struct_offset);

        TypeResolver::resolve(
            &mut parser.type_cache,
            &parser.type_refs,
            &parser.array_elem_offsets,
        );

        let resolved = parser.type_cache.get(&array_offset).unwrap();
        let elem = resolved.pointer_target.as_deref().unwrap();
        assert_eq!(elem.members[0].name, "NewStatus");
        assert_eq!(elem.members[0].offset, 1);
    }

    #[test]
    fn refreshes_array_element_when_enum_variant_signature_changes() {
        let mut parser = DwarfParser::new();
        let enum_offset = 0x10;
        let array_offset = 0x20;

        let stale_variant = EnumVariant::new("OldFault".to_string(), 1);
        let fresh_variant = EnumVariant::new("NewFault".to_string(), 2);

        parser.type_cache.insert(
            enum_offset,
            TypeInfo::enum_type(
                "FaultId".to_string(),
                1,
                TypeEncoding::Unsigned,
                vec![fresh_variant],
                enum_offset,
            ),
        );
        parser.type_cache.insert(
            array_offset,
            TypeInfo::array_type(
                "array[2]".to_string(),
                2,
                TypeInfo::enum_type(
                    "FaultId".to_string(),
                    1,
                    TypeEncoding::Unsigned,
                    vec![stale_variant],
                    enum_offset,
                ),
                vec![2],
                array_offset,
            ),
        );
        parser.array_elem_offsets.insert(array_offset, enum_offset);

        TypeResolver::resolve(
            &mut parser.type_cache,
            &parser.type_refs,
            &parser.array_elem_offsets,
        );

        let resolved = parser.type_cache.get(&array_offset).unwrap();
        let elem = resolved.pointer_target.as_deref().unwrap();
        assert_eq!(elem.variants[0].name, "NewFault");
        assert_eq!(elem.variants[0].value, 2);
    }

    #[test]
    fn resolver_normalizes_relative_bitfield_offsets() {
        let mut parser = DwarfParser::new();
        let struct_offset = 0x10;
        let member = StructMember::new("Status".to_string(), 1, "unsigned char".to_string(), 1)
            .with_bitfield(6, 2, false);

        parser.type_cache.insert(
            struct_offset,
            TypeInfo::struct_type("StatusBits".to_string(), 2, vec![member], struct_offset),
        );

        TypeResolver::resolve(
            &mut parser.type_cache,
            &parser.type_refs,
            &parser.array_elem_offsets,
        );

        let resolved = parser.type_cache.get(&struct_offset).unwrap();
        assert_eq!(resolved.members[0].bit_offset, Some(8));
        assert!(resolved.members[0].bit_offset_is_absolute);
    }
}
