/// PlayerManager 单例 store
///
/// 跨组件共享 PlayerManager 实例：
/// - App.svelte 启动/重开时调用 driveTurn 驱动自动主体
/// - Board.svelte 在 Human 走棋后调用 continueAfterHumanMove 驱动对方
/// - 任意位置可调用 stop 中断自对弈
import { tick } from "svelte";
import { get } from "svelte/store";
import { PlayerManager, type StockfishOptions } from "../players/manager";
import type { GameStateDto, MoveResult, PlayerType } from "../types";
import { settings } from "./settings";
import { updateGameState, aiThinking, aiFailed, aiPick } from "./game";
import { playMoveSounds } from "../sounds/player";
import { gameState } from "./game";

let managerInstance: PlayerManager | null = null;

/// 当前设置构造 Stockfish 选项
function buildSfOpts(): StockfishOptions {
  const s = get(settings);
  return {
    elo: s.stockfishElo,
    skill: s.stockfishSkill,
    useElo: s.useStockfishElo,
  };
}

/// 获取或创建 PlayerManager（按当前 settings 主体组合构造）
function getManager(): PlayerManager {
  const s = get(settings);
  if (!managerInstance) {
    managerInstance = new PlayerManager(s.whitePlayer, s.blackPlayer, buildSfOpts());
  }
  return managerInstance;
}

/// 重置 PlayerManager（切换主体组合或重开时调用）
export function resetManager(white: PlayerType, black: PlayerType): PlayerManager {
  managerInstance = new PlayerManager(white, black, buildSfOpts());
  return managerInstance;
}

/// 获取当前 PlayerManager（不创建）
export function peekManager(): PlayerManager | null {
  return managerInstance;
}

/// AI/鳕鱼走棋后的 FLIP 动画 + 音效
/// 在 updateGameState 之前记录 from 棋子位置，updateGameState + tick 后做动画
async function playAutoMoveAnimation(result: MoveResult): Promise<void> {
  const lm = result.state.last_move;
  if (!lm) {
    updateGameState(result.state);
    return;
  }
  // 走棋前的 FEN（用于音效判断吃子），必须在 updateGameState 之前获取
  const oldFen = get(gameState).fen;
  // UCI 走法：from + to + promotion
  const uci = lm.from + lm.to + (lm.promotion ?? "");

  // updateGameState 前记录 from 棋子位置（DOM 还是旧状态，from 棋子在原位）
  let fromRect: DOMRect | null = null;
  const fromEl = document.querySelector(`[aria-label="${lm.from}"] .piece`) as HTMLElement | null;
  if (fromEl) {
    fromRect = fromEl.getBoundingClientRect();
  }

  updateGameState(result.state);
  await tick();

  // 音效与动画并行播放（落子瞬间发声，与 Lichess 行为一致）
  // 旧实现把 playMoveSounds 放在 runFlipAnimation 之后，导致声音延迟 340ms
  playMoveSounds({
    uci,
    oldFen,
    inCheck: result.state.in_check,
    gameOver: result.game_over,
    status: result.state.status,
  });

  if (lm && fromRect) {
    const toEl = document.querySelector(`[aria-label="${lm.to}"]`) as HTMLElement | null;
    const toRect = toEl?.getBoundingClientRect() ?? null;
    const newPieceEl = toEl?.querySelector(".piece") as HTMLElement | null;
    if (toRect && newPieceEl) {
      await runFlipAnimation(newPieceEl, fromRect, toRect);
    }
  }
}

/// FLIP 动画：棋子从源位置滑动到目标位置，起手抬起，落子回正
async function runFlipAnimation(
  fromEl: HTMLElement,
  fromRect: DOMRect,
  toRect: DOMRect
): Promise<void> {
  const dx = fromRect.left - toRect.left;
  const dy = fromRect.top - toRect.top;
  fromEl.style.transition = "none";
  fromEl.style.transform = `translate(${dx}px, ${dy}px) scale(1.08)`;
  fromEl.style.zIndex = "10";
  fromEl.style.filter = "drop-shadow(0 8px 16px rgba(0,0,0,0.28))";
  void fromEl.offsetHeight;
  fromEl.style.transition =
    "transform 0.32s cubic-bezier(0.16,1,0.3,1), filter 0.32s cubic-bezier(0.16,1,0.3,1)";
  fromEl.style.transform = "translate(0, 0) scale(1)";
  fromEl.style.filter = "drop-shadow(0 1px 2px rgba(0,0,0,0.18))";
  await new Promise((r) => setTimeout(r, 340));
  fromEl.style.zIndex = "";
}

/// 驱动当前轮到方走棋（自动主体连续走，直到 Human 方或游戏结束）
/// 由 App.svelte 在 startGame/resetGame 完成后调用
export async function driveTurn(state: GameStateDto): Promise<void> {
  const manager = getManager();
  aiFailed.set(false);
  await manager.driveTurn(state, async (result) => {
    await playAutoMoveAnimation(result);
  });
}

/// Human 走棋后继续驱动对方走棋
/// 由 Board.svelte 在玩家点击走棋后调用
export async function continueAfterHumanMove(state: GameStateDto): Promise<void> {
  const manager = managerInstance;
  if (!manager) return;
  await manager.continueAfterHumanMove(state, async (result) => {
    await playAutoMoveAnimation(result);
  });
}

/// 停止自对弈循环
export function stopAutoPlay(): void {
  managerInstance?.stop();
}

/// 重置停止状态（不重建实例）
export function resetStopFlag(): void {
  managerInstance?.reset();
}
