import { writable } from "svelte/store";

const STORAGE_KEY = "ai-chess-board-flipped";

function loadInitial(): boolean {
  if (typeof localStorage === "undefined") return false;
  return localStorage.getItem(STORAGE_KEY) === "1";
}

/// 棋盘手动翻转状态（用户偏好，持久化到 localStorage）
/// 与 player_side === "black" 的基础翻转做 XOR 得到最终 flipped
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

export { boardFlipped };
