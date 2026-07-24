/// 前端类型定义，与后端 DTO 对应

export type Side = "white" | "black";
export type GameStatus = "playing" | "thinking" | "checkmate" | "stalemate" | "draw";

export interface MoveDto {
  from: string;
  to: string;
  promotion: string | null;
}

export interface GameStateDto {
  fen: string;
  visual: string;
  player_side: Side;
  turn: Side;
  move_history: string[];
  status: GameStatus;
  winner: Side | null;
  last_move: MoveDto | null;
  legal_moves: string[];
  in_check: boolean;
  ply: number;
}

export interface MoveResult {
  move_str: string;
  state: GameStateDto;
  game_over: boolean;
}

export interface StartGameArgs {
  side: Side;
  api_key: string;
  model: string;
  thinking?: boolean;
  /// 伪思考模式：关闭 API thinking 用提示词模拟，缺省 false
  pseudo_thinking?: boolean;
  thinking_language?: "zh" | "en";
  /// reasoning_effort: "high" | "max"（DeepSeek 仅支持这两档），缺省 "high"
  reasoning_effort?: "high" | "max";
  /// 最少思考 token 数，缺省 0（不限制）
  min_thinking_tokens?: number;
}

/// 持久化设置 DTO（load_state 返回）
export interface SettingsDto {
  api_key: string;
  model: string;
  thinking: boolean;
  pseudo_thinking: boolean;
  thinking_language: "zh" | "en";
  reasoning_effort: "high" | "max";
  min_thinking_tokens: number;
}

/// load_state 命令返回结构
export interface LoadedState {
  game: GameStateDto | null;
  settings: SettingsDto;
}

/// 棋子类型
export type PieceType = "k" | "q" | "r" | "b" | "n" | "p";

/// 棋盘上的格子
export interface SquareData {
  file: number; // 0-7 (a-h)
  rank: number; // 0-7 (1-8)
  piece: { type: PieceType; color: Side } | null;
}

/// 从 FEN 解析棋盘为 8x8 格子数组
export function parseFen(fen: string): SquareData[][] {
  const boardPart = fen.split(" ")[0];
  const rows = boardPart.split("/");
  const board: SquareData[][] = [];
  for (let r = 0; r < 8; r++) {
    const row: SquareData[] = [];
    const rank = 7 - r; // FEN 从 rank 8 开始，转为 0-indexed（0=rank1, 7=rank8）
    let file = 0;
    for (const c of rows[r]) {
      if (/\d/.test(c)) {
        const n = parseInt(c);
        for (let i = 0; i < n; i++) {
          row.push({ file, rank, piece: null });
          file++;
        }
      } else {
        const color = c === c.toUpperCase() ? "white" : "black";
        const type = c.toLowerCase() as PieceType;
        row.push({ file, rank, piece: { type, color } });
        file++;
      }
    }
    board.push(row);
  }
  return board;
}

/// 将格子坐标转为代数记号 (file, rank) -> "e4"
export function squareName(file: number, rank: number): string {
  return String.fromCharCode(97 + file) + (rank + 1);
}
