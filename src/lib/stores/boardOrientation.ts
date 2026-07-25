import { writable, derived } from "svelte/store";
import { gameState } from "./game";

const STORAGE_KEY = "ai-chess-board-flipped";

function loadInitial(): boolean {
  if (typeof localStorage === "undefined") return false;
  return localStorage.getItem(STORAGE_KEY) === "1";
}

/// 棋盘手动翻转状态（用户偏好，持久化到 localStorage）
const boardFlipped = writable<boolean>(loadInitial());

boardFlipped.subscribe((v) => {
  if (typeof localStorage === "undefined") return;
  try {
    localStorage.setItem(STORAGE_KEY, v ? "1" : "0");
  } catch {
    // 忽略写入失败（如隐私模式）
  }
});

/// 切换棋盘翻转状态
export function toggleBoardFlipped() {
  boardFlipped.update((v) => !v);
}

/// 最终棋盘翻转状态：基础翻转（player_side === "black"）与用户手动翻转做 XOR
/// Board/EvalBar 等组件统一订阅此 store，避免重复实现 XOR 逻辑
export const boardEffectiveFlipped = derived(
  [gameState, boardFlipped],
  ([$game, $flipped]) => ($game.player_side === "black") !== $flipped
);

export { boardFlipped };
