---
design_type: phase
created_at: 2026-07-24
parent_initiative: stockfish-lichess-initiative
---

# Phase 1 — 引擎对弈模式（三方主体架构）

## Intent Contract

```
intent: 重构对弈架构为"三方主体任意组合"模型，引入鳕鱼作为可对弈对手（可调ELO/Skill），支持9种主体组合含自对弈
constraints:
  - 不破坏现有 DeepSeek 对局流程（ai_move 命令及流式思考/举棋/成本统计保留）
  - 不破坏现有持久化、动画、音效、思考展示
  - 复用已有 Stockfish engine.ts（WASM Worker，含 setElo/setSkillLevel）
  - 棋盘翻转逻辑保持：仅当恰好一方是Human时按其视角翻转；非Human对弈默认白方在底
success_criteria:
  - 设置抽屉可选白方/黑方主体（人/鳕鱼/DeepSeek）
  - 鳕鱼方可调ELO（1320-3190）或Skill Level（0-20）
  - 9种主体组合均可正常对弈到结束（将军/和棋）
  - 自对弈（鳕鱼vs鳕鱼）可自动走棋，步间有延迟，可停止
  - 鳕鱼走棋时有举棋动画+走子音效（复用现有 aiPick 机制）
  - 旧持久化状态可平滑迁移到新字段
  - cargo check 与 npm run check 通过
risk_level: medium
```

## Verification Contract

```
verify_steps:
  - cargo check --manifest-path src-tauri/Cargo.toml 通过
  - npm run check 0 errors 0 warnings
  - 人(白) vs 鳕鱼(黑)：玩家走e4，鳕鱼响应合法走法；ELO=1500明显弱于ELO=3000
  - 鳕鱼(白) vs 鳕鱼(黑)：自动对弈，步间延迟可见，可中途停止
  - DeepSeek(白) vs 鳕鱼(黑)：DeepSeek走法触发鳕鱼响应，双方自动
  - 人 vs 人：本地双人对弈，无AI介入
  - 切换主体组合后旧对局状态正确清理
  - 鳕鱼走棋触发举棋动画+走子音效
  - 重启应用后白方/黑方主体选择持久化恢复
```

## Governance Contract

```
approval_gates:
  - 三方主体接口设计需人工确认（影响全局架构）
  - 主体选择UI布局需人工验证
  - 鳕鱼ELO对应的实际强度需人工抽样验证（1500/2000/2500各下1局）
rollback:
  - 主体抽象为前端新增层，可回退到"player_side + ai_move"假设
  - 鳕鱼对弈逻辑独立于DeepSeek，可单独禁用
  - 后端仅增字段未改走棋逻辑，可回退字段
ownership: 用户（架构确认+强度验证）+ 开发者（实现）
```

## Scope

| In | Out |
|---|---|
| 三方主体接口抽象（前端 Player 接口 + 三实现） | 后端 Player trait 重构（后端仍按主体类型分发） |
| 主体选择UI（白方/黑方下拉 + 条件难度） | 主体强度可视化图表 |
| 鳕鱼作为对手（可调ELO/Skill） | 鳕鱼思考时间细粒度控制（用默认动态movetime） |
| 9种主体组合对弈 | 时间控制/计时器 |
| 自对弈（鳕鱼vs鳕鱼等，步间延迟300ms，可停止） | 自对弈加速/批量生成棋谱 |
| 鳕鱼走棋举棋动画+音效（复用 aiPick） | 鳕鱼走法SAN解说文字 |
| 持久化白方/黑方主体选择 + 旧状态迁移 | ELO历史记录 |
| 棋盘翻转逻辑适配（非Human对弈默认白底） | 多视角切换 |

## Decisions

