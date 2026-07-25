# 移除 Settings 里的"执方"选项

## 摘要

用户反馈"执方选项好像没意义了"。经分析确认：三方主体架构下，`whitePlayer`/`blackPlayer` 已明确指定谁是人/AI，`side` 选项是冗余的。后端推导逻辑（[commands.rs:79-87](file:///c:/Users/24453/Desktop/AI国象/src-tauri/src/commands.rs#L79-L87)）在"恰好一方是人"时自动推导 `player_side`，`side` 设置被忽略；双自动/双人时 `side` 仅作"兜底"决定 `player_side`（影响棋盘朝向），但用户已有手动反转按钮，不需要这个选项。

方案：移除 Settings.svelte 里的"执方"选项 UI，前端 `side` 固定为 "white"，后端逻辑不变（推导逻辑仍生效，兜底时用 "white"）。

## 当前状态分析

### 执方选项 UI

[Settings.svelte:408-423](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/Settings.svelte#L408-L423)：
```svelte
<div class="section">
  <div class="label">执方（仅双自动/双人时用于初始化）</div>
  <div class="seg-group">
    <button class="seg-btn" class:active={side === "white"} onclick={() => updateSettings({ side: "white" as Side })}>白方 先手</button>
    <button class="seg-btn" class:active={side === "black"} onclick={() => updateSettings({ side: "black" as Side })}>黑方 后手</button>
  </div>
  <p class="hint">恰好一方为「人」时，该方为玩家执方；双方均自动或均为人时此项仅作初始化兜底。</p>
</div>
```

### side 的使用链

1. **前端**：
   - [Settings.svelte:56](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/Settings.svelte#L56) `let side = $derived($settings.side)`
   - [Settings.svelte:104](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/Settings.svelte#L104) `startGame({ side, ... })`
   - [Settings.svelte:157](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/Settings.svelte#L157) `resetGame(side, whitePlayer, blackPlayer)`
   - [App.svelte:314](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L314) `resetGame($settings.side, white, black)`
   - [App.svelte:363](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L363) `startGame({ side: $settings.side, ... })`

2. **后端**：
   - [commands.rs:79-87](file:///c:/Users/24453/Desktop/AI国象/src-tauri/src/commands.rs#L79-L87) `start_game`：推导 `player_side`，`side` 仅在双自动/双人时兜底
   - [commands.rs:516-524](file:///c:/Users/24453/Desktop/AI国象/src-tauri/src/commands.rs#L516-L524) `reset_game`：同上

3. **player_side 的影响**：
   - [Board.svelte:181](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/Board.svelte#L181) 棋盘翻转（现有手动反转按钮可调整）
   - [Board.svelte:52](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/Board.svelte#L52) `aiOnTop` 思考条位置
   - [Board.svelte:563-565](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/Board.svelte#L563-L565) 升变棋子方向
   - [EvalBar.svelte:23](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/EvalBar.svelte#L23) 评估柱翻转

### 为什么"没意义"

1. **恰好一方是人**：`player_side` 自动推导，`side` 被忽略
   - 白方=人、黑方=AI → `player_side`="white"（无论 side 选什么）
   - 白方=AI、黑方=人 → `player_side`="black"（无论 side 选什么）
2. **双自动（如鳕鱼vs鳕鱼）**：没有"玩家"概念，`side` 仅决定棋盘朝向，但手动反转按钮已能做这事
3. **双人模式**：两人都在玩，`side` 仅决定棋盘朝向，手动反转按钮可替代
4. Settings 提示文案自己都承认"仅作初始化兜底"

## 拟议修改

### 修改 1：移除 Settings.svelte 的"执方"选项 UI

**文件**：[src/lib/components/Settings.svelte:408-423](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/Settings.svelte#L408-L423)

**改什么**：删除整个"执方"section。

**为什么**：选项冗余，用户已不需要。

**怎么改**：直接删除这 16 行（含空行）。

### 修改 2：前端 side 固定为 "white"

**文件**：[src/lib/components/Settings.svelte:104](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/Settings.svelte#L104)、[src/lib/components/Settings.svelte:157](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/Settings.svelte#L157)

**改什么**：`startGame` 和 `resetGame` 调用时 `side` 参数固定传 `"white"`。

**为什么**：后端推导逻辑在"恰好一方是人"时会忽略 `side`，双自动/双人时用 `"white"` 兜底即可。用户可用反转按钮调整棋盘朝向。

**怎么改**：
```js
// Settings.svelte:104
const state = await startGame({
  side: "white" as Side,  // 原: side
  // ...
});

// Settings.svelte:157
const state = await resetGame("white" as Side, whitePlayer, blackPlayer);  // 原: side
```

### 修改 3：移除 Settings.svelte 中 side 相关变量

**文件**：[src/lib/components/Settings.svelte:56](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/Settings.svelte#L56)

**改什么**：移除 `let side = $derived($settings.side);`。

**为什么**：UI 已移除，变量不再需要。

**怎么改**：删除该行。

### 修改 4：App.svelte 的 startGame/resetGame 调用同步

**文件**：[src/App.svelte:314](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L314)、[src/App.svelte:363](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L363)

**改什么**：`resetGame` 和 `startGame` 调用时 `side` 参数固定传 `"white"`。

**为什么**：与 Settings.svelte 保持一致。

**怎么改**：
```js
// App.svelte:314
const state = await resetGame("white" as Side, white, black);  // 原: $settings.side

// App.svelte:363
const state = await startGame({
  side: "white" as Side,  // 原: $settings.side
  // ...
});
```

### 修改 5：清理 Settings.svelte 未使用的 Side import（如需）

**文件**：[src/lib/components/Settings.svelte](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/Settings.svelte)

**改什么**：检查 `Side` 类型 import 是否还被其他地方使用，若否则移除。

**为什么**：清理未使用的 import。

**怎么改**：先检查，若 `Side` 仅用于 `side` 变量则移除 import。

## 不改动的部分

- **后端 `side` 参数**：保留，前端传 "white" 即可，后端接口签名不变
- **后端 `player_side` 推导逻辑**：保留，仍用于决定棋盘朝向、思考条位置、升变方向
- **`settings.side` store 字段**：保留（避免 store 结构变动影响持久化兼容），但前端不再使用其值
- **手动反转按钮**：保留，用户可手动调整棋盘朝向

## 假设与决策

| # | 假设/决策 | 说明 |
|---|----------|------|
| 1 | 假设 side 选项对用户无意义 | 三方主体架构下 whitePlayer/blackPlayer 已明确，side 冗余 |
| 2 | 决策前端 side 固定 "white" | 后端推导逻辑在有人时会忽略，无人时用 white 兜底 |
| 3 | 决策保留后端 side 参数 | 避免改动后端接口签名，保持兼容 |
| 4 | 决策保留 settings.side 字段 | 避免 store 结构变动破坏持久化 |
| 5 | 决策保留 player_side 推导 | 棋盘朝向、思考条位置、升变方向仍需 player_side |
| 6 | 不改手动反转按钮 | 用户仍需手动调整朝向，尤其在双自动/双人模式 |

## 验证步骤

1. **构建验证**：`npm run build` 通过，无 error
2. **类型检查**：无未使用变量警告
3. **功能验证**：
   - Settings 里"执方"选项已消失
   - 白方=人、黑方=AI → 开始对局，玩家执白（自动推导）
   - 白方=AI、黑方=人 → 开始对局，玩家执黑（自动推导）
   - 鳕鱼vs鳕鱼 → 开始对局，棋盘默认白在底，可用反转按钮调整
   - 双人模式 → 开始对局，棋盘默认白在底，可用反转按钮调整
   - 反转按钮功能正常
   - 升变棋子方向正确（白方升变白棋，黑方升变黑棋）
   - AI 思考条位置正确（AI 在上方）
