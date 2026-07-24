# PLAN & SPEC: 思考模式深度省钱优化

## 目标

基于 DeepSeek 官方文档调研，进一步降低 API 成本。核心手段：
1. **重试时不传回 reasoning**（省输入 token，重试场景最大浪费点）
2. **prefix caching 确认与保护**（系统消息已命中缓存，确保不被破坏）
3. **规则文本压缩**（减少系统消息输入 token）
4. **英文思考默认提示**（英文 token 比中文少 30-50%）

## 一、调研发现（官方文档确认）

### 1.1 Prefix Caching（自动，已生效）
- **价格差 50 倍**：缓存命中 0.02元/百万 vs 未命中 1元/百万（flash）
- DeepSeek 自动缓存相同前缀，无需额外参数
- 当前系统消息固定不变 → 已命中缓存，系统消息部分几乎免费
- **保护措施**：确保系统消息内容完全不变（不随状态变化）

### 1.2 reasoning_content 多轮拼接规则（关键！）
- 官方明确：两个 user 消息之间，未进行工具调用时，assistant 的 `reasoning_content` **无需参与上下文拼接**，传入会被忽略
- **当前问题**：`commands.rs` 重试时，思考模式下 content 为空，`assistant_text = reasoning`，把长 reasoning 当作 content 传回 assistant 消息
- 这导致：① 浪费输入 token（reasoning 几百 token 每次重试累积）② reasoning 作为 content 传回可能导致模型混乱

### 1.3 reasoning_effort 档位（已实现）
- 官方确认只有 high/max 两档（low/medium 映射为 high，xhigh 映射为 max）
- 已实现 high/max 切换

### 1.4 模型价格对比
| 模型 | 输入(缓存命中) | 输入(未命中) | 输出 |
|---|---|---|---|
| deepseek-v4-flash | 0.02元 | 1元 | 2元 |
| deepseek-v4-pro | 0.025元 | 3元 | 6元 |
- flash 输出比 pro 便宜 3 倍，已支持选择

## 二、当前状态分析

### 重试逻辑问题（commands.rs L161-217）
```rust
let assistant_text = if !content.trim().is_empty() {
    content.clone()
} else {
    reasoning.clone()  // 问题：思考模式下 content 为空，用 reasoning 当 assistant 消息
};
// ...
messages.push(ChatMessage {
    role: "assistant".to_string(),
    content: assistant_text,  // 浪费：把几百 token 的 reasoning 传回
});
```

### 系统消息大小（prompt.rs）
- `CHESS_RULES_BRIEF_ZH` 约 200 字符
- `system_message` 格式化后约 400 字符
- 命中缓存后几乎免费，但仍可压缩减少首次缓存未命中的成本

## 三、具体改动

### 3.1 commands.rs —— 重试时不传回 reasoning

**文件**：`c:\Users\24453\Desktop\AI国象\src-tauri\src\commands.rs`

**改动**（L161-217 重试逻辑）：
- 思考模式下 content 为空时，assistant 消息只传空 content（不传 reasoning）
- 遵循官方规则：reasoning_content 无需拼接，传空 content 即可

```rust
// 修改前
let assistant_text = if !content.trim().is_empty() {
    content.clone()
} else {
    reasoning.clone()
};

// 修改后
// 遵循 DeepSeek 官方规则：思考模式下 reasoning_content 无需参与多轮拼接
// assistant 消息只传 content（即使为空），不传 reasoning，省输入 token
let assistant_text = content.clone();  // 思考模式下可能为空，这是正确的
```

**影响**：重试时不再把几百 token 的 reasoning 作为 content 传回，每次重试省几百输入 token。

### 3.2 prompt.rs —— 规则文本压缩

**文件**：`c:\Users\24453\Desktop\AI国象\src-tauri\src\prompt.rs`

**压缩 `CHESS_RULES_BRIEF_ZH`**（L5-20）：
- 子力价值：`兵1 马3 象3 车5 后9 王∞ 双象+0.5`
- 避免送子：`目标被攻击且无保护=送子禁。兑子须能回吃或有利交换`
- 保留核心规则（UCI 格式、合法走法），压缩说明文字

**压缩 `CHESS_RULES_BRIEF_EN`**（L23-38）：
- 同步压缩英文版

**压缩系统消息格式说明**（L58-90）：
- 去掉重复的 UCI 解释（规则里已有）
- 精简标记说明

### 3.3 Settings.svelte —— 英文思考省钱提示

**文件**：`c:\Users\24453\Desktop\AI国象\src\lib\components\Settings.svelte`

**思考语言 hint 修改**（L165 附近）：
- 当前：`AI 思考过程显示的语言。思考使用精简关键词，不输出完整句子。`
- 改为：`AI 思考过程显示的语言。English 更省 token（推荐省钱）。思考使用精简关键词。`

### 3.4 prefix caching 保护（无代码改动，仅注释）

**文件**：`c:\Users\24453\Desktop\AI国象\src-tauri\src\prompt.rs`

在 `system_message` 函数添加注释说明：
- 系统消息必须保持固定不变（不随游戏状态变化），否则破坏 prefix caching
- 系统消息命中缓存后按 0.02元/百万计费，几乎免费

## 四、预期省钱效果

| 优化点 | 节省 | 场景 |
|---|---|---|
| 重试不传 reasoning | 每次重试省 200-800 输入 token | 重试场景（非法走法时） |
| 规则文本压缩 | 系统消息减少 ~100 token（首次缓存未命中时） | 每局首次请求 |
| 英文思考提示 | 输出 token 减少 30-50% | 用户选英文时 |
| prefix caching | 系统消息几乎免费（已生效） | 每次请求 |

**主要收益**：重试场景省输入 token（最大浪费点），日常请求靠 prefix caching 已优化。

## 五、非目标

- 不改动 reasoning_effort（已实现 high/max）
- 不改动思考模式开关（已实现）
- 不改动模型选择（已实现 flash/pro）
- 不引入对话前缀续写（与举棋动画冲突）
- 不引入 max_tokens 限制（thinking 模式下会截断）
- 不改动举棋动画/音效

## 六、验证

```bash
cd c:\Users\24453\Desktop\AI国象\src-tauri && cargo check && cargo test --lib
cd c:\Users\24453\Desktop\AI国象 && npm run check
```

重点验证：
- cargo check 0 errors
- cargo test 6/6 passed（提取函数未改动）
- npm run check 0 errors
- 重试逻辑：assistant 消息 content 为空时不传 reasoning

## 七、任务清单

1. commands.rs：重试时 assistant 消息只传 content，不传 reasoning
2. prompt.rs：压缩 CHESS_RULES_BRIEF_ZH（子力价值、避免送子）
3. prompt.rs：压缩 CHESS_RULES_BRIEF_EN（同步）
4. prompt.rs：压缩系统消息格式说明 + 添加 caching 保护注释
5. Settings.svelte：思考语言 hint 提示英文更省钱
6. 验证：cargo check + cargo test + npm run check
7. 设定下一个 PLAN 和 SPEC（询问用户）