| # | 决策 | 选择 | 理由 |
|---|---|---|---|
| 1 | 主体接口位置 | 前端Svelte层（PlayerManager编排） | DeepSeek API在后端但经Tauri命令暴露，Stockfish WASM在前端，统一前端编排最自然，改动最小 |
| 2 | 难度调节 | ELO为主（1320-3190）+ Skill Level为高级选项 | lichess风格用ELO直观；Skill Level提供更细粒度控制 |
| 3 | 鳕鱼思考时间 | 按ELO动态：低ELO(1320-1800)用400ms，中(1800-2400)用800ms，高(2400-3190)用1500ms | 低ELO弱化靠随机走法+短思考，高ELO靠深度搜索 |
| 4 | 自对弈步间延迟 | 300ms固定 | 便于观察不卡顿；后续可做可调 |
| 5 | 主体选择UI位置 | 设置抽屉内白方/黑方两个下拉 + 条件性难度区 | 复用现有抽屉，不新增顶部栏 |
| 6 | 棋盘翻转规则 | 恰好一方是Human时按Human视角；双方都非Human时白方在底；双方都是Human时白方在底 | 保持人视角对弈体验，自对弈/双AI时白底观看 |
| 7 | 旧 player_side 兼容 | 加载时按 player_side 推断：player_side=white → whitePlayer=human, blackPlayer=deepseek | 迁移旧持久化状态，无破坏 |
| 8 | 鳕鱼举棋动画 | 鳕鱼走棋前500ms设置 aiPick=bestmove，触发举棋动画 | 复用现有举棋视觉，让鳕鱼"思考"可见 |
| 9 | DeepSeek 实现复用 | DeepSeek Player 内部调用现有 ai_move Tauri命令 + 监听 ai-thinking/ai-pick 事件 | 不重写后端，前端编排层适配事件流 |
| 10 | 双AI思考展示 | 仅当前轮到方是DeepSeek且开关开时展示思考；鳕鱼方不展示思考（可展示引擎info但非Phase1） | 避免双AI思考内容混乱 |

## Surface

**前端**：

- `src/lib/players/` — 新建目录，定义 Player 接口与三种实现
  - `types.ts` — `PlayerType`、`Player` 接口、`MoveResult` 复用
  - `human.ts` — Human 实现（`isAutomatic=false`，走棋由 Board 点击触发编排器）
  - `stockfish.ts` — Stockfish 实现（`isAutomatic=true`，调用 `engine.getBestMove` + 设置 ELO/Skill）
  - `deepseek.ts` — DeepSeek 实现（`isAutomatic=true`，调用 `ai_move` 命令 + 事件监听转发）
  - `manager.ts` — `PlayerManager` 编排器（驱动当前轮到方、自对弈循环、步间延迟、停止控制）
- `src/lib/stores/game.ts` — 新增 `whitePlayer`/`blackPlayer` 状态；`isPlayerTurn` 改为"当前轮到方主体是否为Human"
- `src/lib/stores/settings.ts` — 新增 `whitePlayer`/`blackPlayer`/`stockfishElo`/`stockfishSkill`/`stockfishUseElo` 设置（持久化）
- `src/lib/components/Settings.svelte` — 添加主体选择UI（白方/黑方下拉 + 条件性 ELO/Skill 滑块）
- `src/App.svelte` — 用 `PlayerManager` 驱动对局循环，替代直接 `aiMove` 调用；处理自对弈停止
- `src/lib/components/Board.svelte` — 鳕鱼走棋路径走现有 `aiPick` 举棋动画
- `src/lib/types.ts` — `GameStateDto` 增 `white_player`/`black_player` 字段

**后端**：

- `src-tauri/src/game_state.rs` — `GameState` 增 `white_player`/`black_player` 字段（`PlayerType` 枚举）
- `src-tauri/src/persistence.rs` — 序列化/反序列化新字段，旧状态缺字段时按 `player_side` 推断默认
- `src-tauri/src/commands.rs` — `reset_game` 接收 `white_player`/`black_player` 参数；现有 `ai_move` 不变
- `src-tauri/src/lib.rs` — 加载持久化时应用迁移逻辑

## Risks & Open Questions

| # | 风险 | 缓解 |
|---|---|---|
| 1 | 三方主体重构影响现有 DeepSeek 流程（流式思考、举棋、成本统计） | Player 接口设计保留思考事件钩子；DeepSeek 实现内部复用现有 ai_move + 事件监听，不重写后端 |
| 2 | 自对弈循环可能无限（鳕鱼vs鳕鱼永不结束） | 每步检测游戏结束状态终止循环；提供"停止"按钮；最大步数兜底（如200步） |
| 3 | ELO数值与实际强度对应不准 | 参考 Stockfish 官方 UCI_Elo 范围；用户人工抽样验证三档强度 |
| 4 | 旧持久化状态迁移（无 white/black_player 字段） | 加载时若缺字段，按 player_side 推断默认；Rust 端 serde default + 迁移函数 |
| 5 | 双AI对弈时思考展示应显示谁的 | 仅当显示开关开且当前方是DeepSeek时显示；鳕鱼方不展示思考（Phase1） |
| 6 | Human vs Human 时 isPlayerTurn 逻辑失效 | isPlayerTurn 改为"当前轮到方主体是否为Human"，Human vs Human 时双方都true |
| 7 | 鳕鱼走棋太快来不及看举棋动画 | 鳕鱼走棋前设置 aiPick 后延迟500ms再应用走法，让举棋动画可见 |
| 8 | 主体切换时旧对局残留 | 切换白方/黑方主体时调用 reset_game 清空棋局，避免状态错乱 |
