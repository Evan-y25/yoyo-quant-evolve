# 🚀 yoyo-quant-evolve 启动指南

## ⚠️ 前置条件检查

### 1. **Rust 环境**
```bash
# 检查是否安装
rustc --version
cargo --version

# 如果没安装，使用 Homebrew 安装
brew install rust
```

### 2. **Anthropic API Key**
```bash
# 需要有有效的 Claude API Key
# 形如: sk-...
export ANTHROPIC_API_KEY="sk-your-key-here"
```

### 3. **GitHub CLI（可选，用于自动发 issue 回复）**
```bash
# 检查是否安装
gh --version

# 如果需要安装
brew install gh

# 认证
gh auth login
```

### 4. **Python 3**
```bash
# 脚本使用 Python 来解析 GitHub issues
python3 --version
```

---

## 🎯 启动自动进化（3 种方式）

### **方式 1：直接运行 evolve 脚本（推荐）**

```bash
cd /Users/chao/Project/yoyo-quant-evolve

# 设置 API Key（重要！）
export ANTHROPIC_API_KEY="sk-your-key-here"

# 运行自动进化
./scripts/evolve.sh
```

**预期输出**:
```
=== Round 1: 2026-03-30 01:28 ===
Model: claude-opus-4-6
Provider: anthropic
Timeout: 600s

→ Checking build...
  Build OK.

→ Fetching community issues...
  0 issues loaded.

→ Starting evolution session...
  ...Agent 运行中...

→ Session complete.
```

---

### **方式 2：手动运行 cargo（开发调试用）**

```bash
cd /Users/chao/Project/yoyo-quant-evolve

# 构建项目
cargo build

# 运行测试
cargo test

# 直接运行代理
ANTHROPIC_API_KEY="sk-your-key-here" cargo run
```

---

### **方式 3：GitHub Actions 自动运行（已配置）**

如果你有 GitHub 访问权限，脚本已配置为：
- ⏰ **每 2 小时自动运行一次**
- 📝 **自动提交改动到 git**
- 💬 **自动回复 GitHub issues**
- 📊 **自动更新 JOURNAL.md**

配置文件在: `.github/workflows/`

---

## 📋 环境变量配置

### **必需**
```bash
export ANTHROPIC_API_KEY="sk-..."        # Claude API Key
```

### **可选（高级配置）**
```bash
export REPO="Evan-y25/yoyo-quant-evolve" # GitHub 仓库
export MODEL="claude-opus-4-6"           # LLM 模型
export PROVIDER="anthropic"              # API 提供商
export TIMEOUT="600"                     # 超时时间（秒）
```

---

## 🔄 工作流程（自动进化的 9 个阶段）

**evolve.sh 脚本会自动执行以下步骤**：

1. ✅ **验证** — cargo build & cargo test
2. 📖 **读取** — IDENTITY.md（我是谁）、src/main.rs（我的代码）
3. 🧠 **自评** — 找自己的 bug、不足、需要改进的地方
4. 👥 **社区** — 读 GitHub issues，看用户需求
5. 💡 **决策** — 选择本轮要改进的地方
6. 🛠️ **实现** — 写测试、修改代码、测试通过、提交
7. 📝 **日记** — 写 JOURNAL.md（记录本轮做了什么）
8. 🗺️ **计划** — 更新 ROADMAP.md
9. 💬 **反馈** — 在 GitHub issues 回复用户

---

## 🎯 第一次运行快速检查清单

- [ ] Rust 已安装（`rustc --version`）
- [ ] Cargo 已安装（`cargo --version`）
- [ ] API Key 已设置（`echo $ANTHROPIC_API_KEY`）
- [ ] 在项目目录（`cd /Users/chao/Project/yoyo-quant-evolve`）
- [ ] 脚本可执行（`ls -la scripts/evolve.sh`）

---

## 🚨 常见问题

### **Q: Rust 没装怎么办？**
```bash
brew install rust
```

### **Q: API Key 错误？**
```bash
# 检查 Key 是否有效
export ANTHROPIC_API_KEY="sk-..."
echo $ANTHROPIC_API_KEY   # 看看有没有
```

### **Q: cargo build 失败？**
```bash
# 清理缓存，重新构建
cargo clean
cargo build

# 或者查看详细错误
cargo build --verbose
```

### **Q: 脚本超时？**
```bash
# 增加超时时间（默认 600 秒 = 10 分钟）
TIMEOUT=1200 ./scripts/evolve.sh
```

### **Q: 想看完整日记？**
```bash
cat JOURNAL.md      # 查看所有轮次的日记
cat ROADMAP.md      # 查看开发计划
cat IDENTITY.md     # 查看 Agent 的身份和规则
```

---

## 📊 监控进化进度

### **实时查看日记**
```bash
tail -f /Users/chao/Project/yoyo-quant-evolve/JOURNAL.md
```

### **查看轮次计数**
```bash
cat /Users/chao/Project/yoyo-quant-evolve/ROUND_COUNT
```

### **查看 git 日志**
```bash
cd /Users/chao/Project/yoyo-quant-evolve
git log --oneline | head -20
```

### **查看所有改动**
```bash
git log -p | head -100
```

---

## 🎓 深入理解

- **JOURNAL.md** - 📝 每一轮都会记录做了什么、成功失败了什么
- **ROADMAP.md** - 🗺️ Agent 的长期发展计划
- **IDENTITY.md** - 🎭 Agent 的性格、规则、目标
- **MEMORY.md** - 🧠 Agent 记住的市场模式、用户需求
- **src/main.rs** - 🔧 Agent 的核心代码
- **scripts/evolve.sh** - ⚙️ 自动化进化的驱动脚本

---

## ✨ 启动 Agent 的黄金法则

```bash
# 1. 进入项目目录
cd /Users/chao/Project/yoyo-quant-evolve

# 2. 设置 API Key（最重要！）
export ANTHROPIC_API_KEY="sk-..."

# 3. 运行进化脚本
./scripts/evolve.sh

# 4. 喝杯咖啡，看 Agent 自己变聪明 ☕
```

---

**准备好了吗？** 运行上面的命令启动自动进化！🚀

