---
intent: 重构对弈架构为"三方主体任意组合"模型，引入鳕鱼作为可对弈对手（可调ELO/Skill），支持9种主体组合含自对弈
success_criteria:
  - 设置抽屉可选白方/黑方主体（人/鳕鱼/DeepSeek）
  - 鳕鱼方可调ELO（1320-3190）或Skill Level（0-20）
  - 9种主体组合均可正常对弈到结束（将军/和棋）
  - 自对弈（鳕鱼vs鳕鱼）可自动走棋，步间有延迟，可停止
  - 鳕鱼走棋时有举棋动画+走子音效（复用现有 aiPick 机制）
  - 旧持久化状态可平滑迁移到新字段
  - cargo check 与 npm run check 通过
risk_level: medium
auto_approve: true
worktree: false
---

## Steps

- [ ] **Step 1: 后端定义 PlayerType 枚举并扩展 GameStateDto**
action: 在 `src-tauri/src/game_state.rs` 中：1) 新增 `pub fn player_type_default() -> String { "deepseek".to_string() }`；2) `GameStateDto` 增 `pub white_player: String` 和 `pub black_player: String` 两个字段；3) `StartGameArgs` 增 `#[serde(default)] pub white_player: String` 和 `#[serde(default)] pub black_player: String`（默认空串，由命令侧兜底）。
verify: cargo check --manifest-path src-tauri/Cargo.toml
loop: until exit 0
max_iterations: 3

- [ ] **Step 2: 后端 ChessGame 增加 white/black_player 字段并扩展 game_to_dto**
action: 在 `src-tauri/src/chess_engine.rs` 的 `ChessGame` 结构体增加 `pub white_player: String` 和 `pub black_player: String` 字段；`ChessGame::new` 签名改为 `new(player_side, white_player, black_player)` 初始化字段；在 `game_state.rs` 的 `game_to_dto` 中填充 `white_player: game.white_player.clone()` 和 `black_player: game.black_player.clone()`。
verify: cargo check --manifest-path src-tauri/Cargo.toml
loop: until exit 0
max_iterations: 3

- [ ] **Step 3: 后端 persistence.rs 扩展 SaveData 并加旧存档迁移**
action: 在 `src-tauri/src/persistence.rs` 中：1) `SaveData` 增 `#[serde(default = "player_type_default_white")] pub white_player: String` 和 `#[serde(default = "player_type_default_black")] pub black_player: String`；2) 新增 `fn player_type_default_white() -> String { "human".to_string() }` 和 `fn player_type_default_black() -> String { "deepseek".to_string() }`（旧存档无字段时回退到"人vs DeepSeek"）；3) 在 `load` 函数解析成功后增加迁移逻辑：若 `white_player` 或 `black_player` 为空串，按 `player_side` 推断（player_side=="white" → white=human, black=deepseek；player_side=="black" → white=deepseek, black=human）。
verify: cargo check --manifest-path src-tauri/Cargo.toml
loop: until exit 0
max_iterations: 3

- [ ] **Step 4: 后端 commands.rs 扩展 start_game/reset_game 接收主体参数**
action: 在 `src-tauri/src/commands.rs` 中：1) `start_game` 用 `args.white_player`/`args.black_player`（空串兜底：白方默认"human"，黑方默认"deepseek"），传给 `ChessGame::new`；2) `reset_game` 签名增 `white_player: String` 和 `black_player: String` 参数，传给 `ChessGame::new`；3) `build_save_data` 填充 `white_player`/`black_player` 从 `game.white_player`/`game.black_player` 读取；4) `player_side` 仍按现有逻辑（有Human方则该方为player_side，都非Human则white）。
verify: cargo check --manifest-path src-tauri/Cargo.toml
loop: until exit 0
max_iterations: 3

- [ ] **Step 5: 后端 lib.rs 注册新参数并验证整体编译**
action: 在 `src-tauri/src/lib.rs` 中确认 `reset_game` 命令注册无需改动（Tauri 自动从参数名解析）；运行 `cargo check --manifest-path src-tauri/Cargo.toml` 确保 0 错误 0 警告；运行 `cargo test --manifest-path src-tauri/Cargo.toml` 确保现有测试通过。
verify: cargo test --manifest-path src-tauri/Cargo.toml
loop: until exit 0
max_iterations: 3

- [ ] **Step 6: 前端 types.ts 扩展 GameStateDto 与 StartGameArgs**
action: 在 `src/lib/types.ts` 中：1) `GameStateDto` 增 `white_player: PlayerType` 和 `black_player: PlayerType`；2) `StartGameArgs` 增 `white_player?: PlayerType` 和 `black_player?: PlayerType`；3) 新增 `export type PlayerType = "human" | "stockfish" | "deepseek";`。
verify: npm run check
loop: until exit 0
max_iterations: 3

