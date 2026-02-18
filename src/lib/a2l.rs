use crate::types::{infer_a2l_type, A2lEntry, Variable};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::io::Write;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableChanges {
    pub name: Option<String>,
    pub address: Option<String>,
    pub data_type: Option<String>,
    pub var_type: Option<String>,
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
    pub var_type: String,  // "MEASUREMENT" 或 "CHARACTERISTIC"
    pub data_type: String, // "UBYTE", "UWORD", "FLOAT32_IEEE" 等
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
            output.push_str(&Self::generate_measurement_block(entry));
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

    fn generate_measurement_block(entry: &A2lEntry) -> String {
        let a2l_type = entry.a2l_type.as_str();
        let format_str = Self::get_format_string(a2l_type);

        // bitfield 使用 bit_size 计算上限
        let (min_val, max_val) = if entry.is_bitfield() {
            let size = entry.bit_size.unwrap();
            ("0".to_string(), format!("{}", Self::get_bitfield_max(size)))
        } else {
            let (min, max) = Self::get_min_max(a2l_type);
            (min.to_string(), max.to_string())
        };

        let mut output = String::new();

        output.push_str(&format!(
            "    /begin MEASUREMENT {} \"\"\n",
            entry.full_name
        ));
        output.push_str(&format!(
            "      {} NO_COMPU_METHOD 0 0 {} {}\n",
            a2l_type, min_val, max_val
        ));

        // bitfield 添加 BIT_MASK
        if entry.is_bitfield() {
            let mask = Self::calculate_bit_mask(entry.bit_offset, entry.bit_size);
            output.push_str(&format!("      BIT_MASK 0x{:X}\n", mask));
        }

        output.push_str(&format!("      ECU_ADDRESS 0x{:08X}\n", entry.address));
        output.push_str("      ECU_ADDRESS_EXTENSION 0x0\n");
        output.push_str(&format!("      FORMAT \"{}\"\n", format_str));
        output.push_str(&format!("      SYMBOL_LINK \"{}\" 0\n", entry.full_name));
        output.push_str("    /end MEASUREMENT\n\n");

        output
    }

    fn generate_characteristic_block(entry: &A2lEntry) -> String {
        let a2l_type = entry.a2l_type.as_str();
        let record_layout = Self::get_record_layout(a2l_type);

        // bitfield 使用 bit_size 计算上限
        let max_val = if entry.is_bitfield() {
            let size = entry.bit_size.unwrap();
            format!("{}", Self::get_bitfield_max(size))
        } else {
            let (_, max) = Self::get_min_max(a2l_type);
            max.to_string()
        };

        let mut output = String::new();

        output.push_str(&format!(
            "    /begin CHARACTERISTIC {} \"\"\n",
            entry.full_name
        ));
        output.push_str(&format!(
            "      VALUE 0x{:08X} {} 0 NO_COMPU_METHOD 0 {}\n",
            entry.address, record_layout, max_val
        ));

        // bitfield 添加 BIT_MASK
        if entry.is_bitfield() {
            let mask = Self::calculate_bit_mask(entry.bit_offset, entry.bit_size);
            output.push_str(&format!("      BIT_MASK 0x{:X}\n", mask));
        }

        output.push_str(&format!("      EXTENDED_LIMITS 0 {}\n", max_val));
        output.push_str(&format!("      SYMBOL_LINK \"{}\" 0\n", entry.full_name));
        output.push_str("    /end CHARACTERISTIC\n\n");

        output
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

    fn calculate_bit_mask(bit_offset: Option<usize>, bit_size: Option<usize>) -> u64 {
        if let (Some(offset), Some(size)) = (bit_offset, bit_size) {
            ((1u64 << size) - 1) << offset
        } else {
            0
        }
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
                ExportKind::Measurement => Self::generate_measurement_block(e),
                ExportKind::Characteristic => Self::generate_characteristic_block(e),
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
        is_measurement: bool,
    ) -> Result<String> {
        let mut result = String::new();
        let new_name = changes.name.as_deref().unwrap_or("");
        let new_address = changes.address.as_deref().unwrap_or("");
        let new_data_type = changes.data_type.as_deref().unwrap_or("");
        let change_to_characteristic =
            changes.var_type.as_deref() == Some("CHARACTERISTIC") && is_measurement;
        let change_to_measurement =
            changes.var_type.as_deref() == Some("MEASUREMENT") && !is_measurement;

        let mut original_name = String::new();
        let mut original_address = String::new();
        let mut original_data_type = String::new();

        for line in block_lines {
            let trimmed = line.trim();
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
                }
            }
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if !parts.is_empty() {
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
                if a2l_types.contains(&parts[0]) {
                    original_data_type = parts[0].to_string();
                }
            }
        }

        if change_to_characteristic || change_to_measurement {
            let entry = A2lEntry {
                full_name: if new_name.is_empty() {
                    original_name.clone()
                } else {
                    new_name.to_string()
                },
                address: if new_address.is_empty() {
                    u64::from_str_radix(
                        original_address
                            .trim_start_matches("0x")
                            .trim_start_matches("0X"),
                        16,
                    )
                    .unwrap_or(0)
                } else {
                    u64::from_str_radix(
                        new_address
                            .trim_start_matches("0x")
                            .trim_start_matches("0X"),
                        16,
                    )
                    .unwrap_or(0)
                },
                size: 4,
                a2l_type: if new_data_type.is_empty() {
                    original_data_type.clone()
                } else {
                    new_data_type.to_string()
                },
                type_name: String::new(),
                bit_offset: None,
                bit_size: None,
                array_index: None,
            };
            if change_to_characteristic {
                return Ok(Self::generate_characteristic_block(&entry));
            } else {
                return Ok(Self::generate_measurement_block(&entry));
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

        for line in block_lines {
            let trimmed = line.trim();

            if trimmed.starts_with("/begin MEASUREMENT ")
                || trimmed.starts_with("/begin CHARACTERISTIC ")
            {
                let indent = line.len() - line.trim_start().len();
                let prefix = " ".repeat(indent);
                let block_type = if is_measurement {
                    "MEASUREMENT"
                } else {
                    "CHARACTERISTIC"
                };
                result.push_str(&format!(
                    "{}{} {} {} \"\"\n",
                    prefix, "/begin", block_type, final_name
                ));
                continue;
            }

            if trimmed.starts_with(&original_name)
                && line.contains(&original_name)
                && !line.contains("SYMBOL_LINK")
                && !line.contains("LINK_MAP")
            {
                let indent = line.len() - line.trim_start().len();
                let rest_of_line = trimmed.strip_prefix(&original_name).unwrap_or("");
                result.push_str(&format!(
                    "{}{}{}\n",
                    " ".repeat(indent),
                    final_name,
                    rest_of_line
                ));
                continue;
            }

            if trimmed.starts_with("ECU_ADDRESS") {
                let indent = line.len() - line.trim_start().len();
                result.push_str(&format!(
                    "{}ECU_ADDRESS {}\n",
                    " ".repeat(indent),
                    final_address
                ));
                continue;
            }

            if trimmed.contains("LINK_MAP") {
                let indent = line.len() - line.trim_start().len();
                let addr_num = u64::from_str_radix(
                    final_address
                        .trim_start_matches("0x")
                        .trim_start_matches("0X"),
                    16,
                )
                .unwrap_or(0);
                result.push_str(&format!(
                    "{}LINK_MAP \"{}\" 0x{:X} 0 0 0 0\n",
                    " ".repeat(indent),
                    final_name,
                    addr_num
                ));
                continue;
            }

            if trimmed.starts_with("SYMBOL_LINK") {
                let indent = line.len() - line.trim_start().len();
                result.push_str(&format!(
                    "{}SYMBOL_LINK \"{}\" 0\n",
                    " ".repeat(indent),
                    final_name
                ));
                continue;
            }

            if trimmed.starts_with(&original_data_type) && !original_data_type.is_empty() {
                let indent = line.len() - line.trim_start().len();
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 6 {
                    if is_measurement {
                        result.push_str(&format!(
                            "{}{} NO_COMPU_METHOD 0 0 {} {}\n",
                            " ".repeat(indent),
                            final_data_type,
                            parts[4],
                            parts[5]
                        ));
                    } else {
                        result.push_str(&format!(
                            "{}{} NO_COMPU_METHOD 0 0 {} {}\n",
                            " ".repeat(indent),
                            final_data_type,
                            parts[4],
                            parts[5]
                        ));
                    }
                    continue;
                }
            }

            if is_measurement && trimmed.starts_with("FORMAT") {
                let indent = line.len() - line.trim_start().len();
                let format_str = Self::get_format_string(final_data_type);
                result.push_str(&format!(
                    "{}FORMAT \"{}\"\n",
                    " ".repeat(indent),
                    format_str
                ));
                continue;
            }

            if is_measurement && trimmed.starts_with("DISPLAY") {
                let indent = line.len() - line.trim_start().len();
                let (min_val, max_val) = Self::get_min_max(final_data_type);
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                result.push_str(&format!(
                    "{}DISPLAY {} {} {}\n",
                    " ".repeat(indent),
                    parts[1],
                    min_val,
                    max_val
                ));
                continue;
            }

            result.push_str(line);
            result.push('\n');
        }

        Ok(result)
    }

    /// 统一应用所有变更（修改、删除、添加）
    pub fn apply_changes(content: &str, edits: &[VariableEdit]) -> Result<(String, SaveResult)> {
        let mut result = content.to_string();
        let mut save_result = SaveResult {
            modified: 0,
            deleted: 0,
            added: 0,
            skipped: 0,
        };

        let existing_names = Self::parse_existing_names(content);

        for edit in edits {
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
                            };
                            let kind = match edit.export_mode.as_deref() {
                                Some("characteristic") => ExportKind::Characteristic,
                                _ => ExportKind::Measurement,
                            };
                            let block = match kind {
                                ExportKind::Measurement => Self::generate_measurement_block(&entry),
                                ExportKind::Characteristic => {
                                    Self::generate_characteristic_block(&entry)
                                }
                            };
                            let insert_pos = result
                                .find("/begin GROUP")
                                .or_else(|| result.rfind("/end MEASUREMENT"))
                                .or_else(|| result.rfind("/end CHARACTERISTIC"))
                                .or_else(|| result.rfind("/end MODULE"))
                                .unwrap_or(result.len());

                            // 修复缩进问题：如果插入位置所在行只有空白字符，则移动到行首
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

            // 处理 /begin MEASUREMENT <name> "" 或 /begin CHARACTERISTIC <name> "" 格式
            // 注意：只处理块开始的 /begin 行，不处理嵌套的 /begin IF_DATA 等
            if trimmed.starts_with("/begin MEASUREMENT ")
                || trimmed.starts_with("/begin CHARACTERISTIC ")
            {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                // /begin MEASUREMENT <name> 或 /begin CHARACTERISTIC <name>
                if parts.len() >= 3 {
                    let candidate = parts[2];
                    // 确保不是数字，也不是 A2L 类型
                    if !candidate.parse::<f64>().is_ok() && !a2l_types.contains(&candidate) {
                        name = candidate.to_string();
                        found_first_name = true;
                    }
                }
                continue;
            }

            // 跳过其他 /begin 或 /end 开头的行（嵌套块）
            if trimmed.starts_with('/') {
                continue;
            }

            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            // 如果还没找到变量名，尝试从第一个非数字 token 获取
            if !found_first_name {
                if !parts[0].parse::<f64>().is_ok() && !a2l_types.contains(&parts[0]) {
                    name = parts[0].to_string();
                    found_first_name = true;
                }
            }

            // 查找数据类型 - 格式: <datatype> <conversion> 0 0 <min> <max>
            if parts.len() >= 2 {
                let possible_type = parts[0];
                if a2l_types.contains(&possible_type) {
                    data_type = possible_type.to_string();
                }
            }

            // 查找地址信息 - ECU_ADDRESS 后面跟着地址
            if let Some(addr_pos) = parts.iter().position(|&x| x == "ECU_ADDRESS") {
                if addr_pos + 1 < parts.len() {
                    address = Some(parts[addr_pos + 1].to_string());
                }
            }

            // 对于 CHARACTERISTIC，在 VALUE 中查找地址
            // 格式: VALUE <address> <record_layout> 0 <max>
            if block_type == "CHARACTERISTIC" {
                if let Some(value_pos) = parts.iter().position(|&x| x == "VALUE") {
                    if value_pos + 1 < parts.len() {
                        address = Some(parts[value_pos + 1].to_string());
                    }
                }
            }
        }

        A2lVariable {
            name,
            address,
            var_type: block_type.to_string(),
            data_type,
        }
    }
}
