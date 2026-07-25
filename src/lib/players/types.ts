/// 对弈主体抽象：屏蔽不同走棋来源（人/鳕鱼/DeepSeek）的差异
///
/// Human: 不主动走棋，由 Board 点击触发编排器接收走法
/// Stockfish: 调用 WASM 引擎 getBestMove + 后端 playerMove 应用
/// DeepSeek: 调用后端 ai_move 命令（已应用走法 + 触发流式思考事件）
import type { PlayerType, MoveResult, GameStateDto } from "../types";

export interface Player {
  type: PlayerType;
  /// 是否自动走棋（非 Human）
  isAutomatic: boolean;
  /// 自动主体请求走法并应用，返回走法结果（已更新状态）
  /// 仅 isAutomatic=true 时实现；Human 不实现
  requestMove?: (state: GameStateDto) => Promise<MoveResult>;
}
