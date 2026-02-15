# A2L Editor

从 ELF/DWARF 调试信息生成 A2L 文件的桌面工具。

## 功能特性

- **DWARF 解析**: 深度解析结构体、联合体、位域、多维数组
- **条目展开**: 将嵌套类型展开为可导出的 A2L 条目
- **数据包系统**: 每个 ELF 对应独立的 `.a2ldata` 文件，与 ELF 同目录
- **快速加载**: 数据包加载 ~150ms（首次解析 ~160s）
- **双界面支持**: 
  - **egui 版本**: 轻量级原生界面
  - **Tauri 版本**: 现代化 Web 界面，支持多主题
- **A2L 导出**: 追加到已有 A2L 文件，支持观测变量和标定变量
- **CLI 工具**: 命令行创建数据包、查询条目

## 下载

从 [Releases](https://github.com/horinen/a2l-editor/releases) 页面下载最新版本：

### egui 版本（轻量级）

| 平台 | 文件 |
|------|------|
| Linux | `a2l-editor-linux-x64` |
| Windows | `a2l-editor-windows-x64.exe` |
| macOS | `a2l-editor-macos-x64` |

### Tauri 版本（现代化界面）

| 平台 | 文件 |
|------|------|
| Linux | `a2l-editor-tauri_0.1.0_amd64.AppImage` |
| Linux | `a2l-editor-tauri_0.1.0_amd64.deb` |
| Linux | `a2l-editor-tauri-0.1.0-1.x86_64.rpm` |
| Windows | `a2l-editor-tauri_0.1.0_x64-setup.exe` |
| macOS | `a2l-editor-tauri_0.1.0_x64.dmg` |

---

# Tauri 版本操作手册（推荐）

## 启动程序

```bash
# Linux AppImage（推荐）
chmod +x A2L_Editor_0.1.0_amd64.AppImage
./A2L_Editor_0.1.0_amd64.AppImage

# Linux deb
sudo dpkg -i a2l-editor-tauri_0.1.0_amd64.deb
a2l-editor-tauri

# Windows
双击 a2l-editor-tauri_0.1.0_x64-setup.exe 安装
```

## 主界面说明

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ 文件 | 手册 | 关于                              [🌙 Dark] [Light▼]           │
├─────────────────────────────────────────────────────────────────────────────┤
│ ELF: firmware.elf (436 MB) [导入] │ 包: firmware.elf.a2ldata │ A2L: ... [导入] │
├─────────────────────────────────────────────────────────────────────────────┤
│ A2L 变量 (234)              │ ELF 变量 (133,646)                              │
│ ┌─────────────────────────┐ │ ┌─────────────────────────────────────────────┐ │
│ │ 搜索: [________] [✖]   │ │ │ 搜索: [________] [✖]                       │ │
│ ├─────────────────────────┤ │ ├─────────────────────────────────────────────┤ │
│ │ 变量名    类型    地址  │ │ │ 变量名           类型     地址              │ │
│ │ ─────────────────────── │ │ │ ─────────────────────────────────────────── │ │
│ │ var1      ULONG   0x... │ │ │ struct.member    ULONG    0x70000000        │ │
│ │ var2      FLOAT   0x... │ │ │ another_var      UWORD    0x70000004  ✓     │ │
│ │ ...                     │ │ │ ...                                         │ │
│ └─────────────────────────┘ │ └─────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────────────────────────┤
│ 💡 选择 ELF 变量后右键添加到 A2L                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

## 操作流程

### 1. 打开 ELF 文件

1. 点击 **「文件」→「打开 ELF」**
2. 选择 ELF 文件（支持 `.elf`, `.out`, `.axf` 扩展名）
3. 如果数据包不存在，会弹出对话框询问是否生成

### 2. 选择目标 A2L

点击 **「导入」** 按钮选择要追加的 A2L 文件

### 3. 搜索和选择变量

- **搜索**: 在搜索框输入关键词（大小写不敏感）
- **单选**: 单击变量行
- **多选**: Ctrl + 单击
- **全选**: Ctrl + A
- **键盘导航**: ↑↓ 方向键

### 4. 导出到 A2L

1. 在 ELF 面板选中变量
2. **右键** 打开菜单
3. 选择 **「添加为观测变量」** 或 **「添加为标定变量」**
4. 确认导出

### 5. 删除 A2L 变量

1. 在 A2L 面板选中变量
2. **右键** → **「删除变量」**

### 6. 主题切换

点击右上角主题按钮切换：
- **Dark**: 深色主题（默认）
- **Light**: 浅色主题
- **Midnight**: 午夜蓝主题
- **Ocean**: 海洋蓝主题

---

# egui 版本操作手册

## 启动程序

```bash
# Linux/macOS
./a2l-editor

# Windows
a2l-editor-windows-x64.exe
```

## 主界面说明

```
┌─────────────────────────────────────────────────────────────────┐
│ [打开ELF] [打开数据包] [重新生成缓存] │ [目标A2L] → file.a2l  │
│ [显示详情面板] ☑ │ 状态: 已加载 133646 个条目                    │
├─────────────────────────────────────────────────────────────────┤
│ 搜索: [________] [清除]                                         │
│ 已勾选: 0 条    [添加为观测变量] [添加为标定变量]               │
├─────────────────────────────────────────┬───────────────────────┤
│ ☐ variable_name          @ 0x70000000  │ 名称: variable_name   │
│ ☑ another_var            @ 0x70000004  │ 地址: 0x70000004      │
│ ☐ struct_var.member      @ 0x70000008  │ 大小: 4 字节          │
│ ...                                     │ 类型: ULONG           │
│                                         │ 原始类型: uint32_t    │
├─────────────────────────────────────────┴───────────────────────┤
│ 条目: 133646 / 133646 显示                                      │
└─────────────────────────────────────────────────────────────────┘
```

## 操作流程

### 1. 打开 ELF 文件

1. 点击 **「打开 ELF」** 按钮
2. 选择 ELF 文件（支持 `.elf`, `.out`, `.axf` 扩展名）
3. 首次打开会弹出对话框询问是否生成数据包：
   - **生成数据包**: 在 ELF 同目录创建 `.a2ldata` 文件
   - **选择保存位置**: 自定义数据包路径
   - **取消**: 放弃打开

### 2. 搜索和过滤

- 在搜索框输入关键词（大小写不敏感）
- 支持部分匹配，如输入 `swap` 可匹配 `sst_swapValue`
- 点击 **「清除」** 清空搜索

### 3. 查看条目详情

- 点击条目名称，右侧面板显示详细信息
- 显示内容：完整名称、地址、大小、A2L类型、原始类型

### 4. 勾选要导出的条目

- 勾选条目前的复选框
- 底部显示已勾选数量
- 搜索切换时自动清空勾选

### 5. 导出到 A2L

#### 5.1 选择目标 A2L 文件

点击菜单栏 **「目标 A2L」** 按钮，选择要追加的 A2L 文件。

#### 5.2 添加变量

- **添加为观测变量**: 生成 `MEASUREMENT` 块（只读，用于监控）
- **添加为标定变量**: 生成 `CHARACTERISTIC` 块（可写，用于标定）

#### 5.3 确认追加

弹出的对话框显示：
- 已勾选条目数
- 目标文件路径
- **已存在**: A2L 中已有的变量数
- **新增**: 将要添加的变量数
- **跳过**: 已存在将被跳过的数量

点击 **「追加」** 完成操作。

### 6. 重新生成缓存

点击 **「重新生成缓存」** 可以：
- 覆盖当前位置的数据包
- 选择新的保存位置

### 7. 打开已有数据包

点击 **「打开数据包」** 可以直接加载 `.a2ldata` 文件，无需 ELF。

---

# CLI 操作手册

## 命令概览

```bash
test_core <命令> [参数] [选项]
```

## 常用命令

### 创建数据包

```bash
# 默认位置（ELF 同目录）
test_core create-package firmware.elf

# 自定义位置
test_core create-package firmware.elf -o /path/to/data.a2ldata
```

**输出示例：**
```
解析 ELF 文件: firmware.elf
文件大小: 436.57 MB

深度解析中...
解析完成: 133646 条目

保存数据包...

=== 结果 ===
数据包路径: /path/to/firmware.elf.a2ldata
数据包大小: 23.09 MB
条目数量: 133646
耗时: 161.3 秒
```

### 列出 A2L 条目

```bash
# 列出前 50 条
test_core entries firmware.elf

# 搜索并限制数量
test_core entries firmware.elf "keyword" -n 100
```

**说明：**
- 优先从数据包加载（如果存在）
- 数据包不存在则解析 ELF 并缓存

### 导出 A2L 文件

```bash
# 导出到文件
test_core export firmware.elf -o output.a2l -n 1000

# 输出到控制台
test_core export firmware.elf -n 100
```

### 查看变量类型

```bash
test_core type firmware.elf variable_name
```

### 解析 ELF

```bash
# 基础解析
test_core parse firmware.elf

# 深度解析（含 DWARF）
test_core parse firmware.elf --deep
```

### 调试命令

```bash
# 列出数组类型
test_core arrays firmware.elf 20

# 列出枚举类型
test_core enums firmware.elf 20

# 列出结构体实例
test_core struct-instances firmware.elf 20

# 列出位域结构体
test_core bitfields firmware.elf 20

# 搜索结构体
test_core struct firmware.elf "struct_name"
```

---

# 数据包系统

## 文件结构

```
项目目录/
├── firmware.elf           # 原始 ELF
├── firmware.elf.a2ldata   # 解析数据（自动生成）
```

## 特点

- **独立存储**: 每个 ELF 对应一个数据包
- **同目录存放**: 数据包与 ELF 放在同一目录
- **快速加载**: ~150ms 加载 13 万条目
- **CLI/GUI 兼容**: 两种方式使用相同格式

## 数据包内容

| 字段 | 说明 |
|------|------|
| full_name | 完整名称（含结构体成员路径） |
| address | 内存地址 |
| size | 字节大小 |
| a2l_type | A2L 类型（UBYTE, ULONG 等） |
| type_name | 原始类型名 |
| bit_offset | 位偏移（位域） |
| bit_size | 位大小（位域） |
| array_index | 数组索引 |

---

# 支持的类型

| DWARF 类型 | A2L 类型 | 大小 |
|------------|----------|------|
| `uint8_t`, `char` | UBYTE | 1 |
| `int8_t` | SBYTE | 1 |
| `uint16_t` | UWORD | 2 |
| `int16_t` | SWORD | 2 |
| `uint32_t` | ULONG | 4 |
| `int32_t` | SLONG | 4 |
| `uint64_t` | A_UINT64 | 8 |
| `int64_t` | A_INT64 | 8 |
| `float` | FLOAT32_IEEE | 4 |
| `double` | FLOAT64_IEEE | 8 |

---

# 展开限制

| 限制 | 值 | 说明 |
|------|-----|------|
| 数组展开上限 | 1000 | 单个数组最多展开 1000 元素 |
| 嵌套深度上限 | 50 | 结构体嵌套最多 50 层 |
| 单维度过滤 | =1 | 维度为 1 的中间层自动过滤 |

---

# 从源码构建

## 依赖

- Rust 1.70+
- Node.js 18+
- Linux: `libgtk-3-dev`, `libwebkit2gtk-4.1-dev`

## 编译 egui 版本

```bash
git clone https://github.com/horinen/a2l-editor.git
cd a2l-editor
cargo build --release
```

二进制文件：
- GUI: `target/release/a2l-editor`
- CLI: `target/release/test_core`

## 编译 Tauri 版本

```bash
git clone https://github.com/horinen/a2l-editor.git
cd a2l-editor

# 安装依赖
npm install

# 开发模式
npm run tauri dev

# 生产构建
npm run tauri build
```

输出文件：
- Linux: `target/release/bundle/`（.deb, .rpm, .AppImage）
- Windows: `target/release/bundle/msi/`
- macOS: `target/release/bundle/dmg/`

---

# 性能数据

测试文件: 437MB ELF

| 指标 | 结果 |
|------|------|
| 条目总数 | 133,646 |
| 首次解析 | ~160 秒 |
| 数据包加载 | ~150 ms |
| 数据包大小 | ~23 MB |

---

# 常见问题

## Q: 数据包在哪里？

A: 数据包与 ELF 文件放在同一目录，文件名为 `<elf文件名>.a2ldata`。

## Q: 如何更新数据包？

A: 点击「重新生成缓存」按钮，或使用 CLI 命令 `test_core create-package <elf>`。

## Q: 为什么有些变量没有展开？

A: 可能是数组展开上限限制（1000 元素）或嵌套深度限制（50 层）。

## Q: A2L 文件追加后变量重复怎么办？

A: 程序会自动跳过已存在的变量，显示跳过数量。

## License

MIT
