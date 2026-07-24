---
intent: 后端统一持久化对局进度与设置到 app_data_dir/chess_state.json，重启应用自动恢复到上次局面与配置
success_criteria:
  - 重启应用后未结束对局自动恢复到上次局面，API Key/模型/思考模式/执方设置保留
  - cargo check 与 npm run check 均通过，无 error 无 warning
  - 走棋/悔棋/重开后存档正确更新；无对局时存档为空或不存在
risk_level: medium
auto_approve: false
---

## Steps

- [ ] **Step 1: 新建 persistence.rs 模块定义存档结构与文件 IO**
action: |
  创建文件 `src-tauri/src/persistence.rs`，定义：
  1. `SaveData` 结构（derive Serialize/Deserialize）：
     ```rust
     pub struct SaveData {
         pub has_game: bool,
         pub player_side: String,        // "white" | "black"
         pub fen: String,                // 当前局面 FEN
         pub move_history: Vec<String>,  // 坐标记号列表 e2e4 e7e5 ...
         pub status: String,             // playing | checkmate | stalemate
         pub settings: SettingsSave,
     }
     pub struct SettingsSave {
         pub api_key: String,
         pub model: String,
         pub thinking: bool,
     }
     ```
  2. `save(app: &AppHandle, data: &SaveData) -> Result<(), String>`：
     - 用 `app.path().app_data_dir()` 获取目录
     - 若目录不存在则 `std::fs::create_dir_all`
     - 序列化为 JSON 写入 `chess_state.json`（`serde_json::to_string_pretty`）
     - 失败时返回 Err，但不 panic
  3. `load(app: &AppHandle) -> Option<SaveData>`：
     - 读取 `chess_state.json`，`serde_json::from_str` 解析
     - 任何错误（文件不存在/解析失败）返回 None 并 `log::warn!` 记录
  4. `clear(app: &AppHandle) -> Result<(), String>`：
     - 删除 `chess_state.json`，文件不存在视为成功
loop: false
verify: cargo check --manifest-path src-tauri/Cargo.toml
max_iterations: 3

- [ ] **Step 2: ChessGame 新增 rebuild_from_history 方法**
action: |
  在 `src-tauri/src/chess_engine.rs` 的 `impl ChessGame` 块内新增：
  ```rust
  /// 从初始局面重放走法历史，重建棋局（用于持久化恢复）
  pub fn rebuild_from_history(
      player_side: Color,
      moves: Vec<String>,
  ) -> Result<Self, String> {
      let mut game = Self::new(player_side);
      for mv_str in moves {
          let mv = parse_coord_move(&mv_str)
              .ok_or_else(|| format!("无法解析走法: {}", mv_str))?;
          if !game.is_legal(&mv) {
              return Err(format!("存档含非法走法: {}", mv_str));
          }
          game.make_move(mv).map_err(|e| e)?;
      }
      Ok(game)
  }
  ```
  该方法利用现有 `parse_coord_move`、`is_legal`、`make_move`，从默认初始局面逐步重放，重建 board_history（make_move 内部已 push board_history）。无需新增依赖。
loop: false
verify: cargo check --manifest-path src-tauri/Cargo.toml
max_iterations: 3

- [ ] **Step 3: lib.rs 注册 persistence 模块并在 setup 加载存档**
action: |
  修改 `src-tauri/src/lib.rs`：
  1. 顶部 `mod` 区添加 `mod persistence;`
  2. `AppState::new()` 改为 `AppState::new()` 保持不变（无存档时用默认空状态）
  3. 在 `.setup(|app| { ... })` 闭包内、log plugin 初始化之后，添加存档加载逻辑：
     ```rust
     // 加载持久化存档
     if let Some(save) = persistence::load(app.handle()) {
         let state: &AppState = app.state();
         // 恢复设置
         {
             let mut s = state.settings.lock().unwrap();
             s.api_key = save.settings.api_key;
             s.model = save.settings.model;
             s.thinking = save.settings.thinking;
         }
         // 恢复 DeepSeek 客户端（若 api_key 非空）
         if !save.settings.api_key.is_empty() {
             let client = crate::deepseek::DeepSeekClient::new(
                 save.settings.api_key.clone(),
                 save.settings.model.clone(),
                 save.settings.thinking,
             );
             *state.deepseek.lock().unwrap() = Some(client);
         }
         // 恢复棋局（若有）
         if save.has_game {
             if let Ok(game) = crate::chess_engine::ChessGame::rebuild_from_history(
                 crate::chess_engine::str_to_color(&save.player_side).unwrap_or(chess::Color::White),
                 save.move_history,
             ) {
                 *state.game.lock().unwrap() = Some(game);
             }
         }
     }
     ```
  4. 在 `invoke_handler!` 中注册新命令 `commands::load_state`（Step 5 会创建）
