use a2l_editor::{
    A2lEntry, A2lEntryInfo, A2lEntryStore, A2lGenerator, A2lParser, A2lVariable, CompuMethod,
    CompuMethodType, DataPackage, ElfParser, Endianness, ExportKind, PackageMeta, PreviewResult,
    SaveResult, TabIntpPair, TabVerbPair, VariableChanges, VariableEdit,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::State;

fn read_a2l_file(path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("读取 A2L 文件失败: {}", e))?;
    match String::from_utf8(bytes) {
        Ok(s) => Ok(s),
        Err(err) => {
            eprintln!(
                "警告: A2L 文件包含非 UTF-8 字节，已使用容错解码: {}",
                path.display()
            );
            Ok(String::from_utf8_lossy(err.as_bytes()).into_owned())
        }
    }
}

fn parse_query_anchors(query: &str) -> (&str, bool, bool) {
    let mut starts = false;
    let mut ends = false;
    let mut q = query;
    if q.starts_with('^') {
        starts = true;
        q = &q[1..];
    }
    if q.ends_with('$') {
        ends = true;
        q = &q[..q.len().saturating_sub(1)];
    }
    (q, starts, ends)
}

fn match_name(name_lower: &str, q: &str, starts: bool, ends: bool) -> bool {
    if q.is_empty() {
        return true;
    }
    if starts && ends {
        name_lower == q
    } else if starts {
        name_lower.starts_with(q)
    } else if ends {
        name_lower.ends_with(q)
    } else {
        name_lower.contains(q)
    }
}

fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    let a_bytes = a.as_bytes();
    let b_bytes = b.as_bytes();
    let mut ia = 0;
    let mut ib = 0;
    while ia < a_bytes.len() && ib < b_bytes.len() {
        let ca = a_bytes[ia];
        let cb = b_bytes[ib];
        if ca.is_ascii_digit() && cb.is_ascii_digit() {
            let mut na: u64 = 0;
            while ia < a_bytes.len() && a_bytes[ia].is_ascii_digit() {
                na = na * 10 + (a_bytes[ia] - b'0') as u64;
                ia += 1;
            }
            let mut nb: u64 = 0;
            while ib < b_bytes.len() && b_bytes[ib].is_ascii_digit() {
                nb = nb * 10 + (b_bytes[ib] - b'0') as u64;
                ib += 1;
            }
            match na.cmp(&nb) {
                Ordering::Equal => continue,
                other => return other,
            }
        } else {
            match ca.cmp(&cb) {
                Ordering::Equal => {}
                other => return other,
            }
            ia += 1;
            ib += 1;
        }
    }
    (a_bytes.len() - ia).cmp(&(b_bytes.len() - ib))
}

#[derive(Default)]
pub struct AppState {
    pub store: Option<A2lEntryStore>,
    pub data_package: Option<DataPackage>,
    pub elf_path: Option<PathBuf>,
    pub a2l_path: Option<PathBuf>,
    pub a2l_names: HashSet<String>,
    pub a2l_variables: Vec<A2lVariable>,
    pub endianness: String,
}

#[derive(Serialize)]
pub struct LoadResult {
    pub meta: PackageMetaInfo,
    pub entry_count: usize,
}

#[derive(Serialize, Clone)]
pub struct PackageMetaInfo {
    pub file_name: String,
    pub elf_path: Option<String>,
    pub entry_count: usize,
    pub created_at: i64,
}

impl From<PackageMeta> for PackageMetaInfo {
    fn from(meta: PackageMeta) -> Self {
        PackageMetaInfo {
            file_name: meta.file_name,
            elf_path: meta.elf_path,
            entry_count: meta.entry_count,
            created_at: meta.created_at,
        }
    }
}

#[derive(Serialize)]
pub struct A2lLoadResult {
    pub path: String,
    pub variable_count: usize,
    pub existing_names: Vec<String>,
}

#[derive(Serialize)]
pub struct ExportResult {
    pub added: usize,
    pub skipped: usize,
    pub existing: usize,
}

#[derive(Serialize, Deserialize)]
pub struct EntryInfo {
    pub index: usize,
    pub full_name: String,
    pub address: u64,
    pub size: usize,
    pub a2l_type: String,
    pub type_name: String,
    pub bit_offset: Option<usize>,
    pub bit_size: Option<usize>,
    pub symbol_link: Option<String>,
}

