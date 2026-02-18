use a2l_editor::{
    A2lEntry, A2lEntryInfo, A2lEntryStore, A2lGenerator, A2lParser, A2lVariable, DataPackage,
    ElfParser, ExportKind, PackageMeta, SaveResult, VariableChanges, VariableEdit,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;

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
        }
    }
}

#[derive(Serialize)]
pub struct VariableInfo {
    pub name: String,
    pub address: Option<String>,
    pub data_type: String,
    pub var_type: String,
}

impl From<&A2lVariable> for VariableInfo {
    fn from(var: &A2lVariable) -> Self {
        VariableInfo {
            name: var.name.clone(),
            address: var.address.clone(),
            data_type: var.data_type.clone(),
            var_type: var.var_type.clone(),
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
    let content =
        std::fs::read_to_string(&a2l_path).map_err(|e| format!("读取 A2L 文件失败: {}", e))?;

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
    state: State<Mutex<AppState>>,
) -> Result<Vec<EntryInfo>, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let store = state.store.as_ref().ok_or("未加载 ELF 文件")?;

    let field = sort_field.as_deref().unwrap_or("name");
    let order = sort_order.as_deref().unwrap_or("asc");

    let mut entries: Vec<(usize, &A2lEntry)> = if query.is_empty() {
        store.entries.iter().enumerate().collect()
    } else {
        let q = query.to_lowercase();
        store
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.full_name.to_lowercase().contains(&q))
            .collect()
    };

    // 排序
    entries.sort_by(|a, b| {
        let cmp = match field {
            "address" => a.1.address.cmp(&b.1.address),
            _ => a.1.full_name.cmp(&b.1.full_name),
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
pub fn search_a2l_variables(
    query: String,
    offset: usize,
    limit: usize,
    state: State<Mutex<AppState>>,
) -> Result<Vec<VariableInfo>, String> {
    let state = state.lock().map_err(|e| e.to_string())?;

    let variables: Vec<VariableInfo> = if query.is_empty() {
        state
            .a2l_variables
            .iter()
            .skip(offset)
            .take(limit)
            .map(VariableInfo::from)
            .collect()
    } else {
        let q = query.to_lowercase();
        state
            .a2l_variables
            .iter()
            .filter(|v| v.name.to_lowercase().contains(&q))
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

    let result = A2lGenerator::append_to_file(&entries, &a2l_path, export_kind)
        .map_err(|e| format!("导出失败: {}", e))?;

    // 重新加载 A2L
    let content =
        std::fs::read_to_string(&a2l_path).map_err(|e| format!("重新读取 A2L 失败: {}", e))?;
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

    let content =
        std::fs::read_to_string(a2l_path).map_err(|e| format!("读取 A2L 文件失败: {}", e))?;
    let new_content = A2lGenerator::remove_variables(&content, &names)
        .map_err(|e| format!("删除变量失败: {}", e))?;

    std::fs::write(a2l_path, new_content).map_err(|e| format!("写入 A2L 文件失败: {}", e))?;

    let deleted_count = names.len();

    // 重新加载 A2L
    let content =
        std::fs::read_to_string(a2l_path).map_err(|e| format!("重新读取 A2L 失败: {}", e))?;
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
            {
                Some(VariableChanges {
                    name: e.name,
                    address: e.address,
                    data_type: e.data_type,
                    var_type: e.var_type,
                    bit_mask: e.bit_mask,
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
            }),
            export_mode: e.export_mode,
        })
        .collect();

    let content =
        std::fs::read_to_string(a2l_path).map_err(|e| format!("读取 A2L 文件失败: {}", e))?;

    let (new_content, result) = A2lGenerator::apply_changes(&content, &variable_edits)
        .map_err(|e| format!("应用变更失败: {}", e))?;

    std::fs::write(a2l_path, new_content).map_err(|e| format!("写入 A2L 文件失败: {}", e))?;

    let variables = A2lParser::parse_all_variables(
        &std::fs::read_to_string(a2l_path).map_err(|e| format!("重新读取 A2L 失败: {}", e))?,
    );
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
