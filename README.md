# A2L Editor

从 ELF/DWARF 调试信息生成 A2L 文件的桌面工具。

## 功能特性

- **DWARF 解析**: 深度解析结构体、联合体、位域、多维数组
- **条目展开**: 将嵌套类型展开为可导出的 A2L 条目
- **数据包系统**: 每个 ELF 对应独立的 `.a2ldata` 文件，与 ELF 同目录
- **快速加载**: 数据包加载 ~150ms（首次解析 ~160s）
- **Tauri 版本**: 现代化 Web 界面，支持多主题
- **A2L 导出**: 追加到已有 A2L 文件，支持观测变量和标定变量
- **变量编辑**: 可编辑已有 A2L 变量的名称、地址、数据类型、BIT_MASK
- **格式保留**: 编辑时保留原始格式（缩进、注释、空格）
- **CLI 工具**: 命令行创建数据包、查询条目

## 下载

从 [Releases](https://github.com/horinen/a2l-editor/releases) 页面下载最新版本：

| 平台 | 文件 |
|------|------|
| Linux | `A2L-Editor-Linux-x64.AppImage` |
| Windows | `A2L-Editor-Windows-x64.exe` |
| macOS | `A2L-Editor-macOS-x64.zip` |

---

# 操作手册

## 启动程序

```bash
# Linux AppImage
chmod +x A2L-Editor-Linux-x64.AppImage
./A2L-Editor-Linux-x64.AppImage

# Windows
双击 A2L-Editor-Windows-x64.exe

# macOS
解压 A2L-Editor-macOS-x64.zip，拖入 Applications
```

## 主界面说明

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ 📁 文件 ▼  ❓ 手册  ℹ️ 关于          💾 保存  ↩️ 重置  小端 🎨  v0.1.0       │
├─────────────────────────────────────────────────────────────────────────────┤
│ 📂 ELF: firmware.elf (437 MB, 133,646 条目)                                 │
│ 📦 数据包: firmware.elf.a2ldata                                              │
│ 📄 A2L: output.a2l (234 个变量)                                              │
├─────────────────────────────────────────────────────────────────────────────┤
│ A2L 变量 (234)              │ ELF 变量 (133,646)                              │
│ ┌─────────────────────────┐ │ ┌─────────────────────────────────────────────┐ │
│ │ 🔍 [________] [✖]       │ │ │ 🔍 [________] [✖]                       │ │
│ ├─────────────────────────┤ │ ├─────────────────────────────────────────────┤ │
│ │ 变量名 ¹▲  类型   地址 ²│ │ │ 变量名 ¹▲    类型      地址 ²▼            │ │
│ │ ─────────────────────── │ │ │ ─────────────────────────────────────────── │ │
│ │   var1     ULONG  0x... │ │ │ struct.member ULONG    0x70000000        │ │
│ │   var2    FLOAT  0x... │ │ │ another_var   UWORD    0x70000004  ✓     │ │
│ │ ...                     │ │ │ ...                                         │ │
│ └─────────────────────────┘ │ └─────────────────────────────────────────────┘ │
 │ ┌─────────────────────────┐ │                                                 │
 │ │ 编辑: var1              │ │                                                 │
 │ │ 名称: [var1      ]      │ │                                                 │
 │ │ 地址: [0x70000000]      │ │                                                 │
 │ │ 类型: [ULONG  ▼]        │ │                                                 │
 │ │ BIT_MASK: [0x0F   ]     │ │                                                 │
 │ │       [💾 保存]         │ │                                                 │
 │ └─────────────────────────┘ │                                                 │
├─────────────────────────────────────────────────────────────────────────────┤
│ 💡 单击选择变量，右键打开菜单                                                 │
└─────────────────────────────────────────────────────────────────────────────┘
```

**界面说明**：
- **小端/大端**: 切换字节序设置
- **编辑区域**: 选中单个变量后可编辑属性，点击保存立即写入

## 操作流程

### 1. 打开 ELF 文件

1. 点击 **「文件」→「打开 ELF」**
2. 选择 ELF 文件（支持 `.elf`, `.out`, `.axf` 扩展名）
3. 如果数据包不存在，会弹出对话框询问是否生成

### 2. 选择目标 A2L

点击 **「选择目标 A2L」** 按钮选择要追加的 A2L 文件

### 3. 搜索和选择变量

- **搜索**: 在搜索框输入关键词（大小写不敏感）
- **单选**: 单击变量行
- **多选**: Ctrl + 单击
- **范围选择**: Shift + 单击
- **全选**: Ctrl + A
- **键盘导航**: ↑↓ 方向键

### 4. 导出到 A2L

1. 在 ELF 面板选中变量
2. **右键** 打开菜单
3. 选择 **「添加为观测变量」** 或 **「添加为标定变量」**
4. 变量立即写入 A2L 文件

### 5. 编辑 A2L 变量

1. 在 A2L 面板选中单个变量
2. 下方编辑区域显示变量属性
3. 修改名称、地址、数据类型或 BIT_MASK
4. 点击 **「保存」** 按钮立即写入
5. 编辑时保留原始格式（缩进、注释、空格）
6. 如果变量没有 BIT_MASK 字段，填写后会自动添加到 ECU_ADDRESS 行之前

### 6. 删除 A2L 变量

1. 在 A2L 面板选中变量
2. **右键** → **「删除变量」**
3. 变量立即从 A2L 文件删除

### 7. 主题切换

点击右上角主题按钮切换：
- **Dark**: 深色主题（默认）
- **Light**: 浅色主题
- **Midnight**: 午夜蓝主题
- **Ocean**: 海洋蓝主题

### 8. 字节序设置

点击 **「小端」/「大端」** 按钮切换字节序设置

---

# CLI 操作手册

## 命令概览

```bash
a2l-cli <命令> [参数] [选项]
```

## 常用命令

### 创建数据包

```bash
# 默认位置（ELF 同目录）
a2l-cli create-package firmware.elf

# 自定义位置
a2l-cli create-package firmware.elf -o /path/to/data.a2ldata
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
a2l-cli entries firmware.elf

# 搜索并限制数量
a2l-cli entries firmware.elf "keyword" -n 100
```

**说明：**
- 优先从数据包加载（如果存在）
- 数据包不存在则解析 ELF 并缓存

### 导出 A2L 文件

```bash
# 导出到文件
a2l-cli export firmware.elf -o output.a2l -n 1000

# 输出到控制台
a2l-cli export firmware.elf -n 100
```

### 查看变量类型

```bash
a2l-cli type firmware.elf variable_name
```

### 解析 ELF

```bash
# 基础解析
a2l-cli parse firmware.elf

# 深度解析（含 DWARF）
a2l-cli parse firmware.elf --deep
```

### 调试命令

```bash
# 列出数组类型
a2l-cli arrays firmware.elf 20

# 列出枚举类型
a2l-cli enums firmware.elf 20

# 列出结构体实例
a2l-cli struct-instances firmware.elf 20

# 列出位域结构体
a2l-cli bitfields firmware.elf 20

# 搜索结构体
a2l-cli struct firmware.elf "struct_name"
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
- Linux: `libgtk-3-dev`, `libwebkit2gtk-4.1-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`

## 编译

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

## CLI 工具

```bash
cargo build --release
# 可执行文件: target/release/a2l-cli
```

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

A: 点击「重新生成缓存」按钮，或使用 CLI 命令 `a2l-cli create-package <elf>`。

## Q: 为什么有些变量没有展开？

A: 可能是数组展开上限限制（1000 元素）或嵌套深度限制（50 层）。

## Q: A2L 文件追加后变量重复怎么办？

A: 程序会自动跳过已存在的变量，显示跳过数量。

## Q: 修改变量后如何撤销？

A: 点击「重置」按钮清空所有待保存的更改，或将变量值改回原值。

## Q: 如何切换字节序？

A: 点击 Header 右侧的「小端」/「大端」按钮。

## License

MIT
