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

/// 持久化存档结构（对局 + 设置）
#[derive(Serialize, Deserialize, Clone)]
pub struct SaveData {
    pub has_game: bool,
    pub player_side: String, // "white" | "black"
    pub fen: String,
    pub move_history: Vec<String>,
    pub status: String,
    pub settings: SettingsSave,
    /// 上次 AI 注意事项（跨回合记忆），旧存档无此字段回退空
    #[serde(default)]
    pub last_notes: String,
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

/// 保存存档到 app_data_dir/chess_state.json
pub fn save(app: &AppHandle, data: &SaveData) -> Result<(), String> {
    let path = save_path(app).ok_or_else(|| "无法解析存档路径".to_string())?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建存档目录失败: {}", e))?;
    }
    let json = serde_json::to_string_pretty(data).map_err(|e| format!("序列化存档失败: {}", e))?;
    std::fs::write(&path, json).map_err(|e| format!("写入存档失败: {}", e))?;
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
        Ok(data) => Some(data),
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
