# A2L Editor Tauri 版本 - 项目设计文档

## 1. 项目概述

### 1.1 背景
A2L Editor 是一个从 ELF/DWARF 调试信息生成 A2L 文件的桌面工具。使用 Rust + Tauri 2 + Svelte 5 技术栈。

### 1.2 目标
- 现代化 UI 设计，简洁美观
- 流畅的交互体验（右键菜单、主题切换）
- 支持大规模数据（10万+变量）

### 1.3 范围
| 项目 | 范围 |
|------|------|
| 核心库 (src/lib/) | 核心功能，稳定 |
| CLI (src/bin/a2l_cli.rs) | 命令行工具 |
| 前端 (src-ui/) | Svelte + TailwindCSS |
| 后端 (src-tauri/) | Tauri 2 Commands |

---

## 2. 技术架构

### 2.1 整体架构

```
┌─────────────────────────────────────────────────────────────┐
│                      用户界面层                              │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Svelte + TailwindCSS                    │   │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌──────────┐  │   │
│  │  │ Header  │ │ A2lPanel│ │VarList  │ │ Dialogs  │  │   │
│  │  └─────────┘ └─────────┘ └─────────┘ └──────────┘  │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                           │
                           │ Tauri IPC (invoke)
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                      Tauri 后端层                            │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              commands.rs (Tauri Commands)            │   │
│  │  load_elf │ load_package │ search_entries │ export  │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                           │
                           │ 函数调用
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                      核心库层 (lib/)                         │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐          │
│  │ elf.rs  │ │dwarf.rs │ │ a2l.rs  │ │data_pkg │          │
│  └─────────┘ └─────────┘ └─────────┘ └─────────┘          │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 技术选型

| 层级 | 技术 | 版本 | 原因 |
|------|------|------|------|
| 前端框架 | Svelte | 5.x | 简洁、高性能、学习成本低 |
| UI 组件 | shadcn-svelte | latest | 无样式组件，高度可定制 |
| 样式方案 | TailwindCSS | 3.x | 原子化 CSS，快速开发 |
| 桌面框架 | Tauri | 2.x | 包体小、性能好、Rust 原生 |
| 状态管理 | Svelte Stores | 内置 | 简单高效 |
| 类型系统 | TypeScript | 5.x | 类型安全 |

### 2.3 目录结构

```
a2l-editor/
├── Cargo.toml                    # workspace 配置
├── package.json                  # npm workspace
├── docs/                         # 文档目录
│   ├── DESIGN.md                 # 项目设计文档
│   ├── PLAN.md                   # 项目计划文档
│   ├── TASKS.md                  # 任务清单
│   └── HANDOVER_TAURI.md         # Tauri 版本交接文档
│
├── src/                          # Rust 源码 (共用)
│   ├── main.rs                   # egui 入口 (保留)
│   ├── main_tauri.rs             # Tauri 入口 (新增)
│   ├── commands.rs               # Tauri Commands (新增)
│   ├── lib/                      # 核心库 (不修改)
│   │   ├── mod.rs
│   │   ├── a2l.rs
│   │   ├── cache.rs
│   │   ├── data_package.rs
│   │   ├── dwarf.rs
│   │   ├── elf.rs
│   │   ├── hash.rs
│   │   └── types.rs
│   └── app/                      # egui UI (保留)
│       ├── mod.rs
│       └── ui/
│
├── src-tauri/                    # Tauri 配置 (新增)
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── capabilities/
│   └── icons/
│
└── src-ui/                       # Svelte 前端 (新增)
    ├── package.json
    ├── svelte.config.js
    ├── tailwind.config.js
    ├── vite.config.ts
    ├── tsconfig.json
    ├── static/
    │   └── fonts/
    │       └── NotoSansSC-Regular.otf
    └── src/
        ├── app.html
        ├── app.css
        ├── app.d.ts
        ├── lib/
        │   ├── commands.ts       # Tauri API 封装
        │   ├── stores.ts         # Svelte stores
        │   ├── themes.ts         # 主题配置
        │   ├── types.ts          # TypeScript 类型
        │   └── components/
        │       ├── ui/           # shadcn-svelte 组件
        │       │   ├── button/
        │       │   ├── dialog/
        │       │   ├── input/
        │       │   └── scroll-area/
        │       ├── Header.svelte
        │       ├── FileInfo.svelte
        │       ├── VariableList.svelte
        │       ├── A2lPanel.svelte
        │       ├── ContextMenuElf.svelte
        │       ├── ContextMenuA2l.svelte
        │       ├── StatusBar.svelte
        │       ├── ExportDialog.svelte
        │       ├── GenerateDialog.svelte
        │       ├── AboutDialog.svelte
        │       └── ThemeSwitch.svelte
        └── routes/
            ├── +layout.svelte
            └── +page.svelte
