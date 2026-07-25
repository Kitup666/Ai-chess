/// Stockfish 主体：调用 WASM 引擎求最佳走法，通过后端 playerMove 应用
///
/// 难度通过 ELO（1320-3190）或 Skill Level（0-20）控制。
/// 思考时间按 ELO 动态：低 ELO 短思考（靠随机走法弱化），高 ELO 深搜索。
/// 走棋前设置 aiPick 触发举棋动画，让鳕鱼"思考"可见。
import type { Player } from "./types";
import type { GameStateDto, MoveResult } from "../types";
import { playerMove } from "../api";
import { aiPick, aiThinking, aiFailed } from "../stores/game";
import { loadEngine, getEngine, getBestMove } from "../stockfish/store";

export interface StockfishOptions {
  /// ELO 等级（1320-3190），useElo=true 时生效
  elo: number;
  /// Skill Level（0-20），useElo=false 时生效
  skill: number;
  /// 是否用 ELO 模式（true=UCI_Elo，false=Skill Level）
  useElo: boolean;
}

/// 按 ELO 动态选择思考时间（毫秒）
function movetimeForElo(elo: number): number {
  if (elo < 1800) return 400;
  if (elo < 2400) return 800;
  return 1500;
}

export function createStockfishPlayer(opts: StockfishOptions): Player {
  return {
    type: "stockfish",
    isAutomatic: true,
    async requestMove(state: GameStateDto): Promise<MoveResult> {
      aiThinking.set(true);
      try {
        await loadEngine();
        const engine = getEngine();
        // 设置难度
        if (opts.useElo) {
          await engine.setElo(opts.elo);
        } else {
          await engine.setSkillLevel(opts.skill);
        }
        const movetime = opts.useElo ? movetimeForElo(opts.elo) : 800;
        // 求最佳走法（搜索期间 onInfo 会触发，分析面板可见）
        const best = await getBestMove(state.fen, movetime);
        // 设置举棋动画，让玩家看见鳕鱼"举棋"
        aiPick.set(best.move);
        await new Promise((r) => setTimeout(r, 500));
        // 通过后端 playerMove 应用走法（更新状态、持久化、合法性验证）
        const result = await playerMove(best.move);
        aiPick.set(null);
        return result;
      } catch (e) {
        aiFailed.set(true);
        throw e;
      } finally {
        aiThinking.set(false);
      }
    },
  };
}