impl From<(usize, &A2lEntry)> for EntryInfo {
    fn from((index, entry): (usize, &A2lEntry)) -> Self {
        EntryInfo {
            index,
            full_name: entry.full_name.clone(),
            address: entry.address,
            size: entry.size,
            a2l_type: entry.a2l_type.clone(),
            type_name: entry.type_name.clone(),
            bit_offset: entry.bit_offset,
            bit_size: entry.bit_size,
            symbol_link: None,
        }
    }
}

#[derive(Serialize)]
pub struct VariableInfo {
    pub name: String,
    pub address: Option<String>,
    pub data_type: String,
    pub var_type: String,
    pub bit_mask: Option<String>,
    pub compu_method: Option<String>,
    pub symbol_link: Option<String>,
    pub f: Option<f64>,
    pub offset: Option<f64>,
    pub unit: Option<String>,
}

impl From<&A2lVariable> for VariableInfo {
    fn from(var: &A2lVariable) -> Self {
        VariableInfo {
            name: var.name.clone(),
            address: var.address.clone(),
            data_type: var.data_type.clone(),
            var_type: var.var_type.clone(),
            bit_mask: var.bit_mask.clone(),
            compu_method: var.compu_method.clone(),
            symbol_link: var.symbol_link.clone(),
            f: var.f,
            offset: var.offset,
            unit: var.unit.clone(),
        }
    }
}

#[tauri::command]
pub fn load_elf(path: String, state: State<Mutex<AppState>>) -> Result<LoadResult, String> {
    let elf_path = PathBuf::from(&path);
    let mut state = state.lock().map_err(|e| e.to_string())?;
    state.elf_path = Some(elf_path.clone());

    if DataPackage::exists(&elf_path) {
        load_from_package_internal(&elf_path, &mut state)
    } else {
        Err("数据包不存在，请先生成".to_string())
    }
}

fn load_from_package_internal(
    elf_path: &PathBuf,
    state: &mut AppState,
) -> Result<LoadResult, String> {
    let pkg = DataPackage::open(elf_path).map_err(|e| format!("无法打开数据包: {}", e))?;
    let meta = pkg
        .get_meta()
        .map_err(|e| format!("无法读取元信息: {}", e))?;
    let store = pkg
        .load_entries()
        .map_err(|e| format!("无法加载条目: {}", e))?;
    let entry_count = store.len();

    state.store = Some(store);
    state.data_package = Some(pkg);

    Ok(LoadResult {
        meta: PackageMetaInfo::from(meta),
        entry_count,
    })
}

#[tauri::command]
pub fn load_package(path: String, state: State<Mutex<AppState>>) -> Result<LoadResult, String> {
    let package_path = PathBuf::from(&path);
    let mut state = state.lock().map_err(|e| e.to_string())?;

    let pkg =
        DataPackage::open_path(&package_path).map_err(|e| format!("无法打开数据包: {}", e))?;
    let meta = pkg
        .get_meta()
        .map_err(|e| format!("无法读取元信息: {}", e))?;
    state.elf_path = meta.elf_path.as_ref().map(|p| PathBuf::from(p));

    let store = pkg
        .load_entries()
        .map_err(|e| format!("无法加载条目: {}", e))?;
    let entry_count = store.len();

    state.store = Some(store);
    state.data_package = Some(pkg);

    Ok(LoadResult {
        meta: PackageMetaInfo::from(meta),
        entry_count,
    })
}

#[tauri::command]
pub fn generate_package(
    elf_path: String,
    output_path: Option<String>,
    state: State<Mutex<AppState>>,
) -> Result<PackageMetaInfo, String> {
    let elf = PathBuf::from(&elf_path);
    let parser = ElfParser::parse_deep(&elf).map_err(|e| format!("解析失败: {}", e))?;
    let store = parser.a2l_entries().ok_or("未找到 A2L 条目")?.clone();

    let mut pkg = if let Some(ref output) = output_path {
        DataPackage::create_at(&PathBuf::from(output), &elf)
            .map_err(|e| format!("创建数据包失败: {}", e))?
    } else {
        DataPackage::create(&elf).map_err(|e| format!("创建数据包失败: {}", e))?
    };

    pkg.save_entries(&store)
        .map_err(|e| format!("保存数据包失败: {}", e))?;
    let meta = pkg
        .get_meta()
        .map_err(|e| format!("读取元信息失败: {}", e))?;

    let mut state = state.lock().map_err(|e| e.to_string())?;
    state.store = Some(store);
    state.data_package = Some(pkg);
    state.elf_path = Some(elf);

    Ok(PackageMetaInfo::from(meta))
}