```

---

## 3. UI 设计

### 3.1 整体布局

```
┌──────────────────────────────────────────────────────────────────────────┐
│  ┌─────────────────────┐  ┌────────┐  ┌────────┐       🎨  v0.1.0      │
│  │ 📁 文件         ▼   │  │ ❓ 手册 │  │ ℹ️ 关于 │                        │
│  └─────────────────────┘  └────────┘  └────────┘                        │
├──────────────────────────────────────────────────────────────────────────┤
│  📂 ELF: /path/to/firmware.elf (437 MB, 133,646 条目)       [导入]      │
│  📦 数据包: /path/to/firmware.elf.a2ldata                   [导入]      │
│  📄 A2L: /path/to/output.a2l (1,234 个变量)                 [导入]      │
├──────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌──────────────── A2L 变量 ────────────────┐ ┌─────── ELF 变量 ───────┐│
│  │ 🔍 [搜索 A2L 变量...              ] [✖] │ │ 🔍 [搜索 ELF 变量...] ││
│  │ ─────────────────────────────────────── │ │ ──────────────────────││
│  │ 变量名            类型      地址        │ │ 变量名    类型   地址  ││
│  │ ─────────────────────────────────────── │ │ ──────────────────────││
│  │ var1              ULONG    0x70000000   │ │ var_name  ULONG  0x.. ││
│  │ var2              FLOAT    0x70000004   │ │ another   SWORD  0x.. ││  ← 选中高亮
│  │ var3              UWORD    0x70000008   │ │ existing  FLOAT  0x.. ││  ← 已存在变淡
│  │ ...                                     │ │ ...                   ││
│  │                                         │ │                       ││
│  │ 显示: 1,234                             │ │ 显示: 1,234 / 133,646 ││
│  └─────────────────────────────────────────┘ └───────────────────────┘│
│                                                                          │
├──────────────────────────────────────────────────────────────────────────┤
│  💡 单击选择变量，右键打开菜单                                            │
└──────────────────────────────────────────────────────────────────────────┘
```

### 3.2 顶部菜单栏

```
┌──────────────────────────────────────────────────────────────────────────┐
│  ┌─────────────────────┐  ┌────────┐  ┌────────┐       🎨  v0.1.0      │
│  │ 📁 文件         ▼   │  │ ❓ 手册 │  │ ℹ️ 关于 │                        │
│  └─────────────────────┘  └────────┘  └────────┘                        │
└──────────────────────────────────────────────────────────────────────────┘

文件下拉菜单：
┌─────────────────────┐
│ 📂 打开 ELF...      │
│ 📦 打开数据包...    │
│ 📄 选择目标 A2L...  │
│ ─────────────────── │
│ 🔄 重新生成缓存     │
└─────────────────────┘

手册按钮：
- 打开 README.md 或在线文档

