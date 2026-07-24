use crate::chess_engine::{color_to_str, move_to_coord, parse_coord_move, str_to_color, ChessGame};
use crate::deepseek::DeepSeekClient;
use crate::game_state::{game_to_dto, AppState, GameStateDto, LoadedState, MoveResult, SettingsDto, StartGameArgs};
use crate::move_parser::{extract_move_and_score, parse_and_validate, ParseError};
use crate::persistence::{SaveData, SettingsSave};
use crate::prompt::build_messages;
use chess::BoardStatus;
use tauri::{Emitter, State};

/// 更新 DeepSeek 设置（API Key / 模型 / 思考模式 / 伪思考 / 思考语言 / 思考强度 / 最少思考token / 自洽采样次数），即时生效，不重开游戏
#[tauri::command]
pub async fn update_settings(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    api_key: String,
    model: String,
    thinking: bool,
    pseudo_thinking: bool,
    thinking_language: String,
    reasoning_effort: String,
    min_thinking_tokens: u32,
    self_consistency_samples: u32,
) -> Result<(), String> {
    // 语言空值兜底为 "zh"
    let language = if thinking_language.is_empty() {
        "zh".to_string()
    } else {
        thinking_language
    };
    // effort 值兜底为 "high"
    let effort = if reasoning_effort == "max" {
        "max".to_string()
    } else {
        "high".to_string()
    };
    // 伪思考模式时强制关闭 API thinking（用提示词模拟思考）
    let real_thinking = if pseudo_thinking { false } else { thinking };
    {
        let mut settings = state.settings.lock().unwrap();
        settings.api_key = api_key.clone();
        settings.model = model.clone();
        settings.thinking = thinking;
        settings.pseudo_thinking = pseudo_thinking;
        settings.thinking_language = language.clone();
        settings.reasoning_effort = effort.clone();
        settings.min_thinking_tokens = min_thinking_tokens;
        settings.self_consistency_samples = self_consistency_samples;
    }
    // 重新创建 DeepSeek 客户端（传 real_thinking 而非 thinking）
    let client = DeepSeekClient::new(api_key, model, real_thinking, effort);
    *state.deepseek.lock().unwrap() = Some(client);
    // 持久化
    let save = build_save_data(&state, "playing");
    if let Err(e) = crate::persistence::save(&app, &save) {
        log::warn!("保存设置失败: {}", e);
    }
    Ok(())
}

/// 开始新游戏
#[tauri::command]
pub async fn start_game(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    args: StartGameArgs,
) -> Result<GameStateDto, String> {
    let side = str_to_color(&args.side).ok_or("无效的执方")?;
    let game = ChessGame::new(side);
    // 语言空值兜底为 "zh"
    let language = if args.thinking_language.is_empty() {
        "zh".to_string()
    } else {
        args.thinking_language
    };
    // effort 空值兜底为 "high"
    let effort = if args.reasoning_effort == "max" {
        "max".to_string()
    } else {
        "high".to_string()
    };
    // 伪思考模式时强制关闭 API thinking（用提示词模拟思考）
    let real_thinking = if args.pseudo_thinking { false } else { args.thinking };
    let client = DeepSeekClient::new(args.api_key.clone(), args.model.clone(), real_thinking, effort.clone());

    {
        let mut settings = state.settings.lock().unwrap();
        settings.api_key = args.api_key;
        settings.model = args.model;
        settings.thinking = args.thinking;
        settings.pseudo_thinking = args.pseudo_thinking;
        settings.thinking_language = language;
        settings.reasoning_effort = effort;
        settings.min_thinking_tokens = args.min_thinking_tokens;
        settings.self_consistency_samples = args.self_consistency_samples;
    }
    {
        let mut ds = state.deepseek.lock().unwrap();
        *ds = Some(client);
    }
    // 新游戏清空注意事项（跨回合记忆）
    *state.last_notes.lock().unwrap() = String::new();
    let dto;
    {
        let mut g = state.game.lock().unwrap();
        *g = Some(game);
        dto = game_to_dto(g.as_ref().unwrap(), "playing");
    }
    let save = build_save_data(&state, "playing");
    if let Err(e) = crate::persistence::save(&app, &save) {
        log::warn!("保存存档失败: {}", e);
    }
    Ok(dto)
}

