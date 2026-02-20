# AGENTS.md - A2L Editor 项目指南

## 项目概述

A2L Editor 是一个从 ELF/DWARF 调试信息生成 A2L 文件的桌面工具。
- 后端: Rust (Tauri + 核心库)
- 前端: Svelte 5 + TypeScript + Tailwind CSS

## 构建命令

### Rust 核心库和 CLI
```bash
# 开发构建
cargo build

# 生产构建
cargo build --release

# 运行所有测试
cargo test

# 运行单个测试
cargo test test_format_file_size
cargo test --package a2l-editor test_name

# 运行 CLI 工具
cargo run --bin a2l-cli -- --help
cargo run --bin a2l-cli -- parse firmware.elf --deep
```

### Tauri 应用
```bash
# 安装依赖
npm install

# 开发模式 (热重载)
npm run tauri dev

# 生产构建
npm run tauri build

# 仅构建前端
npm run build

# 前端类型检查 (在 src-ui 目录下)
cd src-ui && npm run check
```

### Linux 系统依赖
```bash
sudo apt-get install libgtk-3-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev
```

## 项目结构

```
a2l-editor/
├── src/lib/           # Rust 核心库
│   ├── mod.rs         # 模块导出
│   ├── types.rs       # 类型定义 (Variable, TypeInfo, A2lEntry 等)
│   ├── a2l.rs         # A2L 生成器、解析器和编辑器
│   ├── elf.rs         # ELF 解析
│   ├── dwarf.rs       # DWARF 调试信息解析
│   ├── cache.rs       # SQLite 缓存
│   ├── data_package.rs # .a2ldata 数据包
│   └── hash.rs        # 文件哈希计算
├── src/bin/           # CLI 工具
│   └── a2l_cli.rs     # 命令行入口
├── src-tauri/         # Tauri 后端
│   ├── src/commands.rs # Tauri 命令 (IPC 接口)
│   └── capabilities/  # Tauri 2.x 权限配置
└── src-ui/            # Svelte 前端
    └── src/lib/       # 前端库
        ├── types.ts   # TypeScript 类型定义
        ├── commands.ts # Tauri 命令调用
        ├── stores.ts  # Svelte stores (状态管理)
        ├── themes.ts  # 主题管理
        └── components/ # Svelte 组件
```

## 核心功能

### A2L 变量添加
- **从 ELF 添加**: 右键选中 ELF 变量 → 添加为观测变量/标定变量
- **手动添加**: 点击 A2L 面板搜索栏右侧 ➕ 按钮，输入变量名、地址、数据类型
- 手动添加支持实时检测变量名是否重复

### A2L 变量编辑
- 在 A2L 面板底部有可拖拽调整大小的编辑区域（拖拽分隔条调整高度）
- 选中单个变量后可编辑：名称、地址、数据类型、BIT_MASK
- 点击"保存"按钮立即写入 A2L 文件

### 复制功能
- 右键菜单支持复制变量名称到剪贴板
- 右键菜单支持复制变量地址到剪贴板

### 多列排序
- 点击列头进行单列排序
- 按住 Shift + 点击列头添加多列排序
- 排序指示器显示优先级数字（如 ¹, ²）

### 列宽调整
- 拖拽列标题之间的分隔线可调整列宽

### 实时保存
- 所有操作（编辑、添加、删除）立即写入文件
- 编辑变量：点击保存按钮后立即生效
- 添加变量：右键添加或手动添加后立即写入
- 删除变量：右键删除后立即生效

### 字节序设置
- Header 右侧有"小端/大端"切换按钮
- 全局设置，存储在后端 AppState.endianness
- 不持久化，每次启动默认小端

### 主题系统
- 支持 4 种主题：Dark, Light, Midnight, Ocean
- 主题设置保存到 localStorage，自动加载

## Rust 代码风格

### 命名约定
- 变量/函数: `snake_case`
- 类型/结构体/枚举: `PascalCase`
- 常量: `SCREAMING_SNAKE_CASE`
- 模块: `snake_case`

### 导入顺序
```rust
// 1. 标准库
use std::collections::HashSet;
use std::path::PathBuf;

// 2. 外部 crate
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

// 3. 内部模块
use crate::types::{A2lEntry, Variable};
```

### 错误处理
- 使用 `anyhow::Result` 作为返回类型
- 使用 `.context()` 添加错误上下文:
```rust
std::fs::read_to_string(path)
    .with_context(|| format!("无法读取文件: {}", path.display()))?;
```
- Tauri 命令返回 `Result<T, String>`, 使用 `.map_err(|e| e.to_string())?`

### 结构体定义
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    pub name: String,
    pub address: u64,
    pub size: usize,
    pub type_name: String,
    pub section: String,
    pub type_info: Option<TypeInfo>,
}
```

### Builder 模式
```rust
impl StructMember {
    pub fn new(name: String, offset: usize, type_name: String, type_size: usize) -> Self {
        Self { name, offset, type_name, type_size, type_offset: None, bit_offset: None, bit_size: None }
    }

    pub fn with_bitfield(mut self, bit_offset: usize, bit_size: usize) -> Self {
        self.bit_offset = Some(bit_offset);
        self.bit_size = Some(bit_size);
        self
    }
}
```

### 常量定义
```rust
pub const MAX_ARRAY_EXPAND: usize = 1000;
pub const MAX_NESTING_DEPTH: usize = 50;
```

## TypeScript/Svelte 代码风格

### 命名约定
- 变量/函数: `camelCase`
- 类型/接口: `PascalCase`
- 组件文件: `PascalCase.svelte`
- Store 变量: `camelCase`

### 导入顺序
```typescript
// 1. Svelte 内置
import { writable, derived } from 'svelte/store';