关于对话框：
┌────────────────────────────────────────┐
│  关于 A2L Editor                 [✖]  │
├────────────────────────────────────────┤
│                                        │
│           A2L Editor                   │
│           版本 0.1.0                   │
│                                        │
│  从 ELF/DWARF 生成 A2L 文件的桌面工具  │
│                                        │
│  技术栈: Rust + Tauri + Svelte         │
│                                        │
│  仓库: github.com/horinen/a2l-editor   │
│                                        │
├────────────────────────────────────────┤
│                              [确定]    │
└────────────────────────────────────────┘
```

### 3.3 文件信息行

```
┌──────────────────────────────────────────────────────────────────────────┐
│  📂 ELF: /path/to/firmware.elf (437 MB, 133,646 条目)       [导入]      │
│  📦 数据包: /path/to/firmware.elf.a2ldata                   [导入]      │
│  📄 A2L: /path/to/output.a2l (1,234 个变量)                 [导入]      │
└──────────────────────────────────────────────────────────────────────────┘
```

**状态显示**：

| 文件 | 未加载 | 已加载 |
|------|--------|--------|
| ELF | `📂 ELF: 未选择 [导入]` | `📂 ELF: firmware.elf (437 MB, 133,646 条目) [导入]` |
| 数据包 | `📦 数据包: 未选择 [导入]` | `📦 数据包: firmware.a2ldata [导入]` |
| A2L | `📄 A2L: 未选择 [导入]` | `📄 A2L: output.a2l (1,234 个变量) [导入]` |

### 3.4 双面板布局

**左侧 A2L 变量面板** (目标文件中的变量)：
```
┌──────────────── A2L 变量 ────────────────┐
│ 🔍 [________________________] [✖]       │
│ ─────────────────────────────────────── │
│    │ 变量名 ¹▲    类型    │ 地址 ²▼    │  ← 可排序的表头
│ ─────────────────────────────────────── │
│ 📊 │ calib_value  UBYTE   │ 0x70000010 │
│ 📊 │ sensor_temp SWORD   │ 0x70000014 │  ← 选中高亮
│ 📈 │ output_val  FLOAT   │ 0x70000018 │  ← 📈=标定, 📊=观测
│ ...                                     │
│                                         │
│ 显示: 1,234                             │
└─────────────────────────────────────────┘
```

**右侧 ELF 变量面板** (可添加的变量)：
```
┌──────────────── ELF 变量 ───────────────┐
│ 🔍 [________________________] [✖]       │
│ ─────────────────────────────────────── │
│ 变量名 ¹▲      类型      │ 地址 ²▼     │  ← 可排序的表头
│ ─────────────────────────────────────── │
│ var_name      ULONG     │ 0x70000000   │
│ another_var   SWORD     │ 0x70000004   │  ← 选中高亮
│ existing_var  FLOAT     │ 0x70000008   │  ← 已存在变淡
│ ...                                     │
│                                         │
│ 显示: 1,234 / 133,646                   │
└─────────────────────────────────────────┘
```

### 3.4.1 排序功能

**表头排序交互**：
```
点击列标题行为：
┌─────────────────────────────────────┐
│ 未排序 → 升序 (▲) → 降序 (▼) → 未排序 │
└─────────────────────────────────────┘

多列排序示例：
┌────────────────────────────────────────────┐
│    │ 变量名 ¹▲ │ 类型 ²▲ │ 地址          │
└────────────────────────────────────────────┘
↑ ¹ 表示第一排序优先级
↑ ² 表示第二排序优先级
```

**排序规则**：
- 默认按变量名升序排序
- 点击列标题添加/切换/移除排序
- 支持 Shift+点击添加第二排序条件
- 数字上标显示排序优先级（¹²³）
- 地址按十六进制数值排序

### 3.4.2 添加变量后自动定位

**流程**：
```
1. 用户在 ELF 面板选择变量
2. 右键 → 添加为观测/标定变量
3. 导出成功后：
   a. 重新加载 A2L 变量列表
   b. 滚动到第一个新添加的变量
   c. 高亮显示（可选）
