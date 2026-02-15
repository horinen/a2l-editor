# A2L Editor Tauri 版本 - 交接文档

## 基本信息

| 项目 | 信息 |
|------|------|
| 位置 | `/home/hori/learn/github/a2l-editor` |
| 仓库 | https://github.com/horinen/a2l-editor
| 当前版本 | v0.0.9 |
| 技术栈 | Rust + Tauri 2 + Svelte 5 + TailwindCSS |
| 状态 | ✅ 完成，所有已知 Bug 已修复 |

---

## 版本历史

| 版本 | 说明 |
|------|------|
| v0.0.9 | 移除 egui，保留 Tauri 版本；修复 Shift 多选和后端排序问题 |
| v0.0.8 | Tauri UI 完成，Playwright 测试 100% 通过 |
| v0.0.4 | egui 版本 (已移除) |

---

## 测试状态

### Playwright 自动化测试 (2026-02-13)

| 测试项 | 状态 |
|--------|------|
| 测试用例 | 9 |
| 通过 | 9 ✅ |
| 失败 | 0 |
| 通过率 | **100%** ✅ |

**测试覆盖**: 布局、主题、搜索、菜单、对话框、键盘导航

**详细报告**: 
- `docs/UI_TEST_REPORT_PLAYWRIGHT.md`
- `src-ui/playwright-report/index.html`

---

## 测试经验总结

### 推荐的 UI 测试方案

| 方案 | 推荐度 | 说明 |
|------|--------|------|
| **Playwright** | ⭐⭐⭐⭐⭐ | 最佳选择，完整交互测试，生成报告 |
| Tauri WebDriver | ⭐⭐⭐⭐ | 完整应用测试，配置复杂 |
| Xvfb + xdotool | ⭐⭐⭐ | 虚拟 X11，需要窗口管理器 |
| ydotool | ⭐⭐ | Wayland 限制多 |

### 测试工具对比

| 工具 | 优点 | 缺点 |
|------|------|------|
| Playwright | 跨浏览器、并行测试、截图/视频 | 仅测试前端 |
| ydotool | 真实桌面测试 | Wayland 不支持虚拟键盘 |
| gnome-screenshot + OCR | 简单、无依赖 | 无法交互 |

**详细指南**: `docs/UI_TESTING_GUIDE.md`

---

## 快速开始

```bash
cd /home/hori/learn/github/a2l-editor

# 安装依赖
npm install

# 开发模式 (Tauri)
npm run tauri dev

# 开发模式 (仅前端)
npm run dev

# CLI 工具
cargo run --bin a2l-cli -- --help

# 构建
npm run build          # 前端
npm run tauri build    # 完整应用

# 运行测试
cd src-ui
npx playwright test              # 运行测试
npx playwright show-report       # 查看报告
```

# 构建 Tauri 版本 (新)
npm run tauri build
```

---

## 项目结构

```
a2l-editor/
├── Cargo.toml                    # workspace + CLI binary
├── package.json                  # npm workspace
├── docs/                         # 文档目录
│   ├── DESIGN.md                 # 项目设计文档
│   ├── PLAN.md                   # 项目计划文档
│   ├── TASKS.md                  # 任务清单 (100% 完成)
│   └── HANDOVER_TAURI.md         # 本文档
│
├── src/                          # Rust 源码
│   ├── lib/                      # 核心库
│   │   ├── mod.rs
│   │   ├── a2l.rs
│   │   ├── cache.rs
│   │   ├── data_package.rs
│   │   ├── dwarf.rs
│   │   ├── elf.rs
│   │   ├── hash.rs
│   │   └── types.rs
│   └── bin/
│       └── a2l_cli.rs            # CLI 工具 (parse, export, create-package 等)
│
├── src-tauri/                    # Tauri 配置
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── capabilities/default.json
│   ├── icons/                    # placeholder icons
│   └── src/
│       ├── main.rs -> ../../src/main_tauri.rs (符号链接)
│       └── commands.rs           # Tauri Commands 实现
│
└── src-ui/                       # Svelte 前端
    ├── package.json
    ├── svelte.config.js
    ├── tailwind.config.js
    ├── vite.config.ts
    ├── tsconfig.json
    ├── static/
    │   └── favicon.png
    └── src/
        ├── app.html
        ├── app.css               # 全局样式 + 4 主题 CSS 变量
        ├── lib/
        │   ├── commands.ts       # Tauri API 封装
        │   ├── stores.ts         # Svelte stores
        │   ├── themes.ts         # 4 主题配置
        │   ├── types.ts          # TypeScript 类型
        │   ├── utils/
        │   │   └── debounce.ts   # 防抖/节流工具
        │   └── components/
        │       ├── Header.svelte
        │       ├── FileInfo.svelte
        │       ├── A2lPanel.svelte
        │       ├── VariableList.svelte
        │       ├── VirtualList.svelte      # 虚拟滚动
        │       ├── StatusBar.svelte
        │       ├── ContextMenuA2l.svelte
        │       ├── ContextMenuElf.svelte
        │       ├── ExportDialog.svelte
        │       ├── GenerateDialog.svelte
        │       ├── AboutDialog.svelte
        │       ├── LoadingOverlay.svelte   # 加载动画
        │       └── VariableDetail.svelte   # 变量详情
        └── routes/
            ├── +layout.svelte
            ├── +layout.ts
            └── +page.svelte
