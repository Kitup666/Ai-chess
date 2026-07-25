# 第一步轮到自动主体时加"开始对局"按钮

## 摘要

当前行为：用户点"开始对弈"浮层后，`handleStart` 立即调用 `driveTurn` 驱动 AI/鳕鱼走第一步。如果用户执黑（白方是 AI），AI 会在用户点"开始对弈"后立即走棋，用户没有"先开局再让 AI 走"的控制感。

用户需求：如果第一步轮到 AI/鳕鱼（自动主体），在棋盘旁加一个"开始对局"按钮，用户点"开始对弈"进入对局后，再点"开始对局"才驱动 AI 走第一步。这与之前"点开局后才能进行对局，防止 AI 自己就开始输出了"的需求一致。

## 当前状态分析

### 当前流程

```
1. 无对局（!started）→ 显示"开始对弈"浮层
2. 用户点"开始对弈" → handleStart()
   → startGame() 创建后端对局
   → updateSettings({ started: true })
   → setTimeout(driveTurn)  ← 立即驱动 AI 走第一步（问题点）
3. 对局中（started && !isResumedGame）
   → 不显示任何浮层
   → AI 由 continueAfterHumanMove 自动驱动
```

### 问题场景

用户执黑 vs DeepSeek 白方：
1. 用户点"开始对弈" → DeepSeek 立即走第一步白棋
2. 用户没有机会"先准备好再开始"

### 相关代码

**handleStart**（[App.svelte:345-386](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L345-L386)）：
```js
async function handleStart() {
  resumedFromPersist = false;
  // ... startGame
  updateSettings({ started: true });
  updateGameState(state);
  // 驱动当前轮到方走棋（若白方是自动主体则开始走，若白方是人则等待点击）
  setTimeout(() => {
    driveTurn(state).catch(...);
  }, 0);  // ← 立即驱动，无人工触发
}
```

**浮层显示条件**（[App.svelte:463-468](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L463-L468)）：
```svelte
{#if !started}
  <button class="overlay-start-btn" onclick={handleStart}>开始对弈</button>
{:else if isResumedGame && currentIsAuto}
  <button class="overlay-start-btn" onclick={handleResume}>继续对局</button>
{/if}
```

**currentIsAuto 定义**（[App.svelte:334-339](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L334-L339)）：
```js
let currentIsAuto = $derived.by(() => {
  const g = $gameState;
  const p = g.turn === "white" ? whitePlayer : blackPlayer;
  return p !== "human";
});
```

## 拟议修改

### 修改 1：`handleStart` 不立即驱动 AI

**文件**：[src/App.svelte:345-386](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L345-L386)

**改什么**：移除 `handleStart` 中的 `setTimeout(driveTurn)` 调用，让对局开始后不自动驱动 AI。

**为什么**：用户希望对 AI 的第一步有明确的人工触发控制，与"点开局后才能进行对局"需求一致。后续 AI 由 `continueAfterHumanMove` 自动驱动，不受影响。

**怎么改**：
```js
async function handleStart() {
  resumedFromPersist = false;
  if (needApiKey && !$settings.apiKey.trim()) {
    showError("请填入 API Key");
    drawerOpen = true;
    return;
  }
  try {
    aiReasoning.set("");
    aiPick.set(null);
    aiFailed.set(false);
    const white = $settings.whitePlayer;
    const black = $settings.blackPlayer;
    const state = await startGame({
      side: $settings.side,
      // ... 其余参数不变
    });
    resetManager(white, black);
    updateSettings({ started: true });
    updateGameState(state);
    // 不自动驱动 AI：若白方是自动主体，由用户点"开始对局"按钮触发
    // 若白方是人，则等待用户点击棋盘走棋
  } catch (e) {
    showError(String(e));
  }
}
```

### 修改 2：新增 `pendingStart` 标志和 `handleStartGame` 函数

**文件**：[src/App.svelte](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte)（script 区，`resumedFromPersist` 附近）

**改什么**：新增 `pendingStart` 状态标志（表示"对局已开始但 AI 第一步未驱动"）和 `handleStartGame` 函数（驱动 AI 第一步）。

**为什么**：需要一个标志区分"对局已开始但 AI 未走第一步"和"对局正常进行中"，用于控制"开始对局"按钮的显示。

**怎么改**：
```js
// 标记对局已开始但第一步 AI 未驱动（用户点"开始对弈"后，若白方是自动主体则等待"开始对局"触发）
let pendingStart = $state(false);

/// 用户点"开始对局"按钮触发 AI 第一步
async function handleStartGame() {
  pendingStart = false;
  if (needApiKey && !$settings.apiKey.trim()) {
    showError("请填入 API Key");
    drawerOpen = true;
    return;
  }
  try {
    aiReasoning.set("");
    aiFailed.set(false);
    const state = $gameState;
    driveTurn(state).catch((e) => {
      showError(String(e));
      aiFailed.set(true);
    });
  } catch (e) {
    showError(String(e));
  }
}
```

