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

const initialSettings: Settings = {
  apiKey: "",
  model: "deepseek-v4-flash",
  thinking: false,
  pseudoThinking: false,
  thinkingLanguage: "zh",
  reasoningEffort: "high",
  minThinkingTokens: 0,
  selfConsistencySamples: 1,
  side: "white",
  started: false,
  whitePlayer: "human",
  blackPlayer: "deepseek",
  stockfishElo: 1500,
  stockfishSkill: 10,
  useStockfishElo: true,
};

export const settings = writable<Settings>(initialSettings);

export function updateSettings(partial: Partial<Settings>) {
  settings.update((s) => ({ ...s, ...partial }));
}