```

---

## 可用版本

| 版本 | 入口 | 构建命令 | 说明 |
|------|------|----------|------|
| Tauri GUI | `src/main_tauri.rs` | `npm run tauri dev` | 桌面应用 |
| CLI | `src/bin/a2l_cli.rs` | `cargo run --bin a2l-cli` | 命令行工具 |

---

## UI 布局

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
│  │ var2              FLOAT    0x70000004   │ │ another   SWORD  0x.. ││
│  │ var3              UWORD    0x70000008   │ │ existing  FLOAT  0x.. ││
│  │ ...                                     │ │ ...                   ││
│  │ 显示: 1,234                             │ │ 显示: 1,234 / 133,646 ││
│  └─────────────────────────────────────────┘ └───────────────────────┘│
│                                                                          │
├──────────────────────────────────────────────────────────────────────────┤
│  💡 单击选择变量，右键打开菜单                                            │
└──────────────────────────────────────────────────────────────────────────┘
```

---

## Tauri Commands API

### 文件操作

```rust
/// 加载 ELF 文件 (自动检测/生成数据包)
#[tauri::command]
fn load_elf(path: String, state: State<Mutex<AppState>>) -> Result<LoadResult, String>

/// 直接加载数据包
#[tauri::command]
fn load_package(path: String, state: State<Mutex<AppState>>) -> Result<LoadResult, String>

/// 生成/重新生成数据包
#[tauri::command]
fn generate_package(elf_path: String, output_path: Option<String>, state: State<Mutex<AppState>>) -> Result<PackageMeta, String>

/// 加载目标 A2L 文件
#[tauri::command]
fn load_a2l(path: String, state: State<Mutex<AppState>>) -> Result<A2lLoadResult, String>
```

### 变量查询

```rust
/// 搜索 ELF 变量
#[tauri::command]
fn search_elf_entries(query: String, offset: usize, limit: usize, state: State<Mutex<AppState>>) -> Vec<A2lEntry>

/// 获取 ELF 变量总数
#[tauri::command]
fn get_elf_count(state: State<Mutex<AppState>>) -> usize

/// 搜索 A2L 变量
#[tauri::command]
fn search_a2l_variables(query: String, offset: usize, limit: usize, state: State<Mutex<AppState>>) -> Vec<A2lVariable>
```

### 导出/删除

```rust
/// 导出变量到 A2L
#[tauri::command]
fn export_entries(indices: Vec<usize>, mode: String, state: State<Mutex<AppState>>) -> Result<ExportResult, String>

/// 从 A2L 删除变量
#[tauri::command]
fn delete_variables(indices: Vec<usize>, state: State<Mutex<AppState>>) -> Result<usize, String>
```

---

## 前端 API 封装

