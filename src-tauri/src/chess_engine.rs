use chess::{Board, BoardStatus, ChessMove, Color, MoveGen, Piece, Square};
use std::str::FromStr;

/// 棋局状态管理：封装 chess crate，提供走法、历史、FEN、可视化等能力
pub struct ChessGame {
    pub board: Board,
    pub move_history: Vec<ChessMove>,
    pub board_history: Vec<Board>, // 用于悔棋
    pub player_side: Color,
    /// 白方主体类型："human" | "stockfish" | "deepseek"
    pub white_player: String,
    /// 黑方主体类型："human" | "stockfish" | "deepseek"
    pub black_player: String,
}

impl ChessGame {
    pub fn new(player_side: Color, white_player: String, black_player: String) -> Self {
        Self {
            board: Board::default(),
            move_history: Vec::new(),
            board_history: Vec::new(),
            player_side,
            white_player,
            black_player,
        }
    }

    /// 当前局面的 FEN 字符串
    pub fn to_fen(&self) -> String {
        self.board.to_string()
    }

    /// 可视化棋盘（用于 DeepSeek 提示）：从 FEN 解析生成文本矩阵
    pub fn to_visual_string(&self) -> String {
        let fen = self.to_fen();
        let board_part = fen.split(' ').next().unwrap_or("");
        let mut result = String::from("  a b c d e f g h\n");
        let mut rank_num: i32 = 8;
        for rank_str in board_part.split('/') {
            result.push_str(&format!("{} ", rank_num));
            for c in rank_str.chars() {
                if c.is_ascii_digit() {
                    let n = c.to_digit(10).unwrap_or(0);
                    for _ in 0..n {
                        result.push_str(". ");
                    }
                } else {
                    result.push(c);
                    result.push(' ');
                }
            }
            result.push_str(&format!("{}\n", rank_num));
            rank_num -= 1;
        }
        result.push_str("  a b c d e f g h");
        result
    }

    /// 所有合法走法
    pub fn legal_moves(&self) -> Vec<ChessMove> {
        MoveGen::new_legal(&self.board).collect()
    }

    /// 合法走法的坐标记号列表
    pub fn legal_moves_str(&self) -> Vec<String> {
        self.legal_moves()
            .iter()
            .map(|mv| move_to_coord(mv))
            .collect()
    }

    /// 检测对方悬空子（己方可白吃且对方无法回吃的棋子）
    ///
    /// 基于 ChessArena 论文(arXiv:2509.24239)发现：LLM 战术推理弱，常错过对方送子。
    /// 预计算悬空子信息，在 user_message 中提供给模型，减轻其战术计算负担。
    ///
    /// 算法：对每个己方吃子走法，模拟走子后检查对方能否回吃目标格。
    /// 无法回吃 → 该对方棋子悬空，可白吃。
    ///
    /// 返回格式：["Qh4", "Bc5"]（大写=白方棋子，小写=黑方棋子）
    pub fn enemy_hanging(&self) -> Vec<String> {
        let mut hanging = Vec::new();
        let my_moves = MoveGen::new_legal(&self.board);
        for mv in my_moves {
            let target = mv.get_dest();
            // 只看吃子走法（目标格有对方棋子）
            if let Some(enemy_piece) = self.board.piece_on(target) {
                // 模拟走子（make_move_new 切换 side_to_move）
                let after = self.board.make_move_new(mv);
                // 检查对方能否回吃目标格
                let can_recapture = MoveGen::new_legal(&after).any(|r| r.get_dest() == target);
                if !can_recapture {
                    // 对方悬空子，可白吃
                    let is_white = self.board.color_on(target) == Some(Color::White);
                    let pc = piece_to_char(enemy_piece);
                    let piece_str = if is_white {
                        pc.to_ascii_uppercase()
                    } else {
                        pc
                    };
                    let sq = square_to_str(target);
                    hanging.push(format!("{}{}", piece_str, sq));
                }
            }
        }
        hanging.sort();
        hanging.dedup();
        hanging
    }

    /// 验证走法是否合法
    pub fn is_legal(&self, mv: &ChessMove) -> bool {
        self.legal_moves().contains(mv)
    }

