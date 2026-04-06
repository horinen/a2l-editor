use crate::types::{infer_a2l_type, A2lEntry, Endianness, Variable};
use anyhow::{Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::io::Write;

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

#[derive(Debug, Clone)]
pub struct CompuMethod {
    pub name: String,
    pub f: f64,
    pub offset: f64,
    pub unit: String,
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

    fn generate_measurement_block_with_compu(
        entry: &A2lEntry,
        compu_method: Option<&str>,
        symbol_link: Option<&str>,
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
            let effective_offset = entry.get_effective_bit_offset(endianness).unwrap();
            let bit_size = entry.bit_size.unwrap();
            let mask = Self::calculate_bit_mask(effective_offset, bit_size);
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
            let link = symbol_link.unwrap_or(&entry.full_name);
            output.push_str(&format!("      SYMBOL_LINK \"{}\" 0\n", link));
        }
        output.push_str("    /end MEASUREMENT\n\n");

        output
    }

    fn generate_characteristic_block_with_compu(
        entry: &A2lEntry,
        compu_method: Option<&str>,
        symbol_link: Option<&str>,
        endianness: Endianness,
    ) -> String {
        let a2l_type = entry.a2l_type.as_str();
        let record_layout = Self::get_record_layout(a2l_type);

        let max_val = if entry.is_bitfield() {
            let size = entry.bit_size.unwrap();
            format!("{}", Self::get_bitfield_max(size))
        } else {
            let (_, max) = Self::get_min_max(a2l_type);
            max.to_string()
        };

        let compu = compu_method.unwrap_or("NO_COMPU_METHOD");
        let link = symbol_link.unwrap_or(&entry.full_name);
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
            let effective_offset = entry.get_effective_bit_offset(endianness).unwrap();
            let bit_size = entry.bit_size.unwrap();
            let mask = Self::calculate_bit_mask(effective_offset, bit_size);
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
            output.push_str(&format!("      SYMBOL_LINK \"{}\" 0\n", link));
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

    fn calculate_bit_mask(effective_bit_offset: usize, bit_size: usize) -> u64 {
        ((1u64 << bit_size) - 1) << effective_bit_offset
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
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("无法读取文件: {}", path.display()))?;

        let existing_names = Self::parse_existing_names(&content);

        let (to_add, to_skip): (Vec<_>, Vec<_>) = entries
            .iter()
            .partition(|e| !existing_names.contains(&e.full_name));

        let new_blocks: String = to_add
            .iter()
            .map(|e| match kind {
                ExportKind::Measurement => {
                    Self::generate_measurement_block_with_compu(e, None, None, endianness)
                }
                ExportKind::Characteristic => {
                    Self::generate_characteristic_block_with_compu(e, None, None, endianness)
                }
            })
            .collect();

        // 优先找到第一个 /begin GROUP 的位置，如果没有则使用 /end MODULE
        let insert_pos = content
            .find("/begin GROUP")
            .or_else(|| {
                content
                    .rfind("/end MEASUREMENT")
                    .or_else(|| content.rfind("/end MODULE"))
            })
            .with_context(|| "无法找到合适的插入位置")?;

        // 修复缩进问题：如果插入位置所在行只有空白字符，则移动到行首
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
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("无法读取文件: {}", path.display()))?;

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
                    // 跳过这个块（删除）
                    i = block_end + 1;
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
                            let symbol_link = edit
                                .changes
                                .as_ref()
                                .and_then(|c| c.symbol_link.as_deref())
                                .or_else(|| entry_info.symbol_link.as_deref());
                            let block = match kind {
                                ExportKind::Measurement => {
                                    Self::generate_measurement_block_with_compu(
                                        &entry,
                                        compu_method,
                                        symbol_link,
                                        endianness,
                                    )
                                }
                                ExportKind::Characteristic => {
                                    Self::generate_characteristic_block_with_compu(
                                        &entry,
                                        compu_method,
                                        symbol_link,
                                        endianness,
                                    )
                                }
                            };
                            let insert_pos = result
                                .find("/begin GROUP")
                                .or_else(|| result.rfind("/end MEASUREMENT"))
                                .or_else(|| result.rfind("/end CHARACTERISTIC"))
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
        let mut f = 1.0;
        let mut offset = 0.0;
        let mut unit = String::new();

        for line in block_lines {
            let trimmed = line.trim();

            if trimmed.starts_with("/begin COMPU_METHOD") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 3 {
                    name = parts[2].to_string();
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

            if trimmed.starts_with("LINEAR") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 4 {
                    unit = parts[3].trim_matches('"').to_string();
                }
                continue;
            }
        }

        if name.is_empty() || name == "NO_COMPU_METHOD" {
            None
        } else {
            Some(CompuMethod {
                name,
                f,
                offset,
                unit,
            })
        }
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
}