loop: false
verify: cargo check --manifest-path src-tauri/Cargo.toml
max_iterations: 3

- [ ] **Step 4: 新增 build_save_data 辅助函数构造存档**
action: |
  在 `src-tauri/src/commands.rs` 顶部 use 区添加 `use crate::persistence::{SaveData, SettingsSave};`，并在文件末尾辅助函数区新增：
  ```rust
  /// 从当前 AppState 构造存档数据（无对局时 has_game=false）
  fn build_save_data(state: &AppState, status: &str) -> SaveData {
      let settings = state.settings.lock().unwrap();
      let settings_save = SettingsSave {
          api_key: settings.api_key.clone(),
          model: settings.model.clone(),
          thinking: settings.thinking,
      };
      let game_lock = state.game.lock().unwrap();
      match game_lock.as_ref() {
          Some(game) => SaveData {
              has_game: true,
              player_side: crate::chess_engine::color_to_str(game.player_side),
              fen: game.to_fen(),
              move_history: game.move_history
                  .iter()
                  .map(|mv| crate::chess_engine::move_to_coord(mv))
                  .collect(),
              status: status.to_string(),
              settings: settings_save,
          },
          None => SaveData {
              has_game: false,
              player_side: "white".to_string(),
              fen: String::new(),
              move_history: vec![],
              status: String::new(),
              settings: settings_save,
          },
      }
  }
  ```
  此函数供 Step 6 各命令调用。
loop: false
verify: cargo check --manifest-path src-tauri/Cargo.toml
max_iterations: 3

- [ ] **Step 5: 新增 load_state 命令供前端启动调用**
action: |
  在 `src-tauri/src/commands.rs` 新增命令：
  ```rust
  /// 加载持久化状态（前端启动时调用，返回当前棋局 DTO + 设置）
  #[tauri::command]
  pub async fn load_state(
      state: State<'_, AppState>,
  ) -> Result<Option<LoadedState>, String> {
      let settings = state.settings.lock().unwrap();
      let api_key = settings.api_key.clone();
      let model = settings.model.clone();
      let thinking = settings.thinking;
      drop(settings);

      let game_lock = state.game.lock().unwrap();
      let game_ref = game_lock.as_ref();
      match game_ref {
          None => {
              // 无对局，仅返回设置
              Ok(Some(LoadedState {
                  game: None,
                  settings: SettingsDto { api_key, model, thinking },
              }))
          }
          Some(game) => {
              let status = game_status_str(game);
              let dto = game_to_dto(game, status);
              Ok(Some(LoadedState {
                  game: Some(dto),
                  settings: SettingsDto { api_key, model, thinking },
              }))
          }
      }
  }
  ```
  并在 `game_state.rs` 新增对应 DTO（或直接放 commands.rs）：
  ```rust
  #[derive(Serialize)]
  pub struct SettingsDto {
      pub api_key: String,
      pub model: String,
      pub thinking: bool,
  }
  #[derive(Serialize)]
  pub struct LoadedState {
      pub game: Option<GameStateDto>,
      pub settings: SettingsDto,
  }
  ```
  在 `src-tauri/src/lib.rs` 的 `invoke_handler!` 中注册 `commands::load_state`。
loop: false
verify: cargo check --manifest-path src-tauri/Cargo.toml
max_iterations: 3

