use crate::types::{infer_a2l_type, A2lEntry, Endianness, Variable};
use anyhow::{Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::io::Write;

fn read_file_lossy(path: &std::path::Path) -> Result<String> {
    let bytes = std::fs::read(path).with_context(|| format!("无法读取文件: {}", path.display()))?;
    match String::from_utf8(bytes) {
        Ok(s) => Ok(s),
        Err(err) => {
            eprintln!(
                "警告: 文件包含非 UTF-8 字节，已使用容错解码: {}",
                path.display()
            );
            Ok(String::from_utf8_lossy(err.as_bytes()).into_owned())
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableChanges {
    pub name: Option<String>,
    pub address: Option<String>,
    pub data_type: Option<String>,
    pub var_type: Option<String>,
    pub bit_mask: Option<String>,
    pub compu_method: Option<String>,
    pub f: Option<f64>,
    pub offset: Option<f64>,
    pub unit: Option<String>,
    pub symbol_link: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableEdit {
    pub action: String,
    pub original_name: String,
    pub changes: Option<VariableChanges>,
    pub entry: Option<A2lEntryInfo>,
    pub export_mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2lEntryInfo {
    pub full_name: String,
    pub address: u64,
    pub size: usize,
    pub a2l_type: String,
    pub type_name: String,
    pub bit_offset: Option<usize>,
    pub bit_size: Option<usize>,
    pub symbol_link: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SaveResult {
    pub modified: usize,
    pub deleted: usize,
    pub added: usize,
    pub skipped: usize,
}

pub struct A2lGenerator {
    project_name: String,
    module_name: String,
    variables: Vec<Variable>,
    entries: Vec<A2lEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[allow(non_camel_case_types)]
#[serde(rename_all = "UPPERCASE")]
pub enum CompuMethodType {
    LINEAR,
    TAB_VERB,
    TAB_INTP,
    IDENTICAL,
}

impl std::fmt::Display for CompuMethodType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompuMethodType::LINEAR => write!(f, "LINEAR"),
            CompuMethodType::TAB_VERB => write!(f, "TAB_VERB"),
            CompuMethodType::TAB_INTP => write!(f, "TAB_INTP"),
            CompuMethodType::IDENTICAL => write!(f, "IDENTICAL"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabVerbPair {
    pub in_val: f64,
    pub verbal: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabIntpPair {
    pub in_val: f64,
    pub out_val: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompuMethod {
    pub name: String,
    pub conversion_type: CompuMethodType,
    pub unit: String,
    pub description: String,
    pub f: f64,
    pub offset: f64,
    pub verb_pairs: Vec<TabVerbPair>,
    pub default_value: String,
    pub intp_pairs: Vec<TabIntpPair>,
}

impl CompuMethod {
    pub fn linear(name: &str, f: f64, offset: f64, unit: &str) -> Self {
        Self {
            name: name.to_string(),
            conversion_type: CompuMethodType::LINEAR,
            unit: unit.to_string(),
            description: format!("y = {} * x + {}", f, offset),
            f,
            offset,
            verb_pairs: Vec::new(),
            default_value: String::new(),
            intp_pairs: Vec::new(),
        }
    }

    pub fn summary(&self) -> String {
        match self.conversion_type {
            CompuMethodType::LINEAR => format!("y = {} * x + {}", self.f, self.offset),
            CompuMethodType::TAB_VERB => format!("{} 项文字表", self.verb_pairs.len()),
            CompuMethodType::TAB_INTP => format!("{} 项插值表", self.intp_pairs.len()),
            CompuMethodType::IDENTICAL => "无转换".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppendResult {
    pub added: usize,
    pub skipped: usize,
    pub existing: usize,
}

pub enum ExportKind {
    Measurement,
    Characteristic,
}

#[derive(Debug, Clone)]
pub struct A2lVariable {
    pub name: String,
    pub address: Option<String>,
    pub var_type: String,
    pub data_type: String,
    pub bit_mask: Option<String>,
    pub compu_method: Option<String>,
    pub symbol_link: Option<String>,
    pub f: Option<f64>,
    pub offset: Option<f64>,
    pub unit: Option<String>,
}

impl A2lGenerator {
    pub fn new(project_name: &str, module_name: &str) -> Self {
        Self {
            project_name: project_name.to_string(),
            module_name: module_name.to_string(),
            variables: Vec::new(),
            entries: Vec::new(),
        }
    }

    pub fn add_variable(&mut self, variable: Variable) {
        self.variables.push(variable);
    }

    pub fn add_variables(&mut self, variables: &[Variable]) {
        self.variables.extend(variables.iter().cloned());
    }

    pub fn add_entry(&mut self, entry: A2lEntry) {
        self.entries.push(entry);
    }

    pub fn add_entries(&mut self, entries: &[A2lEntry]) {
        self.entries.extend(entries.iter().cloned());
    }

    pub fn variable_count(&self) -> usize {
        self.variables.len() + self.entries.len()
    }

    pub fn clear(&mut self) {
        self.variables.clear();
        self.entries.clear();
    }

    pub fn generate(&self) -> String {
        let mut output = String::new();

        output.push_str("/begin ASAP2_VERSION\n");
        output.push_str("  1 71\n");
        output.push_str("/end ASAP2_VERSION\n\n");

        output.push_str(&format!("/begin PROJECT {} \"\"\n", self.project_name));
        output.push_str(&format!("  /begin MODULE {} \"\"\n", self.module_name));

        output.push_str("    /begin CHARACTERISTIC __PLACEHOLDER__ \"\"\n");
        output.push_str("      VALUE 0x0 NO_COMPU_METHOD 0 0 0 0\n");
        output.push_str("    /end CHARACTERISTIC\n\n");

        output.push_str("    /begin COMPU_METHOD\n");
        output.push_str("      NO_COMPU_METHOD \"\" NO_COMPU_VTAB \"\" \"\" \"\"\n");
        output.push_str("    /end COMPU_METHOD\n\n");

        for var in &self.variables {
            output.push_str(&self.generate_measurement(var));
        }

        for entry in &self.entries {
            output.push_str(&Self::generate_measurement_block_with_compu(
                entry,
                None,
                Endianness::Little,
            ));
        }

        output.push_str("  /end MODULE\n");
        output.push_str("/end PROJECT\n");

        output
    }

    fn generate_measurement(&self, var: &Variable) -> String {
        let a2l_type = infer_a2l_type(var.size, &var.type_name);
        let format_str = Self::get_format_string(a2l_type);
        let (min_val, max_val) = Self::get_min_max(a2l_type);

        let mut output = String::new();

        output.push_str(&format!("    /begin MEASUREMENT {} \"\"\n", var.name));
        output.push_str(&format!(
            "      {} NO_COMPU_METHOD 0 0 {} {}\n",
            a2l_type, min_val, max_val
        ));
        output.push_str(&format!("      ECU_ADDRESS 0x{:08X}\n", var.address));
        output.push_str("      ECU_ADDRESS_EXTENSION 0x0\n");
        output.push_str(&format!("      FORMAT \"{}\"\n", format_str));
        output.push_str(&format!("      SYMBOL_LINK \"{}\" 0\n", var.name));
        output.push_str("    /end MEASUREMENT\n\n");

        output
    }

    pub fn generate_measurement_block_with_compu(
        entry: &A2lEntry,
        compu_method: Option<&str>,
        endianness: Endianness,
    ) -> String {
        let a2l_type = entry.a2l_type.as_str();
        let format_str = Self::get_format_string(a2l_type);

        let (min_val, max_val) = if entry.is_bitfield() {
            let size = entry.bit_size.unwrap();
            ("0".to_string(), format!("{}", Self::get_bitfield_max(size)))
        } else {
            let (min, max) = Self::get_min_max(a2l_type);
            (min.to_string(), max.to_string())
        };

        let compu = compu_method.unwrap_or("NO_COMPU_METHOD");
        let mut output = String::new();

        output.push_str(&format!(
            "    /begin MEASUREMENT {} \"\"\n",
            entry.full_name
        ));
        output.push_str(&format!(
            "      {} {} 0 0 {} {}\n",
            a2l_type, compu, min_val, max_val
        ));

        if entry.is_bitfield() {
            let effective_offset = entry.bit_offset.unwrap();
            let bit_size = entry.bit_size.unwrap();
            let mask =
                Self::calculate_bit_mask(effective_offset, bit_size, entry.size * 8, endianness);
            output.push_str(&format!("      BIT_MASK 0x{:X}\n", mask));
        }

        output.push_str(&format!("      ECU_ADDRESS 0x{:08X}\n", entry.address));
        output.push_str("      ECU_ADDRESS_EXTENSION 0x0\n");
        output.push_str(&format!("      FORMAT \"{}\"\n", format_str));
        if entry.is_bitfield() && entry.symbol_link_name.is_some() {
            let sym_name = entry.symbol_link_name.as_ref().unwrap();
            let sym_offset = entry.symbol_link_offset.unwrap_or(0);
            output.push_str(&format!(
                "      SYMBOL_LINK \"{}\" {}\n",
                sym_name, sym_offset
            ));
        } else {
            let link = &entry.full_name;
            output.push_str(&format!("      SYMBOL_LINK \"{}\" 0\n", link));
        }
        output.push_str("    /end MEASUREMENT\n\n");

        output
    }

    fn generate_characteristic_block_with_record_layout(
        entry: &A2lEntry,
        compu_method: Option<&str>,
        endianness: Endianness,
        record_layout: &str,
    ) -> String {
        let a2l_type = entry.a2l_type.as_str();

        let max_val = if entry.is_bitfield() {
            let size = entry.bit_size.unwrap();
            format!("{}", Self::get_bitfield_max(size))
        } else {
            let (_, max) = Self::get_min_max(a2l_type);
            max.to_string()
        };

        let compu = compu_method.unwrap_or("NO_COMPU_METHOD");
        let mut output = String::new();

        output.push_str(&format!(
            "    /begin CHARACTERISTIC {} \"\"\n",
            entry.full_name
        ));
        output.push_str(&format!(
            "      VALUE 0x{:08X} {} 0 {} 0 {}\n",
            entry.address, record_layout, compu, max_val
        ));

        if entry.is_bitfield() {
            let effective_offset = entry.bit_offset.unwrap();
            let bit_size = entry.bit_size.unwrap();
            let mask =
                Self::calculate_bit_mask(effective_offset, bit_size, entry.size * 8, endianness);
            output.push_str(&format!("      BIT_MASK 0x{:X}\n", mask));
        }

        output.push_str(&format!("      EXTENDED_LIMITS 0 {}\n", max_val));
        if entry.is_bitfield() && entry.symbol_link_name.is_some() {
            let sym_name = entry.symbol_link_name.as_ref().unwrap();
            let sym_offset = entry.symbol_link_offset.unwrap_or(0);
            output.push_str(&format!(
                "      SYMBOL_LINK \"{}\" {}\n",
                sym_name, sym_offset
            ));
        } else {
            output.push_str(&format!("      SYMBOL_LINK \"{}\" 0\n", entry.full_name));
        }
        output.push_str("    /end CHARACTERISTIC\n\n");

        output
    }

    pub fn generate_compu_method_name(f: f64, offset: f64) -> String {
        let format_f = if f == f.trunc() {
            format!("{}", f as i64)
        } else {
            format!("{}", f).replace('.', "_")
        };
        let format_offset = if offset == offset.trunc() {
            format!("{}", offset as i64)
        } else {
            format!("{}", offset).replace('.', "_")
        };
        let offset_sign = if offset < 0.0 { "N" } else { "O" };
        format!(
            "CM_F{}_{}{}",
            format_f,
            offset_sign,
            format_offset.replace('-', "")
        )
    }

    pub fn generate_compu_method_block(name: &str, f: f64, offset: f64, unit: &str) -> String {
        let display_unit = if unit.is_empty() { "" } else { unit };
        let format_str = Self::get_format_string_for_compu();
        let description = format!("y = {} * x + {}", f, offset);
        format!(
            "    /begin COMPU_METHOD\n      {} \"{}\"\n      LINEAR \"{}\" \"{}\" \"{}\"\n      COEFFS {} {} 0.0 0.0 0.0 0.0\n    /end COMPU_METHOD\n\n",
            name, description, format_str, display_unit, display_unit, f, offset
        )
    }

    fn get_format_string_for_compu() -> &'static str {
        "%10.4"
    }

    fn get_record_layout(a2l_type: &str) -> &'static str {
        match a2l_type {
            "UBYTE" => "__UByte_Value",
            "SBYTE" => "__SByte_Value",
            "UWORD" => "__UWord_Value",
            "SWORD" => "__SWord_Value",
            "ULONG" => "__ULong_Value",
            "SLONG" => "__SLong_Value",
            "A_UINT64" => "__UInt64_Value",
            "A_INT64" => "__Int64_Value",
            "FLOAT32_IEEE" => "__Float32_Value",
            "FLOAT64_IEEE" => "__Float64_Value",
            _ => "__ULong_Value",
        }
    }

    fn parse_record_layout_names(content: &str) -> HashSet<String> {
        let mut names = HashSet::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("/begin RECORD_LAYOUT") {
                if let Some(name) = rest.split_whitespace().next() {
                    names.insert(name.to_string());
                }
            }
        }
        names
    }

    fn get_record_layout_for_content(a2l_type: &str, layouts: &HashSet<String>) -> String {
        let candidates: &[&str] = match a2l_type {
            "UBYTE" => &["UByte", "__UBYTE_Z", "__UBYTE_S", "__UByte_Value"],
            "SBYTE" => &["SByte", "__SBYTE_Z", "__SBYTE_S", "__SByte_Value"],
            "UWORD" => &["UWord", "__UWORD_Z", "__UWORD_S", "__UWord_Value"],
            "SWORD" => &["SWord", "__SWORD_Z", "__SWORD_S", "__SWord_Value"],
            "ULONG" => &["ULong", "__ULONG_Z", "__ULONG_S", "__ULong_Value"],
            "SLONG" => &["SLong", "__SLONG_Z", "__SLONG_S", "__SLong_Value"],
            "A_UINT64" => &["__A_UINT64_Z", "__A_UINT64_S", "__UInt64_Value"],
            "A_INT64" => &["__A_INT64_Z", "__A_INT64_S", "__Int64_Value"],
            "FLOAT32_IEEE" => &["__FLOAT32_IEEE_Z", "__FLOAT32_IEEE_S", "__Float32_Value"],
            "FLOAT64_IEEE" => &["__FLOAT64_IEEE_Z", "__FLOAT64_IEEE_S", "__Float64_Value"],
            _ => &["__ULong_Value"],
        };

        candidates
            .iter()
            .find(|name| layouts.contains(**name))
            .copied()
            .unwrap_or_else(|| Self::get_record_layout(a2l_type))
            .to_string()
    }

    fn line_start_for_pos(content: &str, pos: usize) -> usize {
        let before = &content[..pos];
        if let Some(last_newline) = before.rfind('\n') {
            let line_start = last_newline + 1;
            let prefix = &content[line_start..pos];
            if prefix.chars().all(|c| c.is_whitespace()) {
                return line_start;
            }
        }
        pos
    }

    fn line_end_after_pos(content: &str, pos: usize) -> usize {
        content[pos..]
            .find('\n')
            .map(|offset| pos + offset + 1)
            .unwrap_or(content.len())
    }

    fn variable_insert_pos(content: &str) -> Result<usize> {
        let first_definition_pos = [
            "/begin COMPU_METHOD",
            "/begin COMPU_TAB",
            "/begin COMPU_VTAB",
            "/begin RECORD_LAYOUT",
            "/begin GROUP",
        ]
        .iter()
        .filter_map(|marker| content.find(marker))
        .min();

        let search_end = first_definition_pos.unwrap_or(content.len());
        let search_region = &content[..search_end];
        let last_variable_end = search_region
            .rfind("/end MEASUREMENT")
            .into_iter()
            .chain(search_region.rfind("/end CHARACTERISTIC"))
            .max();

        if let Some(pos) = last_variable_end {
            return Ok(Self::line_end_after_pos(content, pos));
        }

        if let Some(pos) = first_definition_pos {
            return Ok(Self::line_start_for_pos(content, pos));
        }

        content
            .rfind("/end MODULE")
            .map(|pos| Self::line_start_for_pos(content, pos))
            .with_context(|| "无法找到合适的插入位置")
    }

    fn get_format_string(a2l_type: &str) -> &'static str {
        match a2l_type {
            "UBYTE" | "SBYTE" => "%3.0",
            "UWORD" | "SWORD" => "%5.0",
            "ULONG" | "SLONG" => "%10.0",
            "A_UINT64" | "A_INT64" => "%20.0",
            "FLOAT32_IEEE" => "%10.4",
            "FLOAT64_IEEE" => "%16.8",
            _ => "%10.0",
        }
    }

    fn get_min_max(a2l_type: &str) -> (&'static str, &'static str) {
        match a2l_type {
            "UBYTE" => ("0", "255"),
            "SBYTE" => ("-128", "127"),
            "UWORD" => ("0", "65535"),
            "SWORD" => ("-32768", "32767"),
            "ULONG" => ("0", "4294967295"),
            "SLONG" => ("-2147483648", "2147483647"),
            "A_UINT64" => ("0", "18446744073709551615"),
            "A_INT64" => ("-9223372036854775808", "9223372036854775807"),
            "FLOAT32_IEEE" => ("-3.4E38", "3.4E38"),
            "FLOAT64_IEEE" => ("-1.7E308", "1.7E308"),
            _ => ("0", "0"),
        }
    }

    fn calculate_bit_mask(
        bit_offset: usize,
        bit_size: usize,
        container_size_bits: usize,
        endianness: Endianness,
    ) -> u64 {
        let shift = match endianness {
            Endianness::Little => bit_offset,
            Endianness::Big => container_size_bits.saturating_sub(bit_offset + bit_size),
        };
        ((1u64 << bit_size) - 1) << shift
    }

    fn get_bitfield_max(bit_size: usize) -> u64 {
        (1u64 << bit_size) - 1
    }

    pub fn save(&self, path: &std::path::Path) -> Result<()> {
        let content = self.generate();
        let mut file = std::fs::File::create(path)?;
        file.write_all(content.as_bytes())?;
        Ok(())
    }

    pub fn parse_existing_names(content: &str) -> HashSet<String> {
        let mut names = HashSet::new();
        let mut in_measurement = false;
        let mut in_characteristic = false;

        for line in content.lines() {
            let trimmed = line.trim();

            if trimmed.starts_with("/begin MEASUREMENT") {
                // 从同一行提取变量名，标准格式：/begin MEASUREMENT name ""
                if let Some(rest) = trimmed.strip_prefix("/begin MEASUREMENT") {
                    let name = rest.trim().split_whitespace().next().unwrap_or("");
                    if !name.is_empty() && !name.starts_with('"') {
                        names.insert(name.to_string());
                    }
                }
                in_measurement = true;
                continue;
            }

            if trimmed.starts_with("/begin CHARACTERISTIC") {
                // 从同一行提取变量名，标准格式：/begin CHARACTERISTIC name ""
                if let Some(rest) = trimmed.strip_prefix("/begin CHARACTERISTIC") {
                    let name = rest.trim().split_whitespace().next().unwrap_or("");
                    if !name.is_empty() && !name.starts_with('"') {
                        names.insert(name.to_string());
                    }
                }
                in_characteristic = true;
                continue;
            }

            if trimmed.starts_with("/end MEASUREMENT") {
                in_measurement = false;
                continue;
            }

            if trimmed.starts_with("/end CHARACTERISTIC") {
                in_characteristic = false;
                continue;
            }

            if in_measurement || in_characteristic {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if !parts.is_empty() {
                    let name = parts[0].trim();
                    if !name.is_empty()
                        && !name.starts_with('/')
                        && !name.starts_with('"')
                        && !name.parse::<f64>().is_ok()
                    {
                        names.insert(name.to_string());
                    }
                }
            }
        }

        names
    }

    pub fn append_to_file(
        entries: &[A2lEntry],
        path: &std::path::Path,
        kind: ExportKind,
        endianness: Endianness,
    ) -> Result<AppendResult> {
        let content = read_file_lossy(path)?;

        let existing_names = Self::parse_existing_names(&content);
        let record_layouts = Self::parse_record_layout_names(&content);

        let (to_add, to_skip): (Vec<_>, Vec<_>) = entries
            .iter()
            .partition(|e| !existing_names.contains(&e.full_name));

        let new_blocks: String = to_add
            .iter()
            .map(|e| match kind {
                ExportKind::Measurement => {
                    Self::generate_measurement_block_with_compu(e, None, endianness)
                }
                ExportKind::Characteristic => {
                    let record_layout =
                        Self::get_record_layout_for_content(&e.a2l_type, &record_layouts);
                    Self::generate_characteristic_block_with_record_layout(
                        e,
                        None,
                        endianness,
                        &record_layout,
                    )
                }
            })
            .collect();

        let actual_insert_pos = Self::variable_insert_pos(&content)?;

        let new_content = format!(
            "{}{}{}",
            &content[..actual_insert_pos],
            new_blocks,
            &content[actual_insert_pos..]
        );

        let mut file = std::fs::File::create(path)?;
        file.write_all(new_content.as_bytes())?;

        Ok(AppendResult {
            added: to_add.len(),
            skipped: to_skip.len(),
            existing: existing_names.len(),
        })
    }

    pub fn preview_append(entries: &[A2lEntry], path: &std::path::Path) -> Result<AppendResult> {
        let content = read_file_lossy(path)?;

        let existing_names = Self::parse_existing_names(&content);

        let to_add: Vec<_> = entries
            .iter()
            .filter(|e| !existing_names.contains(&e.full_name))
            .collect();

        let to_skip = entries.len() - to_add.len();

        Ok(AppendResult {
            added: to_add.len(),
            skipped: to_skip,
            existing: existing_names.len(),
        })
    }

    /// 从 A2L 内容中删除指定的变量块
    pub fn remove_variables(content: &str, names: &[String]) -> Result<String> {
        use std::collections::HashSet;

        let names_set: HashSet<&str> = names.iter().map(|s| s.as_str()).collect();
        let mut result = String::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let trimmed = lines[i].trim();

            if trimmed.starts_with("/begin MEASUREMENT")
                || trimmed.starts_with("/begin CHARACTERISTIC")
            {
                let block_start = i;
                let is_measurement = trimmed.contains("MEASUREMENT");
                let end_marker = if is_measurement {
                    "/end MEASUREMENT"
                } else {
                    "/end CHARACTERISTIC"
                };

                // 找块结束
                let mut block_end = i;
                for j in i..lines.len() {
                    if lines[j].trim().starts_with(end_marker) {
                        block_end = j;
                        break;
                    }
                }

                // 获取变量名（从 /begin 行提取，标准格式：/begin MEASUREMENT name ""）
                let begin_prefix = if is_measurement {
                    "/begin MEASUREMENT"
                } else {
                    "/begin CHARACTERISTIC"
                };
                let mut var_name = "";
                if let Some(rest) = trimmed.strip_prefix(begin_prefix) {
                    var_name = rest.trim().split_whitespace().next().unwrap_or("");
                }

                if names_set.contains(var_name) {
                    i = block_end + 1;
                    while i < lines.len() && lines[i].trim().is_empty() {
                        i += 1;
                    }
                    continue;
                } else {
                    // 保留这个块
                    for j in block_start..=block_end {
                        result.push_str(lines[j]);
                        result.push('\n');
                    }
                    i = block_end + 1;
                }
            } else {
                result.push_str(lines[i]);
                result.push('\n');
                i += 1;
            }
        }

        Ok(result)
    }

    /// 修改指定变量的属性
    pub fn modify_variable(
        content: &str,
        original_name: &str,
        changes: &VariableChanges,
    ) -> Result<String> {
        let lines: Vec<&str> = content.lines().collect();
        let mut result = String::new();
        let mut i = 0;

        while i < lines.len() {
            let trimmed = lines[i].trim();

            if trimmed.starts_with("/begin MEASUREMENT")
                || trimmed.starts_with("/begin CHARACTERISTIC")
            {
                let block_start = i;
                let is_measurement = trimmed.contains("MEASUREMENT");
                let end_marker = if is_measurement {
                    "/end MEASUREMENT"
                } else {
                    "/end CHARACTERISTIC"
                };

                let mut block_end = i;
                for j in i..lines.len() {
                    if lines[j].trim().starts_with(end_marker) {
                        block_end = j;
                        break;
                    }
                }

                let begin_prefix = if is_measurement {
                    "/begin MEASUREMENT"
                } else {
                    "/begin CHARACTERISTIC"
                };
                let mut current_var_name = "";
                if let Some(rest) = trimmed.strip_prefix(begin_prefix) {
                    current_var_name = rest.trim().split_whitespace().next().unwrap_or("");
                }

                if current_var_name == original_name {
                    let modified_block = Self::apply_changes_to_block(
                        &lines[block_start..=block_end],
                        changes,
                        is_measurement,
                    )?;
                    result.push_str(&modified_block);
                    i = block_end + 1;
                    continue;
                } else {
                    for j in block_start..=block_end {
                        result.push_str(lines[j]);
                        result.push('\n');
                    }
                    i = block_end + 1;
                }
            } else {
                result.push_str(lines[i]);
                result.push('\n');
                i += 1;
            }
        }

        Ok(result)
    }

    fn apply_changes_to_block(
        block_lines: &[&str],
        changes: &VariableChanges,
        _is_measurement: bool,
    ) -> Result<String> {
        let new_name = changes.name.as_deref().unwrap_or("");
        let new_address = changes.address.as_deref().unwrap_or("");
        let new_data_type = changes.data_type.as_deref().unwrap_or("");
        let new_bit_mask = changes.bit_mask.as_deref().unwrap_or("");
        let new_compu_method = changes.compu_method.as_deref().unwrap_or("");
        let new_symbol_link = changes.symbol_link.as_deref().unwrap_or("");

        let mut original_name = String::new();
        let mut original_address = String::new();
        let mut original_data_type = String::new();
        let mut original_compu_method = String::new();
        let mut original_symbol_link = String::new();
        let mut has_bit_mask = false;
        let mut bit_mask_indent = 0;
        let mut ecu_address_indent = 0;

        let a2l_types = [
            "UBYTE",
            "SBYTE",
            "UWORD",
            "SWORD",
            "ULONG",
            "SLONG",
            "A_UINT64",
            "A_INT64",
            "FLOAT32_IEEE",
            "FLOAT64_IEEE",
        ];

        for line in block_lines {
            let trimmed = line.trim();
            let indent = line.len() - line.trim_start().len();

            if trimmed.starts_with("/begin MEASUREMENT ")
                || trimmed.starts_with("/begin CHARACTERISTIC ")
            {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 3 {
                    original_name = parts[2].to_string();
                }
            }
            if let Some(addr_pos) = trimmed
                .split_whitespace()
                .collect::<Vec<_>>()
                .iter()
                .position(|&x| x == "ECU_ADDRESS")
            {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if addr_pos + 1 < parts.len() {
                    original_address = parts[addr_pos + 1].to_string();
                    ecu_address_indent = indent;
                }
            }
            if trimmed.starts_with("BIT_MASK") {
                has_bit_mask = true;
                bit_mask_indent = indent;
            }
            if trimmed.starts_with("SYMBOL_LINK") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 2 {
                    original_symbol_link = parts[1].trim_matches('"').to_string();
                }
            }
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if !parts.is_empty() && a2l_types.contains(&parts[0]) {
                original_data_type = parts[0].to_string();
                if parts.len() >= 2 {
                    original_compu_method = parts[1].to_string();
                }
            }
            if trimmed.starts_with("VALUE") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 5 {
                    original_compu_method = parts[3].to_string();
                }
            }
        }

        let final_name = if !new_name.is_empty() {
            new_name
        } else {
            &original_name
        };
        let final_address = if !new_address.is_empty() {
            new_address
        } else {
            &original_address
        };
        let final_data_type = if !new_data_type.is_empty() {
            new_data_type
        } else {
            &original_data_type
        };
        let final_compu_method = if !new_compu_method.is_empty() {
            new_compu_method
        } else {
            &original_compu_method
        };
        let final_symbol_link = if !new_symbol_link.is_empty() {
            new_symbol_link
        } else if !original_symbol_link.is_empty() {
            &original_symbol_link
        } else {
            &original_name
        };

        let begin_re = Regex::new(r"^(\s*/begin\s+(?:MEASUREMENT|CHARACTERISTIC)\s+)\S+")?;
        let ecu_addr_re = Regex::new(r"^(\s*ECU_ADDRESS\s+)(0x[0-9a-fA-F]+)")?;
        let value_addr_re = Regex::new(r"^(\s*VALUE\s+)(0x[0-9a-fA-F]+)(\s+.*)$")?;
        let link_map_addr_re = Regex::new(r#"^(.*LINK_MAP\s+"[^"]+"\s+)(0x[0-9a-fA-F]+)(.*)$"#)?;
        let data_type_re = Regex::new(&format!(
            "^(\\s*)({})(\\s+)(\\S+)(\\s+.*)$",
            original_data_type
        ))?;
        let bit_mask_re = Regex::new(r"^(\s*BIT_MASK\s+)(0x[0-9a-fA-F]+)")?;
        let symbol_link_re = Regex::new(r#"^(\s*SYMBOL_LINK\s+")([^"]+)("\s+\d+\s*)$"#)?;

        let mut result = String::new();
        let mut bit_mask_inserted = false;

        for line in block_lines {
            if !new_name.is_empty() {
                if let Some(caps) = begin_re.captures(line) {
                    let mut modified_line = format!("{}{}", &caps[1], final_name);
                    if let Some(rest_start) =
                        line.find(&original_name).map(|p| p + original_name.len())
                    {
                        if rest_start < line.len() {
                            modified_line.push_str(&line[rest_start..]);
                        }
                    }
                    modified_line.push('\n');
                    result.push_str(&modified_line);
                    continue;
                }
            }

            if !new_bit_mask.is_empty() && !has_bit_mask && !bit_mask_inserted {
                if ecu_addr_re.is_match(line) {
                    let indent = if bit_mask_indent > 0 {
                        bit_mask_indent
                    } else {
                        line.len() - line.trim_start().len()
                    };
                    result.push_str(&format!(
                        "{}BIT_MASK {}\n",
                        " ".repeat(indent),
                        new_bit_mask
                    ));
                    bit_mask_inserted = true;
                }
            }

            if !new_address.is_empty() {
                if let Some(caps) = ecu_addr_re.captures(line) {
                    result.push_str(&format!("{}{}\n", &caps[1], final_address));
                    continue;
                }

                if let Some(caps) = value_addr_re.captures(line) {
                    result.push_str(&format!("{}{}{}\n", &caps[1], final_address, &caps[3]));
                    continue;
                }

                if let Some(caps) = link_map_addr_re.captures(line) {
                    result.push_str(&format!("{}{}{}\n", &caps[1], final_address, &caps[3]));
                    continue;
                }
            }

            if !new_data_type.is_empty() && !original_data_type.is_empty() {
                if let Some(caps) = data_type_re.captures(line) {
                    result.push_str(&format!(
                        "{}{}{}{}{}\n",
                        &caps[1], final_data_type, &caps[3], final_compu_method, &caps[5]
                    ));
                    continue;
                }
            }

            if !new_compu_method.is_empty()
                && !original_data_type.is_empty()
                && new_data_type.is_empty()
            {
                if let Some(caps) = data_type_re.captures(line) {
                    result.push_str(&format!(
                        "{}{}{}{}{}\n",
                        &caps[1], &caps[2], &caps[3], final_compu_method, &caps[5]
                    ));
                    continue;
                }
            }

            if !new_bit_mask.is_empty() && has_bit_mask {
                if let Some(caps) = bit_mask_re.captures(line) {
                    result.push_str(&format!("{}{}\n", &caps[1], new_bit_mask));
                    continue;
                }
            }

            if !new_symbol_link.is_empty() || !new_name.is_empty() {
                if let Some(caps) = symbol_link_re.captures(line) {
                    result.push_str(&format!(
                        "{}{}{}{}\n",
                        &caps[1], final_symbol_link, &caps[3], ""
                    ));
                    continue;
                }
            }

            result.push_str(line);
            result.push('\n');
        }

        if !new_bit_mask.is_empty() && !has_bit_mask && !bit_mask_inserted {
            let indent = if ecu_address_indent > 0 {
                ecu_address_indent
            } else {
                4
            };
            result.push_str(&format!(
                "{}BIT_MASK {}\n",
                " ".repeat(indent),
                new_bit_mask
            ));
        }

        Ok(result)
    }

    pub fn apply_changes(
        content: &str,
        edits: &[VariableEdit],
        endianness: Endianness,
    ) -> Result<(String, SaveResult)> {
        use std::collections::HashMap;

        let mut result = content.to_string();
        let mut save_result = SaveResult {
            modified: 0,
            deleted: 0,
            added: 0,
            skipped: 0,
        };

        let existing_names = Self::parse_existing_names(content);
        let record_layouts = Self::parse_record_layout_names(content);

        let existing_compu_methods = A2lParser::parse_compu_methods(content);
        let mut compu_method_map: HashMap<String, String> = existing_compu_methods
            .iter()
            .map(|m| {
                let key = format!("{:.10}_{:.10}", m.f, m.offset);
                (key, m.name.clone())
            })
            .collect();

        let mut new_compu_methods_to_add: Vec<(String, f64, f64, String)> = Vec::new();

        let mut modified_edits: Vec<VariableEdit> = Vec::new();
        for edit in edits {
            let mut modified_edit = edit.clone();
            if let Some(ref changes) = edit.changes {
                if let (Some(f), Some(offset)) = (changes.f, changes.offset) {
                    let key = format!("{:.10}_{:.10}", f, offset);
                    let compu_name = if let Some(existing_name) = compu_method_map.get(&key) {
                        existing_name.clone()
                    } else {
                        let new_name = Self::generate_compu_method_name(f, offset);
                        compu_method_map.insert(key, new_name.clone());
                        let unit = changes.unit.clone().unwrap_or_default();
                        new_compu_methods_to_add.push((new_name.clone(), f, offset, unit));
                        new_name
                    };
                    modified_edit.changes = Some(VariableChanges {
                        compu_method: Some(compu_name),
                        ..changes.clone()
                    });
                }
            }
            modified_edits.push(modified_edit);
        }

        for (name, f, offset, unit) in &new_compu_methods_to_add {
            let block = Self::generate_compu_method_block(name, *f, *offset, unit);
            let insert_pos = result
                .rfind("/end COMPU_METHOD")
                .or_else(|| result.find("/begin MEASUREMENT"))
                .or_else(|| result.find("/begin CHARACTERISTIC"))
                .or_else(|| result.rfind("/end MODULE"))
                .unwrap_or(result.len());

            let actual_insert_pos = {
                let before = &result[..insert_pos];
                if let Some(last_newline) = before.rfind('\n') {
                    let line_start = last_newline + 1;
                    let prefix = &result[line_start..insert_pos];
                    if prefix.chars().all(|c| c.is_whitespace()) {
                        line_start
                    } else {
                        insert_pos
                    }
                } else {
                    0
                }
            };

            result = format!(
                "{}{}{}",
                &result[..actual_insert_pos],
                block,
                &result[actual_insert_pos..]
            );
        }

        for edit in &modified_edits {
            match edit.action.as_str() {
                "modify" => {
                    if let Some(ref changes) = edit.changes {
                        result = Self::modify_variable(&result, &edit.original_name, changes)?;
                        save_result.modified += 1;
                    }
                }
                "delete" => {
                    result = Self::remove_variables(&result, &[edit.original_name.clone()])?;
                    save_result.deleted += 1;
                }
                "add" => {
                    if let Some(ref entry_info) = edit.entry {
                        if existing_names.contains(&entry_info.full_name) {
                            save_result.skipped += 1;
                        } else {
                            let entry = A2lEntry {
                                full_name: entry_info.full_name.clone(),
                                address: entry_info.address,
                                size: entry_info.size,
                                a2l_type: entry_info.a2l_type.clone(),
                                type_name: entry_info.type_name.clone(),
                                bit_offset: entry_info.bit_offset,
                                bit_size: entry_info.bit_size,
                                bit_offset_is_absolute: false,
                                array_index: None,
                                symbol_link_name: entry_info.symbol_link.clone(),
                                symbol_link_offset: entry_info.symbol_link.as_ref().and_then(|s| {
                                    s.split_whitespace()
                                        .last()
                                        .and_then(|n| n.parse::<u64>().ok())
                                }),
                            };
                            let kind = match edit.export_mode.as_deref() {
                                Some("characteristic") => ExportKind::Characteristic,
                                _ => ExportKind::Measurement,
                            };
                            let compu_method = edit
                                .changes
                                .as_ref()
                                .and_then(|c| c.compu_method.as_deref());
                            let block = match kind {
                                ExportKind::Measurement => {
                                    Self::generate_measurement_block_with_compu(
                                        &entry,
                                        compu_method,
                                        endianness,
                                    )
                                }
                                ExportKind::Characteristic => {
                                    let record_layout = Self::get_record_layout_for_content(
                                        &entry.a2l_type,
                                        &record_layouts,
                                    );
                                    Self::generate_characteristic_block_with_record_layout(
                                        &entry,
                                        compu_method,
                                        endianness,
                                        &record_layout,
                                    )
                                }
                            };
                            let actual_insert_pos = Self::variable_insert_pos(&result)?;

                            result = format!(
                                "{}{}{}",
                                &result[..actual_insert_pos],
                                block,
                                &result[actual_insert_pos..]
                            );
                            save_result.added += 1;
                        }
                    }
                }
                _ => {}
            }
        }

        Ok((result, save_result))
    }
}

pub struct A2lParser;

impl A2lParser {
    pub fn parse_measurement_names(content: &str) -> Vec<String> {
        let mut names = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("/begin MEASUREMENT") {
                if let Some(next_line) = content.lines().skip_while(|l| l.trim() != trimmed).nth(1)
                {
                    let parts: Vec<&str> = next_line.trim().split_whitespace().collect();
                    if !parts.is_empty() {
                        names.push(parts[0].to_string());
                    }
                }
            }
        }

        names
    }

    /// 解析 A2L 文件中所有 MEASUREMENT 和 CHARACTERISTIC 变量
    pub fn parse_all_variables(content: &str) -> Vec<A2lVariable> {
        let mut variables = Vec::new();
        let mut in_measurement = false;
        let mut in_characteristic = false;
        let mut current_block_lines = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim();

            if trimmed.starts_with("/begin MEASUREMENT") {
                in_measurement = true;
                in_characteristic = false;
                current_block_lines.clear();
                current_block_lines.push(trimmed);
                continue;
            }

            if trimmed.starts_with("/begin CHARACTERISTIC") {
                in_characteristic = true;
                in_measurement = false;
                current_block_lines.clear();
                current_block_lines.push(trimmed);
                continue;
            }

            if trimmed.starts_with("/end MEASUREMENT") {
                if in_measurement {
                    variables.push(Self::parse_variable_block(
                        &current_block_lines,
                        "MEASUREMENT",
                    ));
                }
                in_measurement = false;
                current_block_lines.clear();
                continue;
            }

            if trimmed.starts_with("/end CHARACTERISTIC") {
                if in_characteristic {
                    variables.push(Self::parse_variable_block(
                        &current_block_lines,
                        "CHARACTERISTIC",
                    ));
                }
                in_characteristic = false;
                current_block_lines.clear();
                continue;
            }

            if in_measurement || in_characteristic {
                current_block_lines.push(trimmed);
            }
        }

        variables
    }

    fn parse_variable_block(block_lines: &[&str], block_type: &str) -> A2lVariable {
        let mut name = String::new();
        let mut address = None;
        let mut data_type = String::new();
        let mut bit_mask = None;
        let mut compu_method = None;
        let mut symbol_link = None;
        let mut found_first_name = false;

        let a2l_types = [
            "UBYTE",
            "SBYTE",
            "UWORD",
            "SWORD",
            "ULONG",
            "SLONG",
            "A_UINT64",
            "A_INT64",
            "FLOAT32_IEEE",
            "FLOAT64_IEEE",
            "FLOAT16",
            "FLOAT64",
            "UFIX16",
            "UFIX32",
            "SFIX16",
            "SFIX32",
        ];

        for line in block_lines {
            let trimmed = line.trim();

            if trimmed.is_empty() {
                continue;
            }

            if trimmed.starts_with("/begin MEASUREMENT ")
                || trimmed.starts_with("/begin CHARACTERISTIC ")
            {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 3 {
                    let candidate = parts[2];
                    if !candidate.parse::<f64>().is_ok() && !a2l_types.contains(&candidate) {
                        name = candidate.to_string();
                        found_first_name = true;
                    }
                }
                continue;
            }

            if trimmed.starts_with('/') {
                continue;
            }

            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            if !found_first_name {
                if !parts[0].parse::<f64>().is_ok() && !a2l_types.contains(&parts[0]) {
                    name = parts[0].to_string();
                    found_first_name = true;
                }
            }

            if parts.len() >= 2 {
                let possible_type = parts[0];
                if a2l_types.contains(&possible_type) {
                    data_type = possible_type.to_string();
                    if parts.len() >= 3 && parts[1] != "0" && !parts[1].starts_with("0x") {
                        compu_method = Some(parts[1].to_string());
                    }
                }
            }

            if let Some(addr_pos) = parts.iter().position(|&x| x == "ECU_ADDRESS") {
                if addr_pos + 1 < parts.len() {
                    address = Some(parts[addr_pos + 1].to_string());
                }
            }

            if block_type == "CHARACTERISTIC" {
                if let Some(value_pos) = parts.iter().position(|&x| x == "VALUE") {
                    if value_pos + 1 < parts.len() {
                        address = Some(parts[value_pos + 1].to_string());
                    }
                    if value_pos + 4 < parts.len() {
                        compu_method = Some(parts[value_pos + 3].to_string());
                    }
                }
            }

            if let Some(mask_pos) = parts.iter().position(|&x| x == "BIT_MASK") {
                if mask_pos + 1 < parts.len() {
                    bit_mask = Some(parts[mask_pos + 1].to_string());
                }
            }

            if let Some(link_pos) = parts.iter().position(|&x| x == "SYMBOL_LINK") {
                if link_pos + 1 < parts.len() {
                    symbol_link = Some(parts[link_pos + 1].trim_matches('"').to_string());
                }
            }
        }

        let (f, offset, unit) = if let Some(ref cm) = compu_method {
            if cm != "NO_COMPU_METHOD" {
                Self::parse_compu_method_coeffs(cm)
            } else {
                (None, None, None)
            }
        } else {
            (None, None, None)
        };

        A2lVariable {
            name,
            address,
            var_type: block_type.to_string(),
            data_type,
            bit_mask,
            compu_method,
            symbol_link,
            f,
            offset,
            unit,
        }
    }

    pub fn parse_compu_methods(content: &str) -> Vec<CompuMethod> {
        let mut methods = Vec::new();
        let mut in_compu_method = false;
        let mut current_block_lines = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim();

            if trimmed.starts_with("/begin COMPU_METHOD") {
                in_compu_method = true;
                current_block_lines.clear();
                current_block_lines.push(trimmed);
                continue;
            }

            if trimmed.starts_with("/end COMPU_METHOD") {
                if in_compu_method && !current_block_lines.is_empty() {
                    if let Some(method) = Self::parse_compu_method_block(&current_block_lines) {
                        methods.push(method);
                    }
                }
                in_compu_method = false;
                current_block_lines.clear();
                continue;
            }

            if in_compu_method {
                current_block_lines.push(trimmed);
            }
        }

        methods
    }

    fn parse_compu_method_block(block_lines: &[&str]) -> Option<CompuMethod> {
        let mut name = String::new();
        let mut description = String::new();
        let mut conversion_type_str = String::new();
        let mut unit = String::new();
        let mut f = 1.0;
        let mut offset = 0.0;
        let mut verb_pairs: Vec<TabVerbPair> = Vec::new();
        let mut default_value = String::new();
        let mut intp_pairs: Vec<TabIntpPair> = Vec::new();

        let mut full_block = block_lines.join("\n");

        for line in block_lines {
            let trimmed = line.trim();

            if trimmed.starts_with("/begin COMPU_METHOD") {
                let parts: Vec<&str> = trimmed.splitn(4, ' ').collect();
                if parts.len() >= 3 {
                    name = parts[2].to_string();
                }
                if trimmed.starts_with('"') {
                    if let Some(start) = trimmed.find('"') {
                        if let Some(end) = trimmed.rfind('"') {
                            if end > start {
                                description = trimmed[start + 1..end].to_string();
                            }
                        }
                    }
                }
                continue;
            }

            if trimmed.starts_with("COEFFS") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 3 {
                    f = parts[1].parse().unwrap_or(1.0);
                    offset = parts[2].parse().unwrap_or(0.0);
                }
                continue;
            }

            if trimmed.starts_with("LINEAR")
                || trimmed.starts_with("TAB_VERB")
                || trimmed.starts_with("TAB_INTP")
                || trimmed.starts_with("IDENTICAL")
            {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                conversion_type_str = parts[0].to_string();
                if parts.len() >= 4 {
                    unit = parts[3].trim_matches('"').to_string();
                }
                continue;
            }

            if trimmed.starts_with("DEFAULT_VALUE") {
                let parts: Vec<&str> = trimmed.splitn(2, ' ').collect();
                if parts.len() >= 2 {
                    default_value = parts[1].trim().trim_matches('"').to_string();
                }
                continue;
            }

            if trimmed.starts_with("COMPU_VTAB") {
                let rest = trimmed.trim_start_matches("COMPU_VTAB");
                verb_pairs = Self::parse_verb_pairs(rest);
                continue;
            }

            if trimmed.starts_with("COMPU_TAB") {
                let rest = trimmed.trim_start_matches("COMPU_TAB");
                let pairs = Self::parse_intp_pairs_from_block(rest);
                intp_pairs = pairs;
                continue;
            }
        }

        if name.is_empty() || name == "NO_COMPU_METHOD" {
            return None;
        }

        let conversion_type = match conversion_type_str.as_str() {
            "TAB_VERB" => CompuMethodType::TAB_VERB,
            "TAB_INTP" => CompuMethodType::TAB_INTP,
            "IDENTICAL" => CompuMethodType::IDENTICAL,
            _ => CompuMethodType::LINEAR,
        };

        if description.is_empty() {
            match conversion_type {
                CompuMethodType::LINEAR => {
                    description = format!("y = {} * x + {}", f, offset);
                }
                _ => {}
            }
        }

        let _ = &mut full_block;

        Some(CompuMethod {
            name,
            conversion_type,
            unit,
            description,
            f,
            offset,
            verb_pairs,
            default_value,
            intp_pairs,
        })
    }

    fn parse_verb_pairs(rest: &str) -> Vec<TabVerbPair> {
        let mut pairs = Vec::new();
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        let mut i = 0;
        while i + 1 < tokens.len() {
            if let Ok(in_val) = tokens[i].parse::<f64>() {
                let verbal = tokens[i + 1].trim_matches('"').to_string();
                pairs.push(TabVerbPair { in_val, verbal });
                i += 2;
            } else {
                i += 1;
            }
        }
        pairs
    }

    fn parse_intp_pairs_from_block(rest: &str) -> Vec<TabIntpPair> {
        let mut pairs = Vec::new();
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        let mut i = 0;
        if tokens.len() >= 2 {
            if let Ok(num_pairs) = tokens[0].parse::<usize>() {
                let _ = tokens[1].parse::<usize>();
                i = 2;
                while i + 1 < tokens.len() && pairs.len() < num_pairs {
                    if let Ok(in_val) = tokens[i].parse::<f64>() {
                        if let Ok(out_val) = tokens[i + 1].parse::<f64>() {
                            pairs.push(TabIntpPair { in_val, out_val });
                            i += 2;
                            continue;
                        }
                    }
                    i += 1;
                }
            }
        }
        while i + 1 < tokens.len() {
            if let Ok(in_val) = tokens[i].parse::<f64>() {
                if let Ok(out_val) = tokens[i + 1].parse::<f64>() {
                    pairs.push(TabIntpPair { in_val, out_val });
                    i += 2;
                    continue;
                }
            }
            i += 1;
        }
        pairs
    }

    fn parse_compu_method_coeffs(method_name: &str) -> (Option<f64>, Option<f64>, Option<String>) {
        if method_name == "NO_COMPU_METHOD" || !method_name.starts_with("CM_F") {
            return (None, None, None);
        }

        let rest = method_name.strip_prefix("CM_F").unwrap_or("");
        let parts: Vec<&str> = rest.split('_').collect();

        if parts.len() >= 2 {
            let f_str = parts[0].replace('_', ".");
            let f = f_str.parse::<f64>().ok();

            let offset_part = parts[1..].join("_");
            let offset_str = if offset_part.starts_with('N') {
                format!("-{}", offset_part[1..].replace('_', "."))
            } else if offset_part.starts_with('O') {
                offset_part[1..].replace('_', ".")
            } else {
                offset_part.replace('_', ".")
            };
            let offset = offset_str.parse::<f64>().ok();

            (f, offset, None)
        } else {
            (None, None, None)
        }
    }

    pub fn generate_compu_method_block_generic(method: &CompuMethod) -> String {
        let display_unit = if method.unit.is_empty() { "" } else { method.unit.as_str() };
        let format_str = A2lGenerator::get_format_string_for_compu();
        let desc = if method.description.is_empty() {
            method.summary()
        } else {
            method.description.clone()
        };

        match method.conversion_type {
            CompuMethodType::LINEAR => {
                format!(
                    "    /begin COMPU_METHOD\n      {} \"{}\"\n      LINEAR \"{}\" \"{}\" \"{}\"\n      COEFFS {} {} 0.0 0.0 0.0 0.0\n    /end COMPU_METHOD\n\n",
                    method.name, desc, format_str, display_unit, display_unit, method.f, method.offset
                )
            }
            CompuMethodType::IDENTICAL => {
                format!(
                    "    /begin COMPU_METHOD\n      {} \"{}\"\n      IDENTICAL \"\" \"{}\" \"{}\"\n    /end COMPU_METHOD\n\n",
                    method.name, desc, display_unit, display_unit
                )
            }
            CompuMethodType::TAB_VERB => {
                let mut vtab_line = String::new();
                for pair in &method.verb_pairs {
                    let val = if pair.in_val == pair.in_val.trunc() {
                        pair.in_val as i64
                    } else {
                        pair.in_val as f64 as i64
                    };
                    vtab_line.push_str(&format!(" {} \"{}\"", val, pair.verbal));
                }
                let default_line = if method.default_value.is_empty() {
                    String::new()
                } else {
                    format!("\n      DEFAULT_VALUE \"{}\"", method.default_value)
                };
                format!(
                    "    /begin COMPU_METHOD\n      {} \"{}\"\n      TAB_VERB \"\" \"{}\" \"{}\"{}\n      COMPU_VTAB{}\n    /end COMPU_METHOD\n\n",
                    method.name, desc, display_unit, display_unit, default_line, vtab_line
                )
            }
            CompuMethodType::TAB_INTP => {
                let count = method.intp_pairs.len();
                let mut tab_lines = String::new();
                for pair in &method.intp_pairs {
                    tab_lines.push_str(&format!("\n      {} {}", pair.in_val, pair.out_val));
                }
                format!(
                    "    /begin COMPU_METHOD\n      {} \"{}\"\n      TAB_INTP \"{}\" \"{}\" \"{}\"\n      COMPU_TAB {} 2{}\n    /end COMPU_METHOD\n\n",
                    method.name, desc, format_str, display_unit, display_unit, count, tab_lines
                )
            }
        }
    }

    pub fn save_compu_method(content: &str, method: &CompuMethod) -> Result<String> {
        let block = Self::generate_compu_method_block_generic(method);
        let begin_marker = format!("/begin COMPU_METHOD");
        let end_marker = "/end COMPU_METHOD";

        let mut find_result: Option<(usize, usize)> = None;
        let mut search_start = 0;
        loop {
            if let Some(begin_pos) = content[search_start..].find(begin_marker.as_str()) {
                let abs_begin = search_start + begin_pos;
                if let Some(rel_end) = content[abs_begin..].find(end_marker) {
                    let abs_end = abs_begin + rel_end + end_marker.len();
                    let block_text = &content[abs_begin..abs_end];
                    let tokens: Vec<&str> = block_text.split_whitespace().collect();
                    if tokens.len() >= 3 && tokens[2] == method.name {
                        let line_start = content[..abs_begin].rfind('\n').map(|p| p + 1).unwrap_or(0);
                        let line_end = content[abs_end..].find('\n').map(|p| abs_end + p + 1).unwrap_or(content.len());
                        find_result = Some((line_start, line_end));
                        break;
                    }
                    search_start = abs_end;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        if let Some((start, end)) = find_result {
            let mut result = String::new();
            result.push_str(&content[..start]);
            result.push_str(&block);
            result.push_str(&content[end..]);
            Ok(result)
        } else {
            let insert_pos = content
                .rfind("/end COMPU_METHOD")
                .or_else(|| content.find("/begin MEASUREMENT"))
                .or_else(|| content.find("/begin CHARACTERISTIC"))
                .or_else(|| content.rfind("/end MODULE"))
                .unwrap_or(content.len());

            let actual_insert_pos = {
                let before = &content[..insert_pos];
                if let Some(last_newline) = before.rfind('\n') {
                    let line_start = last_newline + 1;
                    let prefix = &content[line_start..insert_pos];
                    if prefix.chars().all(|c| c.is_whitespace()) {
                        line_start
                    } else {
                        insert_pos
                    }
                } else {
                    0
                }
            };

            let mut result = String::new();
            result.push_str(&content[..actual_insert_pos]);
            result.push_str(&block);
            result.push_str(&content[actual_insert_pos..]);
            Ok(result)
        }
    }

    pub fn delete_compu_method(content: &str, name: &str) -> Result<String> {
        let begin_marker = "/begin COMPU_METHOD";
        let end_marker = "/end COMPU_METHOD";

        let mut find_result: Option<(usize, usize)> = None;
        let mut search_start = 0;
        loop {
            if let Some(begin_pos) = content[search_start..].find(begin_marker) {
                let abs_begin = search_start + begin_pos;
                if let Some(rel_end) = content[abs_begin..].find(end_marker) {
                    let abs_end = abs_begin + rel_end + end_marker.len();
                    let block_text = &content[abs_begin..abs_end];
                    let tokens: Vec<&str> = block_text.split_whitespace().collect();
                    if tokens.len() >= 3 && tokens[2] == name {
                        let line_start = content[..abs_begin].rfind('\n').map(|p| p + 1).unwrap_or(0);
                        let line_end = content[abs_end..].find('\n').map(|p| abs_end + p + 1).unwrap_or(content.len());
                        find_result = Some((line_start, line_end));
                        break;
                    }
                    search_start = abs_end;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        if let Some((start, end)) = find_result {
            let mut result = String::new();
            result.push_str(&content[..start]);
            result.push_str(&content[end..]);
            Ok(result)
        } else {
            Ok(content.to_string())
        }
    }

    pub fn count_compu_method_refs(content: &str, name: &str) -> usize {
        let variables = A2lParser::parse_all_variables(content);
        variables.iter().filter(|v| v.compu_method.as_deref() == Some(name)).count()
    }

    pub fn preview_compu_method(method: &CompuMethod, raw_values: &[f64]) -> Vec<PreviewResult> {
        raw_values
            .iter()
            .map(|&raw| {
                match method.conversion_type {
                    CompuMethodType::LINEAR => PreviewResult {
                        raw,
                        physical: Some(method.f * raw + method.offset),
                        verbal: None,
                    },
                    CompuMethodType::IDENTICAL => PreviewResult {
                        raw,
                        physical: Some(raw),
                        verbal: None,
                    },
                    CompuMethodType::TAB_VERB => {
                        let verbal = method
                            .verb_pairs
                            .iter()
                            .find(|p| (p.in_val - raw).abs() < f64::EPSILON)
                            .map(|p| p.verbal.clone())
                            .unwrap_or_else(|| {
                                if method.default_value.is_empty() {
                                    "N/A".to_string()
                                } else {
                                    method.default_value.clone()
                                }
                            });
                        PreviewResult {
                            raw,
                            physical: None,
                            verbal: Some(verbal),
                        }
                    }
                    CompuMethodType::TAB_INTP => {
                        let physical = if method.intp_pairs.is_empty() {
                            raw
                        } else if method.intp_pairs.len() == 1 {
                            method.intp_pairs[0].out_val
                        } else {
                            let first = method.intp_pairs.first().unwrap();
                            let last = method.intp_pairs.last().unwrap();
                            if raw <= first.in_val {
                                first.out_val
                            } else if raw >= last.in_val {
                                last.out_val
                            } else {
                                let mut result = last.out_val;
                                for window in method.intp_pairs.windows(2) {
                                    let (p0, p1) = (window[0].clone(), window[1].clone());
                                    if raw >= p0.in_val && raw <= p1.in_val {
                                        let denom = p1.in_val - p0.in_val;
                                        if denom.abs() < f64::EPSILON {
                                            result = p0.out_val;
                                        } else {
                                            let t = (raw - p0.in_val) / denom;
                                            result = p0.out_val + t * (p1.out_val - p0.out_val);
                                        }
                                        break;
                                    }
                                }
                                result
                            }
                        };
                        PreviewResult {
                            raw,
                            physical: Some(physical),
                            verbal: None,
                        }
                    }
                }
            })
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewResult {
    pub raw: f64,
    pub physical: Option<f64>,
    pub verbal: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_symbol_link_from_a2l_variable() {
        let content = r#"
/begin PROJECT P ""
  /begin MODULE M ""
    /begin MEASUREMENT DisplayName ""
      UWORD NO_COMPU_METHOD 0 0 0 65535
      ECU_ADDRESS 0x00000000
      SYMBOL_LINK "RealSymbol" 0
    /end MEASUREMENT
  /end MODULE
/end PROJECT
"#;

        let variables = A2lParser::parse_all_variables(content);

        assert_eq!(variables.len(), 1);
        assert_eq!(variables[0].name, "DisplayName");
        assert_eq!(variables[0].symbol_link.as_deref(), Some("RealSymbol"));
    }

    #[test]
    fn updates_characteristic_value_and_link_map_addresses() {
        let content = r#"
/begin PROJECT P ""
  /begin MODULE M ""
    /begin CHARACTERISTIC CalValue ""
      VALUE 0x00000000 UWord 0 NO_COMPU_METHOD 0 65535
      /begin IF_DATA CANAPE_EXT
        100
        LINK_MAP "CalValue" 0x00000000 0 0 0 1 0x8F 0
      /end IF_DATA
    /end CHARACTERISTIC
  /end MODULE
/end PROJECT
"#;
        let edits = vec![VariableEdit {
            action: "modify".to_string(),
            original_name: "CalValue".to_string(),
            changes: Some(VariableChanges {
                name: None,
                address: Some("0x20000026".to_string()),
                data_type: None,
                var_type: None,
                bit_mask: None,
                compu_method: None,
                f: None,
                offset: None,
                unit: None,
                symbol_link: None,
            }),
            entry: None,
            export_mode: None,
        }];

        let (updated, result) =
            A2lGenerator::apply_changes(content, &edits, Endianness::Little).unwrap();

        assert_eq!(result.modified, 1);
        assert!(updated.contains("VALUE 0x20000026 UWord 0 NO_COMPU_METHOD 0 65535"));
        assert!(updated.contains("LINK_MAP \"CalValue\" 0x20000026 0 0 0 1 0x8F 0"));
    }
}