```typescript
// src-ui/src/lib/commands.ts
import { invoke } from '@tauri-apps/api/core';

export async function loadElf(path: string): Promise<LoadResult> {
  return invoke('load_elf', { path });
}

export async function loadPackage(path: string): Promise<LoadResult> {
  return invoke('load_package', { path });
}

export async function generatePackage(elfPath: string, outputPath?: string): Promise<PackageMeta> {
  return invoke('generate_package', { elfPath, outputPath });
}

export async function loadA2l(path: string): Promise<A2lLoadResult> {
  return invoke('load_a2l', { path });
}

export async function searchElfEntries(query: string, offset = 0, limit = 10000): Promise<A2lEntry[]> {
  return invoke('search_elf_entries', { query, offset, limit });
}

export async function searchA2lVariables(query: string, offset = 0, limit = 10000): Promise<A2lVariable[]> {
  return invoke('search_a2l_variables', { query, offset, limit });
}

export async function exportEntries(indices: number[], mode: 'measurement' | 'characteristic'): Promise<ExportResult> {
  return invoke('export_entries', { indices, mode });
}

export async function deleteVariables(indices: number[]): Promise<number> {
  return invoke('delete_variables', { indices });
}
```

---

## 状态管理

```typescript
// src-ui/src/lib/stores.ts
import { writable, derived } from 'svelte/store';

// ELF 变量 (右侧面板)
export const elfEntries = writable<A2lEntry[]>([]);
export const elfFilteredCount = writable<number>(0);
export const elfTotalCount = writable<number>(0);
export const elfSearchQuery = writable<string>('');
export const elfSelectedIndices = writable<Set<number>>(new Set());

// A2L 变量 (左侧面板)
export const a2lVariables = writable<A2lVariable[]>([]);
export const a2lSearchQuery = writable<string>('');
export const a2lSelectedIndices = writable<Set<number>>(new Set());

// 文件状态
export const elfPath = writable<string | null>(null);
export const packagePath = writable<string | null>(null);
export const a2lPath = writable<string | null>(null);
export const a2lNames = writable<Set<string>>(new Set());  // 已存在的变量名

// 应用状态
export const statusMessage = writable<string>('💡 文件 → 打开 ELF 开始使用');
export const isLoading = writable<boolean>(false);

// 主题
export const currentTheme = writable<string>('dark');

// 派生状态
export const elfSelectedCount = derived(elfSelectedIndices, $set => $set.size);
export const a2lSelectedCount = derived(a2lSelectedIndices, $set => $set.size);
```

---

## 主题配置

```typescript
// src-ui/src/lib/themes.ts
export const themes = {
  dark: {
    name: 'Dark',
    colors: {
      bg: '#0f0f12',
      bgHover: '#1a1a1f',
      bgSelected: '#1e3a5f',
      text: '#e4e4e7',
      textMuted: '#71717a',
      border: '#27272a',
      accent: '#3b82f6',
    }
  },
  light: {
    name: 'Light',
    colors: {
      bg: '#ffffff',
      bgHover: '#f4f4f5',
      bgSelected: '#dbeafe',
      text: '#18181b',
      textMuted: '#a1a1aa',
      border: '#e4e4e7',
      accent: '#3b82f6',
    }
  },
  midnight: {
    name: 'Midnight',
    colors: {
      bg: '#000000',
      bgHover: '#0a0a0a',
      bgSelected: '#0c1929',
      text: '#fafafa',
      textMuted: '#52525b',
      border: '#18181b',
      accent: '#3b82f6',
    }
  },
  ocean: {
    name: 'Ocean',
    colors: {
      bg: '#0c1222',
      bgHover: '#141d32',
      bgSelected: '#1e3a5f',
      text: '#e0f2fe',
      textMuted: '#64748b',
      border: '#1e293b',
      accent: '#06b6d4',
    }
  }
};

export type ThemeName = keyof typeof themes;
```

---

## 关键实现

### 1. 行选中变色