```

### 3.4.3 已解决问题

**问题 #6: Shift 多选选中的变量不对** ✅ 已修复

| 项目 | 描述 |
|------|------|
| 问题 | Shift 多选时选中的变量范围与点击的变量不一致 |
| 原因 | 排序后，`displayEntries` 的索引与 `elfSelectedIndices` 存储的原始索引不一致 |
| 解决方案 | 使用显示位置索引 (`displayIndex`) 替代原始索引，存储在 `lastElfSelectedDisplayPos` 中 |

**问题 #7: ELF 排序只对已加载的10000个变量生效** ✅ 已修复

| 项目 | 描述 |
|------|------|
| 问题 | 更改排序后，只有当前已加载的10000个变量被排序 |
| 原因 | 前端只加载了10000个变量，排序仅在这10000个中进行 |
| 解决方案 | 后端 `search_elf_entries` 支持排序参数（`sort_field`, `sort_order`），在数据库层面排序后返回 |

### 3.5 右键菜单

**左侧 A2L 变量菜单**：
```
┌──────────────────────┐
│ 🗑 删除变量          │
│──────────────────────│
│ 📋 复制名称          │
│ 📋 复制地址          │
│──────────────────────│
│ ✖ 取消选择          │
└──────────────────────┘
```

**右侧 ELF 变量菜单**：
```
┌──────────────────────┐
│ 📊 添加为观测变量    │  ← A2L 未选择时置灰
│ 📈 添加为标定变量    │  ← A2L 未选择时置灰
│──────────────────────│
│ 📋 复制名称          │
│ 📋 复制地址          │
│──────────────────────│
│ ✖ 取消选择          │
└──────────────────────┘
```

### 3.6 底部状态栏 (动态提示式)

| 当前状态 | 显示内容 |
|----------|----------|
| 刚启动，无文件 | `💡 文件 → 打开 ELF 开始使用` |
| ELF 已加载，无选中 | `💡 单击选择变量，右键打开菜单` |
| ELF 已选中，A2L 未选择 | `⚠️ 请先选择目标 A2L 文件` |
| ELF 已选中，A2L 已选择 | `💡 右键 → 添加为观测/标定变量` |
| A2L 已选中 | `💡 右键 → 删除所选变量` |
| 加载中 | `⏳ 正在加载 firmware.elf...` |
| 添加成功 | `✅ 已添加 3 个变量到 output.a2l` |
| 删除成功 | `✅ 已从 output.a2l 删除 2 个变量` |
| 操作失败 | `❌ 导出失败: 文件被占用` |

### 3.7 交互设计

| 交互 | 行为 |
|------|------|
| 单击行 | 单选（清除其他选中） |
| Ctrl + 单击 | 多选/取消选中 |
| Shift + 单击 | 范围选择 |
| 右键行 | 打开上下文菜单 |
| Ctrl + A | 全选当前筛选结果 |
| ↑ / ↓ 键 | 键盘导航 |
| 搜索框 | 实时搜索 + 防抖 (300ms) |
| 点击表头 | 切换排序（未排序→升序→降序→未排序） |
| Shift + 点击表头 | 添加第二排序条件 |

### 3.8 排序交互详情

**单列排序**：
1. 点击列标题 → 按该列升序排序
2. 再次点击 → 变为降序
3. 再次点击 → 取消排序（恢复默认顺序）

**多列排序**：
1. 点击列 A → 按列 A 排序
2. Shift + 点击列 B → 先按列 A，再按列 B 排序
3. 数字上标显示优先级（¹²³）

**默认行为**：
- A2L 和 ELF 面板默认都按变量名升序排序
- 排序状态不持久化（刷新后恢复默认）

### 3.8 视觉反馈

| 状态 | 样式 |
|------|------|
| 未选中 | 透明背景 |
| 悬停 | `bg-hover` 背景色 |
| 选中 | `bg-selected` 背景色 + 左侧蓝色边框 |
| 已存在于 A2L | 文字颜色变淡 (`text-muted`) |

### 3.9 主题系统

| 主题 | 背景色 | 文字色 | 强调色 |
|------|--------|--------|--------|
| Dark (默认) | `#0f0f12` | `#e4e4e7` | `#3b82f6` |
| Light | `#ffffff` | `#18181b` | `#3b82f6` |
| Midnight | `#000000` | `#fafafa` | `#3b82f6` |
| Ocean | `#0c1222` | `#e0f2fe` | `#06b6d4` |

---

## 4. 数据流设计

### 4.1 状态管理

```typescript
// stores.ts

// ELF 变量
export const elfEntries = writable<A2lEntry[]>([]);
export const elfFilteredCount = writable<number>(0);
export const elfTotalCount = writable<number>(0);
export const elfSearchQuery = writable<string>('');
export const elfSelectedIndices = writable<Set<number>>(new Set());

// A2L 变量
export const a2lVariables = writable<A2lVariable[]>([]);
export const a2lSearchQuery = writable<string>('');
export const a2lSelectedIndices = writable<Set<number>>(new Set());

// 文件状态
export const elfPath = writable<string | null>(null);
export const packagePath = writable<string | null>(null);
export const a2lPath = writable<string | null>(null);
export const a2lNames = writable<Set<string>>(new Set());

// 应用状态
export const statusMessage = writable<string>('💡 文件 → 打开 ELF 开始使用');
export const isLoading = writable<boolean>(false);

// 主题
export const currentTheme = writable<string>('dark');

// 派生状态
export const elfSelectedCount = derived(elfSelectedIndices, $set => $set.size);
export const a2lSelectedCount = derived(a2lSelectedIndices, $set => $set.size);

// 排序状态
export type SortField = 'name' | 'address';
export type SortOrder = 'asc' | 'desc';
export interface SortConfig {
  field: SortField;
  order: SortOrder;
}

// 默认按名称升序排序
export const a2lSortConfigs = writable<SortConfig[]>([{ field: 'name', order: 'asc' }]);
export const elfSortConfigs = writable<SortConfig[]>([{ field: 'name', order: 'asc' }]);

// 排序工具函数
export function toggleSort(configs: SortConfig[], field: SortField, shiftKey: boolean): SortConfig[];
export function applySorting<T>(items: T[], configs: SortConfig[], getFieldValue: (item: T, field: SortField) => string | number): T[];
```

