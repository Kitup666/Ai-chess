# 把继续对局和暂停对局放在风的加护边上

## 摘要

用户需求：把"继续对局"和"暂停对局"按钮放在底部状态栏"风的加护"开关旁边。

经澄清：
- 任何对局都能暂停（不限于自对弈模式）
- 单按钮切换文案（暂停↔继续）
- 棋盘中央的浮层按钮（开始对弈/开始对局/继续对局）全部移到状态栏

方案：
1. 移除棋盘中央的 3 个浮层按钮及 `.overlay-start-btn` 样式
2. 在底部状态栏"风的加护"开关旁新增"对局控制"按钮区：
   - `!started` → 显示"开始对弈"按钮
   - `pendingStart && currentIsAuto` → 显示"开始对局"按钮
   - `isResumedGame && currentIsAuto` → 显示"继续对局"按钮
   - 对局进行中（非上述暂停态）→ 显示"暂停"按钮，点击 `stopAutoPlay()` 后进入暂停态
   - 暂停态 → 显示"继续"按钮，点击 `driveTurn()` 恢复
3. 新增 `isPaused` 状态标记暂停（区别于"恢复对局"的 `isResumedGame`）

## 当前状态分析

### 浮层按钮（棋盘中央）

[App.svelte:486-493](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L486-L493)：
```svelte
{#if !started}
  <button class="overlay-start-btn" onclick={handleStart}>开始对弈</button>
{:else if pendingStart && currentIsAuto}
  <button class="overlay-start-btn" onclick={handleStartGame}>开始对局</button>
{:else if isResumedGame && currentIsAuto}
  <button class="overlay-start-btn" onclick={handleResume}>继续对局</button>
{/if}
```

样式 [App.svelte:784-810](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L784-L810)：`.overlay-start-btn` 棋盘中央浮层。

### 状态变量

- [App.svelte:59](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L59) `pendingStart`：第一步轮到 AI 时设 true，等用户点"开始对局"
- [App.svelte:334-335](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L334-L335) `isResumedGame`：恢复持久化对局时设 true
- [App.svelte:338-345](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L338-L345) `currentIsAuto`：当前轮到方是否自动主体

### 处理函数

- [App.svelte:347-387](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L347-L387) `handleStart()`：新开对局
- [App.svelte:389-408](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L389-L408) `handleStartGame()`：驱动 AI 第一步（pendingStart 后）
- [App.svelte:410-416](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L410-L416) `handleResume()`：恢复持久化对局

### 底部状态栏布局

[App.svelte:547-583](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L547-L583) `.actions-cell`：
- 风的加护开关
- 反转棋盘按钮
- 分隔符
- 鳕鱼评估文本

### 暂停机制