### 修改 3：`handleStart` 中根据白方主体设置 `pendingStart`

**文件**：[src/App.svelte:345-386](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L345-L386)

**改什么**：在 `handleStart` 的 `updateGameState(state)` 之后，若白方（第一步轮到方）是自动主体，设置 `pendingStart = true`。

**为什么**：只有第一步轮到自动主体时才需要显示"开始对局"按钮。若白方是人，用户直接点棋盘走棋即可，不需要按钮。

**怎么改**（在修改 1 基础上）：
```js
async function handleStart() {
  resumedFromPersist = false;
  // ... startGame
  resetManager(white, black);
  updateSettings({ started: true });
  updateGameState(state);
  // 若第一步轮到自动主体（白方是 AI/鳕鱼），设置 pendingStart 等待用户点"开始对局"
  // 若白方是人，pendingStart 保持 false，用户直接点棋盘走棋
  if (white !== "human") {
    pendingStart = true;
  }
}
```

### 修改 4：浮层显示条件加入 `pendingStart`

**文件**：[src/App.svelte:463-468](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L463-L468)

**改什么**：在浮层条件中加入 `pendingStart && currentIsAuto` 分支，显示"开始对局"按钮。

**为什么**：对局开始后若第一步轮到自动主体且未驱动，显示"开始对局"让用户触发。

**怎么改**：
```svelte
{#if !started}
  <button class="overlay-start-btn" onclick={handleStart}>开始对弈</button>
{:else if pendingStart && currentIsAuto}
  <button class="overlay-start-btn" onclick={handleStartGame}>开始对局</button>
{:else if isResumedGame && currentIsAuto}
  <button class="overlay-start-btn" onclick={handleResume}>继续对局</button>
{/if}
```

### 修改 5：`handleReset` 中重置 `pendingStart`

**文件**：[src/App.svelte:298-316](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L298-L316)

**改什么**：在 `handleReset` 中重置 `pendingStart = false`，并根据白方主体重新设置。

**为什么**：重开对局与新开对局逻辑一致，若白方是自动主体则需用户点"开始对局"。

**怎么改**：
```js
async function handleReset() {
  drawerOpen = false;
  stopAutoPlay();
  resumedFromPersist = false;
  pendingStart = false;  // 新增：重置
  try {
    aiReasoning.set("");
    aiPick.set(null);
    aiFailed.set(false);
    const white = $settings.whitePlayer;
    const black = $settings.blackPlayer;
    const state = await resetGame($settings.side, white, black);
    resetManager(white, black);
    updateGameState(state);
    // 若白方是自动主体，设置 pendingStart 等待用户点"开始对局"
    if (white !== "human") {
      pendingStart = true;
    }
  } catch (e) {
    showError(String(e));
  }
}
```

注意：原 `handleReset` 中的 `setTimeout(driveTurn)` 也要移除，逻辑与 `handleStart` 一致。

### 修改 6：`handleExit` 重置 `pendingStart`

**文件**：[src/App.svelte:410-417](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L410-L417)

**改什么**：在 `handleExit` 中重置 `pendingStart = false`。

**为什么**：退出对局后清除标志，避免下次新对局误触发。

**怎么改**：
```js
function handleExit() {
  stopAutoPlay();
  updateSettings({ started: false });
  resumedFromPersist = false;
  pendingStart = false;  // 新增
  drawerOpen = true;
}
```

### 修改 7：`handleResume` 重置 `pendingStart`

**文件**：[src/App.svelte:384-405](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L384-L405)

**改什么**：在 `handleResume` 中重置 `pendingStart = false`。

**为什么**：虽然 `pendingStart` 在正常对局中不会为 true，但为防御性编程，确保恢复对局时标志干净。

**怎么改**：
```js
async function handleResume() {
  resumedFromPersist = false;
  pendingStart = false;  // 新增
  // ... 其余不变
}
```

### 修改 8：Settings 组件 `handleStart` 同步逻辑