```svelte
<!-- VariableList.svelte -->
<script lang="ts">
  import { elfSelectedIndices, elfEntries, a2lNames } from '$lib/stores';
  
  function handleClick(e: MouseEvent, index: number) {
    if (e.ctrlKey) {
      // Ctrl+点击: 多选
      const newSet = new Set($elfSelectedIndices);
      newSet.has(index) ? newSet.delete(index) : newSet.add(index);
      elfSelectedIndices.set(newSet);
    } else {
      // 单击: 单选
      elfSelectedIndices.set(new Set([index]));
    }
  }
</script>

{#each $elfEntries as entry, i}
  {@const isSelected = $elfSelectedIndices.has(i)}
  {@const isExisting = $a2lNames.has(entry.full_name)}
  
  <div
    class="row"
    class:selected={isSelected}
    class:existing={isExisting}
    on:click={(e) => handleClick(e, i)}
    on:contextmenu={(e) => handleContextMenu(e, i)}
  >
    <span class="name" class:muted={isExisting}>{entry.full_name}</span>
    <span class="type">{entry.a2l_type}</span>
    <span class="addr">0x{entry.address.toString(16).toUpperCase().padStart(8, '0')}</span>
  </div>
{/each}

<style>
  .row {
    display: flex;
    padding: 6px 12px;
    cursor: pointer;
    border-left: 2px solid transparent;
  }
  .row:hover { background: var(--bg-hover); }
  .row.selected { background: var(--bg-selected); border-left-color: var(--accent); }
  .row.existing .name { color: var(--text-muted); }
</style>
```

### 2. 右键菜单

```svelte
<!-- ContextMenuElf.svelte -->
<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { fly } from 'svelte/transition';
  import { a2lPath } from '$lib/stores';
  
  export let x: number;
  export let y: number;
  export let indices: Set<number>;
  
  const dispatch = createEventDispatcher();
  const canExport = $a2lPath !== null;
  
  function exportAs(mode: 'measurement' | 'characteristic') {
    dispatch('export', { indices: Array.from(indices), mode });
    dispatch('close');
  }
  
  function copyNames() {
    const names = Array.from(indices).map(i => entries[i]?.full_name).filter(Boolean);
    navigator.clipboard.writeText(names.join('\n'));
    dispatch('close');
  }
</script>

<svelte:window on:click={() => dispatch('close')} />

<div class="menu" style="left: {x}px; top: {y}px;" transition:fly={{ duration: 100, y: -5 }}>
  <button class="item" disabled={!canExport} on:click={() => exportAs('measurement')}>
    📊 添加为观测变量
  </button>
  <button class="item" disabled={!canExport} on:click={() => exportAs('characteristic')}>
    📈 添加为标定变量
  </button>
  <div class="divider"></div>
  <button class="item" on:click={copyNames}>📋 复制名称</button>
  <button class="item" on:click={copyAddresses}>📋 复制地址</button>
  <div class="divider"></div>
  <button class="item" on:click={() => dispatch('clear')}>✖ 取消选择</button>
</div>
```

### 3. 状态栏动态提示

```svelte
<!-- StatusBar.svelte -->
<script lang="ts">
  import { derived } from 'svelte/store';
  import { 
    elfPath, elfSelectedIndices, a2lPath, 
    a2lSelectedIndices, statusMessage 
  } from '$lib/stores';
  
  // 动态计算提示信息
  const hint = derived(
    [elfPath, elfSelectedIndices, a2lPath, a2lSelectedIndices, statusMessage],
    ([$elfPath, $elfSelected, $a2lPath, $a2lSelected, $status]) => {
      // 优先显示操作结果
      if ($status && !$status.startsWith('💡')) return $status;
      
      if (!$elfPath) return '💡 文件 → 打开 ELF 开始使用';
      if ($a2lSelected.size > 0) return '💡 右键 → 删除所选变量';
      if ($elfSelected.size > 0 && !$a2lPath) return '⚠️ 请先选择目标 A2L 文件';
      if ($elfSelected.size > 0) return '💡 右键 → 添加为观测/标定变量';
      return '💡 单击选择变量，右键打开菜单';
    }
  );
</script>

<div class="status-bar">{$hint}</div>
```

---

## 数据类型

```typescript
// src-ui/src/lib/types.ts

interface A2lEntry {
  index: number;
  full_name: string;
  address: number;
  size: number;
  a2l_type: string;      // ULONG, SWORD, FLOAT32_IEEE 等
  type_name: string;     // 原始类型名
  bit_offset: number | null;
  bit_size: number | null;
}

interface A2lVariable {
  name: string;
  address: string | null;
  data_type: string;     // ULONG, FLOAT32_IEEE 等
  var_type: 'MEASUREMENT' | 'CHARACTERISTIC';  // 变量类型
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
  added: number;     // 实际添加数量
  skipped: number;   // 跳过数量 (已存在)
  existing: number;  // A2L 中已有变量总数
}
```