- [ ] **Step 7: 前端创建 players/types.ts 定义 Player 接口**
action: 创建 `src/lib/players/types.ts`，定义：1) `export type PlayerType = "human" | "stockfish" | "deepseek";`；2) `export interface Player { type: PlayerType; isAutomatic: boolean; }`；3) `export interface PlayerMoveResult { moveStr: string; state: GameStateDto; gameOver: boolean; }`（复用现有 MoveResult）。
verify:
  type: artifact
  path: src/lib/players/types.ts
  assert:
    kind: exists
loop: false

- [ ] **Step 8: 前端创建 players/human.ts 实现**
action: 创建 `src/lib/players/human.ts`，导出 `createHumanPlayer()` 返回 `{ type: "human", isAutomatic: false }`。Human 不主动请求走棋，由 Board 点击触发编排器接收走法。
verify:
  type: artifact
  path: src/lib/players/human.ts
  assert:
    kind: exists
loop: false

- [ ] **Step 9: 前端创建 players/stockfish.ts 实现**
action: 创建 `src/lib/players/stockfish.ts`，导出 `createStockfishPlayer(opts: { elo: number; skill: number; useElo: boolean })`：1) 返回 `{ type: "stockfish", isAutomatic: true, requestMove }`；2) `requestMove(state)` 内部：调用 `loadEngine()`，若 `useElo` 则 `engine.setElo(elo)` 否则 `engine.setSkillLevel(skill)`，按 ELO 动态选 movetime（<1800→400ms, <2400→800ms, else→1500ms），调用 `engine.getBestMove(state.fen, movetime)`，返回 `{ moveStr: best.move, state: newState, gameOver }`（注意：stockfish 走法需通过后端 player_move 应用以更新状态，这里返回 uci 由 manager 调用 player_move）。
verify:
  type: artifact
  path: src/lib/players/stockfish.ts
  assert:
    kind: exists
loop: false

- [ ] **Step 10: 前端创建 players/deepseek.ts 实现**
action: 创建 `src/lib/players/deepseek.ts`，导出 `createDeepSeekPlayer()`：返回 `{ type: "deepseek", isAutomatic: true, requestMove }`；`requestMove` 内部直接调用现有 `aiMove()` Tauri 命令（已触发 ai-thinking/ai-pick/ai-usage 事件），返回 MoveResult。事件监听仍由 App.svelte 的 onMount 注册（不变）。
verify:
  type: artifact
  path: src/lib/players/deepseek.ts
  assert:
    kind: exists
loop: false

- [ ] **Step 11: 前端创建 players/manager.ts 编排器**
action: 创建 `src/lib/players/manager.ts`，导出 `PlayerManager` 类：1) 持有 whitePlayer/blackPlayer；2) `async driveTurn(state)`：根据 `state.turn` 选当前方 player，若 `isAutomatic` 则调用 `requestMove` → 通过 `player_move` 应用走法 → 更新 gameState → 递归 driveTurn（若对方也自动且游戏未结束）；3) 自对弈步间延迟 300ms（`await new Promise(r => setTimeout(r, 300))`）；4) `stop()` 方法设置停止标志，循环前检查；5) 最大步数兜底 200 步；6) Human 方时不调用，等待 Board 点击事件触发 `onHumanMove(moveStr)` → 应用走法 → driveTurn。
verify:
  type: artifact
  path: src/lib/players/manager.ts
  assert:
    kind: exists
loop: false

- [ ] **Step 12: 前端 settings.ts 增加主体与鳕鱼难度设置**
action: 在 `src/lib/stores/settings.ts` 中：1) `Settings` 接口增 `whitePlayer: PlayerType`、`blackPlayer: PlayerType`、`stockfishElo: number`、`stockfishSkill: number`、`stockfishUseElo: boolean`；2) `initialSettings` 默认 `whitePlayer: "human"`、`blackPlayer: "deepseek"`、`stockfishElo: 1500`、`stockfishSkill: 10`、`stockfishUseElo: true`；3) 导入 `PlayerType` from `../types`。
verify: npm run check
loop: until exit 0
max_iterations: 3

- [ ] **Step 13: 前端 game.ts 扩展 isPlayerTurn 逻辑**
action: 在 `src/lib/stores/game.ts` 中：1) 新增 `export const whitePlayer = writable<PlayerType>("human");` 和 `export const blackPlayer = writable<PlayerType>("deepseek");`；2) `isPlayerTurn` 改为 derived：`$gameState.turn === "white" ? $whitePlayer === "human" : $blackPlayer === "human"` 且 `status === "playing"`；3) 导入 `PlayerType`。
verify: npm run check
loop: until exit 0
max_iterations: 3

