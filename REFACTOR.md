# 重构计划：ELF 解析逻辑

## 第一步：统一 CLI 调用链

### 变更目标

让 CLI 和 Tauri 走完全相同的解析逻辑，使 CLI 成为 Tauri 行为的可靠验证工具。

### 统一数据模型

DataPackage（`.a2ldata`）是唯一的中间产物：

```
有数据包 → 从数据包加载
无数据包 → parse_deep → 生成数据包 → 从数据包加载
```

### 删除 Cache 层

- 删除 `src/lib/cache.rs`
- 删除 `src/lib/hash.rs`（Cache 是唯一使用者）
- 从 `mod.rs` 移除相关 re-export
- 从 `Cargo.toml` 移除 `bincode` 依赖（仅 Cache 使用）

### 具体变更

1. **`parse` 命令** — 无数据包则 `parse_deep` 生成数据包，有数据包则直接加载，显示变量统计和条目数
2. **`entries` 命令** — 从数据包加载 A2lEntry 列表，无数据包则先生成
3. **`create-package` 命令** — 保持不变，已和 Tauri 一致
4. **`export` 命令** — 从数据包加载 A2lEntry 导出到 A2L 文件，和 Tauri 的 `export_entries` 行为一致
5. **合并 9 个 DWARF 调试命令** 为一个 `inspect` 子命令：
   - `struct` / `type` / `arrays` / `enums` / `dwarf-vars` / `struct-instances` / `bitfields` / `debug-member` / `check-offset`
   - 通过参数选择查看内容：`--types` `--structs` `--vars` `--bitfields` `--offset 0xHH`
   - 此命令仍然直接调 DwarfParser，用于问题诊断
6. **`cache` / `clear`** — 删除，Cache 层已移除
