use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

/// 持久化存档文件名
const SAVE_FILE_NAME: &str = "chess_state.json";

/// thinking_language 缺省值（旧存档无此字段时回退）
fn default_lang() -> String {
    "zh".to_string()
}

/// reasoning_effort 缺省值（旧存档无此字段时回退）
fn default_effort() -> String {
    "high".to_string()
}

/// pseudo_thinking 缺省值（旧存档无此字段时回退）
fn default_false() -> bool {
    false
}

/// 持久化的设置子结构
#[derive(Serialize, Deserialize, Clone)]
pub struct SettingsSave {
    pub api_key: String,
    pub model: String,
    pub thinking: bool,
    #[serde(default = "default_false")]
    pub pseudo_thinking: bool,
    #[serde(default = "default_lang")]
    pub thinking_language: String,
    #[serde(default = "default_effort")]
    pub reasoning_effort: String,
    #[serde(default)]
    pub min_thinking_tokens: u32,
    /// Self-Consistency 多采样次数，旧存档无此字段回退 1（关闭）
    #[serde(default = "default_sc_samples")]
    pub self_consistency_samples: u32,
}

/// self_consistency_samples 缺省值（旧存档无此字段时回退 1=关闭）
fn default_sc_samples() -> u32 {
    1
}

/// white_player 缺省值（旧存档无此字段时回退 "human"）
fn default_white_player() -> String {
    "human".to_string()
}

/// black_player 缺省值（旧存档无此字段时回退 "deepseek"）
fn default_black_player() -> String {
    "deepseek".to_string()
}

/// 持久化存档结构（对局 + 设置）
#[derive(Serialize, Deserialize, Clone)]
pub struct SaveData {
    /// 存档格式版本号：用于未来结构变更时的迁移识别
    /// 旧存档无此字段时 #[serde(default)] 回退 1（当前版本）
    #[serde(default = "default_version")]
    pub version: u32,
    pub has_game: bool,
    pub player_side: String, // "white" | "black"
    pub fen: String,
    pub move_history: Vec<String>,
    pub status: String,
    pub settings: SettingsSave,
    /// 上次 AI 注意事项（跨回合记忆），旧存档无此字段回退空
    #[serde(default)]
    pub last_notes: String,
    /// 白方主体类型："human" | "stockfish" | "deepseek"，旧存档无此字段回退 "human"
    #[serde(default = "default_white_player")]
    pub white_player: String,
    /// 黑方主体类型："human" | "stockfish" | "deepseek"，旧存档无此字段回退 "deepseek"
    #[serde(default = "default_black_player")]
    pub black_player: String,
}

/// 当前存档格式版本号
pub const CURRENT_VERSION: u32 = 1;

/// version 字段缺省值（旧存档无此字段时回退 1）
fn default_version() -> u32 {
    1
}

/// 获取存档文件路径（app_data_dir 下）
fn save_path(app: &AppHandle) -> Option<std::path::PathBuf> {
    match app.path().app_data_dir() {
        Ok(dir) => Some(dir.join(SAVE_FILE_NAME)),
        Err(e) => {
            log::warn!("获取 app_data_dir 失败: {}", e);
            None
        }
    }
}

/// 保存存档到 app_data_dir/chess_state.json（原子写入）
///
/// 原子写入策略：先写入临时文件 .tmp，再 rename 到目标文件。
/// std::fs::rename 在同一文件系统下是原子操作（Windows 用 MoveFileExW + REPLACE_EXISTING），
/// 即使写入过程中崩溃，也只会留下 .tmp 文件，不会损坏目标存档。
pub fn save(app: &AppHandle, data: &SaveData) -> Result<(), String> {
    let path = save_path(app).ok_or_else(|| "无法解析存档路径".to_string())?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建存档目录失败: {}", e))?;
    }
    // 写入当前版本号
    let mut data = data.clone();
    data.version = CURRENT_VERSION;
    let json = serde_json::to_string_pretty(&data).map_err(|e| format!("序列化存档失败: {}", e))?;
    // 原子写入：先写临时文件，再 rename
    let tmp_path = path.with_extension("json.tmp");
    std::fs::write(&tmp_path, &json).map_err(|e| format!("写入临时存档失败: {}", e))?;
    std::fs::rename(&tmp_path, &path).map_err(|e| {
        // rename 失败时尝试清理临时文件
        let _ = std::fs::remove_file(&tmp_path);
        format!("重命名存档失败: {}", e)
    })?;
    Ok(())
}

/// 加载存档；文件不存在或解析失败返回 None
pub fn load(app: &AppHandle) -> Option<SaveData> {
    let path = save_path(app)?;
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return None,
        Err(e) => {
            log::warn!("读取存档失败: {}", e);
            return None;
        }
    };
    match serde_json::from_str::<SaveData>(&content) {
        Ok(mut data) => {
            // 版本号检查：若存档版本高于当前版本，记录警告（向前兼容：仍尝试加载）
            if data.version > CURRENT_VERSION {
                log::warn!(
                    "存档版本 {} 高于当前支持版本 {}，可能存在兼容性问题",
                    data.version,
                    CURRENT_VERSION
                );
            }
            // 旧存档迁移：若 white_player/black_player 为空串（手动写入或异常），
            // 按 player_side 推断默认主体组合（人 vs DeepSeek）
            if data.white_player.is_empty() || data.black_player.is_empty() {
                if data.player_side == "black" {
                    // 玩家执黑：白方DeepSeek vs 黑方人
                    if data.white_player.is_empty() {
                        data.white_player = "deepseek".to_string();
                    }
                    if data.black_player.is_empty() {
                        data.black_player = "human".to_string();
                    }
                } else {
                    // 玩家执白（默认）：白方人 vs 黑方DeepSeek
                    if data.white_player.is_empty() {
                        data.white_player = "human".to_string();
                    }
                    if data.black_player.is_empty() {
                        data.black_player = "deepseek".to_string();
                    }
                }
            }
            Some(data)
        }
        Err(e) => {
            log::warn!("解析存档失败，将使用默认状态: {}", e);
            None
        }
    }
}

/// 清除存档（预留：退出对局时调用）
#[allow(dead_code)]
pub fn clear(app: &AppHandle) -> Result<(), String> {
    let path = save_path(app).ok_or_else(|| "无法解析存档路径".to_string())?;
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("删除存档失败: {}", e)),
    }
}
