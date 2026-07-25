# 项目合理性探查与修正 —— 第一阶段（高严重度）

## 摘要

对整个项目进行了系统性代码审查，发现大量合理性问题。按严重程度分为三阶段修复：
- **第一阶段（本 PLAN）**：高严重度 14 项 —— 影响功能、UX 不一致、死代码清理
- **第二阶段（待定）**：中严重度 19 项 —— 代码质量、重复逻辑、动画性能
- **第三阶段（待定）**：低严重度 —— 重构、拆分、微优化

本阶段聚焦"无副作用、立即修、收益高"的问题。

## 当前状态分析

### 高严重度问题清单

#### Bug 类（影响功能）

1. **`in_check()` 实现错误** —— [chess_engine.rs:140-144](file:///c:/Users/24453/Desktop/AI国象/src-tauri/src/chess_engine.rs#L140-L144)
   - 把"将军"误判为"将杀"（用 `BoardStatus::Checkmate` 而非 `board.checkers()`）
   - 影响：将军音效不响、将军国王红框不显示、将军状态文案不触发

2. **暂停/继续竞态条件** —— [playerManager.ts:49-69](file:///c:/Users/24453/Desktop/AI国象/src/lib/stores/playerManager.ts#L49-L69)
   - `driveLoop` 在 `await requestMove` 期间不检查 `stopped`，快速点暂停→继续可能双重并发
   - 影响：可能双重走棋、状态错乱

3. **Stockfish 难度修改不实时生效** —— [Settings.svelte:139-175](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/Settings.svelte#L139-L175)
   - `handleApplySettings` 只在主体变更时 `resetManager`，改难度不重建 player
   - 影响：用户调难度后无效果，直到重开

4. **两个 `handleStart` 行为不一致** —— [App.svelte:350-390](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L350-L390) vs [Settings.svelte:93-135](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/Settings.svelte#L93-L135)
   - App 版用 `pendingStart` 让用户二次确认；Settings 版直接 `driveTurn` 绕过
   - 影响：UX 不一致，违反"点开局后才开始对局"设计

#### 死代码类（清理）

5. **`.bar-btn.active` 死 CSS** —— [App.svelte:908-911](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L908-L911)
6. **`.status-cell[data-kind="win"]` 死 CSS** —— [App.svelte:855-862](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L855-L862)（statusKind 从不返回 "win"）
7. **`liftOffset` 死变量** —— [Board.svelte:97-99](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/Board.svelte#L97-L99)
8. **`settings.side` 死字段** —— [settings.ts:21,44](file:///c:/Users/24453/Desktop/AI国象/src/lib/stores/settings.ts#L21)（前端固定传 "white"）
9. **`legalMovesForSelected` 死 store** —— [game.ts:47-53](file:///c:/Users/24453/Desktop/AI国象/src/lib/stores/game.ts#L47-L53)
10. **`isPlaying` 死 store** —— [game.ts:56-58](file:///c:/Users/24453/Desktop/AI国象/src/lib/stores/game.ts#L56-L58)
11. **`engineVersion` 死 store** —— [stockfish/store.ts:27,170](file:///c:/Users/24453/Desktop/AI国象/src/lib/stockfish/store.ts#L27)
12. **`getGameState` 死 API** —— [api.ts:22-24](file:///c:/Users/24453/Desktop/AI国象/src/lib/api.ts#L22-L24)
13. **`peekManager` 死函数** —— [playerManager.ts:44-46](file:///c:/Users/24453/Desktop/AI国象/src/lib/stores/playerManager.ts#L44-L46)
14. **`applySkillLevel` 死函数** —— [stockfish/store.ts:239-245](file:///c:/Users/24453/Desktop/AI国象/src/lib/stockfish/store.ts#L239-L245)
15. **`playerTurn` 死派生** —— [App.svelte:93](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L93)
16. **Stockfish 死派生 store** —— `lastBestMove`、`bestMove`、`bestScore`、`engineReady`（[stockfish/store.ts](file:///c:/Users/24453/Desktop/AI国象/src/lib/stockfish/store.ts)）
17. **`Player.type` 死字段** —— [players/types.ts:9](file:///c:/Users/24453/Desktop/AI国象/src/lib/players/types.ts#L9)
18. **`dailyCacheHitRate` / `resetDailyCost` 死函数** —— [cost.ts:158-173](file:///c:/Users/24453/Desktop/AI国象/src/lib/stores/cost.ts#L158-L173)

#### 重复代码类（合并）

19. **`flipped` 计算重复** —— [Board.svelte:180-182](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/Board.svelte#L180-L182) vs [EvalBar.svelte:22-25](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/EvalBar.svelte#L22-L25)
20. **`.toggle` 样式重复** —— [App.svelte:1013-1041](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L1013-L1041) vs [Settings.svelte:449-477](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/Settings.svelte#L449-L477)

#### 布局/A11y 类

21. **棋盘尺寸未约束横向视口** —— [Board.svelte:607-608](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/Board.svelte#L607-L608)（窄屏溢出）
22. **底部状态栏窄屏溢出** —— [App.svelte:544-615](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L544-L615)
23. **`:focus-visible` 全局缺失** —— 键盘用户看不到焦点
24. **drawer-close 缺 aria-label** —— [App.svelte:630](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L630)
25. **toast 非 live region** —— [App.svelte:538-540](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L538-L540)

## 拟议修改

### 修改组 A：修复 in_check bug

**文件**：[src-tauri/src/chess_engine.rs:140-144](file:///c:/Users/24453/Desktop/AI国象/src-tauri/src/chess_engine.rs#L140-L144)

**改什么**：用 `board.checkers()` 检测将军。

```rust
pub fn in_check(&self) -> bool {
    !self.board.checkers().is_empty()
}
```

**验证**：`cargo check` 通过；启动对局，走到将军局面时：将军音效响起、国王红框高亮、状态栏显示"被将军"。

### 修改组 B：修复暂停竞态条件

**文件**：[src/lib/players/manager.ts](file:///c:/Users/24453/Desktop/AI国象/src/lib/players/manager.ts)

**改什么**：给 `driveLoop` 加 generation 机制，每次 `driveTurn`/`reset` 自增 generation，旧循环 await 返回后检测 generation 失配即退出。

```ts
private generation = 0;

driveLoop(current: Color) {
  const myGen = this.generation;
  let side = current;
  while (!this.stopped && myGen === this.generation) {
    // ...
    const move = await player.requestMove(side);
    if (this.stopped || myGen !== this.generation) return; // 退出
    // ... onMoveApplied
  }
}

reset() {
  this.generation++; // 让旧循环退出
  this.stopped = true;
  this.stepCount = 0;
}
```

**验证**：快速点暂停→继续→暂停→继续，不出现双重走棋。

### 修改组 C：修复 Stockfish 难度不实时生效

**文件**：[src/lib/components/Settings.svelte:139-175](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/Settings.svelte#L139-L175)

**改什么**：`handleApplySettings` 中检测 stockfish 难度字段（`stockfishElo`、`stockfishSkill`、`useStockfishElo`）是否变化，变化时也调用 `resetManager`。

```ts
const g = $gameState;
const playerChanged = g.white_player !== whitePlayer || g.black_player !== blackPlayer;
const sfOptsChanged = showStockfishOpts && (
  $settings.stockfishElo !== prevSfElo ||
  $settings.stockfishSkill !== prevSfSkill ||
  $settings.useStockfishElo !== prevUseSfElo
);
if (playerChanged || sfOptsChanged) {
  // resetManager + resetGame
}
```

**简化方案**：让 `createStockfishPlayer` 从 `settings` store 动态读取难度，而非闭包捕获。

**验证**：对局中改鳕鱼 ELO，点应用设置，下一步 AI 立即按新难度走棋。

### 修改组 D：统一 handleStart 入口

**文件**：[src/lib/components/Settings.svelte:93-135](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/Settings.svelte#L93-L135)

**改什么**：Settings 的 `handleStart` 移除 `setTimeout(driveTurn)`，改为与 App 一致的 `pendingStart` 逻辑（或直接调用 App 传入的 `onStart` 回调）。

**简化方案**：Settings 的"开始对弈"按钮改为只调 `onStarted()` 回调，由 App 统一处理 pendingStart。Settings 不直接驱动 AI。

**验证**：从 Settings 点"开始对弈"与从底部状态栏点"开始对弈"行为一致。

### 修改组 E：批量删除死代码

**文件与改动**：
- [App.svelte:908-911](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L908-L911) 删 `.bar-btn.active`
- [App.svelte:855-862](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L855-L862) 删 `.status-cell[data-kind="win"]`
- [Board.svelte:97-99](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/Board.svelte#L97-L99) 删 `liftOffset`
- [settings.ts:21,44](file:///c:/Users/24453/Desktop/AI国象/src/lib/stores/settings.ts#L21) 删 `side` 字段（保留后端兼容，前端不再用）
- [game.ts:47-53](file:///c:/Users/24453/Desktop/AI国象/src/lib/stores/game.ts#L47-L53) 删 `legalMovesForSelected`
- [game.ts:56-58](file:///c:/Users/24453/Desktop/AI国象/src/lib/stores/game.ts#L56-L58) 删 `isPlaying`
- [stockfish/store.ts:27,170](file:///c:/Users/24453/Desktop/AI国象/src/lib/stockfish/store.ts#L27) 删 `engineVersion`
- [api.ts:22-24](file:///c:/Users/24453/Desktop/AI国象/src/lib/api.ts#L22-L24) 删 `getGameState`
- [playerManager.ts:44-46](file:///c:/Users/24453/Desktop/AI国象/src/lib/stores/playerManager.ts#L44-L46) 删 `peekManager`
- [stockfish/store.ts:239-245](file:///c:/Users/24453/Desktop/AI国象/src/lib/stockfish/store.ts#L239-L245) 删 `applySkillLevel`
- [App.svelte:93](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L93) 删 `playerTurn`
- [stockfish/store.ts](file:///c:/Users/24453/Desktop/AI国象/src/lib/stockfish/store.ts) 删 `lastBestMove`、`bestMove`、`bestScore`、`engineReady`
- [players/types.ts:9](file:///c:/Users/24453/Desktop/AI国象/src/lib/players/types.ts#L9) 删 `Player.type` 字段
- [cost.ts:158-173](file:///c:/Users/24453/Desktop/AI国象/src/lib/stores/cost.ts#L158-L173) 删 `dailyCacheHitRate`、`resetDailyCost`

**验证**：`npm run build` 通过，无新增 error/warning。

### 修改组 F：提取 flipped derived store

**文件**：[src/lib/stores/boardOrientation.ts](file:///c:/Users/24453/Desktop/AI国象/src/lib/stores/boardOrientation.ts)

**改什么**：新增 `flipped` derived store，整合 `player_side` 和 `boardFlipped`。

```ts
import { derived } from "svelte/store";
import { gameState } from "../stores/game";

export const flipped = derived(
  [gameState, boardFlipped],
  ([$g, $f]) => ($g.player_side === "black") !== $f
);
```

**配套**：[Board.svelte:180-182](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/Board.svelte#L180-L182) 和 [EvalBar.svelte:22-25](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/EvalBar.svelte#L22-L25) 改为 `$flipped`。

**验证**：棋盘朝向、EvalBar 朝向行为不变。

### 修改组 G：提取 .toggle 全局样式

**文件**：[src/app.css](file:///c:/Users/24453/Desktop/AI国象/src/app.css)

**改什么**：将 `.toggle`、`.toggle.on`、`.toggle-thumb`、`.toggle.on .toggle-thumb` 提取到全局，用 CSS 变量控制尺寸。

```css
.toggle {
  --toggle-w: 36px;
  --toggle-h: 20px;
  --toggle-thumb: 14px;
  width: var(--toggle-w);
  height: var(--toggle-h);
  /* ... 其余样式 */
}
```

**配套**：App.svelte 和 Settings.svelte 删除重复的 `.toggle` 定义，Settings 内通过 CSS 变量覆盖尺寸。

**验证**：开关样式视觉不变。

### 修改组 H：修复窄屏布局

**文件**：[src/lib/components/Board.svelte:607-608](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/Board.svelte#L607-L608)、[EvalBar.svelte:82](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/EvalBar.svelte#L82)

**改什么**：棋盘和 EvalBar 尺寸加 `100vw` 约束。

```css
width: min(72vh, 560px, 100vw - 2 * var(--sp-4));
```

**配套**：[App.svelte](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte) 加底部状态栏窄屏媒体查询，<700px 隐藏次要信息。

**验证**：窄屏下棋盘不溢出，状态栏不拥挤。

### 修改组 I：补充 A11y

**文件**：[src/app.css](file:///c:/Users/24453/Desktop/AI国象/src/app.css)、[App.svelte](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte)

**改什么**：
1. app.css 加全局 `:focus-visible` 样式
2. [App.svelte:630](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L630) drawer-close 加 `aria-label="关闭设置抽屉"`
3. [App.svelte:538-540](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L538-L540) toast 加 `role="alert" aria-live="assertive"`
4. [App.svelte:545-547](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L545-L547) 状态栏加 `aria-live="polite"`

**验证**：键盘 Tab 可见焦点，屏幕阅读器能朗读错误和状态变化。

## 假设与决策

| # | 假设/决策 | 说明 |
|---|----------|------|
| 1 | 假设 `board.checkers()` 是 chess crate 的标准 API | 需验证 crate 版本支持 |
| 2 | 决策用 generation 机制解决竞态 | 比 AbortController 简单，且不依赖 player 实现 |
| 3 | 决策 Stockfish 难度改用 store 动态读取 | 比 resetManager 更优雅，无需重建 player |
| 4 | 决策 Settings 的 handleStart 改为调 onStarted 回调 | 统一入口，App 集中处理 pendingStart |
| 5 | 决策保留后端 side 参数 | 避免改后端接口签名，前端已固定传 "white" |
| 6 | 决策删除 settings.side 前端字段 | 后端接口不变，前端不再用 |
| 7 | 决策提取 flipped 到 store | 消除 Board 和 EvalBar 的重复 |
| 8 | 决策提取 .toggle 到全局 | 消除 App 和 Settings 的重复 |
| 9 | 假设 :focus-visible 不影响现有样式 | 全局新增，不覆盖现有 :focus |
| 10 | 决策窄屏阈值 700px | 与现有 900px 阈值协调 |

## 验证步骤

1. **构建验证**：
   - `cargo check` 通过（后端 in_check 修改）
   - `npm run build` 通过，无新增 warning

2. **功能验证**（启动 `npm run tauri dev`）：
   - 将军时：音效响起、国王红框、状态文案
   - 暂停/继续快速点击：无双重走棋
   - 改鳕鱼难度后应用：立即生效
   - Settings 点开始对弈：与底部状态栏行为一致
   - 棋盘翻转、EvalBar 翻转：正常
   - 开关样式：视觉不变
   - 窄屏：棋盘不溢出、状态栏不拥挤
   - 键盘 Tab：可见焦点
   - 屏幕阅读器：朗读错误和状态

3. **回归验证**：
   - 人 vs DeepSeek 对局正常
   - 鳕鱼 vs 鳕鱼 自对弈正常
   - 暂停/继续正常
   - 重开/悔棋正常
   - 持久化恢复正常

## 下一个 PLAN（执行完本阶段后询问用户）

本阶段完成后，需询问用户下一个 PLAN 方向：
- **选项 A**：第二阶段（中严重度 19 项）—— aiFailed/isPaused 漏重置、sfEvalText 跳动、visual 字段、showError 计时器、statusKind win、mate 格式化重复、playerName 重复、ctrl-btn 合并、动画性能、resize 防抖、升变 aria、drawer role、状态栏 live region、抽屉焦点管理、Board 业务下沉
- **选项 B**：第三阶段（低严重度）—— App.svelte 拆分、Settings 拆分、playerManager DOM 解耦、MoveHistory 键盘导航、类型断言清理、性能微优化
- **选项 C**：启动 tauri dev 实测验证第一阶段
- **选项 D**：其他方向（用户指定）
