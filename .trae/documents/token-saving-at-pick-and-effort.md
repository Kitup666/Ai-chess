# PLAN & SPEC: 省钱优化 —— 标签缩减为 @ + reasoning_effort 可调

## 目标

降低 DeepSeek API 输出 token 消耗。核心手段：
1. **举棋标签缩减**：`<pick>UCI</pick>` → `@UCI`（每次举棋从 ~16 token 降到 ~3 token）
2. **最终走法标签缩减**：`<move>UCI</move>` → `@UCI!`（用 `!` 后缀区分最终走法，或保持 `<move>` 不变以保解析稳定性）
3. **reasoning_effort 可调**：新增 high/max 切换（DeepSeek 仅支持这两档），默认 high
4. **非思考模式精简提示词**：去掉 pick 相关说明

## 一、当前状态分析

### 输出 token 消耗点
- **reasoning（最大头）**：`reasoning_effort: high` 固定，思考链可能几百到上千 token
- **`<pick>UCI</pick>` 标签**：AI 每考虑一个候选输出一次，多次输出累计，每次 ~16 字符
- **思考文本**：UCI 坐标 + 符号标记（已精简）

### 关键约束
- DeepSeek `reasoning_effort` 仅支持 `high` 和 `max` 两档（用户确认）
- 举棋动画依赖后端解析举棋信号；只要后端能从文本提取 `@UCI`，动画就能继续工作
- `<move>UCI</move>` 是最终走法解析的关键，解析器已稳定，**不改动**以避免引入解析 bug

### 用户决策
- 举棋标签从 `<pick>UCI</pick>` 缩减为 `@UCI`
- 保留举棋动画（后端改为提取 `@UCI`）
- reasoning_effort 可调（high/max），默认 high

## 二、具体改动

### 2.1 prompt.rs —— 举棋标签改为 @UCI

**文件**：`c:\Users\24453\Desktop\AI国象\src-tauri\src\prompt.rs`

**中文系统消息**（L58-73）：
- `<pick>UCI</pick>` → `@UCI`
- 输出格式说明改为：思考时每聚焦一个候选走法输出 `@UCI`，可多次输出
- 最终走法仍用 `<move>UCI</move>`（保持解析稳定性）

**英文系统消息**（L74-90）：
- 同步修改

**示例**：
```
旧：e2e4 d5? <pick>g1f3</pick> <pick>e1g1</pick> <move>e2e4</move>
新：e2e4 d5? @g1f3 @e1g1 <move>e2e4</move>
```

### 2.2 deepseek.rs —— 举棋检测改为提取 @UCI

**文件**：`c:\Users\24453\Desktop\AI国象\src-tauri\src\deepseek.rs`

**替换 `extract_last_pick` 函数**（L228-253）：
- 旧：匹配 `<pick>UCI</pick>` 完整闭合标签
- 新：匹配 `@` 后跟 4-5 字符 UCI 走法（`@[a-h][1-8][a-h][1-8][qrbn]?`）
- 取最后一个匹配

**保留兜底逻辑**（L193-204）：
- `extract_open_move_tag`（未闭合 `<move>`）保留
- `extract_last_legal_move`（合法走法扫描）保留

**举棋检测优先级调整**（L192-204）：
1. `extract_at_pick`（新：提取 `@UCI`）← 新增，最高优先级
2. `extract_open_move_tag`（未闭合 `<move>`）
3. `extract_last_legal_move`（合法走法兜底扫描）

**测试用例更新**（L328-340）：
- `test_extract_last_pick` → `test_extract_at_pick`
- 测试 `@e2e4`、多个 `@`、大小写、无 `@` 等场景

### 2.3 game_state.rs —— 新增 reasoning_effort 设置

**文件**：`c:\Users\24453\Desktop\AI国象\src-tauri\src\game_state.rs`

**Settings 结构**（L14-23）新增字段：
```rust
/// reasoning_effort: "high" | "max"
pub reasoning_effort: String,
```

**Default**（L25-34）：`reasoning_effort: "high".to_string()`

**SettingsDto**（L91-97）和 **StartGameArgs**（L78-88）新增对应字段。

### 2.4 deepseek.rs —— reasoning_effort 传入客户端

**文件**：`c:\Users\24453\Desktop\AI国象\src-tauri\src\deepseek.rs`

**DeepSeekClient**（L63-69）新增字段：
```rust
reasoning_effort: String,
```

**new()**（L72-79）新增参数 `reasoning_effort: String`。

**build_request()**（L82-98）：
- 旧：`reasoning_effort: if self.thinking { Some("high") } else { None }`
- 新：`reasoning_effort: if self.thinking { Some(self.reasoning_effort.clone()) } else { None }`

