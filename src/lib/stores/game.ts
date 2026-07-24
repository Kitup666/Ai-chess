import { writable, derived } from "svelte/store";
import type { GameStateDto, Side } from "../types";

const initialState: GameStateDto = {
  fen: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
  visual: "",
  player_side: "white",
  turn: "white",
  move_history: [],
  status: "playing",
  winner: null,
  last_move: null,
  legal_moves: [],
  in_check: false,
  ply: 0,
};

export const gameState = writable<GameStateDto>(initialState);
export const selectedSquare = writable<string | null>(null);
export const errorMsg = writable<string | null>(null);
export const aiThinking = writable(false);
/// AI 当前思考内容（流式增量拼接）
export const aiReasoning = writable<string>("");
/// AI 当前举棋的走法（UCI，如 "e2e4"），前端据此播放"举棋犹豫"动画
/// 为 null 时表示未举棋
export const aiPick = writable<string | null>(null);
/// AI 走棋失败标志（true 时底部显示「重新请求」按钮）
export const aiFailed = writable<boolean>(false);
/// 重试信号计数器：每次 +1 触发 Board 重新调用 AI 走棋
export const retrySignal = writable<number>(0);
/// 是否在棋盘上显示 AI 思考浮层（持久化到 localStorage）
export const showThinking = writable<boolean>(
  typeof localStorage !== "undefined" &&
    localStorage.getItem("chess_show_thinking") === "false"
    ? false
    : true
);
showThinking.subscribe((v) => {
  if (typeof localStorage !== "undefined") {
    localStorage.setItem("chess_show_thinking", String(v));
  }
});

/// 当前选中的格子的合法走法
export const legalMovesForSelected = derived(
  [gameState, selectedSquare],
  ([$game, $sel]) => {
    if (!$sel) return [];
    return $game.legal_moves.filter((m) => m.startsWith($sel));
  }
);

/// 游戏是否进行中
export const isPlaying = derived(gameState, ($g) =>
  $g.status === "playing" || $g.status === "thinking"
);

/// 是否轮到玩家
export const isPlayerTurn = derived(gameState, ($g) =>
  $g.turn === $g.player_side && ($g.status === "playing")
);

/// 更新游戏状态
export function updateGameState(state: GameStateDto) {
  gameState.set(state);
}

/// 显示错误（3秒后自动清除）
export function showError(msg: string) {
  errorMsg.set(msg);
  setTimeout(() => errorMsg.set(null), 3000);
}