#[tauri::command]
pub fn load_a2l(path: String, state: State<Mutex<AppState>>) -> Result<A2lLoadResult, String> {
    let a2l_path = PathBuf::from(&path);
    let content = read_a2l_file(&a2l_path)?;

    let variables = A2lParser::parse_all_variables(&content);
    let existing_names: Vec<String> = variables.iter().map(|v| v.name.clone()).collect();
    let name_set: HashSet<String> = existing_names.iter().cloned().collect();

    let result = A2lLoadResult {
        path: path.clone(),
        variable_count: variables.len(),
        existing_names,
    };

    let mut state = state.lock().map_err(|e| e.to_string())?;
    state.a2l_path = Some(a2l_path);
    state.a2l_names = name_set;
    state.a2l_variables = variables;

    Ok(result)
}

#[tauri::command]
pub fn search_elf_entries(
    query: String,
    offset: usize,
    limit: usize,
    sort_field: Option<String>,
    sort_order: Option<String>,
    natural_sort: Option<bool>,
    state: State<Mutex<AppState>>,
) -> Result<Vec<EntryInfo>, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let store = state.store.as_ref().ok_or("未加载 ELF 文件")?;

    let field = sort_field.as_deref().unwrap_or("name");
    let order = sort_order.as_deref().unwrap_or("asc");
    let use_natural = natural_sort.unwrap_or(false);

    let (q, starts, ends) = parse_query_anchors(&query);
    let q_lower = q.to_lowercase();

    let mut entries: Vec<(usize, &A2lEntry)> = if q.is_empty() {
        store.entries.iter().enumerate().collect()
    } else {
        store
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                let name_lower = e.full_name.to_lowercase();
                match_name(&name_lower, &q_lower, starts, ends)
            })
            .collect()
    };

    entries.sort_by(|a, b| {
        let cmp = match field {
            "address" => a.1.address.cmp(&b.1.address),
            _ => if use_natural {
                natural_cmp(&a.1.full_name, &b.1.full_name)
            } else {
                a.1.full_name.cmp(&b.1.full_name)
            },
        };
        if order == "desc" {
            cmp.reverse()
        } else {
            cmp
        }
    });

    let result: Vec<EntryInfo> = entries
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(|(i, e)| EntryInfo::from((i, e)))
        .collect();

    Ok(result)
}

#[tauri::command]
pub fn get_elf_count(state: State<Mutex<AppState>>) -> Result<usize, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    Ok(state.store.as_ref().map(|s| s.len()).unwrap_or(0))
}

#[tauri::command]
pub fn search_elf_count(query: String, state: State<Mutex<AppState>>) -> Result<usize, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let store = state.store.as_ref().ok_or("未加载 ELF 文件")?;
    let (q, starts, ends) = parse_query_anchors(&query);
    let count = if q.is_empty() {
        store.entries.len()
    } else {
        let q_lower = q.to_lowercase();
        store
            .entries
            .iter()
            .filter(|e| {
                let name_lower = e.full_name.to_lowercase();
                match_name(&name_lower, &q_lower, starts, ends)
            })
            .count()
    };
    Ok(count)
}

#[tauri::command]
pub fn search_a2l_variables(
    query: String,
    offset: usize,
    limit: usize,
    state: State<Mutex<AppState>>,
) -> Result<Vec<VariableInfo>, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let (q, starts, ends) = parse_query_anchors(&query);

    let variables: Vec<VariableInfo> = if q.is_empty() {
        state
            .a2l_variables
            .iter()
            .skip(offset)
            .take(limit)
            .map(VariableInfo::from)
            .collect()
    } else {
        let q_lower = q.to_lowercase();
        state
            .a2l_variables
            .iter()
            .filter(|v| {
                let name_lower = v.name.to_lowercase();
                match_name(&name_lower, &q_lower, starts, ends)
            })
            .skip(offset)
            .take(limit)
            .map(VariableInfo::from)
            .collect()
    };

    Ok(variables)
}

