import { writable } from "svelte/store";

/// 后端推送的 token 用量（与 deepseek.rs 的 Usage 对应）
export interface Usage {
  prompt_tokens: number;
  completion_tokens: number;
  total_tokens: number;
  prompt_cache_hit_tokens: number;
  prompt_cache_miss_tokens: number;
}

/// 底部状态栏显示的成本统计
///
/// - 本轮（lastUsage）：最近一次 AI 走棋的输入/输出/缓存命中 token
/// - 今日累计（dailyCost）：当日所有 AI 调用的耗费，按日期重置
/// - 缓存命中率：本轮缓存命中 token / 本轮输入 token
export interface CostStats {
  /// 本轮用量（最近一次 AI 走棋）
  lastUsage: Usage | null;
  /// 今日累计耗费（元）
  dailyCost: number;
  /// 今日累计 token 用量
  dailyPromptTokens: number;
  dailyCompletionTokens: number;
  dailyCachedTokens: number;
}

/// 持久化到 localStorage 的每日累计数据
interface DailyPersist {
  date: string; // YYYY-MM-DD
  cost: number;
  promptTokens: number;
  completionTokens: number;
  cachedTokens: number;
}

const STORAGE_KEY = "chess_daily_cost";

/// 获取今日日期字符串（本地时区 YYYY-MM-DD）
function todayStr(): string {
  const d = new Date();
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${y}-${m}-${day}`;
}

/// 读取持久化的每日数据，日期不符则重置
function loadDaily(): DailyPersist {
  if (typeof localStorage === "undefined") return emptyDaily();
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return emptyDaily();
    const data = JSON.parse(raw) as DailyPersist;
    if (data.date !== todayStr()) return emptyDaily();
    return data;
  } catch {
    return emptyDaily();
  }
}

function emptyDaily(): DailyPersist {
  return {
    date: todayStr(),
    cost: 0,
    promptTokens: 0,
    completionTokens: 0,
    cachedTokens: 0,
  };
}

/// 保存每日数据到 localStorage
function saveDaily(data: DailyPersist): void {
  if (typeof localStorage === "undefined") return;
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(data));
  } catch {
    // 忽略写入失败
  }
}

/// DeepSeek 定价（单位：元/百万 token）
///
/// 官方定价（https://api-docs.deepseek.com/zh-cn/quick_start/pricing）：
/// - deepseek-v4-flash: 输入命中 0.02 / 未命中 1 / 输出 2
/// - deepseek-v4-pro:   输入命中 0.025 / 未命中 3 / 输出 6
interface PriceTier {
  inputHit: number;   // 元/百万
  inputMiss: number;  // 元/百万
  output: number;     // 元/百万
}

const PRICE_TIERS: Record<string, PriceTier> = {
  "deepseek-v4-flash": { inputHit: 0.02, inputMiss: 1, output: 2 },
  "deepseek-v4-pro": { inputHit: 0.025, inputMiss: 3, output: 6 },
};

/// 兜底定价（未知模型用 flash 价）
const DEFAULT_TIER: PriceTier = PRICE_TIERS["deepseek-v4-flash"];

function tierFor(model: string): PriceTier {
  return PRICE_TIERS[model] ?? DEFAULT_TIER;
}

/// 计算单次调用的成本（元），按模型选择定价
function calcCost(u: Usage, model: string): number {
  const tier = tierFor(model);
  const inputMiss = u.prompt_tokens - u.prompt_cache_hit_tokens;
  const inputHit = u.prompt_cache_hit_tokens;
  const output = u.completion_tokens;
  return (
    (inputMiss * tier.inputMiss + inputHit * tier.inputHit + output * tier.output)
    / 1_000_000
  );
}

/// 初始化 store
const initial: CostStats = {
  lastUsage: null,
  dailyCost: loadDaily().cost,
  dailyPromptTokens: loadDaily().promptTokens,
  dailyCompletionTokens: loadDaily().completionTokens,
  dailyCachedTokens: loadDaily().cachedTokens,
};

export const costStats = writable<CostStats>(initial);

/// 更新本轮用量并累加到今日统计
///
/// 由 api.ts 的 onAiUsage 监听器调用，传入当前模型名以选择正确定价。
export function updateCostStats(usage: Usage, model: string): void {
  costStats.update((stats) => {
    const daily = loadDaily();
    // 累加今日统计
    daily.promptTokens += usage.prompt_tokens;
    daily.completionTokens += usage.completion_tokens;
    daily.cachedTokens += usage.prompt_cache_hit_tokens;
    daily.cost += calcCost(usage, model);
    saveDaily(daily);

    return {
      lastUsage: usage,
      dailyCost: daily.cost,
      dailyPromptTokens: daily.promptTokens,
      dailyCompletionTokens: daily.completionTokens,
      dailyCachedTokens: daily.cachedTokens,
    };
  });
}

/// 计算本轮缓存命中率（百分比，保留 1 位小数）
export function lastCacheHitRate(usage: Usage | null): number {
  if (!usage || usage.prompt_tokens === 0) return 0;
  return Math.round((usage.prompt_cache_hit_tokens / usage.prompt_tokens) * 1000) / 10;
}
