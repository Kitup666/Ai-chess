# 修复正常对局中错误显示"继续对局"按钮的 bug

## 摘要

用户与 DeepSeek 对战时，走一步后屏幕中间出现"继续对局"按钮。根因是 `isResumedGame` 的判断条件过于宽泛：只要 `started && move_history.length > 0 && status === "playing"` 就为 true，这在**任何正常进行中的对局**都满足（只要走了至少一步棋且轮到 AI）。

但 `isResumedGame` 的设计意图（[App.svelte:225-226](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L225-L226)）是：**仅在应用启动时从持久化加载的对局**时为 true，因为 onMount 加载持久化对局后不自动驱动 AI，需用户手动点"继续对局"触发。

正常对局中，[Board.svelte:383](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/Board.svelte#L383) 的 `continueAfterHumanMove` 会在用户走棋后自动驱动 AI，**不需要**显示"继续对局"按钮。

## 当前状态分析

### Bug 触发场景

```
用户执白 vs DeepSeek 黑方：
1. 用户走第一步白棋 e2-e4
   → move_history.length = 1 (>0) ✓
   → started = true ✓
   → status = "playing" ✓
   → isResumedGame = true （错误！）
2. 轮到黑方 DeepSeek
   → turn = "black" → blackPlayer = "deepseek" ≠ "human"
   → currentIsAuto = true ✓
3. isResumedGame && currentIsAuto 都为 true
   → 显示"继续对局"按钮（错误！）

正确行为：
- Board.svelte 调用 continueAfterHumanMove 自动驱动 DeepSeek
- 不应显示"继续对局"按钮
```

### 相关代码

**isResumedGame 定义**（[App.svelte:325-327](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L325-L327)）：
```js
let isResumedGame = $derived(
  started && $gameState.move_history.length > 0 && $gameState.status === "playing"
);
```

**显示按钮条件**（[App.svelte:450-454](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L450-L454)）：
```svelte
{#if !started}
  <button class="overlay-start-btn" onclick={handleStart}>开始对弈</button>
{:else if isResumedGame && currentIsAuto}
  <button class="overlay-start-btn" onclick={handleResume}>继续对局</button>
{/if}
```

**onMount 加载持久化对局**（[App.svelte:215-227](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L215-L227)）：
```js
if (loaded.game) {
  updateGameState(loaded.game);
  updateSettings({ ... started: true });
  resetManager(...);
  // 不自动驱动 AI：用户需手动点「开始对弈/继续对局」按钮触发
  // 防止应用启动时 AI 自行输出，确保"点开局后才开始对局"
}
```

**handleResume**（[App.svelte:379-396](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L379-L396)）：
```js
async function handleResume() {
  // ... 调用 driveTurn 驱动 AI
}
```

**Board.svelte 自动驱动**（[Board.svelte:383](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/Board.svelte#L383)）：
```js
continueAfterHumanMove(result.state).catch((e) => { ... });
```

## 拟议修改

### 修改 1：新增 `resumedFromPersist` 标志位

**文件**：[src/App.svelte](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte)（script 区，约第 53 行附近，与 `panelCollapsed` 等状态一起）

**改什么**：新增一个 `$state` 标志位 `resumedFromPersist`，表示"当前对局是否来自持久化加载"。

**为什么**：需要一个明确的标志来区分"应用启动时恢复的对局"和"正常进行中的对局"。现有的 `isResumedGame` 判断条件无法区分这两种情况。

**怎么改**：
```js
// 在 panelCollapsed 等状态附近新增
let resumedFromPersist = $state(false);
```

### 修改 2：onMount 加载持久化对局后设置标志

**文件**：[src/App.svelte:215-227](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L215-L227)

**改什么**：在 onMount 加载持久化对局成功后，将 `resumedFromPersist` 设置为 true。

**为什么**：仅在应用启动时从持久化加载对局才需要显示"继续对局"按钮。

**怎么改**：
```js
if (loaded.game) {
  updateGameState(loaded.game);
  updateSettings({ ... started: true });
  resetManager(...);
  resumedFromPersist = true;  // 新增：标记为恢复的对局
  // 不自动驱动 AI：用户需手动点「继续对局」按钮触发
}
```

### 修改 3：`isResumedGame` 加入 `resumedFromPersist` 条件

**文件**：[src/App.svelte:325-327](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L325-L327)

**改什么**：在 `isResumedGame` 的判断条件中加入 `resumedFromPersist`。

**为什么**：确保只在"应用启动时恢复的对局"才为 true，正常对局中（即使走了多步棋且轮到 AI）也为 false。

**怎么改**：
```js
let isResumedGame = $derived(
  resumedFromPersist && started && $gameState.move_history.length > 0 && $gameState.status === "playing"
);
```

### 修改 4：`handleResume` 调用后重置标志

**文件**：[src/App.svelte:379-396](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L379-L396)

**改什么**：在 `handleResume` 函数开头将 `resumedFromPersist` 重置为 false。

**为什么**：用户点"继续对局"后，对局进入正常流程，后续由 `continueAfterHumanMove` 自动驱动 AI，不应再显示按钮。

**怎么改**：
```js
async function handleResume() {
  resumedFromPersist = false;  // 新增：重置标志，按钮消失
  if (needApiKey && !$settings.apiKey.trim()) {
    showError("请填入 API Key");
    drawerOpen = true;
    return;
  }
  // ... 其余不变
}
```

### 修改 5：`handleStart` 和 `handleReset` 重置标志

**文件**：[src/App.svelte:293-317](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L293-L317)（handleReset）、[src/App.svelte:338-376](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L338-L376)（handleStart）

**改什么**：在 `handleStart` 和 `handleReset` 函数开头将 `resumedFromPersist` 重置为 false。

**为什么**：新开对局或重开对局都不是"恢复的对局"，应确保标志为 false，避免误显示按钮。

**怎么改**：
```js
async function handleStart() {
  resumedFromPersist = false;  // 新增
  // ... 其余不变
}

async function handleReset() {
  drawerOpen = false;
  stopAutoPlay();
  resumedFromPersist = false;  // 新增
  // ... 其余不变
}
```

### 修改 6：`handleExit` 重置标志

**文件**：[src/App.svelte:398-403](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L398-L403)

**改什么**：在 `handleExit` 函数中重置 `resumedFromPersist`。

**为什么**：退出对局后应清除标志，避免下次新对局误触发。

**怎么改**：
```js
function handleExit() {
  stopAutoPlay();
  updateSettings({ started: false });
  resumedFromPersist = false;  // 新增
  drawerOpen = true;
}
```

## 假设与决策

| # | 假设/决策 | 说明 |
|---|----------|------|
| 1 | 假设正常对局中 AI 自动驱动 | Board.svelte 的 continueAfterHumanMove 已实现，无需用户手动触发 |
| 2 | 假设持久化恢复的对局需手动驱动 | onMount 注释明确"不自动驱动 AI"，需用户点"继续对局" |
| 3 | 决策用 `resumedFromPersist` 标志位 | 比修改 `isResumedGame` 判断条件更清晰，语义明确 |
| 4 | 决策在 handleResume 中重置标志 | 用户触发继续后，对局进入正常流程，按钮应消失 |
| 5 | 决策在 handleStart/handleReset/handleExit 中重置 | 防止标志位污染后续对局 |
| 6 | 不改 handleResume 的 driveTurn 调用 | driveTurn 会驱动当前轮到方，逻辑正确 |

## 验证步骤

1. **构建验证**：`npm run build` 通过，无 error
2. **场景验证**（启动 `npm run tauri dev`）：
   - **场景 A：正常对局（用户 vs DeepSeek）**
     - 点"开始对弈"开始新对局
     - 用户走第一步白棋
     - **不应**显示"继续对局"按钮
     - DeepSeek 自动走棋（continueAfterHumanMove 驱动）
     - 轮到用户时等待点击，轮到 DeepSeek 时自动走
   - **场景 B：持久化恢复对局**
     - 走几步棋后关闭应用
     - 重新启动应用，自动加载持久化对局
     - 如果当前轮到 AI → **应**显示"继续对局"按钮
     - 点"继续对局" → 按钮消失，AI 自动走棋
     - 后续正常对局，不再显示按钮
   - **场景 C：重开对局**
     - 对局中点"重开" → 新对局开始
     - 用户走第一步 → **不应**显示"继续对局"按钮
   - **场景 D：退出对局后再开始**
     - 点"退出当前对局" → started=false
     - 点"开始对弈" → 新对局开始
     - 用户走第一步 → **不应**显示"继续对局"按钮
3. **AI 失败重试验证**：DeepSeek 走棋失败时，"重新请求"按钮可用，不显示"继续对局"

## 执行顺序

1. 修改 1：新增 `resumedFromPersist` 状态
2. 修改 2：onMount 加载持久化对局后设置标志
3. 修改 3：`isResumedGame` 加入 `resumedFromPersist` 条件
4. 修改 4：`handleResume` 重置标志
5. 修改 5：`handleStart` 和 `handleReset` 重置标志
6. 修改 6：`handleExit` 重置标志
7. 运行 `npm run build` 验证
8. 询问用户是否需要启动 `npm run tauri dev` 进行场景验证
