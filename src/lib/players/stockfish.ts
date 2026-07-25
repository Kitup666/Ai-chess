/// Stockfish 主体：调用 WASM 引擎求最佳走法，通过后端 playerMove 应用
///
/// 难度通过 ELO（1320-3190）或 Skill Level（0-20）控制。
/// 思考时间按 ELO 动态：低 ELO 短思考（靠随机走法弱化），高 ELO 深搜索。
/// 走棋前设置 aiPick 触发举棋动画，让鳕鱼"思考"可见。
///
/// 难度读取策略：每次 requestMove 时从 settings store 实时读取，
/// 这样用户在 Settings 中调整难度后立即生效，无需 resetManager 重建实例。
import type { Player } from "./types";
import type { GameStateDto, MoveResult } from "../types";
import { playerMove } from "../api";
import { aiPick, aiThinking, aiFailed } from "../stores/game";
import { loadEngine, getEngine, getBestMove } from "../stockfish/store";
import { get } from "svelte/store";
import { settings } from "../stores/settings";

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

export function createStockfishPlayer(_opts: StockfishOptions): Player {
  return {
    type: "stockfish",
    isAutomatic: true,
    async requestMove(state: GameStateDto): Promise<MoveResult> {
      aiThinking.set(true);
      try {
        await loadEngine();
        const engine = getEngine();
        // 实时从 settings store 读取难度（用户调整后立即生效，无需重建 PlayerManager）
        const s = get(settings);
        const useElo = s.useStockfishElo;
        const elo = s.stockfishElo;
        const skill = s.stockfishSkill;
        // 设置难度
        if (useElo) {
          await engine.setElo(elo);
        } else {
          await engine.setSkillLevel(skill);
        }
        const movetime = useElo ? movetimeForElo(elo) : 800;
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