- [ ] **Step 14: 前端 api.ts 扩展 resetGame 签名**
action: 在 `src/lib/api.ts` 中：1) `resetGame` 增 `whitePlayer`/`blackPlayer` 参数，传递给后端 `reset_game` 命令；2) `startGame` 已通过 `StartGameArgs` 传递（增 white_player/black_player 字段）；3) 若有 `updateSettingsApi` 等其他调用，保持不变。
verify: npm run check
loop: until exit 0
max_iterations: 3

- [ ] **Step 15: 前端 Settings.svelte 添加主体选择UI**
action: 在 `src/lib/components/Settings.svelte` 中：1) 在"执方"区块上方新增"对弈双方"区块：白方下拉（人/鳕鱼/DeepSeek）+ 黑方下拉（人/鳕鱼/DeepSeek）；2) 当某方选"鳕鱼"时显示 ELO 滑块（1320-3190，步长20）+ Skill Level 滑块（0-20）+ "用ELO/用Skill"切换；3) 删除原"执方"白/黑选择（被主体选择取代）；4) `handleStart` 传递 `white_player`/`black_player`；5) 主体切换时若已开始对局则提示需重开。样式复用现有 `.seg-group`/`.toggle`/`.number-input`。
verify: npm run check
loop: until exit 0
max_iterations: 3

- [ ] **Step 16: 前端 App.svelte 用 PlayerManager 驱动对局循环**
action: 在 `src/App.svelte` 中：1) 导入 `PlayerManager` 和主体相关 stores；2) 创建 `playerManager` 实例；3) `handleReset` 改为调用 `resetGame($settings.whitePlayer, $settings.blackPlayer)`，设置 `whitePlayer`/`blackPlayer` stores，若白方自动则 `playerManager.driveTurn(state)`；4) Board 的 `onMove` 回调改为：应用玩家走法后 `playerManager.driveTurn(newState)`（若对方自动）；5) 自对弈时显示"停止"按钮调用 `playerManager.stop()`；6) `handleUndo` 后若当前方自动也触发 driveTurn；7) 棋盘翻转 `flipped` 改为：恰好一方是Human时按Human视角，都非Human时白底。
verify: npm run check
loop: until exit 0
max_iterations: 3

- [ ] **Step 17: 前端 Board.svelte 验证鳕鱼走棋举棋动画路径**
action: 在 `src/lib/components/Board.svelte` 中：1) 确认 `aiPick` store 仍由 DeepSeek 事件驱动（不变）；2) 为 Stockfish 走棋添加举棋动画：在 PlayerManager 应用 stockfish 走法前 500ms 设置 `aiPick.set(bestMove)`，走法应用后 `aiPick.set(null)`；3) 验证 `liftingSquare`/`pickTargetSquare` 派生对 stockfish 走法生效（复用现有逻辑）；4) 走子音效在 `updateGameState` 后触发（复用现有）。
verify: npm run check
loop: until exit 0
max_iterations: 3

- [ ] **Step 18: 前端整体编译验证**
action: 运行 `npm run check` 确保 0 错误 0 警告；修复所有类型错误和未使用变量。
verify: npm run check
loop: until exit 0
max_iterations: 5

- [ ] **Step 19: 后端整体编译与测试验证**
action: 运行 `cargo check --manifest-path src-tauri/Cargo.toml` 和 `cargo test --manifest-path src-tauri/Cargo.toml` 确保 0 错误且所有测试通过。
verify:
  - type: shell
    command: cargo check --manifest-path src-tauri/Cargo.toml
  - type: shell
    command: cargo test --manifest-path src-tauri/Cargo.toml
loop: until all exit 0
max_iterations: 3

- [ ] **Step 20: 启动应用验证4种典型组合**
action: 启动 `npm run tauri dev`，验证：1) 人(白) vs 鳕鱼(黑，ELO=1500)：走 e4 后鳕鱼响应合法走法，举棋动画可见，音效播放；2) 鳕鱼(白) vs 鳕鱼(黑)：自动对弈，步间延迟可见，点"停止"可中断；3) 人 vs 人：双方点击走棋无AI介入；4) 切换主体组合后旧对局清理。截图记录。
verify:
  type: human-review
  check: 4种组合均正常对弈，鳕鱼举棋动画可见，自对弈可停止，主体切换后状态正确
gate: human
loop: false
