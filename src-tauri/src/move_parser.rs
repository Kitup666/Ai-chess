use crate::chess_engine::{parse_coord_move, ChessGame};
use chess::ChessMove;

/// 解析 DeepSeek 响应并验证走法合法性
pub enum ParseError {
    /// 响应中找不到有效走法格式
    InvalidFormat(String),
    /// 走法格式正确但不合法（附带合法走法列表供重试）
    IllegalMove(String, Vec<String>),
}

/// 从 DeepSeek 的回复文本中提取并验证走法
///
/// 解析优先级：
/// 1. `<move>...</move>` 标签（最可靠，避免误匹配推理中的坐标）
/// 2. 纯走法格式（4-5 字符，去除空白后整段就是走法）
/// 3. 最后一行扫描（回退方案）
pub fn parse_and_validate(response: &str, game: &ChessGame) -> Result<ChessMove, ParseError> {
    let candidate = extract_move_token(response);
    let candidate = match candidate {
        Some(c) => c,
        None => return Err(ParseError::InvalidFormat(response.to_string())),
    };

    let mv = match parse_coord_move(&candidate) {
        Some(m) => m,
        None => return Err(ParseError::InvalidFormat(candidate)),
    };

    if game.is_legal(&mv) {
        Ok(mv)
    } else {
        Err(ParseError::IllegalMove(
            candidate,
            game.legal_moves_str(),
        ))
    }
}

/// 从文本中提取走法 token
///
/// 策略：
/// 1. 优先提取 `<move>...</move>` 标签内容（标签内只允许走法本身）
/// 2. 若无标签，检查整段去除空白/markdown 后是否就是 4-5 字符的走法
/// 3. 回退：取最后一行非空文本的第一个走法匹配
fn extract_move_token(text: &str) -> Option<String> {
    // 1. 优先解析 <move>...</move> 标签（可能有多个，取最后一个）
    if let Some(mv) = extract_from_move_tags(text) {
        return Some(mv);
    }

    // 2. 整段是否就是走法（去除 markdown 代码块标记和空白）
    // 注：思考模式的推理在 reasoning_content 字段，不会出现在此处 content
    let trimmed = text.replace("```", "").trim().to_string();
    if is_pure_move(&trimmed) {
        return Some(trimmed.to_lowercase());
    }

    // 3. 回退：按行从后往前找，每行取第一个走法匹配
    let lines: Vec<&str> = text
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();
    for line in lines.iter().rev() {
        // 跳过明显是推理的行（包含中文说明、冒号等）
        if line.contains("：") || line.contains("legal") || line.contains("Legal") {
            continue;
        }
        if let Some(mv) = scan_line_for_first_move(line) {
            return Some(mv);
        }
    }

    None
}

