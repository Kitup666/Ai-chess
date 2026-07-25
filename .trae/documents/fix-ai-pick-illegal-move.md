# 修复 DeepSeek 举棋动画显示非法走法（从空格出子/操纵对方棋子）

## 总结

用户反馈"DeepSeek 有时候会从没有子的地方出子，甚至控制我的棋子"。根因是 `extract_at_pick` 函数（[deepseek.rs](file:///c:/Users/24453/Desktop/AI国象/src-tauri/src/deepseek.rs) 第377行）从 AI 思考文本中提取 `@UCI` 举棋标记时，**只校验 UCI 格式（4-5 字符），不校验是否是当前局面的合法走法**。

当 AI 在思考中分析对手应招时（如 `%应对 e2e4→黑d7d5→白e4xd5` 或 `@d7d5 因黑应招`），文本中的对方走法（d7d5 是黑兵走法）会被 `extract_at_pick` 误提取为举棋，emit 给前端，导致举棋动画显示"从 d7 出子"。但 d7 上是黑兵（对方棋子）或已无子，用户感知为"AI 操纵我的棋子/从空格出子"。

**重要澄清**：最终落子走 `parse_and_validate` 严格验证（[commands.rs](file:///c:/Users/24453/Desktop/AI国象/src-tauri/src/commands.rs) 第30、295、389 行），所以**实际落子一定是合法的**。bug 只影响思考过程中的举棋动画显示。

## 当前状态分析

### 举棋提取优先级链（[deepseek.rs:316-341](file:///c:/Users/24453/Desktop/AI国象/src-tauri/src/deepseek.rs#L316-L341)）

```rust
let new_pick = extract_closed_move_tag(&full_content, &legal_moves)      // ✅ 校验合法
    .or_else(|| extract_closed_move_tag(&full_reasoning, &legal_moves)) // ✅ 校验合法
    .or_else(|| extract_at_pick(&full_reasoning))                       // ❌ 不校验！
    .or_else(|| extract_at_pick(&full_content))                         // ❌ 不校验！
    .or_else(|| extract_open_move_tag(&full_reasoning, &legal_moves))   // ✅ 校验合法
    .or_else(|| extract_open_move_tag(&full_content, &legal_moves))     // ✅ 校验合法
    .or_else(|| {
        let text = if !full_reasoning.is_empty() { &full_reasoning } else { &full_content };
        extract_last_legal_move(text, &legal_moves)                     // ✅ 校验合法
    });
```

四个举棋提取函数中，**只有 `extract_at_pick` 没有传入 `legal_moves` 参数**，是唯一的安全漏洞。

### bug 触发场景

AI 系统提示词（[prompt.rs](file:///c:/Users/24453/Desktop/AI国象/src-tauri/src/prompt.rs)）要求模型在思考中输出 `%应对 候选UCI→对手应招→我方反招` 行分析对手应招。这些"对手应招"也是 UCI 格式（如 d7d5、e7e5），且模型偶尔会用 `@UCI` 前缀标注它们。`extract_at_pick` 把这些对方走法误识为己方举棋。

### 前端无过滤

[App.svelte:243-246](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L243-L246) 直接把 ai-pick 事件 UCI 写入 store，[Board.svelte:64-91](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/Board.svelte#L64-L91) 据此播放举棋动画。前端不做合法性校验，完全依赖后端。

## 提议修改

### 修改 1：`extract_at_pick` 增加 `legal_moves` 参数并严格校验

**文件**：[c:\Users\24453\Desktop\AI国象\src-tauri\src\deepseek.rs](file:///c:/Users/24453/Desktop/AI国象/src-tauri/src/deepseek.rs)

**What**：
1. 函数签名从 `fn extract_at_pick(text: &str) -> Option<String>` 改为 `fn extract_at_pick(text: &str, legal_moves: &[String]) -> Option<String>`
2. 在每个 `best = Some(mv)` 赋值前，增加 `legal_moves.contains(&mv)` 校验（大小写不敏感比较）
3. 不在合法走法列表中的 @UCI 标记直接跳过（不作为举棋）

**Why**：与同文件中 `extract_open_move_tag`、`extract_closed_move_tag`、`extract_last_legal_move` 三个函数保持一致的合法性校验策略。这是单点修复，不引入新抽象。

**How**（伪代码）：
```rust
fn extract_at_pick(text: &str, legal_moves: &[String]) -> Option<String> {
    if legal_moves.is_empty() {
        return None;
    }
    // 预计算小写合法走法集合
    let legal_lower: std::collections::HashSet<String> = 
        legal_moves.iter().map(|m| m.to_lowercase()).collect();
    
    // ... 原有扫描逻辑 ...
    // 每次匹配到候选 UCI 时：
    if legal_lower.contains(&mv) {
        best = Some(mv);
    }
    // 否则跳过该 @UCI，继续扫描下一个
}
```

### 修改 2：更新 `chat_stream` 中的调用点

**文件**：[c:\Users\24453\Desktop\AI国象\src-tauri\src\deepseek.rs:319-320](file:///c:/Users/24453/Desktop/AI国象/src-tauri/src/deepseek.rs#L319-L320)

**What**：两处 `extract_at_pick(&full_reasoning)` / `extract_at_pick(&full_content)` 调用改为传入 `&legal_moves`。

```rust
.or_else(|| extract_at_pick(&full_reasoning, &legal_moves))
.or_else(|| extract_at_pick(&full_content, &legal_moves))
```

### 修改 3：补充单元测试

**文件**：[c:\Users\24453\Desktop\AI国象\src-tauri\src\deepseek.rs](file:///c:/Users/24453/Desktop/AI国象/src-tauri/src/deepseek.rs) 测试模块

**What**：在 `test_extract_at_pick` 测试下方新增测试，覆盖关键场景：

```rust
#[test]
fn test_extract_at_pick_validates_legal() {
    let legal = vec!["e2e4".to_string(), "d2d4".to_string(), "g1f3".to_string()];
    
    // @UCI 在合法列表中 → 正常提取
    assert_eq!(extract_at_pick("@e2e4 不好，@d2d4 更强", &legal), Some("d2d4".to_string()));
    
    // @UCI 不在合法列表中（对手走法）→ 跳过该 @UCI，返回 None 或下一个合法的
    assert_eq!(extract_at_pick("@d7d5 因黑应招", &legal), None);
    
    // 混合：@d7d5（非法）+ @e2e4（合法）→ 返回 e2e4
    assert_eq!(extract_at_pick("@d7d5 黑应招 @e2e4 控中", &legal), Some("e2e4".to_string()));
    
    // 空 legal_moves → 返回 None
    assert_eq!(extract_at_pick("@e2e4", &[]), None);
    
    // 大小写：legal_moves 大写时也能匹配 @e2e4
    let legal_mixed = vec!["E2E4".to_string()];
    assert_eq!(extract_at_pick("@e2e4", &legal_mixed), Some("e2e4".to_string()));
}
```

同时更新现有 `test_extract_at_pick` 测试：原测试调用 `extract_at_pick(text)` 无 legal_moves 参数，需改为 `extract_at_pick(text, &legal)`，并构造一个包含所有用例走法的 legal_moves 向量。

## 假设与决策

1. **假设**：用户报告的"从空格出子/操纵对方棋子"指的是**举棋动画**显示异常，而非最终落子非法。基于代码分析：最终落子必走 `parse_and_validate` 严格校验，不可能落子非法；只有举棋动画 emit 未校验。
2. **决策**：只修复 `extract_at_pick`，不改动其他三个已经正确的提取函数。最小化改动范围。
3. **决策**：不修改 prompt.rs 让 AI 不输出对方走法 —— `%应对` 行对 AI 战术推理有价值（分析对手应招），不应禁用。从解析端堵漏洞更合适。
4. **决策**：前端不增加合法性过滤 —— 后端是单一数据源，前端过滤会造成逻辑分散。后端修复后前端自然正确。

## 验证步骤

1. **单元测试**：`cd src-tauri && cargo test extract_at_pick` 全部通过（含新增的合法性校验测试）
2. **编译验证**：`cd src-tauri && cargo build` 无错误无警告
3. **手动场景验证**（可选，用户操作）：
   - 启动应用，人 vs DeepSeek 对局
   - 走几步让 AI 进入中局，观察思考过程中举棋动画
   - 确认举棋动画只显示当前己方合法走法，不再出现对方走法（如黑方应招 d7d5）
   - 确认举棋起点格子上一定有己方棋子
4. **回归验证**：举棋动画在 AI 输出 `<move>` 标签后仍能正确切换到最终走法（`extract_closed_move_tag` 优先级最高，不受影响）

## 后续 PLAN（待用户确认）

修复完成后，建议的下一步方向（需用户选择）：
- A) 复现验证：实际对局中观察多局，确认 bug 不再出现
- B) 举棋体验优化：举棋动画在 @UCI 频繁切换时可能闪烁，可加防抖
- C) 回到之前规划的 Lichess 风格 UI 改造
- D) 其他用户指定方向
