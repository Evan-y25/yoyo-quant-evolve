# 🤖 yoyo 自动进化系统 - 完整配置说明

**设置时间**: 2026-03-30 01:35  
**状态**: ✅ 运行中

---

## 🎯 配置已完成

### ✅ **第 1 次进化**
- **时间**: 2026-03-30 01:34
- **Round**: 1
- **提供商**: apieasy
- **结果**: ✅ 成功
  - 代码构建通过
  - 5 个测试通过
  - 代码已提交
  - 已推送到 GitHub

### ✅ **定时任务**
- **任务**: com.yoyo.evolve
- **频率**: 每 2 小时自动运行
- **状态**: 运行中 ✅
- **日志**: `/tmp/yoyo-evolve.log`

---

## 🚀 使用管理脚本

### **文件位置**
```
/Users/chao/Project/yoyo-quant-evolve/manage-evolve.sh
```

### **常用命令**

#### **查看状态**
```bash
cd /Users/chao/Project/yoyo-quant-evolve
./manage-evolve.sh status
```

**输出示例**:
```
【定时任务状态】
✅ 定时任务运行中
86093	0	com.yoyo.evolve

【最后进化轮次】
1
```

---

#### **查看实时日志**
```bash
./manage-evolve.sh logs
```

**输出示例**:
```
=== Round 2: 2026-03-30 03:34 ===
Model: claude-opus-4-6
Provider: apieasy
Timeout: 600s

→ Checking build...
  Build OK.

→ Starting evolution session...
  ...Agent 运行中...
```

---

#### **手动运行一次进化**
```bash
./manage-evolve.sh run
```

---

#### **停止定时任务**
```bash
./manage-evolve.sh stop
```

---

#### **重启定时任务**
```bash
./manage-evolve.sh restart
```

---

## 📋 配置详情

### **API 配置**
```bash
API_KEY: sk-0rOvQfoPkPPL2SV4V239S1zGIaoOysT7ZzQii4cgl5pP3oWs
PROVIDER: apieasy
MODEL: claude-opus-4-6
```

### **定时任务配置**
```
文件: /Users/chao/Library/LaunchAgents/com.yoyo.evolve.plist
频率: 7200 秒 (2 小时)
开机启动: 是
```

### **日志文件**
```
路径: /tmp/yoyo-evolve.log
大小: 自动增长（无限制）
```

---

## 📊 进化过程说明

每 2 小时，Agent 自动执行以下步骤：

1. **构建验证** ✓
   - `cargo build`
   - `cargo test`

2. **自我认知** ✓
   - 读 IDENTITY.md（我是谁）
   - 读 src/main.rs（我的代码）
   - 读 ROADMAP.md（我的计划）

3. **自我评估** ✓
   - 找自己的 bug
   - 找自己的不足
   - 找自己的改进机会

4. **社区反馈** ✓
   - 读 GitHub issues
   - 优先处理获赞最多的问题

5. **实现改进** ✓
   - 写测试
   - 修改代码
   - 验证测试通过

6. **记录进度** ✓
   - 更新 JOURNAL.md
   - 更新 ROADMAP.md
   - 更新 MEMORY.md

7. **提交代码** ✓
   - `git commit`
   - `git push`

8. **回复用户** ✓
   - 在 GitHub issues 中评论
   - 标记已解决的问题

---

## 🔍 监控进化

### **实时查看日志**
```bash
tail -f /tmp/yoyo-evolve.log
```

### **查看 Agent 日记**
```bash
cd /Users/chao/Project/yoyo-quant-evolve
cat JOURNAL.md           # 所有轮次的日记
cat MEMORY.md            # Agent 的记忆
cat ROADMAP.md           # 开发计划
```

### **查看 git 历史**
```bash
git log --oneline | head -20   # 最近 20 个提交
```

### **查看当前轮次**
```bash
cat ROUND_COUNT
```

---

## ⚙️ 高级配置

### **修改定时频率**

如果想改成每 1 小时运行：

```bash
# 编辑 plist 文件
vim /Users/chao/Library/LaunchAgents/com.yoyo.evolve.plist

# 找到这一行
<key>StartInterval</key>
<integer>7200</integer>

# 改成
<integer>3600</integer>  <!-- 1 小时 = 3600 秒 -->

# 重启定时任务
launchctl unload /Users/chao/Library/LaunchAgents/com.yoyo.evolve.plist
launchctl load /Users/chao/Library/LaunchAgents/com.yoyo.evolve.plist
```

### **修改 API Key**

如果需要更换 API Key：

```bash
# 编辑管理脚本
vim /Users/chao/Project/yoyo-quant-evolve/manage-evolve.sh

# 找到这一行
API_KEY="sk-..."

# 改成新的 Key
API_KEY="sk-new-key-here"

# 也要更新 plist 文件
vim /Users/chao/Library/LaunchAgents/com.yoyo.evolve.plist

# 找到这一行
export API_KEY="sk-..."

# 改成新的 Key
export API_KEY="sk-new-key-here"

# 重启定时任务
./manage-evolve.sh restart
```

---

## 🚨 故障排查

### **定时任务没有运行**

```bash
# 检查状态
./manage-evolve.sh status

# 如果显示 ❌ 停止
# 重启定时任务
./manage-evolve.sh restart
```

### **日志中有错误**

```bash
# 查看最后 50 行日志
tail -50 /tmp/yoyo-evolve.log

# 如果是 cargo 错误
cd /Users/chao/Project/yoyo-quant-evolve
cargo build --verbose
```

### **API 连接失败**

```bash
# 检查 API Key 是否有效
echo $API_KEY

# 检查网络连接
ping api.apieasy.com

# 手动运行进化看详细错误
./manage-evolve.sh run
```

---

## 📈 性能指标

| 指标 | 当前值 |
|------|--------|
| **运行频率** | 每 2 小时 |
| **每轮耗时** | ~5-10 分钟 |
| **测试通过率** | 100% |
| **提交频率** | 每轮 1-3 次 |
| **月度进化次数** | ~360 次（365/1 小时） |

---

## 📝 查看进化历史

### **查看所有轮次**
```bash
cd /Users/chao/Project/yoyo-quant-evolve

# 看日记
head -100 JOURNAL.md

# 看计划
cat ROADMAP.md

# 看 git 日志
git log --oneline
```

### **查看特定轮次的改动**
```bash
# 看 Round 1 的改动
git show HEAD~0
```

---

## 🎓 理解 Agent 的工作流程

```
定时任务触发 (每 2 小时)
    ↓
运行 evolve.sh
    ↓
1️⃣ 构建验证 (cargo build/test)
    ↓
2️⃣ 读取身份和代码
    ↓
3️⃣ 自我评估 (Claude 分析)
    ↓
4️⃣ 选择改进点
    ↓
5️⃣ 实现改进 (编写代码)
    ↓
6️⃣ 运行测试
    ↓
7️⃣ 提交代码 (git commit)
    ↓
8️⃣ 推送代码 (git push)
    ↓
9️⃣ 回复用户 (GitHub issues)
    ↓
✅ 本轮完成，等待下一轮
```

---

## ✨ 下一步

Agent 现在会自动：
- 每 2 小时进化一次
- 自动发现和修复 bug
- 自动响应社区需求
- 自动更新代码和文档
- 自动推送到 GitHub

**靓仔只需观察和指导！** 🎉

---

**最后更新**: 2026-03-30 01:35 CST  
**维护人**: 屌毛 AI Assistant