/// 从 `<move>...</move>` 标签中提取走法
/// 取最后一个标签的内容（最可能是最终结论）
fn extract_from_move_tags(text: &str) -> Option<String> {
    let mut last_match: Option<String> = None;
    let lower = text.to_lowercase();
    let bytes = lower.as_bytes();

    let tag_open = b"<move>";
    let tag_close = b"</move>";

    let mut start = 0;
    while start + tag_open.len() <= bytes.len() {
        // 查找 <move>
        if let Some(pos) = find_subslice(&bytes[start..], tag_open) {
            let content_start = start + pos + tag_open.len();
            // 查找对应的 </move>
            if content_start + tag_close.len() <= bytes.len() {
                if let Some(close_pos) = find_subslice(&bytes[content_start..], tag_close) {
                    let content = &text[content_start..content_start + close_pos];
                    let cleaned = content.trim().to_string();
                    if !cleaned.is_empty() {
                        last_match = Some(cleaned);
                    }
                    start = content_start + close_pos + tag_close.len();
                    continue;
                }
            }
        }
        break;
    }
    last_match
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// 判断字符串是否是纯走法（4-5 字符的 UCI 记号）
fn is_pure_move(s: &str) -> bool {
    let len = s.len();
    if len != 4 && len != 5 {
        return false;
    }
    let chars: Vec<char> = s.chars().collect();
    if !is_file(chars[0]) || !is_rank(chars[1]) || !is_file(chars[2]) || !is_rank(chars[3]) {
        return false;
    }
    if len == 5 && !is_promo(chars[4]) {
        return false;
    }
    true
}

/// 从单行文本中扫描第一个走法匹配
fn scan_line_for_first_move(line: &str) -> Option<String> {
    let lower = line.to_lowercase();
    let chars: Vec<char> = lower.chars().collect();

    let mut i = 0;
    while i + 3 < chars.len() {
        let c0 = chars[i];
        let c1 = chars[i + 1];
        let c2 = chars[i + 2];
        let c3 = chars[i + 3];

        if is_file(c0) && is_rank(c1) {
            // 带分隔符：e2-e4 / e2 e4 / e2,e4
            if (c2 == '-' || c2 == ' ' || c2 == ',') && i + 4 < chars.len() {
                let c3b = chars[i + 3];
                let c4 = chars[i + 4];
                if is_file(c3b) && is_rank(c4) {
                    let mut token = String::new();
                    token.push(c0);
                    token.push(c1);
                    token.push(c3b);
                    token.push(c4);
                    if i + 5 < chars.len() && is_promo(chars[i + 5]) {
                        token.push(chars[i + 5]);
                    }
                    return Some(token);
                }
            }
            // 无分隔符：e2e4
            if is_file(c2) && is_rank(c3) {
                let mut token = String::new();
                token.push(c0);
                token.push(c1);
                token.push(c2);
                token.push(c3);
                if i + 4 < chars.len() && is_promo(chars[i + 4]) {
                    token.push(chars[i + 4]);
                }
                return Some(token);
            }
        }
        i += 1;
    }
    None
}

fn is_file(c: char) -> bool {
    ('a'..='h').contains(&c)
}

fn is_rank(c: char) -> bool {
    ('1'..='8').contains(&c)
}

fn is_promo(c: char) -> bool {
    matches!(c, 'q' | 'r' | 'b' | 'n')
}

/// 从 AI 输出中提取最终走法及其评分
///
/// 走法从 `<move>UCI</move>` 标签提取。
/// 评分从 @UCI 行的 `分:N` 或 `score:N` 提取（取该走法最后一个 @UCI 行的评分）。
/// 未找到评分返回 0。
///
/// 用于 Self-Consistency 多采样投票：每个采样返回 (走法, 评分)，
/// 按"出现次数 × 100 + 平均评分"排序选最佳。
pub fn extract_move_and_score(content: &str, reasoning: &str) -> (String, i32) {
    // 优先从 content 提取走法，content 为空则从 reasoning 提取
    let mv = extract_from_move_tags(content)
        .or_else(|| extract_from_move_tags(reasoning))
        .unwrap_or_default();
    if mv.is_empty() {
        return (String::new(), 0);
    }
    let mv_lower = mv.to_lowercase();
    // 从 reasoning 和 content 中找该走法的评分
    let score = extract_score_for_move(reasoning, &mv_lower)
        .or_else(|| extract_score_for_move(content, &mv_lower))
        .unwrap_or(0);
    (mv_lower, score)
}

/// 从文本中找指定走法最后一个 @UCI 行的评分
///
/// 匹配格式：`@e2e4 因控中心 分:+3` 或 `@e2e4 because center score:+3`
/// 返回分:N 中的 N（支持 +N、-N、0）
fn extract_score_for_move(text: &str, move_uci: &str) -> Option<i32> {
    let lower = text.to_lowercase();
    let move_lower = move_uci.to_lowercase();
    let mut best_score: Option<i32> = None;
    for line in lower.lines() {
        let trimmed = line.trim();
        // 查找 @<move_uci> 开头的行
        if let Some(rest) = trimmed.strip_prefix(&format!("@{}", move_lower)) {
            // 在该行中找 分:N 或 score:N
            let score = extract_score_from_line(rest)
                .or_else(|| extract_score_from_line(trimmed));
            if let Some(s) = score {
                best_score = Some(s); // 取最后一个匹配
            }
        }
    }
    best_score
}

/// 从单行文本中提取 分:N 或 score:N 评分
fn extract_score_from_line(line: &str) -> Option<i32> {
    // 查找 "分:" 或 "score:" 后的整数（含正负号）
    for prefix in &["分:", "score:"] {
        if let Some(pos) = line.find(prefix) {
            let after = &line[pos + prefix.len()..];
            // 解析整数（可能带 + 或 - 号）
            let num_str: String = after
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '+' || *c == '-')
                .collect();
            if let Ok(n) = num_str.parse::<i32>() {
                return Some(n);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_move_tag_extraction() {
        assert_eq!(
            extract_move_token("Let me think... <move>e2e4</move>"),
            Some("e2e4".to_string())
        );
        assert_eq!(
            extract_move_token("分析：e2e4 不错，但 <move>d2d4</move> 更好"),
            Some("d2d4".to_string())
        );
        assert_eq!(
            extract_move_token("推理 e2e4，再想 g1f3，最终 <move>g1f3</move>"),
            Some("g1f3".to_string())
        );
    }

    #[test]
    fn test_pure_move() {
        assert_eq!(extract_move_token("e2e4"), Some("e2e4".to_string()));
        assert_eq!(extract_move_token("  e7e8q  "), Some("e7e8q".to_string()));
    }

    #[test]
    fn test_avoid_legal_moves_line() {
        // 不应从 "Legal moves:" 行误匹配
        assert_eq!(
            extract_move_token("Legal moves: e2e4 d2d4\n\n<move>c2c4</move>"),
            Some("c2c4".to_string())
        );
    }

    #[test]
    fn test_extract_move_and_score() {
        // 中文格式：@UCI 连接词 理由 分:N
        let (mv, score) = extract_move_and_score(
            "<move>e2e4</move>",
            "@e2e4 因控中心 分:+3\n@d2d4 则争d5 分:+3",
        );
        assert_eq!(mv, "e2e4");
        assert_eq!(score, 3);

        // 英文格式：@UCI connector reason score:N
        let (mv, score) = extract_move_and_score(
            "<move>d2d4</move>",
            "@e2e4 because center score:+3\n@d2d4 so contest d5 score:+5",
        );
        assert_eq!(mv, "d2d4");
        assert_eq!(score, 5);

        // 负评分
        let (mv, score) = extract_move_and_score(
            "<move>a2a3</move>",
            "@a2a3 因防b4 分:-1",
        );
        assert_eq!(mv, "a2a3");
        assert_eq!(score, -1);

        // 无评分行返回 0
        let (mv, score) = extract_move_and_score(
            "<move>e2e4</move>",
            "纯文本思考无评分",
        );
        assert_eq!(mv, "e2e4");
        assert_eq!(score, 0);

        // 走法在 reasoning 中（思考模式下 content 可能为空）
        let (mv, score) = extract_move_and_score(
            "",
            "@e2e4 因控中心 分:+3\n<move>e2e4</move>",
        );
        assert_eq!(mv, "e2e4");
        assert_eq!(score, 3);

        // 多次出现取最后一个评分
        let (mv, score) = extract_move_and_score(
            "<move>e2e4</move>",
            "@e2e4 因控中心 分:+3\n@e2e4 则改主意 分:+1",
        );
        assert_eq!(mv, "e2e4");
        assert_eq!(score, 1);

        // 无走法返回空
        let (mv, _) = extract_move_and_score("纯文本无走法", "也无走法");
        assert_eq!(mv, "");
    }
}
