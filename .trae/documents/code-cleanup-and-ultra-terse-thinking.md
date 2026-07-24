# PLAN & SPEC: 代码优化/清理 + 思考极致精简

## 目标

1. **代码优化/清理**：移除调试诊断日志、ping 调试命令，精简代码，优化错误处理
2. **思考极致精简**：思考不要有完整自然语言表述，改用符号化/缩写式思考，AI 自己能明白即可，大幅降低 token 消耗

## 一、思考极致精简（prompt.rs）

### 现状
当前提示词要求"用关键词和短语思考，不要写完整句子"，但仍包含自然语言（如「中心控制」「e4 占据中心」）。

### 目标
思考改为**符号化/缩写式**，不输出任何完整自然语言表述：
- 用棋盘坐标 + 符号 + 简短标记
- 去除"占据"、"弱点"、"进攻"等动词/形容词
- AI 自己能理解决策意图即可，人类无需完全看懂

### 精简前 vs 精简后对比

精简前（当前）：
```
中心控制 e4 占据中心 d5 弱点 王翼进攻 <pick>e2e4</pick>
```

精简后（目标）：
```
e4@d5? Nf3 Kside <pick>e2e4</pick>
```

### 修改方案

#### 中文版系统消息
```
你是国际象棋大师。从合法走法中选出最强的一手。

思考格式（极致精简，禁止自然语言句子）：
- 只用坐标、符号、简短标记思考。例：e4@d5? Nf3 Kside+ O-O
- 禁止完整句子、禁止动词形容词。错误示例：「占据中心」「弱点在d5」
- 正确示例：@center、d5?、Kside+、Qx、O-O
- 标记含义（AI 自行理解）：@=位置、?=疑问、+=优势、x=吃子、O-O=易位

输出格式：
- 思考时每聚焦一个候选走法就输出 <pick>UCI</pick>，可多次输出
- 重要：输出 <move> 前必须先输出至少一个 <pick>
- 最终走法用 <move>UCI</move> 包裹
- <pick> 和 <move> 必须是合法走法之一
- 思考模式开启时，分析（含 <pick>）放 reasoning，<move> 放 content
```

#### 英文版系统消息
```
You are a chess grandmaster. Pick the strongest move from legal moves.

THINKING FORMAT (ULTRA-TERSE, NO NATURAL LANGUAGE SENTENCES):
- Think ONLY in coordinates, symbols, short tokens. E.g.: e4@d5? Nf3 Kside+ O-O
- NO full sentences, NO verbs/adjectives. BAD: "controls center", "d5 is weak"
- GOOD: @center, d5?, Kside+, Qx, O-O
- Token meanings (AI self-interprets): @=position, ?=questionable, +=advantage, x=capture, O-O=castle

OUTPUT FORMAT:
- Wrap each candidate as <pick>UCI</pick> while deliberating. Output multiple times.
- IMPORTANT: Before <move>, you MUST output at least one <pick>.
- Final move: <move>UCI</move>.
- <pick> and <move> must be legal moves.
- If thinking mode on: analysis (incl <pick>) in reasoning, <move> in content.
```

## 二、代码优化/清理

### 2.1 移除诊断日志（deepseek.rs）

移除所有 `[diag]` 前缀的诊断日志（开发期排查用，生产环境无需）：
- `[diag] chat_stream 请求发起` 日志
- `[diag] HTTP 响应` 日志
- `[diag] SSE 首个 data 行` 日志
- `[diag] chunk N` 前10个chunk的解析日志
- `[diag] 流结束` 日志
- `[diag-raw]` 原始行日志
- `raw_lines` 收集逻辑

保留：
- `[pick] 检测到新 pick` 日志（运维有价值）
- API 错误日志（warn 级别）
- `emit` 失败警告日志

### 2.2 移除 ping 调试命令

移除以下文件中的 ping 相关代码：
- `deepseek.rs`：`PingResult` 结构体、`ping()` 方法
- `commands.rs`：`ping_deepseek` 命令
- `lib.rs`：注册 `ping_deepseek` 的 invoke_handler
- `api.ts`：`pingDeepseek` 函数、`PingResult` 接口
- `Settings.svelte`：测试连接按钮、pingResult 展示 UI

### 2.3 移除 api_key_masked 诊断日志（commands.rs）

移除 `[diag] ai_move 使用 client api_key=` 日志，保留 `api_key_masked()` 方法（ping 用），如果 ping 移除则一并移除。

### 2.4 精简 commands.rs 的 ai_move 诊断日志

移除 `[diag] attempt N content=... reasoning=...` 和预览日志，保留错误日志。

## 任务清单

1. **prompt.rs**：修改中英文系统消息，改为符号化/缩写式思考
2. **deepseek.rs**：移除所有 `[diag]` 和 `[diag-raw]` 诊断日志及 `raw_lines` 收集逻辑
3. **deepseek.rs**：移除 `PingResult` 结构体和 `ping()` 方法
4. **commands.rs**：移除 `ping_deepseek` 命令和诊断日志
5. **lib.rs**：移除 `ping_deepseek` 注册
6. **api.ts**：移除 `pingDeepseek` 函数和 `PingResult` 接口
7. **Settings.svelte**：移除测试连接按钮和 ping 结果展示 UI
8. **验证**：cargo check + cargo test + npm run check
9. **下一个 PLAN**：询问用户下一个方向

## 非目标

- 不修改棋子动画/音效逻辑
- 不修改思考语言切换 UI（中文/英文仍可选）
- 不修改持久化逻辑
- 不修改 FLIP 动画
- 不重构整体架构

## 验证命令

```bash
cd c:\Users\24453\Desktop\AI国象\src-tauri && cargo check && cargo test --lib
cd c:\Users\24453\Desktop\AI国象 && npm run check
```
