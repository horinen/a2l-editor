# A2L Editor Tauri 版本 - 项目计划文档

## 项目信息

| 项目 | 信息 |
|------|------|
| 项目名称 | A2L Editor Tauri 迁移 |
| 开始日期 | 待定 |
| 预计工期 | 4-5 天 |
| 负责人 | - |
| 状态 | 📋 计划中 |

---

## 里程碑

| 里程碑 | 预计完成 | 交付物 |
|--------|----------|--------|
| M1: 项目初始化 | Day 0.5 | Tauri + Svelte 项目骨架 |
| M2: 后端集成 | Day 1 | Tauri Commands 可调用 |
| M3: UI 骨架 | Day 2 | 基础组件、布局完成 |
| M4: 交互完成 | Day 3 | 右键菜单、键盘导航 |
| M5: 主题系统 | Day 3.5 | 4 个主题可切换 |
| M6: 功能完整 | Day 4 | 所有功能可用 |
| M7: 测试发布 | Day 5 | 测试通过、可发布 |

---

## 详细计划

### 阶段 1: 项目初始化 (0.5 天)

**目标**: 创建 Tauri + Svelte 项目骨架

**任务清单**:
1. 初始化 npm workspace
2. 创建 `src-ui/` 目录
3. 配置 SvelteKit + Vite
4. 配置 TailwindCSS
5. 安装 shadcn-svelte
6. 创建 `src-tauri/` 目录
7. 配置 tauri.conf.json
8. 配置 Cargo.toml workspace

**验收标准**:
- [ ] `npm run dev` 能启动前端开发服务器
- [ ] `npm run tauri dev` 能启动 Tauri 应用
- [ ] TailwindCSS 样式生效
- [ ] 空白页面正常显示

---

### 阶段 2: 后端集成 (0.5 天)

**目标**: Tauri Commands 可调用核心库

**任务清单**:
1. 创建 `src/commands.rs`
2. 实现 `load_elf` 命令
3. 实现 `load_package` 命令
4. 实现 `generate_package` 命令
5. 实现 `load_a2l` 命令
6. 实现 `search_elf_entries` 命令
7. 实现 `search_a2l_variables` 命令
8. 实现 `export_entries` 命令
9. 实现 `delete_variables` 命令
10. 创建 `src/main_tauri.rs`
11. 配置 Tauri 状态管理
12. 前端 `commands.ts` 封装

**验收标准**:
- [ ] 前端能调用 `load_elf` 并获取返回值
- [ ] 前端能调用 `search_elf_entries` 获取变量列表
- [ ] 错误信息能正确传递到前端

---

### 阶段 3: UI 组件开发 (1.5 天)

**目标**: 完成所有 UI 组件

**任务清单**:

#### 3.1 基础组件 (0.5 天)
- [ ] 安装配置 shadcn-svelte
- [ ] Button 组件
- [ ] Input 组件
- [ ] Dialog 组件
- [ ] ScrollArea 组件
- [ ] DropdownMenu 组件

#### 3.2 Header 组件 (0.25 天)
- [ ] 文件下拉菜单
- [ ] 手册按钮
- [ ] 关于按钮
- [ ] 主题切换
- [ ] 版本号显示

#### 3.3 FileInfo 组件 (0.25 天)
- [ ] ELF 文件信息行 + 导入按钮
- [ ] 数据包信息行 + 导入按钮
- [ ] A2L 文件信息行 + 导入按钮
- [ ] 状态样式 (未选择/已加载/加载中/错误)

#### 3.4 A2lPanel 组件 (0.25 天)
- [ ] 搜索框
- [ ] 3 列表格布局 (变量名、类型、地址)
- [ ] 行选中样式
- [ ] 分页信息

#### 3.5 VariableList 组件 (0.25 天)
- [ ] 搜索框
- [ ] 3 列表格布局 (变量名、类型、地址)
- [ ] 行选中样式
- [ ] 已存在变量变淡
- [ ] 分页信息

**验收标准**:
- [ ] Header 显示正确，下拉菜单可用
- [ ] FileInfo 显示当前加载的文件
- [ ] 两个面板能显示数据
- [ ] 选中样式正确
- [ ] 搜索过滤正常

---

### 阶段 4: 交互逻辑 (0.5 天)

**目标**: 完成右键菜单和键盘交互

**任务清单**:

1. ContextMenuA2l 组件
   - [ ] 菜单定位
   - [ ] 删除变量
   - [ ] 复制功能
   - [ ] 取消选择

2. ContextMenuElf 组件
   - [ ] 添加观测变量
   - [ ] 添加标定变量
   - [ ] A2L 未选择时置灰
   - [ ] 复制功能
   - [ ] 取消选择

3. 键盘交互
   - [ ] Ctrl+点击多选
   - [ ] Ctrl+A 全选
   - [ ] ↑↓ 导航

4. StatusBar 组件
   - [ ] 动态提示逻辑
   - [ ] 操作结果显示

**验收标准**:
- [ ] 右键菜单正确显示
- [ ] 菜单操作功能正常
- [ ] 键盘快捷键正常
- [ ] 状态栏提示正确