// 2. 外部库
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';

// 3. 内部模块 ($lib 别名)
import { currentTheme } from '$lib/stores';
import type { A2lEntry } from '$lib/types';
```

### 类型定义
```typescript
export interface A2lEntry {
  index: number;
  full_name: string;
  address: number;
  size: number;
  a2l_type: string;
  type_name: string;
  bit_offset: number | null;
  bit_size: number | null;
}

export type ExportMode = 'measurement' | 'characteristic';

// 编辑操作类型（用于 saveA2lChanges）
export type EditActionType = 'modify' | 'delete' | 'add';

export interface A2lVariableEdit {
  action: EditActionType;
  originalName: string;
  name?: string;
  address?: string;
  data_type?: string;
  var_type?: 'MEASUREMENT' | 'CHARACTERISTIC';
  bit_mask?: string;
  entry?: A2lEntry;
  exportMode?: ExportMode;
}
```

### Svelte 5 Runes
```svelte
<script lang="ts">
  let count = $state(0);
  let doubled = $derived(count * 2);
  
  $effect(() => {
    console.log('count changed:', count);
  });
</script>
```

### Store 使用
```typescript
// 定义 store
export const elfEntries = writable<A2lEntry[]>([]);
export const endianness = writable<'little' | 'big'>('little');

// 在组件中使用
import { elfEntries, endianness } from '$lib/stores';
// $elfEntries, $endianness 自动订阅
```

### 异步函数
```typescript
export async function loadElf(path: string): Promise<LoadResult> {
  return invoke('load_elf', { path });
}

export async function loadPackage(path: string): Promise<LoadResult> {
  return invoke('load_package', { path });
}

export async function saveA2lChanges(edits: A2lVariableEdit[]): Promise<SaveResult> {
  return invoke('save_a2l_changes', { edits });
}

export async function setEndianness(endianness: 'little' | 'big'): Promise<void> {
  return invoke('set_endianness', { endianness });
}
```

## Tauri 命令约定

### Rust 端完整命令列表
```rust
// 文件加载
#[tauri::command]
pub fn load_elf(path: String, state: State<Mutex<AppState>>) -> Result<LoadResult, String>

#[tauri::command]
pub fn load_package(path: String, state: State<Mutex<AppState>>) -> Result<LoadResult, String>

#[tauri::command]
pub fn generate_package(elf_path: String, output_path: Option<String>, state: State<Mutex<AppState>>) -> Result<PackageMetaInfo, String>

#[tauri::command]
pub fn load_a2l(path: String, state: State<Mutex<AppState>>) -> Result<A2lLoadResult, String>

// 条目查询
#[tauri::command]
pub fn search_elf_entries(query: String, offset: usize, limit: usize, sort_field: Option<String>, sort_order: Option<String>, state: State<Mutex<AppState>>) -> Result<Vec<EntryInfo>, String>

#[tauri::command]
pub fn get_elf_count(state: State<Mutex<AppState>>) -> Result<usize, String>

#[tauri::command]
pub fn search_a2l_variables(query: String, offset: usize, limit: usize, state: State<Mutex<AppState>>) -> Result<Vec<VariableInfo>, String>

// 导出/编辑
#[tauri::command]
pub fn export_entries(indices: Vec<usize>, mode: String, state: State<Mutex<AppState>>) -> Result<ExportResult, String>

#[tauri::command]
pub fn delete_variables(names: Vec<String>, state: State<Mutex<AppState>>) -> Result<usize, String>

#[tauri::command]
pub fn save_a2l_changes(edits: Vec<VariableEditInput>, state: State<Mutex<AppState>>) -> Result<SaveResult, String>

// 设置
#[tauri::command]
pub fn set_endianness(endianness: String, state: State<Mutex<AppState>>) -> Result<(), String>
```

### TypeScript 端调用
```typescript
export async function searchElfEntries(
  query: string, 
  offset = 0, 
  limit = 10000,
  sortField: 'name' | 'address' = 'name',
  sortOrder: 'asc' | 'desc' = 'asc'
): Promise<A2lEntry[]> {
  return invoke('search_elf_entries', { query, offset, limit, sortField, sortOrder });
}
```

## 前端组件说明

### 主要组件
- `A2lPanel.svelte`: A2L 变量列表面板，含搜索、排序、编辑区域
- `VariableList.svelte`: ELF 变量列表面板，虚拟滚动
- `VirtualList.svelte`: 通用虚拟滚动组件，支持 `scrollToIndex()` 方法
- `A2lEditor.svelte`: 变量编辑表单
- `AddVariableDialog.svelte`: 手动添加变量对话框
- `ContextMenuA2l.svelte`: A2L 右键菜单（删除、复制）
- `ContextMenuElf.svelte`: ELF 右键菜单（导出、复制）
- `Header.svelte`: 顶部导航栏

### UI 交互特性
- 列宽拖拽调整：通过 `.col-resize` 分隔线
- 编辑区域高度拖拽：通过 `.editor-resizer` 分隔条
- 多列排序：`toggleSort()` 函数支持 Shift 键添加排序列
- 虚拟滚动：`VirtualList` 使用 ResizeObserver 监听容器大小

## 测试

### Rust 测试
- 测试位于源文件内的 `#[cfg(test)]` 模块
- 运行: `cargo test`
- 运行单个测试: `cargo test test_name`

### 前端测试
- 暂无测试框架配置

## 注释

- 代码注释使用中文（与现有代码一致）
- 公共 API 应有文档注释
- 避免无用注释，代码应自解释

## 提交前检查

1. Rust 代码: `cargo build && cargo test`
2. 前端代码: `cd src-ui && npm run check`
3. 完整构建: `npm run tauri build`
