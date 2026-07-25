---
design_type: initiative
created_at: 2026-07-24
---

# 像lichess那样引入鳕鱼 — 三方主体对弈增强

## 问题与背景

当前应用已完成 Stockfish 分析模式（评估柱、多PV变着、最佳走法箭头、深度流式更新），但鳕鱼仅作为"分析工具"存在，未作为"对弈主体"。用户希望像 lichess 那样全面引入鳕鱼：作为可对弈的对手、作为复盘评估器、作为实时走法提示源、并接入开局/残局库。

更关键的是，用户提出"把人、鳕鱼、DeepSeek 当成三个人来看，各自可以排列组合，也可以和自己打"。这意味着需要从"玩家 vs DeepSeek"的二元假设，重构为"三方主体任意组合"的通用对弈架构。现有代码的 `player_side` 字段假设玩家执某方、对手固定是 DeepSeek，这一假设需要在架构层面打破。

## 愿景

构建一个统一的三方主体对弈平台：

- **主体类型**：Human（人）、Stockfish（鳕鱼）、DeepSeek（AI）
- **任意组合**：白方与黑方各自独立选择主体，支持 3×3=9 种组合（含自对弈）
- **自对弈**：鳕鱼vs鳕鱼（用于复盘演示/观赏）、DeepSeek vs DeepSeek（用于测试/AI自演）、人vs人（本地双人）
- **统一体验**：无论对手是谁，走棋流程、动画、音效、状态管理、思考展示一致

在此架构上，分4个phase逐步实现 lichess 级别的鳕鱼体验。

## 非目标

- 在线对战/联网（仅本地）
- ELO积分系统/排行榜（仅难度调节，不追踪用户ELO）
- PGN导入导出（已有走法历史数组，不额外做PGN格式互转）
- 时间控制/计时器（非本次范围）
- 多开对局（单对局为主）
- 棋谱分享/社交
- API Key 加密存储（维持现状）

## 三方主体架构

### 主体接口

定义统一的"对弈主体"抽象，屏蔽不同走棋来源的差异。接口位于前端 Svelte 层，因为 Stockfish WASM 在前端、DeepSeek 通过 Tauri 命令暴露给前端，前端编排最自然：

```
type PlayerType = "human" | "stockfish" | "deepseek";

interface Player {
  type: PlayerType;
  // 请求走棋：传入当前局面，返回走法
  // Human: 不主动返回，等用户点击棋盘后由编排器接收
  // Stockfish: 调用 WASM 引擎 getBestMove（设置 ELO/Skill 后）
  // DeepSeek: 调用后端 Tauri 命令 ai_move（触发流式思考事件）
  requestMove(state: GameState): Promise<MoveResult> | void;
  // 是否自动走棋（非Human）
  isAutomatic: boolean;
}
```

### 混合部署

- **Human**：纯前端，点击棋盘走棋，`isAutomatic=false`
- **Stockfish**：纯前端，WASM Worker（复用已有 `engine.ts`），`isAutomatic=true`
- **DeepSeek**：后端 Rust 调用 API（复用已有 `commands.rs`），前端通过 Tauri 命令触发，`isAutomatic=true`

前端编排器（PlayerManager）驱动对局循环：当前轮到方 → 若 `isAutomatic` 则调用 `requestMove` → 应用走法 → 切换轮到方 → 重复。Human 方则等待点击事件。自对弈时两方都 `isAutomatic`，编排器循环驱动，步间加延迟便于观察。

### 状态扩展

现有 `gameState.player_side` 假设玩家执某方、对手固定是 DeepSeek。重构为：

```
white_player: PlayerType  // 白方主体
black_player: PlayerType  // 黑方主体
// player_side 保留用于"Human视角的棋盘翻转"
// 仅当恰好有一方是Human时有意义；双方都非Human时默认白方在底
```

### UI选择

开始对局时，在设置抽屉选择白方主体和黑方主体。某方是 Stockfish 时显示 ELO/Skill 选择；某方是 DeepSeek 时需要 API Key（复用现有）。

## Phase 分解

| Phase | 主题 | 依赖 | 核心交付 |
|---|---|---|---|
| Phase 1 | 引擎对弈模式 | 三方主体架构 | 鳕鱼作为对手（可调ELO/Skill），支持9种主体组合，自对弈 |
| Phase 2 | 实时走法提示增强 | Phase 1 架构 | 悬停棋子显示候选走法+评估，走法评估变化即时反馈 |
| Phase 3 | 走法复盘评估 | Phase 1 走法历史 | 每步标色（最佳/好/不准确/失误/漏算），全局accuracy%，评估曲线图 |
| Phase 4 | 开局/残局库接入 | 无强依赖 | 走法匹配Lichess开局探索器，残局库查表 |

执行顺序（用户确认）：对弈优先 → 提示 → 复盘 → 数据源。

## 风险与开放问题

| # | 风险/问题 | 缓解/方向 |
|---|---|---|
| 1 | 三方主体重构影响现有 DeepSeek 对局流程 | 保留 ai_move 命令，前端编排层适配；现有持久化状态需迁移白方/黑方主体字段 |
| 2 | 自对弈（鳕鱼vs鳕鱼）前端循环可能阻塞UI | 鳕鱼在Worker中运行，不阻塞；加步间延迟（如300ms）便于观察 |
| 3 | DeepSeek 自对弈消耗API成本 | 提示用户成本，默认不自动开始；可设最大步数 |
| 4 | 主体选择UI复杂度 | 简化为两个下拉+条件性难度/API Key |
| 5 | Phase 3 accuracy% 算法 | 参考 lichess：与引擎最佳走法偏差映射到0-100%，需多深度评估每步 |
| 6 | Phase 4 开局库数据源 | Lichess opening explorer API 或本地masters数据库 |
| 7 | 双AI对弈时思考展示应显示谁的 | 仅当显示开关开且当前方是DeepSeek时显示；鳕鱼方可选展示引擎搜索info |
| 8 | 旧持久化状态迁移（无 white/black_player 字段） | 加载时若缺字段，按 player_side 推断默认（player_side=white → 白Human+黑DeepSeek） |
