# PLAN & SPEC: 注意事项机制（跨回合记忆）

## 目标

当前每次请求独立无历史，AI 重复思考局面。引入"注意事项"机制：
1. AI 走完子后输出 `!!注意事项`（局面评估+关键威胁+战略方向）
2. 后端提取并存到 AppState
3. 下次请求把上次的注意事项注入 user_message，避免重复思考
4. 持久化注意事项，重启后恢复

## 一、当前状态分析

### 消息构建（prompt.rs L164-166）
```rust
pub fn build_messages(game, ai_side, language) -> Vec<ChatMessage> {
    vec![system_message(language), user_message(game, ai_side, language)]
}
```
- 每次只构建 [system, user] 两条消息，无历史对话
- AI 看不到之前思考，每次重新分析局面

### AppState（game_state.rs L8-12）
```rust
pub struct AppState {
    pub game: Mutex<Option<ChessGame>>,
    pub deepseek: Mutex<Option<DeepSeekClient>>,
    pub settings: Mutex<Settings>,
}
```
- 无注意事项字段

### ai_move 流程（commands.rs L136-217）
- 构建消息 → chat_stream → 解析走法 → 应用 → 重试
- 无注意事项提取/存储

## 二、具体改动

### 2.1 game_state.rs —— AppState 新增 last_notes

**文件**：`c:\Users\24453\Desktop\AI国象\src-tauri\src\game_state.rs`

**AppState 新增字段**（L8-12）：
```rust
pub struct AppState {
    pub game: Mutex<Option<ChessGame>>,
    pub deepseek: Mutex<Option<DeepSeekClient>>,
    pub settings: Mutex<Settings>,
    /// 上次 AI 走子后输出的注意事项（局面+威胁+战略），下次请求注入 user_message
    pub last_notes: Mutex<String>,
}
```

**AppState::new()**（L40-48）：初始化 `last_notes: Mutex::new(String::new())`

### 2.2 deepseek.rs —— 新增 extract_notes 函数

**文件**：`c:\Users\24453\Desktop\AI国象\src-tauri\src\deepseek.rs`

**新增函数**（放在 extract_at_pick 附近）：
```rust
/// 从 AI 输出中提取 `!!` 前缀的注意事项
/// 
/// AI 走完子后输出 `!!局面+威胁+战略` 作为下次思考的参考。
/// 格式：`!!` 后跟注意事项文本，到行尾或下一个标签结束。
/// 优先从 content 提取（走法后），content 为空则从 reasoning 提取。
fn extract_notes(content: &str, reasoning: &str) -> String {
    // 优先从 content 提取（走法标签后的 !! ）
    if let Some(notes) = extract_notes_from_text(content) {
        return notes;
    }
    // content 无则从 reasoning 提取
    if let Some(notes) = extract_notes_from_text(reasoning) {
        return notes;
    }
    String::new()
}

fn extract_notes_from_text(text: &str) -> Option<String> {
    // 找最后一个 !! （可能多次输出，取最新）
    let mut best: Option<String> = None;
    let mut search_from = 0;
    while let Some(pos) = text[search_from..].find("!!") {
        let start = search_from + pos + 2; // 跳过 !!
        // 注意事项到行尾或下一个标签 < 或 @ 结束
        let rest = &text[start..];
        let end = rest.find(|c: char| c == '\n' || c == '<' || c == '@').unwrap_or(rest.len());
        let notes = rest[..end].trim().to_string();
        if !notes.is_empty() {
            best = Some(notes);
        }
        search_from = start;
    }
    best
}
```

**新增测试**（test 模块）：
```rust
#[test]
fn test_extract_notes() {
    // content 中提取
    assert_eq!(extract_notes("<move>e2e4</move> !!白优+0.5 黑威胁d5", ""), "白优+0.5 黑威胁d5".to_string());
    // 多次输出取最新
    assert_eq!(extract_notes("!!白优 !!白优+0.5", ""), "白优+0.5".to_string());
    // content 为空从 reasoning 提取
    assert_eq!(extract_notes("", "!!黑威胁f7 白攻王翼"), "黑威胁f7 白攻王翼".to_string());
    // 无 !!
    assert_eq!(extract_notes("e2e4 <move>e2e4</move>", "思考中"), "".to_string());
    // !! 后到行尾
    assert_eq!(extract_notes("!!白优+0.5\n其他", ""), "白优+0.5".to_string());
}
```

### 2.3 prompt.rs —— 系统消息 + user_message 改造

**文件**：`c:\Users\24453\Desktop\AI国象\src-tauri\src\prompt.rs`

**系统消息新增注意事项说明**（L50-72 系统消息内容）：
- 中文：`输出：走完<move>后，输出 !!注意事项（局面评估+关键威胁+战略方向，精简关键词）。例：!!白优+0.5 黑威胁d5 白攻王翼`
- 英文：`OUTPUT: After <move>, output !!notes (evaluation+threats+strategy, terse). E.g. !!White+0.5 Black threatens d5 White attacks kingside`

