use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

// DeepSeek API 地址（不带 /v1，与官方文档 curl 示例一致）
const DEEPSEEK_API_URL: &str = "https://api.deepseek.com/chat/completions";

/// 流式请求体
///
/// 参照 Kitode 实现：
/// - 不传 max_tokens：thinking 模式下传会限制 reasoning + content 总长导致截断；
///   非 thinking 模式让模型用上限（deepseek-chat 8192）
/// - 不传 temperature：让模型用默认值
/// - thinking 放 body 顶层（extra_body 仅 OpenAI Python SDK 展开，直连 HTTP 无效）
/// - reasoning_effort 仅在 thinking 开启且非 auto 时传
/// - stream_options.include_usage: 流式响应最后一个 chunk 返回 usage 字段
/// - max_tokens: 伪思考模式必须设置较大值（8192），否则 content 输出会被截断
///   真实思考模式不传（让 thinking + content 共享上限，避免思考被截断）
#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
    thinking: ThinkingConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning_effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
    stream_options: StreamOptions,
}

#[derive(Serialize)]
struct ThinkingConfig {
    #[serde(rename = "type")]
    kind: String,
}

#[derive(Serialize)]
struct StreamOptions {
    include_usage: bool,
}

/// API 返回的 token 用量统计
///
/// DeepSeek usage 字段：
/// - prompt_tokens: 输入 token 总数（含缓存命中）
/// - completion_tokens: 输出 token 数
/// - prompt_cache_hit_tokens: 缓存命中的输入 token 数
/// - prompt_cache_miss_tokens: 缓存未命中的输入 token 数
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Usage {
    #[serde(default)]
    pub prompt_tokens: u64,
    #[serde(default)]
    pub completion_tokens: u64,
    #[serde(default)]
    pub total_tokens: u64,
    #[serde(default)]
    pub prompt_cache_hit_tokens: u64,
    #[serde(default)]
    pub prompt_cache_miss_tokens: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

// 流式响应 chunk 结构
//
// 关键：流式响应中 content / reasoning_content 可能为 null（思考阶段 content=null，
// 回答阶段 reasoning_content=null）。用 String + #[serde(default)] 接收会在 null 时
// 反序列化失败（default 仅在字段缺失时生效，null 仍会报错），导致整个 chunk 被丢弃，
// 最终 content 和 reasoning 全为空。必须用 Option<String> 接收 null。
//
// 最后一个 chunk 可能 choices 为空数组且包含 usage 字段（stream_options.include_usage=true）
#[derive(Deserialize)]
struct StreamChunk {
    #[serde(default)]
    choices: Vec<StreamChoice>,
    #[serde(default)]
    usage: Option<Usage>,
}

#[derive(Deserialize)]
struct StreamChoice {
    delta: StreamDelta,
}

#[derive(Deserialize)]
struct StreamDelta {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    reasoning_content: Option<String>,
}

#[derive(Clone)]
pub struct DeepSeekClient {
    client: Client,
    api_key: String,
    model: String,
    thinking: bool,
    /// reasoning_effort: "high" | "max"（DeepSeek 仅支持这两档），仅 thinking 开启时生效
    reasoning_effort: String,
}

impl DeepSeekClient {
    pub fn new(api_key: String, model: String, thinking: bool, reasoning_effort: String) -> Self {
        // effort 值兜底：空或非法值回退 "high"
        let effort = if reasoning_effort == "max" {
            "max".to_string()
        } else {
            "high".to_string()
        };
        Self {
            client: Client::new(),
            api_key,
            model,
            thinking,
            reasoning_effort: effort,
        }
    }

    /// 构建请求体（参照 Kitode 实现），stream 控制是否流式
    /// `pseudo` 控制是否伪思考模式：true 时设置较大 max_tokens 确保 content 输出不截断
    /// `temperature` 控制采样温度：None=默认，Some(0.7)=多采样时增加多样性
    fn build_request(&self, messages: Vec<ChatMessage>, stream: bool, pseudo: bool, temperature: Option<f64>) -> ChatRequest {
        ChatRequest {
            model: self.model.clone(),
            messages,
            stream,
            thinking: ThinkingConfig {
                // DeepSeek 默认 enabled，关闭时必须显式发 disabled，否则仍会思考
                kind: if self.thinking { "enabled" } else { "disabled" }.to_string(),
            },
            // reasoning_effort 仅在 thinking 开启时传，使用用户选择的强度
            reasoning_effort: if self.thinking {
                Some(self.reasoning_effort.clone())
            } else {
                None
            },
            // max_tokens：
            // - 伪思考模式：必须设置较大值（8192），否则 content 中的 <LMTHINK> 思考会被截断
            // - 真实思考模式：不传（让 thinking + content 共享上限，避免思考被截断）
            max_tokens: if pseudo { Some(8192) } else { None },
            // temperature：多采样时设置 0.7 增加多样性，单次采样时不传（用默认值）
            temperature,
            // 流式时请求返回 usage（最后一个 chunk 包含 token 用量）
            stream_options: StreamOptions { include_usage: stream },
        }
    }

    /// 流式对话：边接收边推送思考内容到前端，返回 (content, reasoning_content, usage)
    ///
    /// 思考模式下，模型可能把所有内容（含走法）都放进 reasoning_content，
    /// 而 content 为空。因此需同时返回两者，由调用方决定从哪个字段解析走法。
    ///
    /// `legal_moves` 用于举棋兜底：当 AI 未输出 `<pick>` 标签时，扫描思考文本中
    /// 最后出现的合法 UCI 走法作为当前举棋，实时 emit `ai-pick` 事件驱动举棋动画。
    ///
    /// 返回的 `Usage` 包含 token 用量（prompt/completion/cache_hit），
    /// 由调用方通过 `ai-usage` 事件推送到前端显示。
    pub async fn chat_stream(
        &self,
        messages: Vec<ChatMessage>,
        app: &AppHandle,
        legal_moves: Vec<String>,
        pseudo: bool,
        silent: bool,
        temperature: Option<f64>,
    ) -> Result<(String, String, Usage), String> {
        let req = self.build_request(messages, true, pseudo, temperature);

        let resp = self
            .client
            .post(DEEPSEEK_API_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&req)
            .send()
            .await
            .map_err(|e| format!("请求失败: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("API 错误 {}: {}", status, body));
        }

        // 流式解析 SSE
        let mut full_content = String::new();
        let mut full_reasoning = String::new();
        let mut usage = Usage::default();
        let mut stream = resp.bytes_stream();
        let mut buffer = String::new();
        // 记录上次已推送的 pick，避免重复 emit
        let mut last_pick: Option<String> = None;

        // 伪思考模式状态：跨 chunk 解析 <LMTHINK>..</LMTHINK> 标签
        // in_lmthink=true 时，content 中的文本是伪思考内容，推送为 ai-thinking
        // pseudo_buffer 缓冲未处理的 content（可能含不完整标签）
        let mut in_lmthink = false;
        let mut pseudo_buffer = String::new();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| format!("读取流失败: {}", e))?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            // 按行处理 SSE
            while let Some(newline_pos) = buffer.find('\n') {
                let line: String = buffer.drain(..=newline_pos).collect();
                let line = line.trim().to_string();
                if line.is_empty() {
                    continue;
                }

                // SSE 行以 "data: " 或 "data:" 开头
                // 兼容 "data:foo"（无空格）前缀：strip 后去掉可能的一个前导空格
                let data = if let Some(stripped) = line.strip_prefix("data: ") {
                    stripped
                } else if let Some(stripped) = line.strip_prefix("data:") {
                    stripped.trim_start()
                } else {
                    continue;
                };

                // 流结束标志
                if data == "[DONE]" {
                    return Ok((full_content, full_reasoning, usage));
                }

                // 解析 JSON chunk
                if let Ok(parsed) = serde_json::from_str::<StreamChunk>(data) {
                    // 捕获 usage（最后一个 chunk 包含）
                    if let Some(u) = parsed.usage {
                        usage = u;
                    }
                    if let Some(choice) = parsed.choices.into_iter().next() {
                        let delta = choice.delta;
                        // null → 空字符串，统一处理
                        let c = delta.content.unwrap_or_default();
                        let r = delta.reasoning_content.unwrap_or_default();

                        // 推送思考内容增量到前端（静默采样时不推送）
                        let mut reasoning_changed = false;
                        if !r.is_empty() {
                            full_reasoning.push_str(&r);
                            if !silent {
                                if let Err(e) = app.emit("ai-thinking", &r) {
                                    log::warn!("emit ai-thinking 失败: {}", e);
                                }
                            }
                            reasoning_changed = true;
                        }

                        // 累积最终 content
                        let mut content_changed = false;
                        if !c.is_empty() {
                            full_content.push_str(&c);
                            content_changed = true;

                            // 伪思考模式：从 content 中解析 <LMTHINK>..</LMTHINK> 标签
                            // 标签内的内容作为思考增量推送前端 + 累积到 full_reasoning
                            if pseudo {
                                pseudo_buffer.push_str(&c);
                                let mut pseudo_reasoning_delta = String::new();
                                loop {
                                    if in_lmthink {
                                        // 在 LMTHINK 内，查找结束标签 </LMTHINK>
                                        if let Some(pos) = pseudo_buffer.find("</LMTHINK>") {
                                            pseudo_reasoning_delta.push_str(&pseudo_buffer[..pos]);
                                            pseudo_buffer = pseudo_buffer[pos + 10..].to_string();
                                            in_lmthink = false;
                                        } else {
                                            // 没找到结束标签，安全分割（保留可能是标签前缀的末尾）
                                            let safe_end = find_safe_split(&pseudo_buffer, "</LMTHINK>");
                                            if safe_end > 0 {
                                                pseudo_reasoning_delta.push_str(&pseudo_buffer[..safe_end]);
                                                pseudo_buffer = pseudo_buffer[safe_end..].to_string();
                                            }
                                            break;
                                        }
                                    } else {
                                        // 在 LMTHINK 外，查找开始标签 <LMTHINK>
                                        if let Some(pos) = pseudo_buffer.find("<LMTHINK>") {
                                            // 标签前的内容不是思考，丢弃
                                            pseudo_buffer = pseudo_buffer[pos + 9..].to_string();
                                            in_lmthink = true;
                                        } else {
                                            // 没找到开始标签，安全分割（保留可能是标签前缀的末尾）
                                            let safe_end = find_safe_split(&pseudo_buffer, "<LMTHINK>");
                                            pseudo_buffer = pseudo_buffer[safe_end..].to_string();
                                            break;
                                        }
                                    }
                                }
                                // 推送伪思考增量（静默采样时不推送）
                                if !pseudo_reasoning_delta.is_empty() {
                                    full_reasoning.push_str(&pseudo_reasoning_delta);
                                    if !silent {
                                        if let Err(e) = app.emit("ai-thinking", &pseudo_reasoning_delta) {
                                            log::warn!("emit ai-thinking 失败: {}", e);
                                        }
                                    }
                                    reasoning_changed = true;
                                }
                            }
                        }

                        // 举棋检测：reasoning 或 content 变化时尝试提取当前举棋
                        // 优先级：闭合 <move> > @UCI 标记 > 未闭合 <move> 提前检测 > 合法走法兜底扫描
                        // 闭合 <move> 优先级最高：确保落子前 pick 与最终走法一致
                        if reasoning_changed || content_changed {
                            let new_pick = extract_closed_move_tag(&full_content, &legal_moves)
                                .or_else(|| extract_closed_move_tag(&full_reasoning, &legal_moves))
                                .or_else(|| extract_at_pick(&full_reasoning))
                                .or_else(|| extract_at_pick(&full_content))
                                .or_else(|| extract_open_move_tag(&full_reasoning, &legal_moves))
                                .or_else(|| extract_open_move_tag(&full_content, &legal_moves))
                                .or_else(|| {
                                    let text = if !full_reasoning.is_empty() {
                                        &full_reasoning
                                    } else {
                                        &full_content
                                    };
                                    extract_last_legal_move(text, &legal_moves)
                                });
                            if let Some(ref pick) = new_pick {
                                if last_pick.as_deref() != Some(pick.as_str()) {
                                    last_pick = Some(pick.clone());
                                    log::info!("[pick] {}", pick);
                                    if !silent {
                                        if let Err(e) = app.emit("ai-pick", pick) {
                                            log::warn!("emit ai-pick 失败: {}", e);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok((full_content, full_reasoning, usage))
    }
}

/// 安全分割位置：返回 pos，使 buffer[..pos] 不包含 tag 的任何前缀
/// buffer[pos..] 可能是 tag 的前缀（需保留到下次处理，避免漏判跨 chunk 标签）
///
/// 用于伪思考模式流式解析：当 buffer 末尾可能是 `<LMTHINK>` 或 `</LMTHINK>` 的
/// 不完整前缀时，只处理安全部分，剩余保留到下个 chunk 拼接后再判断。
fn find_safe_split(buffer: &str, tag: &str) -> usize {
    let buf_bytes = buffer.as_bytes();
    let tag_bytes = tag.as_bytes();
    // 最多检查 tag.len()-1 长度的前缀（完整标签会被 find 直接命中）
    let max_overlap = std::cmp::min(buffer.len(), tag.len() - 1);
    for overlap in (1..=max_overlap).rev() {
        // 比较 buffer 末尾 overlap 字节与 tag 前 overlap 字节
        if &buf_bytes[buffer.len() - overlap..] == &tag_bytes[..overlap] {
            return buffer.len() - overlap;
        }
    }
    buffer.len()
}

/// 从文本中提取最后一个 `@UCI` 形式的举棋标记
///
/// 用于"举棋"动画：模型在思考过程中可能多次输出 @UCI，取最新一个表示当前考虑的走法。
/// UCI 走法格式：4 字符（如 e2e4）或 5 字符升变（如 e7e8q）。
/// 匹配规则：`@` 后紧跟 4-5 字符 UCI，边界为非字母数字字符（或字符串首尾）。
fn extract_at_pick(text: &str) -> Option<String> {
    let lower = text.to_lowercase();
    let bytes = lower.as_bytes();
    let mut best: Option<String> = None;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'@' {
            // 尝试提取 @ 后 4 或 5 字符 UCI
            let remaining = &lower[i + 1..];
            // 5 字符升变：[a-h][1-8][a-h][1-8][qrbn]
            if remaining.len() >= 5 {
                let c5: Vec<char> = remaining.chars().take(5).collect();
                if is_uci_char(c5[0]) && is_uci_digit(c5[1])
                    && is_uci_char(c5[2]) && is_uci_digit(c5[3])
                    && matches!(c5[4], 'q' | 'r' | 'b' | 'n')
                {
                    // 边界检查：第 6 个字符（如有）不能是字母数字（避免 @e2e4abc 误匹配）
                    let after = remaining.chars().nth(5);
                    if after.map_or(true, |c| !c.is_alphanumeric()) {
                        let mv: String = c5.iter().collect();
                        best = Some(mv);
                        i += 6;
                        continue;
                    }
                }
            }
            // 4 字符：[a-h][1-8][a-h][1-8]
            if remaining.len() >= 4 {
                let c4: Vec<char> = remaining.chars().take(4).collect();
                if is_uci_char(c4[0]) && is_uci_digit(c4[1])
                    && is_uci_char(c4[2]) && is_uci_digit(c4[3])
                {
                    let after = remaining.chars().nth(4);
                    if after.map_or(true, |c| !c.is_alphanumeric()) {
                        let mv: String = c4.iter().collect();
                        best = Some(mv);
                        i += 5;
                        continue;
                    }
                }
            }
        }
        i += 1;
    }
    best
}

#[inline]
fn is_uci_char(c: char) -> bool {
    matches!(c, 'a' | 'b' | 'c' | 'd' | 'e' | 'f' | 'g' | 'h')
}

#[inline]
fn is_uci_digit(c: char) -> bool {
    matches!(c, '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8')
}

/// 从 AI 输出中提取 `!!` 前缀的本步总结
///
/// AI 走完子后输出 `!!思路+理由+提防+应对` 作为下次思考的参考。
/// 格式：`!!` 后跟本步总结文本，到行尾或下一个标签（`<` 或 `@`）结束。
/// 优先从 content 提取（走法后），content 为空则从 reasoning 提取。
/// 多次输出 `!!` 时取最后一个（最新）。
pub fn extract_notes(content: &str, reasoning: &str) -> String {
    if let Some(notes) = extract_notes_from_text(content) {
        return notes;
    }
    if let Some(notes) = extract_notes_from_text(reasoning) {
        return notes;
    }
    String::new()
}

fn extract_notes_from_text(text: &str) -> Option<String> {
    let mut best: Option<String> = None;
    let mut search_from = 0;
    while let Some(pos) = text[search_from..].find("!!") {
        let start = search_from + pos + 2; // 跳过 !!
        let rest = &text[start..];
        // 注意事项到行尾或下一个标签 < 或 @ 结束
        let end = rest
            .find(|c: char| c == '\n' || c == '<' || c == '@')
            .unwrap_or(rest.len());
        let notes = rest[..end].trim().to_string();
        if !notes.is_empty() {
            best = Some(notes);
        }
        search_from = start;
    }
    best
}

/// 提前检测未闭合的 `<move>` 标签：`<move>` 已出现但 `</move>` 还没到
///
/// 用于思考模式关闭或 AI 未输出 `<pick>` 时，在 `<move>` 标签开始生成但未闭合时
/// 就触发举棋动画。提取 `<move>` 后面的 4-5 字符（UCI 走法），如果是合法走法则返回。
fn extract_open_move_tag(text: &str, legal_moves: &[String]) -> Option<String> {
    if legal_moves.is_empty() {
        return None;
    }
    let lower = text.to_lowercase();
    let tag_open = "<move>";
    let tag_close = "</move>";

    // 找最后一个 <move> 开标签
    let last_open = lower.rfind(tag_open)?;
    let content_start = last_open + tag_open.len();
    if content_start > lower.len() {
        return None;
    }

    // 检查是否已闭合：如果 <move> 后面有 </move>，说明已闭合，不在此处理
    let after_open = &lower[content_start..];
    if after_open.contains(tag_close) {
        return None;
    }

    // 未闭合：提取 <move> 后面的内容，尝试匹配合法走法
    // UCI 走法 4-5 字符（升变 5 字符）
    let remaining: String = after_open.trim().chars().take(5).collect();
    let remaining_lower = remaining.to_lowercase();

    for mv in legal_moves {
        let mv_lower = mv.to_lowercase();
        if remaining_lower.starts_with(&mv_lower) && remaining_lower.len() >= mv_lower.len() {
            return Some(mv_lower);
        }
    }
    None
}

/// 提取已闭合的 `<move>UCI</move>` 标签内容
///
/// 当 AI 输出完整的 `<move>e2e4</move>` 后，立即把 pick 更新为最终走法，
/// 确保落子动画前显示的 pick 与最终走法一致（避免 pick 还停留在最后一个 @UCI）。
/// 多次输出取最后一个（最新）。
fn extract_closed_move_tag(text: &str, legal_moves: &[String]) -> Option<String> {
    if legal_moves.is_empty() {
        return None;
    }
    let lower = text.to_lowercase();
    let tag_open = "<move>";
    let tag_close = "</move>";

    let mut best: Option<String> = None;
    let mut search_from = 0;
    while let Some(open_pos) = lower[search_from..].find(tag_open) {
        let content_start = search_from + open_pos + tag_open.len();
        if content_start > lower.len() {
            break;
        }
        let after_open = &lower[content_start..];
        if let Some(close_pos) = after_open.find(tag_close) {
            // 已闭合：提取 <move> 和 </move> 之间的内容
            let inner = after_open[..close_pos].trim();
            // 取前 5 字符（UCI 走法 4-5 字符）
            let mv_str: String = inner.chars().take(5).collect();
            for mv in legal_moves {
                let mv_lower = mv.to_lowercase();
                if mv_str == mv_lower {
                    best = Some(mv_lower);
                    break;
                }
            }
            search_from = content_start + close_pos + tag_close.len();
        } else {
            // 未闭合，跳出
            break;
        }
    }
    best
}

/// 兜底举棋扫描：在思考文本末尾窗口中找最后出现的、属于合法走法列表的 UCI 走法
///
/// 用于 AI 未输出 `<pick>` 标签时实时驱动举棋动画：
/// - 只扫描末尾 200 字符窗口，避免早期提到的走法一直被当作当前举棋
/// - 大小写不敏感匹配
/// - 返回窗口中位置最大的合法走法（即最近提到的）
fn extract_last_legal_move(text: &str, legal_moves: &[String]) -> Option<String> {
    if legal_moves.is_empty() {
        return None;
    }
    // 取末尾 200 字符窗口
    let chars: Vec<char> = text.chars().collect();
    let start = if chars.len() > 200 { chars.len() - 200 } else { 0 };
    let window: String = chars[start..].iter().collect();
    let window_lower = window.to_lowercase();

    let mut best: Option<(usize, String)> = None;
    for mv in legal_moves {
        let mv_lower = mv.to_lowercase();
        if let Some(pos) = window_lower.rfind(&mv_lower) {
            // 取位置最大的（最近提到的）
            if best.as_ref().map_or(true, |(p, _)| pos > *p) {
                best = Some((pos, mv_lower.clone()));
            }
        }
    }
    best.map(|(_, mv)| mv)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_at_pick() {
        // 单个 @UCI
        assert_eq!(extract_at_pick("分析 @e2e4"), Some("e2e4".to_string()));
        // 多个 @UCI，取最后一个
        assert_eq!(
            extract_at_pick("@e2e4 不好，@d2d4 更强"),
            Some("d2d4".to_string())
        );
        // 5 字符升变
        assert_eq!(extract_at_pick("@e7e8q"), Some("e7e8q".to_string()));
        // 无 @
        assert_eq!(extract_at_pick("纯文本思考"), None);
        // 大小写兼容
        assert_eq!(extract_at_pick("@E2E4"), Some("e2e4".to_string()));
        // @ 后非 UCI 不匹配
        assert_eq!(extract_at_pick("@center @king"), None);
        // 边界：@e2e4 后跟字母不匹配（避免 @e2e4abc 误匹配）
        assert_eq!(extract_at_pick("@e2e4abc"), None);
        // @e2e4 后跟空格匹配
        assert_eq!(extract_at_pick("@e2e4 @g1f3"), Some("g1f3".to_string()));
        // @e2e4 在字符串末尾匹配
        assert_eq!(extract_at_pick("考虑 @e2e4"), Some("e2e4".to_string()));
    }

    #[test]
    fn test_extract_notes() {
        // content 中提取
        assert_eq!(
            extract_notes("<move>e2e4</move> !!白优+0.5 黑威胁d5", ""),
            "白优+0.5 黑威胁d5".to_string()
        );
        // 多次输出取最新
        assert_eq!(
            extract_notes("!!白优 !!白优+0.5", ""),
            "白优+0.5".to_string()
        );
        // content 为空从 reasoning 提取
        assert_eq!(
            extract_notes("", "!!黑威胁f7 白攻王翼"),
            "黑威胁f7 白攻王翼".to_string()
        );
        // 无 !!
        assert_eq!(extract_notes("e2e4 <move>e2e4</move>", "思考中"), "");
        // !! 后到行尾
        assert_eq!(extract_notes("!!白优+0.5\n其他", ""), "白优+0.5".to_string());
        // !! 后遇 < 标签结束
        assert_eq!(
            extract_notes("!!白优<move>e2e4</move>", ""),
            "白优".to_string()
        );
        // !! 后遇 @ 结束
        assert_eq!(extract_notes("!!白优 @e2e4", ""), "白优".to_string());
        // 空内容
        assert_eq!(extract_notes("", ""), "");
    }

    #[test]
    fn test_extract_last_legal_move() {
        let legal = vec![
            "e2e4".to_string(),
            "d2d4".to_string(),
            "g1f3".to_string(),
        ];
        // 取最后出现的合法走法
        assert_eq!(
            extract_last_legal_move("考虑 e2e4 然后 d2d4", &legal),
            Some("d2d4".to_string())
        );
        // 大小写兼容
        assert_eq!(
            extract_last_legal_move("G1F3 不错", &legal),
            Some("g1f3".to_string())
        );
        // 无合法走法出现
        assert_eq!(extract_last_legal_move("纯文本无走法", &legal), None);
        // 窗口外的不算：合法走法在开头，窗口是末尾200字符，不含走法
        let long = format!("{}{}", "e2e4", "x".repeat(210));
        assert_eq!(extract_last_legal_move(&long, &legal), None);
        // 窗口内有走法
        let long2 = format!("{}{}", "x".repeat(210), "e2e4 然后 g1f3");
        assert_eq!(extract_last_legal_move(&long2, &legal), Some("g1f3".to_string()));
        // 空合法列表
        assert_eq!(extract_last_legal_move("e2e4", &[]), None);
    }

    #[test]
    fn test_extract_open_move_tag() {
        let legal = vec![
            "e2e4".to_string(),
            "d2d4".to_string(),
            "e7e8q".to_string(),
        ];
        // 未闭合的 <move> 标签，提取 4 字符走法
        assert_eq!(
            extract_open_move_tag("分析完毕 <move>e2e4", &legal),
            Some("e2e4".to_string())
        );
        // 未闭合的 <move> 标签，提取 5 字符升变走法
        assert_eq!(
            extract_open_move_tag("<move>e7e8q", &legal),
            Some("e7e8q".to_string())
        );
        // 已闭合的 <move> 标签不处理
        assert_eq!(
            extract_open_move_tag("<move>e2e4</move>", &legal),
            None
        );
        // 无 <move> 标签
        assert_eq!(
            extract_open_move_tag("纯文本思考", &legal),
            None
        );
        // <move> 后内容不是合法走法
        assert_eq!(
            extract_open_move_tag("<move>xxxx", &legal),
            None
        );
        // 大小写兼容
        assert_eq!(
            extract_open_move_tag("<MOVE>E2E4", &legal),
            Some("e2e4".to_string())
        );
        // 空合法列表
        assert_eq!(
            extract_open_move_tag("<move>e2e4", &[]),
            None
        );
    }
}
