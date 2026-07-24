use crate::chess_engine::{color_opposite, color_to_str, ChessGame};
use crate::deepseek::DeepSeekClient;
use chess::BoardStatus;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

/// 全局应用状态（由 Tauri 管理）
pub struct AppState {
    pub game: Mutex<Option<ChessGame>>,
    pub deepseek: Mutex<Option<DeepSeekClient>>,
    pub settings: Mutex<Settings>,
    /// 上次 AI 走子后输出的注意事项（局面+威胁+战略），下次请求注入 user_message
    pub last_notes: Mutex<String>,
}

#[derive(Clone)]
pub struct Settings {
    pub api_key: String,
    pub model: String,
    /// 是否启用思考模式（DeepSeek API 默认 enabled，返回 reasoning_content 思维链）
    pub thinking: bool,
    /// 伪思考模式：关闭 API thinking，改用提示词在 content 中用 ▾思考▾ 包裹思考输出
    /// 这样提示词完全控制输出格式，省 thinking 模式的输出 token 成本
    pub pseudo_thinking: bool,
    /// 思考语言："zh" | "en"，影响 prompt 语言
    pub thinking_language: String,
    /// reasoning_effort: "high" | "max"（DeepSeek 仅支持这两档）
    pub reasoning_effort: String,
    /// 最少思考 token 数：注入提示词要求 AI 至少输出这么多 token 的思考内容
    /// 0 表示不限制。用于强制 AI 深度思考。
    pub min_thinking_tokens: u32,
    /// Self-Consistency 多采样次数：1=关闭（单次采样），>1=多采样投票
    /// 多采样时第1次正常流式输出思考，后续静默采样，投票选最佳走法
    pub self_consistency_samples: u32,
    pub max_retries: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            model: "deepseek-v4-flash".to_string(),
            thinking: true,
            pseudo_thinking: false,
            thinking_language: "zh".to_string(),
            reasoning_effort: "high".to_string(),
            min_thinking_tokens: 0,
            self_consistency_samples: 1,
            max_retries: 5,
        }
    }
}

impl AppState {
    pub fn new() -> Self {
        Self {
            game: Mutex::new(None),
            deepseek: Mutex::new(None),
            settings: Mutex::new(Settings::default()),
            last_notes: Mutex::new(String::new()),
        }
    }
}

// ========== 前端通信 DTO ==========

#[derive(Serialize)]
pub struct MoveDto {
    pub from: String,
    pub to: String,
    pub promotion: Option<String>,
}

#[derive(Serialize)]
pub struct GameStateDto {
    pub fen: String,
    pub visual: String,
    pub player_side: String,
    pub turn: String,
    pub move_history: Vec<String>,
    pub status: String, // playing | thinking | checkmate | stalemate | draw
    pub winner: Option<String>,
    pub last_move: Option<MoveDto>,
    pub legal_moves: Vec<String>,
    pub in_check: bool,
    pub ply: usize,
}

#[derive(Serialize)]
pub struct MoveResult {
    pub move_str: String,
    pub state: GameStateDto,
    pub game_over: bool,
}

#[derive(Deserialize)]
pub struct StartGameArgs {
    pub side: String,
    pub api_key: String,
    pub model: String,
    #[serde(default)]
    pub thinking: bool,
    /// 伪思考模式：关闭 API thinking 用提示词模拟，缺省 false
    #[serde(default)]
    pub pseudo_thinking: bool,
    /// 思考语言："zh" | "en"，缺省 "zh"
    #[serde(default)]
    pub thinking_language: String,
    /// reasoning_effort: "high" | "max"，缺省 "high"
    #[serde(default)]
    pub reasoning_effort: String,
    /// 最少思考 token 数，缺省 0（不限制）
    #[serde(default)]
    pub min_thinking_tokens: u32,
    /// Self-Consistency 多采样次数，缺省 1（关闭）
    #[serde(default)]
    pub self_consistency_samples: u32,
}

/// 持久化设置 DTO（前端启动加载用）
#[derive(Serialize)]
pub struct SettingsDto {
    pub api_key: String,
    pub model: String,
    pub thinking: bool,
    pub pseudo_thinking: bool,
    pub thinking_language: String,
    pub reasoning_effort: String,
    pub min_thinking_tokens: u32,
    pub self_consistency_samples: u32,
}

/// load_state 命令返回结构
#[derive(Serialize)]
pub struct LoadedState {
    pub game: Option<GameStateDto>,
    pub settings: SettingsDto,
}

/// 从 ChessGame 构建 DTO
pub fn game_to_dto(game: &ChessGame, status: &str) -> GameStateDto {
    let winner = match game.status() {
        BoardStatus::Checkmate => Some(color_to_str(color_opposite(game.side_to_move()))),
        _ => None,
    };
    let last_move = game.move_history.last().map(|mv| {
        let from = crate::chess_engine::square_to_str(mv.get_source());
        let to = crate::chess_engine::square_to_str(mv.get_dest());
        let promotion = mv.get_promotion().map(|p| {
            crate::chess_engine::piece_to_char(p).to_string()
        });
        MoveDto { from, to, promotion }
    });
    GameStateDto {
        fen: game.to_fen(),
        visual: game.to_visual_string(),
        player_side: color_to_str(game.player_side),
        turn: color_to_str(game.side_to_move()),
        move_history: game
            .move_history
            .iter()
            .map(|mv| crate::chess_engine::move_to_coord(mv))
            .collect(),
        status: status.to_string(),
        winner,
        last_move,
        legal_moves: game.legal_moves_str(),
        in_check: game.in_check(),
        ply: game.ply_count(),
    }
}