#[tauri::command]
pub fn export_entries(
    indices: Vec<usize>,
    mode: String,
    state: State<Mutex<AppState>>,
) -> Result<ExportResult, String> {
    let mut state = state.lock().map_err(|e| e.to_string())?;

    let a2l_path = state
        .a2l_path
        .as_ref()
        .ok_or("未选择目标 A2L 文件")?
        .clone();
    let store = state.store.as_ref().ok_or("未加载 ELF 文件")?;

    let entries: Vec<A2lEntry> = indices
        .iter()
        .filter_map(|&i| store.entries.get(i).cloned())
        .collect();

    if entries.is_empty() {
        return Err("没有选中任何条目".to_string());
    }

    let export_kind = match mode.as_str() {
        "measurement" => ExportKind::Measurement,
        "characteristic" => ExportKind::Characteristic,
        _ => return Err("无效的导出模式".to_string()),
    };

    let endianness = if state.endianness == "big" {
        Endianness::Big
    } else {
        Endianness::Little
    };

    let result = A2lGenerator::append_to_file(&entries, &a2l_path, export_kind, endianness)
        .map_err(|e| format!("导出失败: {}", e))?;

    // 重新加载 A2L
    let content = read_a2l_file(&a2l_path)?;
    let variables = A2lParser::parse_all_variables(&content);
    state.a2l_variables = variables;
    state.a2l_names = state.a2l_variables.iter().map(|v| v.name.clone()).collect();

    Ok(ExportResult {
        added: result.added,
        skipped: result.skipped,
        existing: result.existing,
    })
}

#[tauri::command]
pub fn delete_variables(
    names: Vec<String>,
    state: State<Mutex<AppState>>,
) -> Result<usize, String> {
    let mut state = state.lock().map_err(|e| e.to_string())?;

    let a2l_path = state.a2l_path.as_ref().ok_or("未选择目标 A2L 文件")?;

    if names.is_empty() {
        return Err("没有选中任何变量".to_string());
    }

    let content = read_a2l_file(a2l_path)?;
    let new_content = A2lGenerator::remove_variables(&content, &names)
        .map_err(|e| format!("删除变量失败: {}", e))?;

    std::fs::write(a2l_path, new_content).map_err(|e| format!("写入 A2L 文件失败: {}", e))?;

    let deleted_count = names.len();

    // 重新加载 A2L
    let content = read_a2l_file(a2l_path)?;
    let variables = A2lParser::parse_all_variables(&content);
    state.a2l_variables = variables;
    state.a2l_names = state.a2l_variables.iter().map(|v| v.name.clone()).collect();

    Ok(deleted_count)
}

#[derive(Serialize, Deserialize)]
pub struct VariableEditInput {
    pub action: String,
    pub original_name: String,
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
    pub entry: Option<EntryInfo>,
    pub export_mode: Option<String>,
}

#[tauri::command]
pub fn save_a2l_changes(
    edits: Vec<VariableEditInput>,
    state: State<Mutex<AppState>>,
) -> Result<SaveResult, String> {
    let mut state = state.lock().map_err(|e| e.to_string())?;

    let a2l_path = state.a2l_path.as_ref().ok_or("未选择目标 A2L 文件")?;

    let variable_edits: Vec<VariableEdit> = edits
        .into_iter()
        .map(|e| VariableEdit {
            action: e.action,
            original_name: e.original_name,
            changes: if e.name.is_some()
                || e.address.is_some()
                || e.data_type.is_some()
                || e.var_type.is_some()
                || e.bit_mask.is_some()
                || e.compu_method.is_some()
                || e.f.is_some()
                || e.offset.is_some()
                || e.unit.is_some()
                || e.symbol_link.is_some()
            {
                Some(VariableChanges {
                    name: e.name,
                    address: e.address,
                    data_type: e.data_type,
                    var_type: e.var_type,
                    bit_mask: e.bit_mask,
                    compu_method: e.compu_method,
                    f: e.f,
                    offset: e.offset,
                    unit: e.unit,
                    symbol_link: e.symbol_link,
                })
            } else {
                None
            },
            entry: e.entry.map(|info| A2lEntryInfo {
                full_name: info.full_name,
                address: info.address,
                size: info.size,
                a2l_type: info.a2l_type,
                type_name: info.type_name,
                bit_offset: info.bit_offset,
                bit_size: info.bit_size,
                symbol_link: info.symbol_link,
            }),
            export_mode: e.export_mode,
        })
        .collect();

    let content = read_a2l_file(a2l_path)?;

    let endianness = if state.endianness == "big" {
        Endianness::Big
    } else {
        Endianness::Little
    };

    let (new_content, result) = A2lGenerator::apply_changes(&content, &variable_edits, endianness)
        .map_err(|e| format!("应用变更失败: {}", e))?;

    std::fs::write(a2l_path, new_content).map_err(|e| format!("写入 A2L 文件失败: {}", e))?;

    let variables = A2lParser::parse_all_variables(&read_a2l_file(a2l_path)?);
    state.a2l_variables = variables;
    state.a2l_names = state.a2l_variables.iter().map(|v| v.name.clone()).collect();

    Ok(result)
}

