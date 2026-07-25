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
  white_player: "human",
  black_player: "deepseek",
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

/// 当前轮到方主体是否为 Human（用于决定是否允许手动点击走棋）
/// 三方主体架构下：白方 turn 看 white_player，黑方 turn 看 black_player
export const isPlayerTurn = derived(gameState, ($g) => {
  if ($g.status !== "playing") return false;
  const currentPlayer = $g.turn === "white" ? $g.white_player : $g.black_player;
  return currentPlayer === "human";
});

/// 更新游戏状态
export function updateGameState(state: GameStateDto) {
  gameState.set(state);
}

/// 显示错误（3秒后自动清除）
export function showError(msg: string) {
  errorMsg.set(msg);
  setTimeout(() => errorMsg.set(null), 3000);
}