- [playerManager.ts:133-135](file:///c:/Users/24453/Desktop/AI国象/src/lib/stores/playerManager.ts#L133-L135) `stopAutoPlay()`：停止自对弈循环（设 `stopped=true`）
- [playerManager.ts:138-140](file:///c:/Users/24453/Desktop/AI国象/src/lib/stores/playerManager.ts#L138-L140) `resetStopFlag()`：重置停止状态
- [manager.ts:72-74](file:///c:/Users/24453/Desktop/AI国象/src/lib/players/manager.ts#L72-L74) `stop()`：设 `stopped=true`
- [manager.ts:77-80](file:///c:/Users/24453/Desktop/AI国象/src/lib/players/manager.ts#L77-L80) `reset()`：设 `stopped=false, stepCount=0`

**注意**：`driveTurn()` 内部会 `this.stopped = false; this.stepCount = 0;`（[manager.ts:36-37](file:///c:/Users/24453/Desktop/AI国象/src/lib/players/manager.ts#L36-L37)），所以暂停后调用 `driveTurn()` 可恢复。但 `continueAfterHumanMove()` 不重置 stopped，暂停后需用 `driveTurn()` 恢复（不是 `continueAfterHumanMove`）。

## 拟议修改

### 修改 1：新增 isPaused 状态和 handlePauseResume 函数

**文件**：[src/App.svelte](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte)

**改什么**：在 `pendingStart` 附近新增 `isPaused` 状态，新增 `handlePauseResume` 函数。

**为什么**：需要标记暂停态，暂停时显示"继续"按钮，继续时调用 `driveTurn` 恢复。

**怎么改**：
```ts
// 在 pendingStart 附近
let pendingStart = $state(false);
let isPaused = $state(false);  // 新增：用户手动暂停（区别于 isResumedGame 的恢复态）

// 新增 handlePauseResume 函数（在 handleResume 附近）
/// 暂停/继续对局（单按钮切换）
function handlePauseResume() {
  if (isPaused) {
    // 继续：驱动当前轮到方走棋
    isPaused = false;
    resetStopFlag();
    driveTurn($gameState).catch((e) => {
      showError(String(e));
      aiFailed.set(true);
    });
  } else {
    // 暂停：停止自对弈循环
    isPaused = true;
    stopAutoPlay();
  }
}
```

### 修改 2：移除棋盘中央浮层按钮

**文件**：[src/App.svelte:486-493](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L486-L493)

**改什么**：删除 `.board-area` 内的浮层按钮注释和 3 个 `{#if}` 分支。

**为什么**：用户要求全部移到状态栏。

**怎么改**：删除这 8 行。

### 修改 3：在底部状态栏加对局控制按钮

**文件**：[src/App.svelte:547-583](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L547-L583) `.actions-cell`

**改什么**：在"风的加护"开关前加对局控制按钮区，根据状态显示不同按钮。

**为什么**：用户要求放在"风的加护"边上。

**怎么改**：
```svelte
<div class="actions-cell">
  <!-- 对局控制按钮（原棋盘中央浮层，移到底部状态栏） -->
  <div class="game-controls">
    {#if !started}
      <button class="ctrl-btn primary" onclick={handleStart}>开始对弈</button>
    {:else if pendingStart && currentIsAuto}
      <button class="ctrl-btn primary" onclick={handleStartGame}>开始对局</button>
    {:else if isResumedGame && currentIsAuto}
      <button class="ctrl-btn primary" onclick={handleResume}>继续对局</button>
    {:else if isPaused}
      <button class="ctrl-btn" onclick={handlePauseResume}>继续</button>
    {:else}
      <button class="ctrl-btn" onclick={handlePauseResume} disabled={!currentIsAuto}>暂停</button>
    {/if}
  </div>

  <!-- 风的加护开关（显示 AI 思考内容） -->
  <label class="thinking-toggle" class:on={$showThinking}>
    ...
  </label>
  ...
</div>
```

**注意**：
- 暂停按钮在 `currentIsAuto` 为 false 时禁用（人方走棋时无需暂停）
- `isResumedGame` 和 `isPaused` 互斥（恢复态优先于暂停态显示）
- 顺序：开始对弈/开始对局/继续对局 > 暂停/继续

### 修改 4：重置 isPaused 状态

**文件**：[src/App.svelte](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte)（多个处理函数）

**改什么**：在 `handleStart`、`handleStartGame`、`handleResume`、`handleReset`、`handleExit` 中重置 `isPaused = false`。

**为什么**：新开/恢复/重开对局时清除暂停态。

**怎么改**：在各函数的 `pendingStart = false;` 附近加 `isPaused = false;`。

### 修改 5：移除 .overlay-start-btn 样式

**文件**：[src/App.svelte:784-810](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L784-L810)

**改什么**：删除 `.overlay-start-btn` 及其 `:hover` 样式。

**为什么**：浮层按钮已移除，样式无用。

**怎么改**：删除这约 27 行 CSS。

### 修改 6：新增 .ctrl-btn 和 .game-controls 样式

**文件**：[src/App.svelte](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte)（样式区）

**改什么**：新增对局控制按钮样式，与 `.flip-toggle` 风格协调。

**为什么**：新按钮需要样式。

**怎么改**：
```css
/* 对局控制按钮（开始对弈/暂停/继续等） */
.game-controls {
  display: flex;
  align-items: center;
  gap: var(--sp-2);
}
.ctrl-btn {
  padding: var(--sp-1) var(--sp-3);
  border-radius: var(--r-sm);
  border: 1px solid var(--line);
  background: var(--bg-soft);
  color: var(--ink-muted);
  font-size: 12px;
  cursor: pointer;
  transition: background 0.2s var(--ease), color 0.2s var(--ease), border-color 0.2s var(--ease);
}
.ctrl-btn:hover:not(:disabled) {
  background: var(--surface-2);
}
.ctrl-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
.ctrl-btn.primary {
  background: var(--accent);
  color: #fff;
  border-color: var(--accent);
}
.ctrl-btn.primary:hover {
  opacity: 0.9;
}
```

## 假设与决策

| # | 假设/决策 | 说明 |
|---|----------|------|
| 1 | 假设任何对局都能暂停 | 用户明确要求任何对局都能暂停 |
| 2 | 决策单按钮切换文案 | 用户选择"暂停↔继续"单按钮 |
| 3 | 决策全部浮层移到状态栏 | 用户选择全部移到状态栏 |
| 4 | 假设暂停后用 driveTurn 恢复 | driveTurn 内部重置 stopped=false，可恢复自对弈循环 |
| 5 | 决策暂停按钮在人方时禁用 | 人方走棋时无需暂停（等待用户点击，本就暂停） |
| 6 | 决策 isResumedGame 优先于 isPaused | 恢复持久化对局时显示"继续对局"而非"继续" |
| 7 | 决策按钮在"风的加护"前 | 用户要求"边上"，放在前面更突出 |
| 8 | 决策 primary 样式用于开始类按钮 | 开始对弈/开始对局/继续对局用 accent 色突出 |

## 验证步骤

1. **构建验证**：`npm run build` 通过，无 error
2. **视觉验证**（启动 `npm run tauri dev`）：
   - 棋盘中央无浮层按钮
   - 底部状态栏"风的加护"左侧显示对局控制按钮
   - 无对局时显示"开始对弈"（accent 色）
   - 点"开始对弈"→ 若白方是 AI，显示"开始对局"（accent 色）
   - 点"开始对局"→ AI 走棋，显示"暂停"
   - 点"暂停"→ 停止自对弈，按钮变"继续"
   - 点"继续"→ AI 继续走棋，按钮变"暂停"
   - 人方走棋时"暂停"按钮禁用（灰色）
   - 恢复持久化对局时显示"继续对局"（accent 色）
3. **场景验证**：
   - 人 vs DeepSeek：人走棋→"暂停"禁用→AI 思考时"暂停"可点→暂停→"继续"恢复
   - 鳕鱼 vs 鳕鱼：点"开始对局"→自对弈→"暂停"→"继续"循环正常
   - 重开对局：isPaused 重置
   - 退出对局：isPaused 重置
4. **功能验证**：
   - 暂停后走棋功能正常（人方仍可点击走棋？需确认）
   - 暂停后 AI 不会再走棋
   - 继续后 AI 正确走棋