---

## 测试文件

| 文件 | 路径 |
|------|------|
| ELF 文件 | `/home/hori/learn/github/newTest.elf` (437 MB) |
| 数据包 | `/home/hori/learn/github/newTest.elf.a2ldata` |
| 参考 A2L | `/home/hori/learn/github/SF30E NEW TEST.a2l` |

---

## 交互汇总

| 交互 | 行为 |
|------|------|
| 单击行 | 单选（清除其他选中） |
| Ctrl + 单击 | 多选/取消选中 |
| 右键行 | 打开上下文菜单 |
| Ctrl + A | 全选当前筛选结果 |
| ↑ / ↓ 键 | 键盘导航 |
| 搜索框 | 实时搜索 + 防抖 (300ms) |

---

## 常见问题

### Q: 如何切换 egui 和 Tauri 版本？

```bash
# egui 版本
cargo run --bin a2l-editor

# Tauri 版本
npm run tauri dev
```

### Q: 主题不生效？

检查 `app.css` 中的 CSS 变量是否正确定义，确保 `ThemeSwitch` 组件正确调用 `applyTheme()`。

### Q: 右键菜单位置不对？

使用 `event.clientX` 和 `event.clientY` 获取鼠标位置，注意处理边界情况（菜单超出屏幕时调整位置）。

### Q: 大数据量卡顿？

- 使用虚拟滚动 (svelte-virtual-list)
- 分页加载 (offset + limit)
- 搜索防抖 (debounce 300ms)

### Q: Tauri 开发模式启动慢？

首次启动需要编译 Rust 代码，后续启动会快很多。可以先用 `npm run dev` 单独开发前端。

---

## 注意事项

1. **核心库稳定性**: `src/lib/` 是核心库，修改需谨慎
2. **CLI 工具**: `a2l-cli` 提供命令行功能，可用于脚本自动化
3. **中文字体**: 需要在 Tauri 版本中配置中文字体 (static/fonts/)
4. **CI/CD**: 需要更新 GitHub Actions 支持 Tauri 打包
5. **文件对话框**: 使用 `@tauri-apps/plugin-dialog`
6. **文档实时更新**: 开发过程中必须实时更新 `docs/` 下的文档
   - 完成任务 → 勾选 TASKS.md
   - API 变更 → 更新本文档
   - 发现问题 → 记录到 TASKS.md 问题追踪表

---

## 后续优化

### 已完成 ✅
- [x] Ctrl+A 全选
- [x] ↑↓ 键盘导航
- [x] 4 主题系统 (Dark/Light/Midnight/Ocean)
- [x] 主题持久化 (localStorage)
- [x] 右键菜单 (A2L/ELF)
- [x] 剪贴板复制
- [x] Linux 打包 (.deb, .rpm, .AppImage)
- [x] 搜索防抖 (300ms)
- [x] 虚拟滚动 (VirtualList.svelte)
- [x] 加载状态动画 (LoadingOverlay.svelte)
- [x] 面板宽度持久化
- [x] 右键菜单边界检测
- [x] **Playwright 自动化测试 (100% 通过)**
- [x] **v0.0.9: 移除 egui，修复 Shift 多选和后端排序**

### 待完成 ⬜
- [ ] GUI 功能测试（需在桌面环境手动操作）
- [ ] Windows/macOS 打包
- [ ] Tauri WebDriver 端到端测试
- [ ] 性能测试（10 万条目）
- [ ] 快捷键系统 (可配置)
- [ ] 变量详情面板
- [ ] 导入/导出配置
- [ ] 多语言支持
- [ ] CI/CD GitHub Actions 配置

---

## 文档索引

| 文档 | 说明 |
|------|------|
| `docs/TASKS.md` | 任务清单和进度 |
| `docs/DESIGN.md` | UI 设计文档 |
| `docs/PLAN.md` | 项目计划 |
| `docs/HANDOVER_TAURI.md` | 本文档 |
| `docs/UI_TESTING_GUIDE.md` | UI 测试经验总结 |
| `docs/UI_TEST_REPORT_PLAYWRIGHT.md` | Playwright 测试报告 |
| `src-ui/playwright-report/index.html` | HTML 测试报告 |