- [ ] **Step 6: 在状态变更命令末尾触发保存**
action: |
  修改 `src-tauri/src/commands.rs` 中以下命令，在返回前调用 `persistence::save`：
  1. `start_game`：在 `Ok(dto)` 前插入
     ```rust
     let save = build_save_data(&state, "playing");
     let _ = persistence::save(&app, &save);
     ```
     （需给 `start_game` 加 `app: tauri::AppHandle` 参数）
  2. `player_move`：在 `Ok(MoveResult {...})` 前，用 `game_status_str(game)` 的 status 保存
     （需加 `app: tauri::AppHandle` 参数）
  3. `ai_move`：在两个 return Ok 处和兜底 return Ok 处，分别用对应 status 保存（已有 app 参数）
  4. `undo_move`：在 `Ok(...)` 前保存
     （需加 `app: tauri::AppHandle` 参数）
  5. `reset_game`：在 `Ok(dto)` 前保存（新对局 playing 状态）
     （需加 `app: tauri::AppHandle` 参数）
  注意：`app: tauri::AppHandle` 由 Tauri 自动注入，前端调用无需传参。保存失败仅 `log::warn!` 不阻断返回。
loop: false
verify: cargo check --manifest-path src-tauri/Cargo.toml
max_iterations: 3

- [ ] **Step 7: 前端 api.ts 新增 loadState 调用**
action: |
  在 `src/lib/api.ts` 添加：
  ```typescript
  /// 启动时加载持久化状态（对局 + 设置）
  export async function loadState(): Promise<LoadedState | null> {
    return invoke<LoadedState | null>("load_state");
  }
  ```
  并在 `src/lib/types.ts` 新增对应类型：
  ```typescript
  export interface SettingsDto {
    api_key: string;
    model: string;
    thinking: boolean;
  }
  export interface LoadedState {
    game: GameStateDto | null;
    settings: SettingsDto;
  }
  ```
loop: false
verify: npm run check
max_iterations: 3

- [ ] **Step 8: 前端 App.svelte 启动时调用 loadState 恢复对局与设置**
action: |
  修改 `src/App.svelte`：
  1. import 区添加 `import { loadState } from "./lib/api";` 和 `import { settings as settingsStore } from "./lib/stores/settings";`（如未导入）
  2. 在 `onMount` 内（现有事件监听之前）添加：
     ```typescript
     // 启动加载持久化状态
     try {
       const loaded = await loadState();
       if (loaded) {
         // 恢复设置
         updateSettings({
           apiKey: loaded.settings.api_key,
           model: loaded.settings.model,
           thinking: loaded.settings.thinking,
         });
         // 恢复对局
         if (loaded.game) {
           updateGameState(loaded.game);
           updateSettings({ started: true });
         }
       }
     } catch (e) {
       showError(String(e));
     }
     ```
  3. 注意 onMount 回调改为 async（Svelte 5 支持）
loop: false
verify: npm run check
max_iterations: 3

- [ ] **Step 9: 前端 settings store 同步后端模型（移除前端独立默认值偏差）**
action: |
  检查 `src/lib/stores/settings.ts`，确认 `initialSettings` 与后端 `Settings::default()` 一致（model: "deepseek-v4-flash", thinking: false, apiKey: ""）。若已一致则无需改动；若有偏差则对齐。此步确保 loadState 失败时前端默认值与后端一致。
loop: false
verify: npm run check
max_iterations: 3

- [ ] **Step 10: 全量编译验证**
action: |
  运行后端与前端检查，确认无 error 无 warning。
  - 后端：`cargo check --manifest-path src-tauri/Cargo.toml`
  - 前端：`npm run check`
  若有 warning 需修复（如未使用导入、dead_code）。
loop: until both commands pass with 0 errors and 0 warnings
verify:
  - type: shell
    command: cargo check --manifest-path src-tauri/Cargo.toml
  - type: shell
    command: npm run check
max_iterations: 3

- [ ] **Step 11: 人工验证恢复流程**
action: |
  启动 dev 应用（`npm run tauri dev` 或 `dev.bat`），人工执行以下流程：
  1. 在设置抽屉填入 API Key，选择模型与执方，开始对弈，走若干步（如 1. e4 e5 2. Nf3）
  2. 完全关闭应用窗口
  3. 重新启动应用
  4. 验证：棋盘自动恢复到关闭前的局面，底部状态栏显示正确回合数，API Key 与设置已填回
  5. 点击悔棋，验证可正常撤销
  6. 点击重开，验证新对局开始且存档被覆盖
  7. 关闭应用再开，验证恢复的是重开后的新对局
gate: human
loop: false
verify:
  type: human-review
  check: 重启后对局与设置自动恢复，悔棋/重开后存档正确更新