### 2.5 commands.rs —— 传递 reasoning_effort

**文件**：`c:\Users\24453\Desktop\AI国象\src-tauri\src\commands.rs`

**update_settings**（L10-42）：
- 新增参数 `reasoning_effort: String`
- 写入 settings 和 client

**start_game**（L44-83）：
- 从 args 读取 reasoning_effort，传给 client

**ai_move**（L121-）：
- 从 settings 读取 reasoning_effort（已在 client 中）

### 2.6 lib.rs —— 启动恢复 reasoning_effort

**文件**：`c:\Users\24453\Desktop\AI国象\src-tauri\src\lib.rs`

**setup 恢复设置**（L29-40）：
- 恢复 `reasoning_effort` 字段
- 传给 DeepSeekClient::new

### 2.7 persistence.rs —— 持久化 reasoning_effort

**文件**：`c:\Users\24453\Desktop\AI国象\src-tauri\src\persistence.rs`

**SettingsSave**（L13-20）新增：
```rust
#[serde(default = "default_effort")]
pub reasoning_effort: String,
```
新增 `default_effort()` 返回 `"high"`。

### 2.8 前端类型与 API

**`src/lib/types.ts`**：
- `StartGameArgs` 新增 `reasoning_effort?: string`
- `SettingsDto` 新增 `reasoning_effort: string`

**`src/lib/api.ts`**：
- `updateSettingsApi` 新增参数 `reasoningEffort: string`

### 2.9 前端 settings store

**`src/lib/stores/settings.ts`**：
- `Settings` 新增 `reasoningEffort: "high" | "max"`
- 初始值 `"high"`

### 2.10 Settings.svelte —— UI 新增思考强度选择

**文件**：`c:\Users\24453\Desktop\AI国象\src\lib\components\Settings.svelte`

在「思考模式」开关下方新增「思考强度」分段选择器：
- 仅在 thinking 开启时显示
- 两个选项：`High · 省钱` / `Max · 最强`
- hint 说明：High 平衡强度与成本，Max 最强但更贵

**handleStart / handleApplySettings**：
- 传递 reasoningEffort 参数

### 2.11 App.svelte —— loadState 恢复 reasoning_effort

**文件**：`c:\Users\24453\Desktop\AI国象\src\App.svelte`

**onMount loadState**（L64-84）：
- 恢复 `reasoningEffort` 到 settings

## 三、验证

```bash
cd c:\Users\24453\Desktop\AI国象\src-tauri && cargo check && cargo test --lib
cd c:\Users\24453\Desktop\AI国象 && npm run check
```

重点验证：
- `cargo test` 中 `test_extract_at_pick` 通过
- `npm run check` 0 errors
- 旧存档（无 reasoning_effort 字段）能正常加载，回退 "high"

## 四、预期省 token 效果

以一局 AI 思考 5 次举棋为例：
- 旧：`<pick>e2e4</pick><pick>g1f3</pick><pick>e1g1</pick><pick>d2d4</pick><pick>b1c3</pick>` ≈ 80 字符
- 新：`@e2e4 @g1f3 @e1g1 @d2d4 @b1c3` ≈ 30 字符

输出 token 减少约 60%（举棋标签部分），整体输出 token 视思考链长度减少 10-30%。

## 五、非目标

- 不改动 `<move>UCI</move>` 最终走法标签（解析稳定性）
- 不改动棋子动画/音效
- 不改动思考语言切换
- 不改动 FLIP 动画
- 不引入 max_tokens 限制（thinking 模式下会截断）

## 六、任务清单

1. prompt.rs：`<pick>UCI</pick>` → `@UCI`（中英文系统消息）
2. deepseek.rs：新增 `extract_at_pick` 函数，替换 `extract_last_pick`
3. deepseek.rs：举棋检测优先级调整，`extract_at_pick` 最高
4. deepseek.rs：测试用例更新
5. game_state.rs：Settings/SettingsDto/StartGameArgs 新增 reasoning_effort
6. deepseek.rs：DeepSeekClient 新增 reasoning_effort 字段和参数
7. commands.rs：update_settings/start_game 传递 reasoning_effort
8. lib.rs：启动恢复 reasoning_effort
9. persistence.rs：SettingsSave 新增 reasoning_effort + default
10. types.ts：StartGameArgs/SettingsDto 新增字段
11. api.ts：updateSettingsApi 新增参数
12. settings.ts：Settings 新增 reasoningEffort
13. Settings.svelte：UI 新增思考强度选择 + 传参
14. App.svelte：loadState 恢复 reasoningEffort
15. 验证：cargo check + cargo test + npm run check
16. 设定下一个 PLAN 和 SPEC（询问用户）
