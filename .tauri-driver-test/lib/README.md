# Tauri Driver 测试

使用 tauri-driver 进行 A2L Editor 的端到端功能测试。

## 目录结构

```
.tauri-driver-test/
├── lib/                          # 版本控制（上传到 Git）
│   ├── test-patterns.md          # 测试模式参考（用户维护）
│   ├── test-cases.json           # 测试用例
│   ├── test-plan.json            # 测试计划
│   └── README.md                 # 本文件
│
├── results/                      # 测试结果（不上传）
│   ├── test-report.json
│   └── screenshots/
│
├── *.mjs                         # 执行脚本（不上传）
├── review.json                   # 审查结果（不上传）
└── final-report.md               # 最终报告（不上传）
```

## 快速开始

### 1. 环境准备

```bash
# 安装 tauri-driver
cargo install tauri-driver --locked

# 安装系统依赖 (Ubuntu/Debian)
sudo apt install xvfb webkit2gtk-driver

# 安装 Node.js 依赖
npm install selenium-webdriver --save-dev

# 构建应用
cargo tauri build --debug
```

### 2. 运行测试

```bash
# 启动 tauri-driver
DISPLAY=:99 tauri-driver &

# 运行测试
node .tauri-driver-test/test-executor.mjs
```

### 3. 查看结果

- 测试报告：`.tauri-driver-test/results/test-report.json`
- 截图：`.tauri-driver-test/results/screenshots/`

## 维护测试模式

`test-patterns.md` 是测试模式参考文件，包含：

1. **测试模式分类** - 枚举设置类、文件加载类、状态切换类等
2. **步骤模板** - 可复用的测试步骤
3. **必须覆盖的场景** - 每类功能必须测试的场景
4. **项目特定注意事项** - 选择器、已知问题等
5. **已知 Bug 回归测试** - 修复过的 bug 的验证用例

### 添加新测试模式

1. 在 `test-patterns.md` 中添加新模式
2. AI 运行测试时会自动读取并应用

### 添加回归测试

1. 在 `test-patterns.md` 的"已知 Bug 回归测试"部分添加
2. 格式：

```markdown
### BUG-XXX: Bug 标题
- **Issue**: #xxx
- **触发条件**: 描述
- **验证步骤**:
```json
[
  { "action": "execute_script", "script": "..." }
]
```
```

## .gitignore 配置

```gitignore
# Tauri Driver Test - 临时文件
.tauri-driver-test/results/
.tauri-driver-test/*.mjs
.tauri-driver-test/review.json
.tauri-driver-test/final-report.md
```
