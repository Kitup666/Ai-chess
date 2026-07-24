# PLAN: 修复提子动画 / 思考框颤动 / 思考语言精简

## 问题描述

用户反馈三个问题：
1. **提子动画没看到**：可能是没触发，也可能是 AI 输出格式不对
2. **思考框出现时布局会颤动一下**：需要修复
3. **思考语言精简**：不用输出完整语句（已实现，需验证）
4. **思考语言能在设置里调整**（已实现，需验证）

## 根因分析

### 问题 1：提子动画不触发

**后端**（`deepseek.rs`）已实现：
- `extract_last_pick()`：提取 `<pick>UCI</pick>` 标签
- `extract_last_legal_move()`：兜底扫描末尾 200 字符窗口中的合法 UCI
- 流式过程中 emit `ai-pick` 事件

**前端**（`Board.svelte`）已实现：
- `liftingSquare` 依赖 `$aiPick` 和 `$aiThinking`
- `class:lifting` 触发 `piece-hesitate` 动画

**根因**：
- `triggerAiMove` 中 `aiMove()` 返回后**立即** `aiThinking.set(false)`，导致 `liftingSquare` 变 null，动画瞬间停止
- 思考模式关闭时，`content` 流式过程中只有最终的 `<move>`，兜底扫描在 `<move>` 完整出现时才匹配，此时 `aiMove()` 即将返回，动画时间几乎为零
- 思考模式开启时，AI 可能不遵守 `<pick>` 格式，关键词思考中可能不提及完整 UCI

### 问题 2：思考框布局颤动

**根因**：`.thinking-strip` 虽然注释说"绝对定位"，但实际 CSS 没有 `position: absolute`，它作为 flex 子元素参与 `.board-wrap` 布局流，出现/消失时挤压棋盘导致颤动。

### 问题 3 & 4：思考语言设置 / 精简

**状态**：已实现
- `Settings.svelte` 有中文/英文切换控件
- `prompt.rs` 中 `system_message` 和 `user_message` 支持 language 参数
- 提示词已要求 AI 使用关键词短语思考

## 修复方案

### 修复 1：思考框布局颤动（Board.svelte）

将 `.thinking-strip` 改为真正的绝对定位：

```css
.thinking-strip {
  position: absolute;          /* 新增：绝对定位 */
  left: 50%;                   /* 新增：水平居中 */
  transform: translateX(-50%); /* 新增：水平居中偏移 */
  /* 移除 flex-shrink: 0 */
  /* 其余样式保持不变 */
}

/* AI 在上方时，思考条放在棋盘上方 */
.board-frame.ai-top .thinking-strip {
  bottom: 100%;
  margin-bottom: var(--sp-3);
}

/* AI 在下方时，思考条放在棋盘下方 */
.board-frame:not(.ai-top) .thinking-strip {
  top: 100%;
  margin-top: var(--sp-3);
}
```

`.board-wrap` 已有 `position: relative`，作为定位上下文。思考条脱离布局流，不再挤压棋盘。

### 修复 2：提子动画不触发

#### 2a. 前端延迟落子（Board.svelte）

`triggerAiMove` 中，`aiMove()` 返回后不立即应用走法，而是保持 `aiThinking=true` 一小段时间让举棋动画可见：

```typescript
async function triggerAiMove() {
  aiThinking.set(true);
  try {
    const result = await aiMove();
    const lm = result.state.last_move;

    // 新增：如果 AI 举棋了，延迟一小段时间让举棋动画可见
    if ($aiPick && lm && $aiPick.startsWith(lm.from)) {
      await new Promise((r) => setTimeout(r, 600)); // 举棋犹豫 600ms
    }

    // 记录 from 棋子位置
    let fromRect: DOMRect | null = null;
    if (lm) {
      const fromEl = document.querySelector(`[aria-label="${lm.from}"] .piece`) as HTMLElement | null;
      if (fromEl) fromRect = fromEl.getBoundingClientRect();
    }

    updateGameState(result.state);
    await tick();
    // ... FLIP 动画 ...
  } finally {
    aiThinking.set(false);
    aiPick.set(null);
  }
}
```

#### 2b. 强化提示词（prompt.rs）

在系统消息中明确要求 AI **必须**先输出至少一个 `<pick>` 再输出 `<move>`：

中文版新增：
```
- 重要：在输出 <move> 之前，必须先输出至少一个 <pick> 表示你正在考虑的走法。
```

英文版新增：
```
- IMPORTANT: Before outputting <move>, you MUST output at least one <pick> indicating the move you're considering.
```

#### 2c. 后端兜底优化（deepseek.rs）

当思考模式关闭且 content 流式过程中没有 `<pick>` 时，在 `<move>` 标签开始出现时（即 `<move>` 已生成但未闭合）就触发举棋：

当前 `extract_last_pick` 只匹配完整闭合的 `<pick>...</pick>`。新增对未闭合 `<move>` 的检测：当检测到 `<move>` 开头但未闭合时，提取后面的 4-5 字符作为举棋（如果属于合法走法）。

### 修复 3 & 4：验证思考语言设置/精简

无需代码改动，执行验证即可：
- 切换中文/英文设置，确认提示词语言切换
- 观察 AI 思考输出是否为关键词/短语

## 任务清单

1. **Board.svelte**：`.thinking-strip` 改为绝对定位，修复布局颤动
2. **Board.svelte**：`triggerAiMove` 延迟落子，让举棋动画可见
3. **prompt.rs**：强化提示词，要求 AI 必须先输出 `<pick>` 再输出 `<move>`
4. **deepseek.rs**：新增未闭合 `<move>` 标签的提前检测，触发举棋
5. **验证**：`cargo check` + `npm run check`
6. **下一个 PLAN**：询问用户下一个 PLAN 和 SPEC 方向

## 非目标

- 不修改音效逻辑
- 不修改 FLIP 移动动画
- 不修改将军/将杀特效
- 不修改思考语言切换 UI（已实现）
- 不修改思考内容轮换显示逻辑

## 验证命令

```bash
cd c:\Users\24453\Desktop\AI国象\src-tauri && cargo check
cd c:\Users\24453\Desktop\AI国象 && npm run check
```
