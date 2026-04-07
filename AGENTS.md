# AGENTS.md - A2L Editor 项目指南

## 项目概述

A2L Editor 是一个从 ELF/DWARF 调试信息生成 A2L 文件的桌面工具。
- 后端: Rust (Tauri 2.x + 核心库)
- 前端: Svelte 5 + TypeScript + Tailwind CSS + SvelteKit (adapter-static)
- 评论语言: 中文

## 构建命令

```bash
# Rust 核心库 + CLI
cargo build                        # 开发构建
cargo test                         # 运行所有测试
cargo test test_name               # 运行单个测试（匹配函数名子串）
cargo run --bin a2l-cli -- --help  # CLI 工具

# Tauri 应用（从仓库根目录运行）
npm install                        # 安装前端依赖
npm run tauri dev                  # 开发模式（热重载，端口 5173）
npm run tauri build                # 生产构建

# 仅前端
npm run dev                        # 前端开发服务器
npm run build                      # 前端生产构建
```

### 提交前验证

```bash
cargo build && cargo test          # Rust 编译 + 测试
cd src-ui && npm run check         # Svelte 类型检查（svelte-check）
```

### Linux 系统依赖

```bash
sudo apt-get install libgtk-3-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev
```

## 架构

三个 Cargo target 共享一个核心库：

```
src/lib/mod.rs          → a2l_editor crate (lib)
src/bin/a2l_cli.rs      → a2l-cli binary (CLI 工具)
src-tauri/src/main.rs   → a2l-editor-tauri binary (Tauri 桌面应用)
```

`src-tauri` 依赖根 `a2l_editor` crate（`Cargo.toml` 中 `a2l-editor = { path = ".." }`）。

### 前端结构

- `src-ui/` 是一个 SvelteKit 项目，使用 `adapter-static` 输出到 `src-ui/build/`
- `src-ui/src/lib/` — 共享库：types.ts, commands.ts, stores.ts, themes.ts
- `src-ui/src/lib/components/` — 所有 Svelte 组件
- `src-ui/src/routes/+page.svelte` — 唯一页面入口
- 前端通过 `withGlobalTauri: true` 暴露 Tauri API，组件中直接使用 `$lib/` 导入

### IPC 边界

Tauri 命令定义在 `src-tauri/src/commands.rs`，前端调用在 `src-ui/src/lib/commands.ts`。
命名映射：Rust `snake_case` → TypeScript `camelCase`（如 `load_elf` → `loadElf`）。

### 状态管理

`AppState` 在 `commands.rs` 中定义，通过 `Mutex<AppState>` 管理。不持久化，每次启动重置。
前端使用 Svelte stores（`src-ui/src/lib/stores.ts`）。

## 关键约定

### Rust
- 错误处理：核心库用 `anyhow::Result` + `.context()`；Tauri 命令返回 `Result<T, String>` + `.map_err(|e| e.to_string())?`
- Builder 模式：用 `with_*` 方法链式调用（见 `types.rs` 中 `StructMember`）
- 非 MSVC 目标使用 jemalloc 全局分配器（`src/lib/mod.rs`）
- Release profile: LTO + codegen-units=1 + strip + panic=abort

### TypeScript / Svelte
- 使用 Svelte 5 runes：`$state()`, `$derived()`, `$effect()`
- Store 用 `writable`/`derived`，组件中 `$storeName` 自动订阅
- 导入顺序：Svelte 内置 → 外部库 → `$lib/` 内部模块
- 组件文件命名：`PascalCase.svelte`
- 代码注释使用中文

## 易错点

- **前端类型检查** 必须在 `src-ui/` 目录下运行 `npm run check`，不在根目录
- **Tauri 命令** 在根 `package.json` 中通过 `npm run tauri dev` / `npm run tauri build` 调用，不是 `npm run tauri:dev`
- **工作区结构**：根 `package.json` 使用 npm workspaces，`src-ui` 是子包。根目录脚本会代理到 workspace，如 `npm run build` → `npm run build --workspace=src-ui`
- **测试** 全部是 Rust 单元测试（`#[cfg(test)]` 模块），前端无测试框架
- **CLI binary name** 是 `a2l-cli`，不是 `a2l_editor`。运行方式：`cargo run --bin a2l-cli`
- **`src/main_tauri.rs`** 是残留文件，实际入口在 `src-tauri/src/main.rs`
- `src-ui/vite.test.config.ts` 用于 tauri-driver 测试时 mock Tauri API，非日常开发使用

## Tauri 命令参考

所有命令定义在 `src-tauri/src/commands.rs`，当前命令列表：

| 命令 | 用途 |
|------|------|
| `load_elf` | 加载 ELF 文件 |
| `load_package` | 加载 .a2ldata 数据包 |
| `generate_package` | 从 ELF 生成数据包 |
| `load_a2l` | 加载 A2L 文件 |
| `search_elf_entries` | 搜索 ELF 条目 |
| `get_elf_count` | 获取 ELF 条目总数 |
| `search_a2l_variables` | 搜索 A2L 变量 |
| `export_entries` | 导出条目到 A2L |
| `delete_variables` | 删除 A2L 变量 |
| `save_a2l_changes` | 保存 A2L 编辑 |
| `set_endianness` | 设置字节序 |

## CI

- 触发：推送 `v*` tag
- 三个平台并行构建：Linux (AppImage), Windows (exe), macOS (dmg)
- 使用 Node 20 + stable Rust
