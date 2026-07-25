mod chess_engine;
mod commands;
mod deepseek;
mod game_state;
mod move_parser;
mod persistence;
mod prompt;

use game_state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::new())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            if let Some(window) = app.get_webview_window("main") {
                if let Ok(img) = image::open("icons/icon.png") {
                    let rgba = img.to_rgba8();
                    let (w, h) = rgba.dimensions();
                    let icon = tauri::image::Image::new_owned(rgba.into_raw(), w, h);
                    let _ = window.set_icon(icon);
                }
            }

            // 加载持久化存档，恢复设置与对局
            if let Some(save) = persistence::load(app.handle()) {
                let state = app.state::<AppState>();
                // 恢复设置
                {
                    let mut s = state.settings.lock().unwrap();
                    s.api_key = save.settings.api_key.clone();
                    s.model = save.settings.model.clone();
                    s.thinking = save.settings.thinking;
                    s.pseudo_thinking = save.settings.pseudo_thinking;
                    // 旧存档可能无 thinking_language，回退 "zh"
                    s.thinking_language = if save.settings.thinking_language.is_empty() {
                        "zh".to_string()
                    } else {
                        save.settings.thinking_language.clone()
                    };
                    // 旧存档可能无 reasoning_effort，回退 "high"
                    s.reasoning_effort = if save.settings.reasoning_effort == "max" {
                        "max".to_string()
                    } else {
                        "high".to_string()
                    };
                    s.min_thinking_tokens = save.settings.min_thinking_tokens;
                    s.self_consistency_samples = save.settings.self_consistency_samples;
                }
                // 恢复注意事项（跨回合记忆）
                *state.last_notes.lock().unwrap() = save.last_notes.clone();
                // 恢复 DeepSeek 客户端（若 api_key 非空）
                if !save.settings.api_key.is_empty() {
                    let effort = if save.settings.reasoning_effort == "max" {
                        "max".to_string()
                    } else {
                        "high".to_string()
                    };
                    // 伪思考模式时强制关闭 API thinking
                    let real_thinking = if save.settings.pseudo_thinking {
                        false
                    } else {
                        save.settings.thinking
                    };
                    let client = crate::deepseek::DeepSeekClient::new(
                        save.settings.api_key.clone(),
                        save.settings.model.clone(),
                        real_thinking,
                        effort,
                    );
                    *state.deepseek.lock().unwrap() = Some(client);
                }
                // 恢复棋局（若有）
                if save.has_game {
                    let player_side = crate::chess_engine::str_to_color(&save.player_side)
                        .unwrap_or(chess::Color::White);
                    match crate::chess_engine::ChessGame::rebuild_from_history(
                        player_side,
                        save.white_player.clone(),
                        save.black_player.clone(),
                        save.move_history,
                    ) {
                        Ok(game) => {
                            *state.game.lock().unwrap() = Some(game);
                        }
                        Err(e) => {
                            log::warn!("恢复对局失败，将使用空状态: {}", e);
                        }
                    }
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::start_game,
            commands::player_move,
            commands::ai_move,
            commands::get_game_state,
            commands::reset_game,
            commands::undo_move,
            commands::load_state,
            commands::update_settings,
            commands::cancel_deepseek,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
