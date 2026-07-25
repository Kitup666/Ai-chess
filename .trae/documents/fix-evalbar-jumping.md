# 修复胜率条一直跳的问题

## 摘要

用户反馈"左边的胜率条一直搁那跳跳跳跳"。

**根因**：EvalBar 直接订阅 `$multiPVList[0].score`，Stockfish 搜索时每个深度迭代、每次 PV 变化都会触发 `onInfo` 回调更新 `multiPVList`，导致 `whiteScore` 和 `whiteRatio` 实时变化，配合 CSS `transition: height 0.4s` 过渡动画，形成"一直跳"的视觉效果。

**Lichess 做法**：EvalBar 只在搜索完成（bestmove）或深度稳定后才更新，而非每次 info 都更新。

方案：引入"稳定评估值"概念，只在以下情况更新 EvalBar 显示：
1. 搜索完成（bestmove 返回）
2. 深度达到当前最高深度且停留超过 500ms（深度稳定）

简化实现：用 `derived` + 节流，只在深度递增时更新显示值，同深度内的多次更新不刷新显示。

## 当前状态分析

### 数据流

1. Stockfish 搜索 → `onInfo` 回调（[store.ts:111](file:///c:/Users/24453/Desktop/AI国象/src/lib/stockfish/store.ts#L111)）
2. `multiPVList.update()` 每次 info 都更新（[store.ts:115-127](file:///c:/Users/24453/Desktop/AI国象/src/lib/stockfish/store.ts#L115-L127)）
3. EvalBar 订阅 `$multiPVList[0].score`（[EvalBar.svelte:16](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/EvalBar.svelte#L16)）
4. `whiteScore` = `scoreFromWhitePerspective(info.score, turn)`（[EvalBar.svelte:18](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/EvalBar.svelte#L18)）
5. `whiteRatio` = `0.5 + clamped/1000`（[EvalBar.svelte:33](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/EvalBar.svelte#L33)）
6. CSS `transition: height 0.4s`（[EvalBar.svelte:102,108](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/EvalBar.svelte#L102-L108)）

### 更新频率

Stockfish 在搜索过程中：
- 每个深度迭代完成时发 `info depth N`
- 搜索中会发 `info depth N currmovenumber X`（当前在评估第几个走法）
- 找到更好走法时发 `info depth N score cp X pv ...`
- 同一深度内可能多次更新 score（找到更好走法）

### 问题表现

- 深度从 1→2→3...→18 递增，每个深度的 score 都不同，EvalBar 跳
- 同一深度内 score 也可能变化，EvalBar 跳
- 0.4s 过渡动画还没完成就被新值打断

## 拟议修改

### 修改 1：引入稳定评估值 store

**文件**：`src/lib/stockfish/store.ts`

**改什么**：新增 `stableScore` store，只在深度递增时更新，同深度内的多次更新不刷新。

**为什么**：避免每个 info 都触发 EvalBar 更新，只在深度提升时更新显示值。

**怎么改**：
```ts
/// 稳定评估值（只在深度递增时更新，避免 EvalBar 跳动）
/// 同深度内的多次 score 变化不刷新显示
let lastStableDepth = 0;
export const stableScore = writable<{ score: SearchInfo["score"] | null; depth: number } | null>(null);

// 在 onInfo 回调内，multiPVList.update 之后：
if (pv === 1 && info.depth > lastStableDepth) {
  lastStableDepth = info.depth;
  stableScore.set({ score: info.score, depth: info.depth });
}

// 在 startAnalysis 和 resetEngine 中重置：
lastStableDepth = 0;
stableScore.set(null);

// 在 bestmove 返回时也更新（最终值）：
// onBestMove 回调内保持 stableScore 不变（最后一个深度的值就是最终值）
```

### 修改 2：EvalBar 使用 stableScore

**文件**：[src/lib/components/EvalBar.svelte](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/EvalBar.svelte)

**改什么**：将 `whiteScore` 从订阅 `multiPVList` 改为订阅 `stableScore`。

**为什么**：只在深度递增时更新，避免同深度内多次变化导致跳动。

**怎么改**：
```ts
import { stableScore, scoreFromWhitePerspective } from "../stockfish/store";

// 原：
// let whiteScore = $derived.by(() => {
//   const info = $multiPVList[0];
//   if (!info?.score) return null;
//   return scoreFromWhitePerspective(info.score, $gameState.turn as "white" | "black");
// });

// 改为：
let whiteScore = $derived.by(() => {
  const stable = $stableScore;
  if (!stable?.score) return null;
  return scoreFromWhitePerspective(stable.score, $gameState.turn as "white" | "black");
});
```

### 修改 3：bestmove 返回时更新最终值（可选优化）

**文件**：`src/lib/stockfish/store.ts`

**改什么**：在 `onBestMove` 回调内，用 `multiPVList[0]` 的最终 score 更新 stableScore。

**为什么**：确保搜索完成后 EvalBar 显示最终深度的不动值，而非倒数第二个深度。

**怎么改**：
```ts
engineInstance.onBestMove = (best) => {
  lastBestMove.set(best);
  isAnalyzing.set(false);
  isPaused.set(false);
  // 搜索完成：用最终值更新 stableScore
  const finalInfo = get(multiPVList)[0];
  if (finalInfo?.score) {
    stableScore.set({ score: finalInfo.score, depth: finalInfo.depth });
  }
};
```

## 假设与决策

| # | 假设/决策 | 说明 |
|---|----------|------|
| 1 | 假设跳动主因是同深度内多次 score 更新 | 深度递增时跳是正常的（分数变化），但同深度内跳是冗余的 |
| 2 | 决策只在深度递增时更新 | 最简单有效，与 Lichess 行为一致 |
| 3 | 决策保留 0.4s 过渡动画 | 深度递增时的平滑过渡是好的视觉反馈 |
| 4 | 决策不改变 multiPVList | 其他组件（AnalysisPanel）仍需实时 PV 数据 |
| 5 | 假设 bestmove 时最终值已在 multiPVList[0] | Stockfish bestmove 前最后一个 info 是最终深度 |

## 验证步骤

1. **构建验证**：`npm run build` 通过，无 error
2. **视觉验证**（启动 `npm run tauri dev`）：
   - 启用鳕鱼引擎，开始对局
   - AI 走棋后触发分析，观察 EvalBar
   - **预期**：EvalBar 只在深度递增时更新（1→2→3...），同深度内不再跳动
   - 深度递增时有 0.4s 平滑过渡（正常）
   - 搜索完成后显示最终深度的稳定值
   - 不再出现"一直跳"的现象
3. **功能验证**：
   - AnalysisPanel 的 PV 列表仍实时更新（不受影响）
   - 底部状态栏的评估文本仍实时更新（若需要可同步改用 stableScore，但当前方案保留实时）
   - 暂停/恢复分析后 EvalBar 正确显示
