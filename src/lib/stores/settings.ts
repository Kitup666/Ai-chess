import { writable } from "svelte/store";
import type { Side, PlayerType } from "../types";

export interface Settings {
  apiKey: string;
  model: string;
  /// 是否启用思考模式（仅对支持思考的模型生效）
  thinking: boolean;
  /// 伪思考模式：关闭 API thinking，改用提示词在 content 中用 <LMTHINK>..</LMTHINK> 模拟思考
  /// 完全由提示词控制输出格式，省 thinking 模式输出 token 成本
  pseudoThinking: boolean;
  /// 思考语言："zh" | "en"，影响 AI 思考提示词与显示语言
  thinkingLanguage: "zh" | "en";
  /// reasoning_effort: "high" | "max"（DeepSeek 仅支持这两档），仅 thinking 开启时生效
  reasoningEffort: "high" | "max";
  /// 最少思考 token 数：注入提示词要求 AI 至少输出这么多 token 的思考内容
  /// 0 表示不限制。用于强制 AI 深度思考。
  minThinkingTokens: number;
  /// Self-Consistency 多采样次数（1=关闭，>1=多次采样取多数走法）
  selfConsistencySamples: number;
  /// 思考链显示位置："board"=棋盘旁（默认，单行轮换），"left"=左边栏（多行滚动）
  thinkingPosition: "board" | "left";
  side: Side;
  started: boolean;
  /// 白方主体类型
  whitePlayer: PlayerType;
  /// 黑方主体类型
  blackPlayer: PlayerType;
  /// 鳕鱼 ELO 等级（1320-3190），useStockfishElo=true 时生效
  stockfishElo: number;
  /// 鳕鱼 Skill Level（0-20），useStockfishElo=false 时生效
  stockfishSkill: number;
  /// 是否用 ELO 模式控制鳕鱼难度（true=UCI_Elo，false=Skill Level）
  useStockfishElo: boolean;
}

/// 鳕鱼难度相关设置的 localStorage 持久化
/// 这三项设置仅前端使用（不经过后端），独立持久化到 localStorage
/// 后端 SettingsSave 不包含这些字段，避免 Rust 端结构变更
const SF_ELO_KEY = "chess_sf_elo";
const SF_SKILL_KEY = "chess_sf_skill";
const SF_USE_ELO_KEY = "chess_sf_use_elo";
const THINKING_POS_KEY = "chess_thinking_pos";

function loadSfElo(): number {
  if (typeof localStorage === "undefined") return 1500;
  const v = parseInt(localStorage.getItem(SF_ELO_KEY) || "", 10);
  return Number.isFinite(v) && v >= 1320 && v <= 3190 ? v : 1500;
}
function loadSfSkill(): number {
  if (typeof localStorage === "undefined") return 10;
  const v = parseInt(localStorage.getItem(SF_SKILL_KEY) || "", 10);
  return Number.isFinite(v) && v >= 0 && v <= 20 ? v : 10;
}
function loadSfUseElo(): boolean {
  if (typeof localStorage === "undefined") return true;
  return localStorage.getItem(SF_USE_ELO_KEY) !== "false";
}
function loadThinkingPos(): "board" | "left" {
  if (typeof localStorage === "undefined") return "board";
  const v = localStorage.getItem(THINKING_POS_KEY);
  if (v === "left") return "left";
  return "board";
}

const initialSettings: Settings = {
  apiKey: "",
  model: "deepseek-v4-flash",
  thinking: false,
  pseudoThinking: false,
  thinkingLanguage: "zh",
  reasoningEffort: "high",
  minThinkingTokens: 0,
  selfConsistencySamples: 1,
  thinkingPosition: loadThinkingPos(),
  side: "white",
  started: false,
  whitePlayer: "human",
  blackPlayer: "deepseek",
  stockfishElo: loadSfElo(),
  stockfishSkill: loadSfSkill(),
  useStockfishElo: loadSfUseElo(),
};

export const settings = writable<Settings>(initialSettings);

/// 订阅 stockfish 难度设置变化，持久化到 localStorage
settings.subscribe((s) => {
  if (typeof localStorage === "undefined") return;
  localStorage.setItem(SF_ELO_KEY, String(s.stockfishElo));
  localStorage.setItem(SF_SKILL_KEY, String(s.stockfishSkill));
  localStorage.setItem(SF_USE_ELO_KEY, String(s.useStockfishElo));
  localStorage.setItem(THINKING_POS_KEY, s.thinkingPosition);
});

export function updateSettings(partial: Partial<Settings>) {
  settings.update((s) => ({ ...s, ...partial }));
}