---

### 阶段 5: 对话框与主题 (0.5 天)

**目标**: 完成对话框和主题系统

**任务清单**:

#### 5.1 对话框 (0.25 天)
- [ ] ExportDialog 组件 - 导出确认
- [ ] GenerateDialog 组件 - 生成数据包
- [ ] AboutDialog 组件 - 关于信息

#### 5.2 主题系统 (0.25 天)
- [ ] themes.ts 配置 (4 个主题)
- [ ] CSS 变量定义
- [ ] ThemeSwitch 组件
- [ ] 主题持久化 (localStorage)

**验收标准**:
- [ ] 导出对话框正确显示预览
- [ ] 4 个主题可切换
- [ ] 主题选择被记住

---

### 阶段 6: 集成测试 (0.5 天)

**目标**: 功能测试和问题修复

**任务清单**:

1. 功能测试
   - [ ] 打开 ELF 文件
   - [ ] 打开数据包
   - [ ] 选择目标 A2L
   - [ ] 搜索变量
   - [ ] 选中并导出
   - [ ] 删除 A2L 变量

2. 性能测试
   - [ ] 10 万条目加载
   - [ ] 搜索响应速度
   - [ ] 内存占用

3. 跨平台测试
   - [ ] Linux 测试
   - [ ] Windows 测试 (如有条件)

**验收标准**:
- [ ] 所有功能正常
- [ ] 无明显性能问题
- [ ] 无崩溃

---

### 阶段 7: 打包发布 (0.5 天)

**目标**: 构建发布版本

**任务清单**:
- [ ] 更新版本号
- [ ] Linux 打包 (.deb, .AppImage)
- [ ] Windows 打包 (.msi, .exe)
- [ ] macOS 打包 (如有条件)
- [ ] 更新 README.md
- [ ] 创建 GitHub Release

**验收标准**:
- [ ] 打包产物正常运行
- [ ] 文档更新完成

---

## 文档更新要求

**重要**: 开发过程中必须实时更新文档，确保文档与代码同步。

### 更新时机

| 文档 | 更新时机 | 更新内容 |
|------|----------|----------|
| TASKS.md | 每完成一个任务 | 勾选对应任务项 `[ ]` → `[x]` |
| TASKS.md | 发现新问题 | 添加到问题追踪表 |
| TASKS.md | 进度变化 | 更新进度概览 |
| HANDOVER_TAURI.md | API 变更时 | 更新 Commands 和类型定义 |
| DESIGN.md | 设计变更时 | 更新 UI 设计或架构 |
| PLAN.md | 里程碑完成 | 更新里程碑状态 |

### 每日工作流程

```
1. 开始工作前
   └─ 查看 TASKS.md 确认今日任务

2. 完成任务后
   └─ 立即勾选 TASKS.md 中对应项

3. 发现问题后
   └─ 记录到 TASKS.md 问题追踪表

4. API 变更时
   └─ 同步更新 HANDOVER_TAURI.md

5. 结束工作时
   └─ 更新 TASKS.md 进度概览
   └─ 更新 PLAN.md 状态（如需）
```

### 文档位置

所有文档位于 `docs/` 目录：
- `DESIGN.md` - 项目设计文档
- `PLAN.md` - 项目计划文档
- `TASKS.md` - 任务清单
- `HANDOVER_TAURI.md` - Tauri 版本交接文档

---

## 风险与缓解

| 风险 | 可能性 | 影响 | 缓解措施 |
|------|--------|------|----------|
| Tauri 2 兼容性问题 | 中 | 高 | 提前验证 Tauri 2 + Svelte 5 |
| 虚拟滚动性能 | 低 | 中 | 先用普通滚动，后续优化 |
| 大数据量渲染 | 中 | 中 | 分页/虚拟滚动 |
| 跨平台打包问题 | 中 | 低 | 使用 GitHub CI 自动打包 |
| 文档与代码不同步 | 中 | 中 | 严格遵守文档更新流程 |

---

## 依赖关系

```
阶段 1 (初始化)
    │
    ▼
阶段 2 (后端) ──────┬──────▶ 阶段 3 (UI)
    │              │            │
    │              │            ▼
    │              └──────▶ 阶段 4 (交互)
    │                           │
    ▼                           ▼
阶段 5 (对话框/主题) ◀──────────┘
    │
    ▼
阶段 6 (测试)
    │
    ▼
阶段 7 (发布)
```

---

## 开发环境要求

### 必需
- Node.js 18+
- npm 或 pnpm
- Rust stable (1.70+)
- Tauri CLI 2.x

### 平台依赖

**Linux**:
```bash
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev
```

**Windows**:
- Microsoft Edge WebView2
- Visual Studio Build Tools

**macOS**:
- Xcode Command Line Tools

---

## 参考资料

- [Tauri 2 文档](https://v2.tauri.app/)
- [Svelte 5 文档](https://svelte-5-preview.librejs.org/)
- [TailwindCSS 文档](https://tailwindcss.com/docs)
- [shadcn-svelte](https://www.shadcn-svelte.com/)
