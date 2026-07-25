# 在主界面右下角加反转棋盘功能

## 摘要

用户需求：在主界面右下角加一个反转棋盘按钮，点击翻转棋盘方向。

当前 `flipped` 是只读 `$derived`（[Board.svelte:176](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/Board.svelte#L176) `let flipped = $derived($gameState.player_side === "black")`），仅根据玩家执方自动翻转，无法外部手动切换。

方案：新增一个 `boardFlipped` 独立 store（持久化到 localStorage），与 `player_side === "black"` 基础翻转做 XOR 得到最终 `flipped`。在 Board.svelte 的 `.board` 容器右下角加绝对定位按钮，点击切换 `boardFlipped`。Board 和 EvalBar 同时响应最终 `flipped`。

## 当前状态分析

### flipped 相关代码

**Board.svelte**（[Board.svelte:176](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/Board.svelte#L176)）：
```js
let flipped = $derived($gameState.player_side === "black");
```
- 影响：`ranks`/`files` 数组顺序（[Board.svelte:252-253](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/Board.svelte#L252-L253)）、UCI→坐标转换（[Board.svelte:181-194](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/Board.svelte#L181-L194)）、`.board` class（[Board.svelte:454](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/Board.svelte#L454)）、thinkingOverlay 位置（[Board.svelte:49-50](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/Board.svelte#L49-L50)）

**EvalBar.svelte**（[EvalBar.svelte:21](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/EvalBar.svelte#L21)）：
```js
let flipped = $derived($gameState.player_side === "black");
```
- 影响：`.eval-bar` class（[EvalBar.svelte:55](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/EvalBar.svelte#L55)）、`.eval-label` 位置（[EvalBar.svelte:68,70](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/EvalBar.svelte#L68-L70)）

### board-area 布局

[App.svelte:480-493](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L480-L493)：
```svelte
<div class="board-area" class:with-eval={showEvalBar}>
  {#if showEvalBar}<EvalBar />{/if}
  <Board />
  <!-- 浮层按钮 -->
</div>
```

`.board-area` 是 flex 布局，包含 EvalBar 和 Board。Board 内部的 `.board` 是实际棋盘容器。

### 现有 store 模式

[Settings.svelte](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/Settings.svelte) 用 `settings` store + `updateSettings`。音效用独立的 `getSoundVolume/setSoundVolume` localStorage 模块。

## 拟议修改

### 修改 1：新建 boardOrientation store

**文件**：`src/lib/stores/boardOrientation.ts`（新建）

**改什么**：新建一个 Svelte store，持久化到 localStorage，提供 `toggleBoardFlipped()` 和 `isBoardFlipped()` 方法。

**为什么**：
- 需要跨组件共享（Board 和 EvalBar 都要读）
- 需要持久化（用户偏好应跨会话保留）
- 遵循音效模块的独立 localStorage 模式，避免污染 settings store

**怎么改**：
```ts
import { writable } from "svelte/store";
import { browser } from "$app/environment";

const STORAGE_KEY = "ai-chess-board-flipped";

function loadInitial(): boolean {
  if (!browser) return false;
  try {
    return localStorage.getItem(STORAGE_KEY) === "1";
  } catch {
    return false;
  }
}

const boardFlipped = writable<boolean>(loadInitial());

if (browser) {
  boardFlipped.subscribe((v) => {
    try {
      localStorage.setItem(STORAGE_KEY, v ? "1" : "0");
    } catch {}
  });
}

export function toggleBoardFlipped() {
  boardFlipped.update((v) => !v);
}

export function isBoardFlipped(): boolean {
  let v = false;
  boardFlipped.subscribe((x) => (v = x))();
  return v;
}

export { boardFlipped };
```

### 修改 2：Board.svelte 引入 store 并计算最终 flipped

**文件**：[src/lib/components/Board.svelte:176](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/Board.svelte#L176)

**改什么**：引入 `boardFlipped` store，将 `flipped` 改为基础翻转（玩家执黑）与用户翻转的 XOR。

**为什么**：用户切换的翻转应叠加在"玩家执黑自动翻转"基础上，互不干扰。XOR 逻辑：
- 玩家执白 + 不手动翻转 = 白在底（正常）
- 玩家执白 + 手动翻转 = 黑在底（看对方视角）
- 玩家执黑 + 不手动翻转 = 黑在底（自动翻转看自己方）
- 玩家执黑 + 手动翻转 = 白在底（看对方视角）

**怎么改**：
```ts
import { boardFlipped } from "../stores/boardOrientation";

// 原：let flipped = $derived($gameState.player_side === "black");
// 改为：基础翻转（玩家执黑自动翻转）与用户手动翻转做 XOR
let flipped = $derived(
  ($gameState.player_side === "black") !== $boardFlipped
);
```

### 修改 3：EvalBar.svelte 同步引入 store

**文件**：[src/lib/components/EvalBar.svelte:21](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/EvalBar.svelte#L21)

**改什么**：引入 `boardFlipped` store，将 `flipped` 改为 XOR 逻辑。

**为什么**：EvalBar 必须与 Board 同步翻转，否则评估柱方向与棋盘不一致。

**怎么改**：
```ts
import { boardFlipped } from "../stores/boardOrientation";

let flipped = $derived(
  ($gameState.player_side === "black") !== $boardFlipped
);
```

### 修改 4：Board.svelte 加反转按钮

**文件**：[src/lib/components/Board.svelte](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/Board.svelte)（template 区，`.board` 容器内或外）

**改什么**：在棋盘右下角加一个绝对定位的"反转"按钮，点击调用 `toggleBoardFlipped()`。

**为什么**：用户要求在主界面右下角加反转功能。

**怎么改**：
```svelte
<script>
  // 引入 toggleBoardFlipped
  import { toggleBoardFlipped } from "../stores/boardOrientation";
</script>

<!-- 在 .board 容器外层加一个包裹容器，或直接在 .board 内绝对定位 -->
<div class="board-wrapper">
  <div class="board" class:flipped>
    <!-- ... 原有棋盘内容 ... -->
  </div>
  <button
    class="flip-btn"
    onclick={toggleBoardFlipped}
    title="反转棋盘"
    aria-label="反转棋盘"
  >
    ⇅
  </button>
</div>
```

**按钮样式**（绝对定位到右下角）：
```css
.board-wrapper {
  position: relative;
  /* 保持原 .board 的尺寸约束 */
}
.flip-btn {
  position: absolute;
  right: 8px;
  bottom: 8px;
  z-index: 10;
  width: 36px;
  height: 36px;
  border-radius: 6px;
  border: 1px solid var(--line);
  background: var(--surface);
  color: var(--text);
  font-size: 18px;
  line-height: 1;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  opacity: 0.6;
  transition: opacity 0.15s, background 0.15s;
}
.flip-btn:hover {
  opacity: 1;
  background: var(--surface-elevated, var(--surface));
}
```

**注意**：当前 `.board` 容器可能已有尺寸约束（如 `width: min(70vh, 100%)`），需将 `.board` 包裹在 `.board-wrapper` 内，wrapper 继承尺寸，按钮绝对定位到 wrapper 右下角。需先读取 Board.svelte 的 `.board` 样式确认。

### 修改 5：反转按钮状态指示

**文件**：[src/lib/components/Board.svelte](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/Board.svelte)

**改什么**：按钮根据 `$boardFlipped` 状态显示不同样式（如激活态高亮）。

**为什么**：让用户清楚当前是否处于手动翻转状态。

**怎么改**：
```svelte
<button
  class="flip-btn"
  class:active={$boardFlipped}
  onclick={toggleBoardFlipped}
  title={$boardFlipped ? "恢复默认方向" : "反转棋盘"}
  aria-label="反转棋盘"
>
  ⇅
</button>
```

```css
.flip-btn.active {
  background: var(--accent);
  color: #fff;
  opacity: 1;
}
```

## 假设与决策

| # | 假设/决策 | 说明 |
|---|----------|------|
| 1 | 假设翻转用 XOR 逻辑 | 基础翻转（玩家执黑）与手动翻转独立叠加，互不干扰 |
| 2 | 假设 boardFlipped 需持久化 | 用户偏好应跨会话保留，与音效设置一致 |
| 3 | 决策用独立 store 而非 settings store | 遵循音效模块模式，避免 settings store 膨胀，且 settings store 走 Rust 后端，UI 偏好不应走后端 |
| 4 | 决策按钮放在 Board.svelte 内部 | 按钮与棋盘绑定，无论桌面/窄屏都可见 |
| 5 | 决策用 ⇅ 图标 | 简洁通用，无需 SVG 资源 |
| 6 | 决策按钮半透明，hover 时不透明 | 不干扰棋盘视觉，需要时易发现 |
| 7 | 决策按钮在 .board-wrapper 右下角 | 用户要求位置，且与 Lichess 反转按钮位置一致 |

## 验证步骤

1. **构建验证**：`npm run build` 通过，无 error
2. **视觉验证**（启动 `npm run tauri dev`）：
   - **桌面端**：
     - 棋盘右下角显示 ⇅ 按钮，半透明
     - 鼠标悬停按钮变不透明
     - 点击按钮 → 棋盘立即翻转，按钮变高亮（accent 色）
     - 再次点击 → 棋盘恢复，按钮恢复半透明
     - EvalBar 同步翻转方向
     - 翻转后走棋功能正常（点击格子位置正确）
     - 翻转后 AI 思考浮层位置正确（上方/下方随翻转切换）
     - 翻转后引擎箭头坐标正确
   - **窄屏**：按钮同样在右下角可见
   - **持久化**：翻转后关闭应用，重启后保持翻转状态
3. **场景验证**：
   - 玩家执白 + 不翻转 → 白在底（正常）
   - 玩家执白 + 翻转 → 黑在底
   - 玩家执黑 + 不翻转 → 黑在底（自动翻转看自己方）
   - 玩家执黑 + 翻转 → 白在底
   - 重开对局 → 翻转状态保持（不随对局重置）
4. **功能验证**：
   - 翻转后走棋正常
   - 翻转后悔棋正常
   - 翻转后鳕鱼引擎箭头正常

## 执行顺序

1. 修改 1：新建 boardOrientation store
2. 修改 2：Board.svelte 引入 store 并改 flipped 逻辑
3. 修改 3：EvalBar.svelte 同步引入 store
4. 修改 4+5：Board.svelte 加反转按钮（含状态指示）
5. 运行 `npm run build` 验证
6. 询问用户是否需要启动 `npm run tauri dev` 进行视觉验证
