import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { GameStateDto, LoadedState, MoveResult, StartGameArgs, Side, PlayerType } from "./types";
import type { Usage } from "./stores/cost";

/// 开始新游戏
export async function startGame(args: StartGameArgs): Promise<GameStateDto> {
  return invoke<GameStateDto>("start_game", { args });
}

/// 玩家走棋
export async function playerMove(moveStr: string): Promise<MoveResult> {
  return invoke<MoveResult>("player_move", { moveStr });
}

/// DeepSeek AI 走棋
export async function aiMove(): Promise<MoveResult> {
  return invoke<MoveResult>("ai_move");
}

/// 获取当前游戏状态
export async function getGameState(): Promise<GameStateDto> {
  return invoke<GameStateDto>("get_game_state");
}

/// 重置游戏（保留 DeepSeek 配置，可更换主体组合）
/// @param side Human 视角执方（双自动或双人时仅用于初始化 turn 推导兜底）
/// @param whitePlayer 白方主体
/// @param blackPlayer 黑方主体
export async function resetGame(
  side: Side,
  whitePlayer: PlayerType = "human",
  blackPlayer: PlayerType = "deepseek"
): Promise<GameStateDto> {
  return invoke<GameStateDto>("reset_game", {
    side,
    whitePlayer,
    blackPlayer,
  });
}

/// 更新 DeepSeek 设置（API Key / 模型 / 思考模式 / 伪思考 / 思考语言 / 思考强度 / 最少思考token / 自洽采样次数），即时生效
export async function updateSettingsApi(
  apiKey: string,
  model: string,
  thinking: boolean,
  pseudoThinking: boolean,
  thinkingLanguage: "zh" | "en",
  reasoningEffort: "high" | "max",
  minThinkingTokens: number,
  selfConsistencySamples: number
): Promise<void> {
  return invoke("update_settings", { apiKey, model, thinking, pseudoThinking, thinkingLanguage, reasoningEffort, minThinkingTokens, selfConsistencySamples });
}

/// 悔棋
export async function undoMove(): Promise<GameStateDto> {
  return invoke<GameStateDto>("undo_move");
}

/// 启动时加载持久化状态（对局 + 设置）
export async function loadState(): Promise<LoadedState | null> {
  return invoke<LoadedState | null>("load_state");
}

/// 监听 AI 思考增量（流式）
export async function onAiThinking(cb: (chunk: string) => void): Promise<UnlistenFn> {
  return listen<string>("ai-thinking", (e) => cb(e.payload));
}

/// 监听 AI 思考重置（重试时清空）
export async function onAiThinkingReset(cb: () => void): Promise<UnlistenFn> {
  return listen("ai-thinking-reset", () => cb());
}

/// 监听 AI 举棋（reasoning 中检测到 <pick>UCI</pick>，前端播放举棋动画）
export async function onAiPick(cb: (uci: string) => void): Promise<UnlistenFn> {
  return listen<string>("ai-pick", (e) => cb(e.payload));
}

/// 监听 AI 举棋重置（清空举棋状态）
export async function onAiPickReset(cb: () => void): Promise<UnlistenFn> {
  return listen("ai-pick-reset", () => cb());
}

/// 监听 AI 本轮 token 用量（底部状态栏显示）
export async function onAiUsage(cb: (usage: Usage) => void): Promise<UnlistenFn> {
  return listen<Usage>("ai-usage", (e) => cb(e.payload));
}