    /// 应用走法
    pub fn make_move(&mut self, mv: ChessMove) -> Result<(), String> {
        if !self.is_legal(&mv) {
            return Err("非法走法".to_string());
        }
        self.board_history.push(self.board);
        let mut new_board = self.board;
        self.board.make_move(mv, &mut new_board);
        self.board = new_board;
        self.move_history.push(mv);
        Ok(())
    }

    /// 当前轮到谁
    pub fn side_to_move(&self) -> Color {
        self.board.side_to_move()
    }

    /// 游戏状态
    pub fn status(&self) -> BoardStatus {
        self.board.status()
    }

    /// 是否将军（当前走棋方被将军）
    pub fn in_check(&self) -> bool {
        // chess crate 的 BoardStatus 仅区分 Ongoing/Checkmate/Stalemate
        // 通过 checkers() 检测当前走棋方被将军的棋子数量
        self.board.checkers().popcnt() > 0
    }

    /// 悔棋（撤销最后一步）
    pub fn undo(&mut self) -> bool {
        if let Some(prev) = self.board_history.pop() {
            self.board = prev;
            self.move_history.pop();
            true
        } else {
            false
        }
    }

    /// 总步数
    pub fn ply_count(&self) -> usize {
        self.move_history.len()
    }

    /// 从初始局面重放走法历史，重建棋局（用于持久化恢复）
    pub fn rebuild_from_history(
        player_side: Color,
        white_player: String,
        black_player: String,
        moves: Vec<String>,
    ) -> Result<Self, String> {
        let mut game = Self::new(player_side, white_player, black_player);
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
}

/// 将 ChessMove 转为坐标记号（如 e2e4，升变 e7e8q）
pub fn move_to_coord(mv: &ChessMove) -> String {
    let from = square_to_str(mv.get_source());
    let to = square_to_str(mv.get_dest());
    match mv.get_promotion() {
        Some(p) => format!("{}{}{}", from, to, piece_to_char(p)),
        None => format!("{}{}", from, to),
    }
}

/// Square 转字符串（如 e4）
pub fn square_to_str(sq: Square) -> String {
    let idx = sq.to_index();
    let file = idx % 8;
    let rank = idx / 8;
    let file_char = (b'a' + file as u8) as char;
    let rank_char = (b'1' + rank as u8) as char;
    format!("{}{}", file_char, rank_char)
}

/// 从字符串解析 Square（如 "e4"）
pub fn parse_square(s: &str) -> Option<Square> {
    Square::from_str(s).ok()
}

/// Piece 转字符（小写，用于升变记号和 FEN 黑方）
pub fn piece_to_char(p: Piece) -> char {
    match p {
        Piece::Pawn => 'p',
        Piece::Knight => 'n',
        Piece::Bishop => 'b',
        Piece::Rook => 'r',
        Piece::Queen => 'q',
        Piece::King => 'k',
    }
}

/// 从字符解析 Piece
pub fn char_to_piece(c: char) -> Option<Piece> {
    match c.to_ascii_lowercase() {
        'p' => Some(Piece::Pawn),
        'n' => Some(Piece::Knight),
        'b' => Some(Piece::Bishop),
        'r' => Some(Piece::Rook),
        'q' => Some(Piece::Queen),
        'k' => Some(Piece::King),
        _ => None,
    }
}

/// 从坐标记号解析走法（如 e2e4、e7e8q），不验证合法性
pub fn parse_coord_move(s: &str) -> Option<ChessMove> {
    let s = s.trim();
    if s.len() < 4 || s.len() > 5 {
        return None;
    }
    let from = parse_square(&s[0..2])?;
    let to = parse_square(&s[2..4])?;
    let promotion = if s.len() == 5 {
        Some(char_to_piece(s.as_bytes()[4] as char)?)
    } else {
        None
    };
    Some(ChessMove::new(from, to, promotion))
}

/// Color 转字符串
pub fn color_to_str(c: Color) -> String {
    match c {
        Color::White => "white".to_string(),
        Color::Black => "black".to_string(),
    }
}

/// 从字符串解析 Color
pub fn str_to_color(s: &str) -> Option<Color> {
    match s.to_lowercase().as_str() {
        "white" | "w" => Some(Color::White),
        "black" | "b" => Some(Color::Black),
        _ => None,
    }
}

/// Color 取反
pub fn color_opposite(c: Color) -> Color {
    match c {
        Color::White => Color::Black,
        Color::Black => Color::White,
    }
}