### 4.2 Tauri Commands

```rust
// commands.rs

// 文件操作
#[tauri::command] fn load_elf(path: String) -> Result<LoadResult, String>;
#[tauri::command] fn load_package(path: String) -> Result<LoadResult, String>;
#[tauri::command] fn generate_package(elf: String, output: Option<String>) -> Result<PackageMeta, String>;
#[tauri::command] fn load_a2l(path: String) -> Result<A2lLoadResult, String>;

// 变量查询
#[tauri::command] fn search_elf_entries(query: String, offset: usize, limit: usize) -> Vec<A2lEntry>;
#[tauri::command] fn get_elf_count() -> usize;
#[tauri::command] fn search_a2l_variables(query: String, offset: usize, limit: usize) -> Vec<A2lVariable>;

// 导出/删除
#[tauri::command] fn export_entries(indices: Vec<usize>, mode: String) -> Result<ExportResult, String>;
#[tauri::command] fn delete_variables(indices: Vec<usize>) -> Result<usize, String>;
```

### 4.3 数据类型

```typescript
// types.ts

interface A2lEntry {
  index: number;
  full_name: string;
  address: number;
  size: number;
  a2l_type: string;
  type_name: string;
  bit_offset: number | null;
  bit_size: number | null;
}

interface A2lVariable {
  name: string;
  address: string | null;
  data_type: string;     // ULONG, FLOAT32_IEEE 等
  var_type: 'MEASUREMENT' | 'CHARACTERISTIC';  // 观测/标定类型
  bit_mask: string | null;
  compu_method: string | null;  // 引用的 COMPU_METHOD 名称
  f: number | null;             // 斜率系数
  offset: number | null;        // 偏移量
  unit: string | null;          // 物理单位
}

interface LoadResult {
  meta: PackageMeta;
  entry_count: number;
}

interface PackageMeta {
  file_name: string;
  elf_path: string | null;
  entry_count: number;
  created_at: number;
}

interface A2lLoadResult {
  path: string;
  variable_count: number;
  existing_names: string[];
}

interface ExportResult {
  added: number;
  skipped: number;
  existing: number;
}

// 排序相关类型
type SortField = 'name' | 'address';
type SortOrder = 'asc' | 'desc';
interface SortConfig {
  field: SortField;
  order: SortOrder;
}
```

---

## 5. 对话框设计

### 5.1 导出确认对话框

```
┌────────────────────────────────────────┐
│  导出到 A2L                      [✖]  │
├────────────────────────────────────────┤
│                                        │
│  将 3 个变量添加为观测变量             │
│                                        │
│  目标文件: output.a2l                  │
│                                        │
│  ┌────────────────────────────────┐   │
│  │ 已存在: 1,200                  │   │
│  │ 新增: 2                        │   │
│  │ 跳过: 1 (已存在于 A2L)         │   │
│  └────────────────────────────────┘   │
│                                        │
├────────────────────────────────────────┤
│              [取消]  [确认追加]        │
└────────────────────────────────────────┘
```

### 5.2 生成数据包对话框

```
┌────────────────────────────────────────┐
│  生成数据包                      [✖]  │
├────────────────────────────────────────┤
│                                        │
│  ELF 文件: firmware.elf (437 MB)       │
│                                        │
│  数据包将保存到:                       │
│  /path/to/firmware.elf.a2ldata        │
│                                        │
│  ⚠ 首次解析大型 ELF 可能需要几分钟     │
│                                        │
├────────────────────────────────────────┤
│        [选择其他位置]  [生成]          │
└────────────────────────────────────────┘
```

---

## 6. 构建与部署

### 6.1 开发命令

```bash
# 安装依赖
npm install

# 开发模式 (热重载)
npm run tauri dev

# 构建 egui 版本 (旧)
cargo run --bin a2l-editor

# 构建 Tauri 版本 (新)
npm run tauri build
```

### 6.2 构建产物

| 版本 | 命令 | 产物 |
|------|------|------|
| egui | `cargo build --release` | `target/release/a2l-editor` |
| Tauri | `npm run tauri build` | `src-tauri/target/release/bundle/` |

### 6.3 支持平台

| 平台 | 格式 |
|------|------|
| Linux | .deb, .AppImage |
| Windows | .msi, .exe |
| macOS | .dmg, .app |