/// 玩家走棋
#[tauri::command]
pub async fn player_move(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    move_str: String,
) -> Result<MoveResult, String> {
    let mv = parse_coord_move(&move_str).ok_or("走法格式错误")?;

    let mut game_lock = state.game.lock().unwrap();
    let game = game_lock.as_mut().ok_or("游戏未开始")?;

    if !game.is_legal(&mv) {
        return Err("非法走法".to_string());
    }

    let coord = move_to_coord(&mv);
    game.make_move(mv).map_err(|e| e)?;

    let status = game_status_str(game);
    let game_over = is_game_over(game);
    let dto = game_to_dto(game, &status);
    drop(game_lock);

    let save = build_save_data(&state, &status);
    if let Err(e) = crate::persistence::save(&app, &save) {
        log::warn!("保存存档失败: {}", e);
    }

    Ok(MoveResult {
        move_str: coord,
        state: dto,
        game_over,
    })
}

/// DeepSeek AI 走棋（流式输出思考内容 + 非法走法重试 + Self-Consistency 多采样投票）
///
/// 流程：
/// 1. 如果 self_consistency_samples > 1（多采样模式）：
///    a. 第1次采样：正常流式输出思考（用户能看到思考过程）
///    b. 后续 N-1 次采样：并行静默采样（temperature=0.7 增加多样性）
///    c. 收集所有采样的走法+评分，投票选最佳走法
///    d. 如果投票冠军合法：应用并返回；非法：回退到 VAM 重试
/// 2. VAM 迭代裁剪重采样：仅对"走法解析失败"重试；API 错误直接返回
/// 3. 兜底：取第一个合法走法
#[tauri::command]
pub async fn ai_move(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<MoveResult, String> {
    // 1. 获取配置（合法走法列表用于举棋兜底；messages 在循环内重建以支持 VAM 迭代裁剪）
    let (max_retries, legal_moves, ai_side, lang, notes, pseudo, min_tokens, sc_samples) = {
        let game_lock = state.game.lock().unwrap();
        let game = game_lock.as_ref().ok_or("游戏未开始")?;
        let settings = state.settings.lock().unwrap();
        let ai_side = color_to_str(game.side_to_move());
        let lang = settings.thinking_language.clone();
        let notes = state.last_notes.lock().unwrap().clone();
        let pseudo = settings.pseudo_thinking;
        let min_tokens = settings.min_thinking_tokens;
        let sc_samples = settings.self_consistency_samples.max(1);
        let legal = game.legal_moves_str();
        (settings.max_retries, legal, ai_side, lang, notes, pseudo, min_tokens, sc_samples)
    };

    // 2. 获取 DeepSeek 客户端（克隆后释放锁）
    let client = {
        let ds_lock = state.deepseek.lock().unwrap();
        ds_lock.as_ref().ok_or("DeepSeek 未配置，请先设置 API Key")?.clone()
    };

    // 3. Self-Consistency 多采样（sc_samples > 1 时生效）
    //    第1次正常流式输出思考，后续并行静默采样，投票选最佳走法
    if sc_samples > 1 {
        // 3a. 第1次采样：正常流式输出思考（用户能看到思考过程）
        let messages = {
            let game_lock = state.game.lock().unwrap();
            let game = game_lock.as_ref().ok_or("游戏未开始")?;
            build_messages(game, &ai_side, &lang, &notes, &[], pseudo, min_tokens)
        };
        let (content1, reasoning1, usage1) = client
            .chat_stream(messages, &app, legal_moves.clone(), pseudo, false, None)
            .await?;

        // 3b. 后续 N-1 次并行静默采样（temperature=0.7 增加多样性）
        let mut messages_list = Vec::new();
        for _ in 1..sc_samples {
            let messages = {
                let game_lock = state.game.lock().unwrap();
                let game = game_lock.as_ref().ok_or("游戏未开始")?;
                build_messages(game, &ai_side, &lang, &notes, &[], pseudo, min_tokens)
            };
            messages_list.push(messages);
        }

        let mut handles = Vec::new();
        for messages in messages_list {
            let client_c = client.clone();
            let app_c = app.clone();
            let legal_c = legal_moves.clone();
            handles.push(tokio::spawn(async move {
                client_c
                    .chat_stream(messages, &app_c, legal_c, pseudo, true, Some(0.7))
                    .await
            }));
        }

        // 3c. 收集所有采样的走法+评分
        let mut samples = vec![extract_move_and_score(&content1, &reasoning1)];
        for handle in handles {
            if let Ok(Ok((c, r, _))) = handle.await {
                samples.push(extract_move_and_score(&c, &r));
            }
        }

        // 3d. 投票选最佳走法
        if let Some(best_uci) = vote_best_move(&samples) {
            log::info!("[sc] 多采样投票冠军: {} (共 {} 个采样)", best_uci, samples.len());
            // 验证投票冠军合法性
            let mut game_lock = state.game.lock().unwrap();
            let game = game_lock.as_mut().ok_or("游戏未开始")?;
            if let Some(mv) = parse_coord_move(&best_uci) {
                if game.is_legal(&mv) {
                    // 投票冠军合法，应用走法
                    let coord = move_to_coord(&mv);
                    game.make_move(mv).map_err(|e| e)?;
                    let status = game_status_str(game);
                    let game_over = is_game_over(game);
                    let dto = game_to_dto(game, &status);
                    drop(game_lock);
                    // emit pick 更新为投票冠军（确保落子动画一致）
                    let _ = app.emit("ai-pick", &best_uci);
                    // 提取本步总结（用第1次采样的 content/reasoning）
                    let notes = crate::deepseek::extract_notes(&content1, &reasoning1);
                    if !notes.is_empty() {
                        *state.last_notes.lock().unwrap() = notes;
                    }
                    let save = build_save_data(&state, &status);
                    if let Err(e) = crate::persistence::save(&app, &save) {
                        log::warn!("保存存档失败: {}", e);
                    }
                    let _ = app.emit("ai-usage", &usage1);
                    return Ok(MoveResult {
                        move_str: coord,
                        state: dto,
                        game_over,
                    });
                }
            }
            drop(game_lock);
            log::warn!("[sc] 投票冠军 {} 非法，回退到 VAM 重试", best_uci);
        }

        // 3e. 投票冠军非法或无走法，用第1次采样结果走 VAM 重试
        // 先解析第1次采样的走法
        let mut failed_moves: Vec<String> = Vec::new();
        {
            let mut game_lock = state.game.lock().unwrap();
            let game = game_lock.as_mut().ok_or("游戏未开始")?;
            let parse_result = if !content1.trim().is_empty() {
                parse_and_validate(&content1, game)
            } else {
                parse_and_validate(&reasoning1, game)
            };
            match &parse_result {
                Ok(mv) => {
                    let coord = move_to_coord(mv);
                    game.make_move(mv.clone()).map_err(|e| e)?;
                    let status = game_status_str(game);
                    let game_over = is_game_over(game);
                    let dto = game_to_dto(game, &status);
                    drop(game_lock);
                    let notes = crate::deepseek::extract_notes(&content1, &reasoning1);
                    if !notes.is_empty() {
                        *state.last_notes.lock().unwrap() = notes;
                    }
                    let save = build_save_data(&state, &status);
                    if let Err(e) = crate::persistence::save(&app, &save) {
                        log::warn!("保存存档失败: {}", e);
                    }
                    let _ = app.emit("ai-usage", &usage1);
                    return Ok(MoveResult {
                        move_str: coord,
                        state: dto,
                        game_over,
                    });
                }
                Err(ParseError::IllegalMove(bad, _)) => {
                    failed_moves.push(bad.clone());
                }
                Err(ParseError::InvalidFormat(_)) => {}
            }
            drop(game_lock);
        }

        // 3f. 第1次采样也非法，继续 VAM 重试（从第2次开始）
        let _ = app.emit("ai-thinking-reset", ());
        let _ = app.emit("ai-pick-reset", ());
        return ai_move_vam_retry(
            &state, &app, &client, &legal_moves, &ai_side, &lang, &notes, pseudo, min_tokens,
            max_retries, &failed_moves,
        )
        .await;
    }

    // 4. 单采样模式：直接走 VAM 重试循环
    ai_move_vam_retry(
        &state, &app, &client, &legal_moves, &ai_side, &lang, &notes, pseudo, min_tokens,
        max_retries, &[],
    )
    .await
}

/// VAM 迭代裁剪重采样循环
///
/// 每次重试重建 messages（system+user 两条），不累积历史，省输入 token。
/// 失败走法加入 failed_moves，下轮从合法走法列表中移除，缩窄动作空间提高命中率。
async fn ai_move_vam_retry(
    state: &State<'_, AppState>,
    app: &tauri::AppHandle,
    client: &DeepSeekClient,
    legal_moves: &[String],
    ai_side: &str,
    lang: &str,
    notes: &str,
    pseudo: bool,
    min_tokens: u32,
    max_retries: u32,
    initial_failed: &[String],
) -> Result<MoveResult, String> {
    let mut last_error = String::new();
    let mut failed_moves: Vec<String> = initial_failed.to_vec();

    for _attempt in 0..max_retries {
        // 重试时通知前端清空上一次思考和 pick
        if _attempt > 0 || !failed_moves.is_empty() {
            let _ = app.emit("ai-thinking-reset", ());
            let _ = app.emit("ai-pick-reset", ());
        }
        // 重建 messages（排除失败走法），消息始终为 system+user 两条，不增长
        let messages = {
            let game_lock = state.game.lock().unwrap();
            let game = game_lock.as_ref().ok_or("游戏未开始")?;
            build_messages(game, ai_side, lang, notes, &failed_moves, pseudo, min_tokens)
        };
        let (content, reasoning, usage) = client
            .chat_stream(messages, app, legal_moves.to_vec(), pseudo, false, None)
            .await?;

        // 锁定棋局，解析验证并尝试应用
        let mut game_lock = state.game.lock().unwrap();
        let game = game_lock.as_mut().ok_or("游戏未开始")?;

        let parse_result = if !content.trim().is_empty() {
            parse_and_validate(&content, game)
        } else {
            parse_and_validate(&reasoning, game)
        };

        match parse_result {
            Ok(mv) => {
                let coord = move_to_coord(&mv);
                game.make_move(mv).map_err(|e| e)?;
                let status = game_status_str(game);
                let game_over = is_game_over(game);
                let dto = game_to_dto(game, &status);
                drop(game_lock);
                // 提取本步总结并存到 AppState（跨回合记忆）
                let notes = crate::deepseek::extract_notes(&content, &reasoning);
                if !notes.is_empty() {
                    *state.last_notes.lock().unwrap() = notes;
                }
                let save = build_save_data(state, &status);
                if let Err(e) = crate::persistence::save(app, &save) {
                    log::warn!("保存存档失败: {}", e);
                }
                // 发送本轮 token 用量到前端（底部状态栏显示）
                let _ = app.emit("ai-usage", &usage);
                return Ok(MoveResult {
                    move_str: coord,
                    state: dto,
                    game_over,
                });
            }
            Err(ParseError::IllegalMove(bad, _legal)) => {
                // VAM：记录失败走法，下轮从合法列表移除，缩窄动作空间
                failed_moves.push(bad.clone());
                last_error = format!("非法走法 {}", bad);
            }
            Err(ParseError::InvalidFormat(s)) => {
                last_error = format!("无法解析: {}", s);
            }
        }
        drop(game_lock);
    }

    // 兜底：取第一个合法走法
    let mut game_lock = state.game.lock().unwrap();
    let game = game_lock.as_mut().ok_or("游戏未开始")?;
    let legal = game.legal_moves();
    if legal.is_empty() {
        return Err(format!(
            "DeepSeek 连续 {} 次返回非法走法且无合法走法: {}",
            max_retries, last_error
        ));
    }
    let fallback = legal[0];
    let coord = move_to_coord(&fallback);
    log::warn!("AI 走棋失败，使用兜底走法: {}", coord);
    game.make_move(fallback).map_err(|e| e)?;
    let status = game_status_str(game);
    let game_over = is_game_over(game);
    let dto = game_to_dto(game, &status);
    drop(game_lock);
    let save = build_save_data(state, &status);
    if let Err(e) = crate::persistence::save(app, &save) {
        log::warn!("保存存档失败: {}", e);
    }
    Ok(MoveResult {
        move_str: coord,
        state: dto,
        game_over,
    })
}

/// Self-Consistency 投票：从多个采样的 (走法, 评分) 中选最佳走法
///
/// 投票规则：每个走法的得分 = 出现次数 × 100 + 平均评分
/// 出现次数越多、评分越高的走法越可能被选中。
fn vote_best_move(samples: &[(String, i32)]) -> Option<String> {
    use std::collections::HashMap;
    let mut stats: HashMap<&str, (u32, i64)> = HashMap::new(); // (count, sum_score)
    for (mv, score) in samples {
        if mv.is_empty() {
            continue;
        }
        let entry = stats.entry(mv.as_str()).or_insert((0, 0));
        entry.0 += 1;
        entry.1 += *score as i64;
    }
    if stats.is_empty() {
        return None;
    }
    // 选"出现次数 × 100 + 平均评分"最高的
    stats
        .into_iter()
        .max_by_key(|(_, (count, sum))| {
            let avg = if *count > 0 { sum / *count as i64 } else { 0 };
            *count as i64 * 100 + avg
        })
        .map(|(mv, _)| mv.to_string())
}

/// 获取当前游戏状态
#[tauri::command]
pub async fn get_game_state(state: State<'_, AppState>) -> Result<GameStateDto, String> {
    let game_lock = state.game.lock().unwrap();
    let game = game_lock.as_ref().ok_or("游戏未开始")?;
    Ok(game_to_dto(game, &game_status_str(game)))
}

/// 重置游戏（保留 DeepSeek 配置）
#[tauri::command]
pub async fn reset_game(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    side: String,
) -> Result<GameStateDto, String> {
    let player_side = str_to_color(&side).ok_or("无效的执方")?;
    let game = ChessGame::new(player_side);
    let mut game_lock = state.game.lock().unwrap();
    *game_lock = Some(game);
    // 重开清空注意事项（跨回合记忆）
    *state.last_notes.lock().unwrap() = String::new();
    let dto = game_to_dto(game_lock.as_ref().unwrap(), "playing");
    drop(game_lock);
    let save = build_save_data(&state, "playing");
    if let Err(e) = crate::persistence::save(&app, &save) {
        log::warn!("保存存档失败: {}", e);
    }
    Ok(dto)
}

/// 悔棋（撤销最近一步）
#[tauri::command]
pub async fn undo_move(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<GameStateDto, String> {
    let mut game_lock = state.game.lock().unwrap();
    let game = game_lock.as_mut().ok_or("游戏未开始")?;
    if !game.undo() {
        return Err("没有可撤销的走法".to_string());
    }
    let status = game_status_str(game);
    let dto = game_to_dto(game, &status);
    drop(game_lock);
    let save = build_save_data(&state, &status);
    if let Err(e) = crate::persistence::save(&app, &save) {
        log::warn!("保存存档失败: {}", e);
    }
    Ok(dto)
}

// ========== 辅助函数 ==========

fn game_status_str(game: &ChessGame) -> &'static str {
    match game.status() {
        BoardStatus::Ongoing => "playing",
        BoardStatus::Checkmate => "checkmate",
        BoardStatus::Stalemate => "stalemate",
    }
}

fn is_game_over(game: &ChessGame) -> bool {
    !matches!(game.status(), BoardStatus::Ongoing)
}

/// 从当前 AppState 构造存档数据（无对局时 has_game=false）
fn build_save_data(state: &AppState, status: &str) -> SaveData {
    let settings = state.settings.lock().unwrap();
    let settings_save = SettingsSave {
        api_key: settings.api_key.clone(),
        model: settings.model.clone(),
        thinking: settings.thinking,
        pseudo_thinking: settings.pseudo_thinking,
        thinking_language: settings.thinking_language.clone(),
        reasoning_effort: settings.reasoning_effort.clone(),
        min_thinking_tokens: settings.min_thinking_tokens,
        self_consistency_samples: settings.self_consistency_samples,
    };
    let last_notes = state.last_notes.lock().unwrap().clone();
    let game_lock = state.game.lock().unwrap();
    match game_lock.as_ref() {
        Some(game) => SaveData {
            has_game: true,
            player_side: color_to_str(game.player_side),
            fen: game.to_fen(),
            move_history: game
                .move_history
                .iter()
                .map(|mv| move_to_coord(mv))
                .collect(),
            status: status.to_string(),
            settings: settings_save,
            last_notes,
        },
        None => SaveData {
            has_game: false,
            player_side: "white".to_string(),
            fen: String::new(),
            move_history: vec![],
            status: String::new(),
            settings: settings_save,
            last_notes,
        },
    }
}

/// 加载持久化状态（前端启动时调用，返回当前棋局 DTO + 设置）
#[tauri::command]
pub async fn load_state(
    state: State<'_, AppState>,
) -> Result<Option<LoadedState>, String> {
    let settings = state.settings.lock().unwrap();
    let api_key = settings.api_key.clone();
    let model = settings.model.clone();
    let thinking = settings.thinking;
    let pseudo_thinking = settings.pseudo_thinking;
    let thinking_language = settings.thinking_language.clone();
    let reasoning_effort = settings.reasoning_effort.clone();
    let min_thinking_tokens = settings.min_thinking_tokens;
    let self_consistency_samples = settings.self_consistency_samples;
    drop(settings);

    let game_lock = state.game.lock().unwrap();
    match game_lock.as_ref() {
        None => Ok(Some(LoadedState {
            game: None,
            settings: SettingsDto { api_key, model, thinking, pseudo_thinking, thinking_language, reasoning_effort, min_thinking_tokens, self_consistency_samples },
        })),
        Some(game) => {
            let status = game_status_str(game);
            let dto = game_to_dto(game, status);
            Ok(Some(LoadedState {
                game: Some(dto),
                settings: SettingsDto { api_key, model, thinking, pseudo_thinking, thinking_language, reasoning_effort, min_thinking_tokens, self_consistency_samples },
            }))
        }
    }
}