**build_messages 签名改造**（L164-166）：
```rust
pub fn build_messages(game: &ChessGame, ai_side: &str, language: &str, last_notes: &str) -> Vec<ChatMessage> {
    vec![system_message(language), user_message(game, ai_side, language, last_notes)]
}
```

**user_message 注入注意事项**（L83-159）：
- 新增参数 `last_notes: &str`
- 如果 last_notes 非空，在 user_message 中注入：`上次注意事项: {last_notes}`（中文）/ `Last notes: {last_notes}`（英文）
- 放在 FEN 之后、合法走法之前，提供上下文

### 2.4 commands.rs —— ai_move 提取并存储注意事项

**文件**：`c:\Users\24453\Desktop\AI国象\src-tauri\src\commands.rs`

**ai_move 改造**（L136-217）：

1. **构建消息时注入 last_notes**（L149）：
```rust
let notes = state.last_notes.lock().unwrap().clone();
let msgs = build_messages(game, &ai_side, &lang, &notes);
```

2. **成功走子后提取并存储注意事项**（L194-205 成功分支）：
```rust
// 提取注意事项并存到 AppState
let notes = crate::deepseek::extract_notes(&content, &reasoning);
if !notes.is_empty() {
    *state.last_notes.lock().unwrap() = notes.clone();
    // 持久化
    let save = build_save_data(&state, "playing");
    if let Err(e) = crate::persistence::save(&app, &save) {
        log::warn!("保存存档失败: {}", e);
    }
}
```

3. **重开/新游戏时清空注意事项**（start_game, reset_game）：
```rust
*state.last_notes.lock().unwrap() = String::new();
```

### 2.5 persistence.rs —— 持久化 last_notes

**文件**：`c:\Users\24453\Desktop\AI国象\src-tauri\src\persistence.rs`

**SaveData 新增字段**：
```rust
#[derive(Serialize, Deserialize, Clone)]
pub struct SaveData {
    pub has_game: bool,
    pub status: String,
    pub fen: String,
    pub player_side: String,
    pub move_history: Vec<String>,
    pub ply: u32,
    pub settings: SettingsSave,
    /// 上次 AI 注意事项（跨回合记忆），旧存档无此字段回退空
    #[serde(default)]
    pub last_notes: String,
}
```

**build_save_data 新增**（commands.rs）：
```rust
let last_notes = state.last_notes.lock().unwrap().clone();
// ... 写入 SaveData
```

### 2.6 lib.rs —— 启动恢复 last_notes

**文件**：`c:\Users\24453\Desktop\AI国象\src-tauri\src\lib.rs`

**setup 恢复逻辑**（L29-40 附近）：
```rust
// 恢复注意事项
*state.last_notes.lock().unwrap() = save.last_notes.clone();
```

### 2.7 前端无改动

注意事项机制完全在后端，前端不需要改动（不显示注意事项，只用于 AI 跨回合记忆）。

## 三、注意事项内容设计

AI 走完子后输出 `!!` 后的注意事项，包含三部分（精简关键词）：
1. **局面评估**：谁优、优势多大。例：`白优+0.5`、`均势`、`黑略优`
2. **关键威胁**：对方的威胁、自己的机会。例：`黑威胁d5`、`白有机会f7`
3. **战略方向**：下一步方向。例：`白攻王翼`、`黑守中心`、`准备O-O`

**示例**：
```
<move>e2e4</move> !!白优+0.5 黑威胁d5 白攻王翼
```

## 四、预期效果

- **省 token**：AI 不用每次重新评估局面，参考上次注意事项直接聚焦关键点
- **质量提升**：AI 有连续性，不会忘记之前的战略方向
- **额外成本**：每次输出多 ~20-30 token（注意事项），但省下的重复思考 token 更多

## 五、非目标

- 不在前端显示注意事项（纯后端记忆机制）
- 不改动举棋动画/音效
- 不改动思考模式/强度/语言切换
- 不引入完整对话历史（只传注意事项，不传完整 reasoning）

## 六、验证

```bash
cd c:\Users\24453\Desktop\AI国象\src-tauri && cargo check && cargo test --lib
cd c:\Users\24453\Desktop\AI国象 && npm run check
```

重点验证：
- cargo test 中 `test_extract_notes` 通过
- cargo check 0 errors
- npm run check 0 errors
- 旧存档（无 last_notes 字段）能正常加载，回退空字符串

## 七、任务清单

1. game_state.rs：AppState 新增 last_notes 字段 + new() 初始化
2. deepseek.rs：新增 extract_notes 和 extract_notes_from_text 函数
3. deepseek.rs：新增 test_extract_notes 测试用例
4. prompt.rs：系统消息新增注意事项输出说明（中英文）
5. prompt.rs：build_messages 新增 last_notes 参数
6. prompt.rs：user_message 注入上次注意事项（中英文）
7. commands.rs：ai_move 注入 last_notes + 成功后提取存储
8. commands.rs：start_game/reset_game 清空 last_notes
9. commands.rs：build_save_data 写入 last_notes
10. persistence.rs：SaveData 新增 last_notes 字段（serde default）
11. lib.rs：启动恢复 last_notes
12. 验证：cargo check + cargo test + npm run check
13. 设定下一个 PLAN 和 SPEC（询问用户）