#[tauri::command]
pub fn set_endianness(endianness: String, state: State<Mutex<AppState>>) -> Result<(), String> {
    if endianness != "little" && endianness != "big" {
        return Err("无效的字节序，必须是 'little' 或 'big'".to_string());
    }
    let mut state = state.lock().map_err(|e| e.to_string())?;
    state.endianness = endianness;
    Ok(())
}

#[derive(Serialize)]
pub struct UpdateAddressResult {
    pub updated: usize,
    pub skipped: usize,
}

#[tauri::command]
pub fn update_a2l_addresses(state: State<Mutex<AppState>>) -> Result<UpdateAddressResult, String> {
    let mut state = state.lock().map_err(|e| e.to_string())?;

    let a2l_path = state.a2l_path.as_ref().ok_or("未选择目标 A2L 文件")?;
    let store = state.store.as_ref().ok_or("未加载 ELF 文件")?;

    if state.a2l_variables.is_empty() {
        return Err("A2L 文件中没有变量".to_string());
    }

    let mut edits: Vec<VariableEdit> = Vec::new();
    let mut updated = 0;
    let mut skipped = 0;

    for var in &state.a2l_variables {
        let entry = store.get_by_name(&var.name).or_else(|| {
            var.symbol_link
                .as_deref()
                .and_then(|name| store.get_by_name(name))
        });

        if let Some(entry) = entry {
            let new_address = format!("0x{:08X}", entry.address);
            edits.push(VariableEdit {
                action: "modify".to_string(),
                original_name: var.name.clone(),
                changes: Some(VariableChanges {
                    name: None,
                    address: Some(new_address),
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
            });
            updated += 1;
        } else {
            skipped += 1;
        }
    }

    if edits.is_empty() {
        return Ok(UpdateAddressResult {
            updated: 0,
            skipped,
        });
    }

    let content = read_a2l_file(a2l_path)?;

    let endianness = if state.endianness == "big" {
        Endianness::Big
    } else {
        Endianness::Little
    };
    let (new_content, _) = A2lGenerator::apply_changes(&content, &edits, endianness)
        .map_err(|e| format!("更新地址失败: {}", e))?;

    std::fs::write(a2l_path, new_content).map_err(|e| format!("写入 A2L 文件失败: {}", e))?;

    let variables = A2lParser::parse_all_variables(&read_a2l_file(a2l_path)?);
    state.a2l_variables = variables;
    state.a2l_names = state.a2l_variables.iter().map(|v| v.name.clone()).collect();

    Ok(UpdateAddressResult { updated, skipped })
}

#[derive(Serialize)]
pub struct CompuMethodSummary {
    pub name: String,
    pub conversion_type: String,
    pub summary: String,
    pub unit: String,
    pub ref_count: usize,
}

#[derive(Serialize, Deserialize)]
pub struct CompuMethodInput {
    pub name: String,
    pub conversion_type: String,
    pub unit: String,
    pub description: String,
    pub f: f64,
    pub offset: f64,
    pub verb_pairs: Vec<TabVerbPairInput>,
    pub default_value: String,
    pub intp_pairs: Vec<TabIntpPairInput>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TabVerbPairInput {
    pub in_val: f64,
    pub verbal: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TabIntpPairInput {
    pub in_val: f64,
    pub out_val: f64,
}

impl From<CompuMethodInput> for CompuMethod {
    fn from(input: CompuMethodInput) -> Self {
        let conversion_type = match input.conversion_type.as_str() {
            "TAB_VERB" => CompuMethodType::TAB_VERB,
            "TAB_INTP" => CompuMethodType::TAB_INTP,
            "IDENTICAL" => CompuMethodType::IDENTICAL,
            _ => CompuMethodType::LINEAR,
        };
        CompuMethod {
            name: input.name,
            conversion_type,
            unit: input.unit,
            description: input.description,
            f: input.f,
            offset: input.offset,
            verb_pairs: input
                .verb_pairs
                .into_iter()
                .map(|p| TabVerbPair {
                    in_val: p.in_val,
                    verbal: p.verbal,
                })
                .collect(),
            default_value: input.default_value,
            intp_pairs: input
                .intp_pairs
                .into_iter()
                .map(|p| TabIntpPair {
                    in_val: p.in_val,
                    out_val: p.out_val,
                })
                .collect(),
        }
    }
}

#[tauri::command]
pub fn list_compu_methods(
    state: State<Mutex<AppState>>,
) -> Result<Vec<CompuMethodSummary>, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let a2l_path = state.a2l_path.as_ref().ok_or("未加载 A2L 文件")?;
    let content = read_a2l_file(a2l_path)?;
    let methods = A2lParser::parse_compu_methods(&content);
    let variables = &state.a2l_variables;

    let summaries: Vec<CompuMethodSummary> = methods
        .iter()
        .map(|m| {
            let ref_count = variables
                .iter()
                .filter(|v| v.compu_method.as_deref() == Some(&m.name))
                .count();
            CompuMethodSummary {
                name: m.name.clone(),
                conversion_type: m.conversion_type.to_string(),
                summary: m.summary(),
                unit: m.unit.clone(),
                ref_count,
            }
        })
        .collect();

    Ok(summaries)
}

#[tauri::command]
pub fn get_compu_method(
    name: String,
    state: State<Mutex<AppState>>,
) -> Result<CompuMethod, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let a2l_path = state.a2l_path.as_ref().ok_or("未加载 A2L 文件")?;
    let content = read_a2l_file(a2l_path)?;
    let methods = A2lParser::parse_compu_methods(&content);

    methods
        .into_iter()
        .find(|m| m.name == name)
        .ok_or_else(|| format!("未找到 COMPU_METHOD: {}", name))
}

#[tauri::command]
pub fn save_compu_method_cmd(
    method: CompuMethodInput,
    state: State<Mutex<AppState>>,
) -> Result<(), String> {
    let mut state = state.lock().map_err(|e| e.to_string())?;
    let a2l_path = state.a2l_path.as_ref().ok_or("未加载 A2L 文件")?;
    let content = read_a2l_file(a2l_path)?;

    let cm: CompuMethod = method.into();
    let new_content = A2lParser::save_compu_method(&content, &cm)
        .map_err(|e| format!("保存 COMPU_METHOD 失败: {}", e))?;

    std::fs::write(a2l_path, new_content).map_err(|e| format!("写入 A2L 文件失败: {}", e))?;

    let variables = A2lParser::parse_all_variables(&read_a2l_file(a2l_path)?);
    state.a2l_variables = variables;
    state.a2l_names = state.a2l_variables.iter().map(|v| v.name.clone()).collect();

    Ok(())
}

#[tauri::command]
pub fn delete_compu_method_cmd(
    name: String,
    state: State<Mutex<AppState>>,
) -> Result<usize, String> {
    let mut state = state.lock().map_err(|e| e.to_string())?;
    let a2l_path = state.a2l_path.as_ref().ok_or("未加载 A2L 文件")?;
    let content = read_a2l_file(a2l_path)?;

    let ref_count = A2lParser::count_compu_method_refs(&content, &name);

    let new_content = A2lParser::delete_compu_method(&content, &name)
        .map_err(|e| format!("删除 COMPU_METHOD 失败: {}", e))?;

    std::fs::write(a2l_path, new_content).map_err(|e| format!("写入 A2L 文件失败: {}", e))?;

    let variables = A2lParser::parse_all_variables(&read_a2l_file(a2l_path)?);
    state.a2l_variables = variables;
    state.a2l_names = state.a2l_variables.iter().map(|v| v.name.clone()).collect();

    Ok(ref_count)
}

#[tauri::command]
pub fn preview_compu_method_cmd(
    method: CompuMethodInput,
    raw_values: Vec<f64>,
) -> Result<Vec<PreviewResult>, String> {
    let cm: CompuMethod = method.into();
    let results = A2lParser::preview_compu_method(&cm, &raw_values);
    Ok(results)
}
