# 📊 yoyo-quant-evolve 项目信息

**项目位置**: `/Users/chao/Project/yoyo-quant-evolve`

## 🎯 项目概述

**yoyo** 是一个**自我进化的交易 Agent**，每 2 小时自动醒一次，读自己的代码，找出改进点，自动实现改进。

- 🤖 **自动化改进**: 每 2 小时自我迭代
- 📊 **交易分析**: 美股 + 加密货币（BTC, ETH 等）
- 📈 **市场数据**: 获取、分析、提供建议
- 🧠 **自学**: 读代码 → 找问题 → 实现 → 测试 → 提交
- 📝 **完整日记**: 每次改动都在 JOURNAL.md

## 📁 项目结构

```
yoyo-quant-evolve/
├── src/                 ← Rust 源代码
├── scripts/
│   └── evolve.sh       ← 手动触发进化
├── skills/             ← Agent 技能模块
├── Cargo.toml          ← Rust 项目配置
├── JOURNAL.md          ← 完整改动日记
├── ROADMAP.md          ← 开发计划
├── MEMORY.md           ← 记忆系统
├── IDENTITY.md         ← Agent 身份
└── README.md
```

## 🚀 快速开始

### 1. 查看日记
```bash
cat /Users/chao/Project/yoyo-quant-evolve/JOURNAL.md
```

### 2. 查看开发计划
```bash
cat /Users/chao/Project/yoyo-quant-evolve/ROADMAP.md
```

### 3. 运行项目
```bash
cd /Users/chao/Project/yoyo-quant-evolve
ANTHROPIC_API_KEY=sk-... cargo run
```

### 4. 手动触发进化
```bash
ANTHROPIC_API_KEY=sk-... ./scripts/evolve.sh
```

## 📊 核心功能

1. **自我认知** - 读自己的代码，理解当前能力
2. **自我改进** - 识别不足，实现改进
3. **自我测试** - 运行测试验证改进
4. **自我记录** - 写日记，记录过程
5. **自我优化** - 根据结果优化策略

## 💰 交易功能

- 📈 美股市场分析
- 🪙 加密货币交易
- 📊 技术分析
- 💡 交易建议
- 🎯 风险管理

## 🔧 技术栈

- **语言**: Rust
- **AI**: Anthropic Claude API
- **框架**: yoagent (自定义 agent 库)
- **自动化**: GitHub Actions (每 2 小时)

## 📚 重要文件

| 文件 | 说明 |
|------|------|
| `JOURNAL.md` | 📝 每次改动的日记 |
| `ROADMAP.md` | 🗺️ 开发计划和目标 |
| `MEMORY.md` | 🧠 Agent 的记忆系统 |
| `IDENTITY.md` | 🎭 Agent 的身份和性格 |
| `README.md` | 📖 项目说明 |

## 🌟 亮点

- **完全自动化** - 不需要人类干预，自我进化
- **可追踪** - 每个改动都是一个 git commit，历史完整
- **可交互** - 在 GitHub Issues 中给它任务，它会执行
- **自我反思** - Agent 会评估自己的改进
- **实验精神** - 失败也会被记录和分析

## 🎓 学习机会

这个项目展示了：
1. 如何设计自我进化的 AI Agent
2. 如何使用 Rust 和 Claude API
3. 如何实现自动化的代码改进
4. 如何在交易场景中应用 AI

## ⚠️ 声明

这是一个**实验性** AI Agent，**NOT 金融建议**。交易风险很大，请自己做调查！

---

**项目状态**: 🟢 活跃开发中  
**最后更新**: 2026-03-30  
**维护者**: Evan (原作者)