**文件**：[src/lib/components/Settings.svelte:94-136](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/Settings.svelte#L94-L136)

**改什么**：Settings 组件内的 `handleStart` 也有 `setTimeout(driveTurn)`，需要同步移除，但 **Settings 组件作为子组件无法直接设置 App 的 `pendingStart` 状态**。

**为什么**：Settings 组件的 `handleStart` 是抽屉/标签内的开始逻辑，与 App 的 `handleStart` 重复。

**决策**：由于 Settings 组件内的"开始对弈"按钮会调用 `onStarted` 回调，且 App 已有浮层"开始对弈"按钮作为主入口，**Settings 组件内的开始逻辑可保留但需与 App 一致**。最简方案：**Settings 组件的 `handleStart` 仍立即驱动 AI**（保持向后兼容），但 App 的浮层"开始对弈"走新逻辑（pendingStart）。

**更优方案**：由于 Settings 组件的 `handleStart` 主要在抽屉模式下使用（窄屏），而窄屏的浮层按钮也在，用户更可能点浮层。因此 **Settings 组件的 `handleStart` 也应移除 `setTimeout(driveTurn)`**，但需通过回调通知 App 设置 `pendingStart`。

**最终决策**：为保持简单，**Settings 组件的 `handleStart` 保留原逻辑（立即驱动）**，因为：
- Settings 组件的"开始对弈"按钮在抽屉/标签内，用户已主动打开抽屉/标签，期望立即开始
- 浮层"开始对弈"是主入口，走新的 `pendingStart` 逻辑
- 两个入口行为略有差异可接受，且 Settings 的 `onStarted` 回调会关闭抽屉

**怎么改**：Settings.svelte **不改动**。

## 假设与决策

| # | 假设/决策 | 说明 |
|---|----------|------|
| 1 | 假设用户希望对 AI 第一步有人工触发 | 与之前"点开局后才能进行对局"需求一致 |
| 2 | 假设白方是人时不需要"开始对局"按钮 | 用户直接点棋盘走第一步即可 |
| 3 | 决策用 `pendingStart` 标志区分"已开始但未驱动 AI" | 语义明确，与 `resumedFromPersist` 模式一致 |
| 4 | 决策 Settings 组件不改 | Settings 内的开始按钮是次要入口，保留原逻辑避免复杂回调 |
| 5 | 决策 `handleReset` 也走 pendingStart 逻辑 | 重开对局与新开对局行为一致 |
| 6 | 不改 `continueAfterHumanMove` | 用户走棋后 AI 仍自动驱动，只有第一步需人工触发 |

## 验证步骤

1. **构建验证**：`npm run build` 通过，无 error
2. **场景验证**（启动 `npm run tauri dev`）：
   - **场景 A：用户执白 vs DeepSeek 黑方**
     - 点"开始对弈" → 进入对局，`pendingStart=false`（白方是人）
     - **不显示**"开始对局"按钮
     - 用户直接点棋盘走第一步白棋
     - DeepSeek 由 `continueAfterHumanMove` 自动驱动
   - **场景 B：用户执黑 vs DeepSeek 白方**
     - 点"开始对弈" → 进入对局，`pendingStart=true`（白方是 AI）
     - **显示**"开始对局"按钮
     - 用户点"开始对局" → DeepSeek 走第一步白棋，按钮消失
     - 用户走黑棋后，DeepSeek 自动走（continueAfterHumanMove）
   - **场景 C：鳕鱼 vs 鳕鱼自对弈**
     - 点"开始对弈" → 进入对局，`pendingStart=true`（白方是鳕鱼）
     - **显示**"开始对局"按钮
     - 用户点"开始对局" → 鳕鱼开始自对弈
   - **场景 D：重开对局**
     - 对局中点"重开" → 新对局开始
     - 若白方是 AI → 显示"开始对局"按钮
     - 若白方是人 → 不显示，直接走棋
   - **场景 E：持久化恢复**
     - 应用重启加载持久化对局
     - 若轮到 AI → 显示"继续对局"按钮（原逻辑不变）
     - `pendingStart` 不参与恢复场景
3. **功能验证**：
   - DeepSeek 失败时"重新请求"按钮可用
   - 悔棋后轮到 AI 时 AI 自动走（continueAfterHumanMove）

## 执行顺序

1. 修改 1：`handleStart` 移除 `setTimeout(driveTurn)`
2. 修改 2：新增 `pendingStart` 状态和 `handleStartGame` 函数
3. 修改 3：`handleStart` 中根据白方主体设置 `pendingStart`
4. 修改 4：浮层显示条件加入 `pendingStart`
5. 修改 5：`handleReset` 重置 `pendingStart` 并移除 `setTimeout(driveTurn)`
6. 修改 6：`handleExit` 重置 `pendingStart`
7. 修改 7：`handleResume` 重置 `pendingStart`
8. 运行 `npm run build` 验证
9. 询问用户是否需要启动 `npm run tauri dev` 进行场景验证
