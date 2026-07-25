/// Stockfish 引擎状态管理（Svelte store）
///
/// 提供引擎单例、状态、评估信息、多PV变着、最佳走法的响应式管理。
/// 难度/深度设置持久化到 localStorage。
///
/// 评估值说明：UCI score 是"当前轮到方"视角（cp 35 表示当前方优势 +0.35）。
/// UI 显示时需根据 FEN 的 turn 转换为白方视角。

import { writable, derived, get } from "svelte/store";
import {
  StockfishEngine,
  type EngineStatus,
  type SearchInfo,
  type BestMove,
} from "./engine";

/// 引擎单例（全局唯一）
let engineInstance: StockfishEngine | null = null;

/// 最近一次分析的 FEN（用于暂停后恢复、重新分析）
let lastAnalyzedFen: string = "";

/// 引擎状态：unloaded | loading | ready | searching
export const engineStatus = writable<EngineStatus>("unloaded");

/// 引擎版本（从 id name 行解析，如 "Stockfish 18 ...")
export const engineVersion = writable<string>("");

/// 最新评估信息（multipv=1 的 info 行，即最佳走法的搜索信息）
/// 保留向后兼容（App.svelte 底部评估显示用）
export const currentInfo = writable<SearchInfo | null>(null);

/// 多 PV 列表：按 multipv 序号排序的 info 数组（index 0 = 最佳）
/// 每个深度递增时实时刷新，深度回退（新搜索）时清空
export const multiPVList = writable<SearchInfo[]>([]);

/// 稳定评估值（只在深度递增时更新，避免 EvalBar 跳动）
/// 同深度内的多次 score 变化不刷新显示
let lastStableDepth = 0;
export const stableScore = writable<{ score: SearchInfo["score"] | null; depth: number } | null>(null);

/// 最近一次搜索的最佳走法结果（bestmove 行）
export const lastBestMove = writable<BestMove | null>(null);

/// 是否正在分析（searching 状态且用于分析模式）
export const isAnalyzing = writable<boolean>(false);

/// 是否暂停（停止搜索但保留结果，可恢复）
export const isPaused = writable<boolean>(false);

/// MultiPV 条数（分析模式输出前 N 条变着，1=只最佳）
export const multiPVCount = writable<number>(3);

/// 难度等级（0-20），持久化
/// 10=中等人类水平，20=最强（默认），0=最弱（随机走法）
export const stockfishSkill = writable<number>(
  typeof localStorage !== "undefined" && localStorage.getItem("chess_sf_skill")
    ? parseInt(localStorage.getItem("chess_sf_skill")!, 10) || 20
    : 20
);
stockfishSkill.subscribe((v) => {
  if (typeof localStorage !== "undefined") {
    localStorage.setItem("chess_sf_skill", String(v));
  }
});

/// 分析深度（plies），持久化
export const analysisDepth = writable<number>(
  typeof localStorage !== "undefined" && localStorage.getItem("chess_sf_depth")
    ? parseInt(localStorage.getItem("chess_sf_depth")!, 10) || 18
    : 18
);
analysisDepth.subscribe((v) => {
  if (typeof localStorage !== "undefined") {
    localStorage.setItem("chess_sf_depth", String(v));
  }
});

/// 搜索进度（来自 multipv=1 的 info 行）
export const searchProgress = writable<{ depth: number; seldepth: number; nodes: number; nps: number; time: number }>({
  depth: 0,
  seldepth: 0,
  nodes: 0,
  nps: 0,
  time: 0,
});

/// 引擎是否已就绪（ready 或 searching）
export const engineReady = derived(engineStatus, (s) => s === "ready" || s === "searching");

/// 最佳走法（multiPVList[0] 的 pv 首步）
export const bestMove = derived(multiPVList, (list) => list[0]?.pv?.[0] ?? null);

/// 最佳评估分数（原始，当前轮到方视角）
export const bestScore = derived(multiPVList, (list) => list[0]?.score ?? null);

/// 当前评估分数（白方视角，厘兵值）
/// 正数=白优，负数=黑优。mate 用大数表示（±100000）
/// 需传入当前轮到方（"white" | "black"）进行视角转换
export function scoreFromWhitePerspective(
  score: { type: "cp" | "mate"; value: number } | undefined,
  turn: "white" | "black"
): number | null {
  if (!score) return null;
  // UCI score 是当前轮到方视角：白方走时正值=白优，黑方走时正值=黑优
  // 转换为白方视角：黑方走时取反
  const raw = score.type === "mate" ? (score.value > 0 ? 100000 : -100000) : score.value;
  return turn === "black" ? -raw : raw;
}

/// 获取或创建引擎实例（单例）
export function getEngine(): StockfishEngine {
  if (!engineInstance) {
    engineInstance = new StockfishEngine();
    engineInstance.onStatusChange = (s) => engineStatus.set(s);
    engineInstance.onInfo = (info) => {
      if (info.depth === 0) return;
      const pv = info.multipv ?? 1;
      // 按 multipv 序号覆盖对应位置
      multiPVList.update((list) => {
        const next = list.slice();
        // 检测深度回退（新一轮搜索）：若新 info 深度小于列表中已有同 pv 的深度，清空重启
        const existing = next[pv - 1];
        if (existing && info.depth < existing.depth) {
          // 新一轮搜索，清空列表只保留当前
          const fresh: SearchInfo[] = [];
          fresh[pv - 1] = info;
          return fresh;
        }
        next[pv - 1] = info;
        return next;
      });
      // multipv=1 的信息同步到 currentInfo 和 searchProgress
      if (pv === 1) {
        currentInfo.set(info);
        searchProgress.set({
          depth: info.depth,
          seldepth: info.seldepth ?? 0,
          nodes: info.nodes ?? 0,
          nps: info.nps ?? 0,
          time: info.time ?? 0,
        });
        // 稳定评估值：只在深度递增时更新，避免 EvalBar 同深度内跳动
        if (info.depth > lastStableDepth) {
          lastStableDepth = info.depth;
          stableScore.set({ score: info.score, depth: info.depth });
        }
      }
    };
    engineInstance.onBestMove = (best) => {
      lastBestMove.set(best);
      isAnalyzing.set(false);
      isPaused.set(false);
      // 搜索完成：用最终深度值更新 stableScore（bestmove 前最后一个 info 即最终深度）
      const finalInfo = get(multiPVList)[0];
      if (finalInfo?.score) {
        stableScore.set({ score: finalInfo.score, depth: finalInfo.depth });
      }
    };
  }
  return engineInstance;
}

/// 加载引擎（如果未加载）
/// 加载后应用难度和 MultiPV 设置
export async function loadEngine(): Promise<void> {
  const engine = getEngine();
  if (engine.status === "unloaded") {
    await engine.load();
    engineVersion.set(engine.version);
    // 应用持久化的设置
    await engine.setSkillLevel(get(stockfishSkill));
    await engine.setMultiPV(get(multiPVCount));
    await engine.newGame();
  }
}

/// 分析指定局面
///
/// @param fen 局面 FEN
/// @param depth 搜索深度（可选，默认用 analysisDepth store 值）
export async function analyzePosition(fen: string, depth?: number): Promise<void> {
  const engine = getEngine();
  await loadEngine();
  lastAnalyzedFen = fen;
  isAnalyzing.set(true);
  isPaused.set(false);
  multiPVList.set([]);
  currentInfo.set(null);
  searchProgress.set({ depth: 0, seldepth: 0, nodes: 0, nps: 0, time: 0 });
  lastStableDepth = 0;
  stableScore.set(null);
  const d = depth ?? get(analysisDepth);
  engine.search(fen, { depth: d });
}

/// 停止分析（完全停止）
export function stopAnalysis(): void {
  if (engineInstance) {
    engineInstance.stop();
    isAnalyzing.set(false);
    isPaused.set(false);
  }
}

/// 暂停分析（停止搜索但保留结果，可恢复）
export function pauseAnalysis(): void {
  if (engineInstance && get(isAnalyzing)) {
    engineInstance.stop();
    isPaused.set(true);
    isAnalyzing.set(false);
  }
}

/// 恢复分析（从暂停状态继续，重新搜索当前局面）
export async function resumeAnalysis(): Promise<void> {
  if (!lastAnalyzedFen) return;
  isPaused.set(false);
  await analyzePosition(lastAnalyzedFen);
}

/// 重新分析当前局面
export async function reanalyze(): Promise<void> {
  if (!lastAnalyzedFen) return;
  await analyzePosition(lastAnalyzedFen);
}

/// 请求最佳走法（Promise 模式，用于引擎对弈）
///
/// @param fen 局面 FEN
/// @param movetime 思考时间（毫秒），默认 1000
export async function getBestMove(fen: string, movetime: number = 1000): Promise<BestMove> {
  const engine = getEngine();
  await loadEngine();
  return engine.getBestMove(fen, { movetime });
}

/// 应用难度设置到引擎
export async function applySkillLevel(level: number): Promise<void> {
  stockfishSkill.set(level);
  const engine = getEngine();
  if (engine.status === "ready" || engine.status === "searching") {
    await engine.setSkillLevel(level);
  }
}

/// 应用 MultiPV 设置到引擎（设置后若正在分析则重新分析以应用新值）
export async function applyMultiPV(n: number): Promise<void> {
  multiPVCount.set(n);
  const engine = getEngine();
  if (engine.status === "ready" || engine.status === "searching") {
    await engine.setMultiPV(n);
    // 若正在分析，重新搜索以应用新的 MultiPV
    if (get(isAnalyzing) && lastAnalyzedFen) {
      await analyzePosition(lastAnalyzedFen);
    }
  }
}

/// 应用分析深度设置（持久化，下次分析生效）
export function applyAnalysisDepth(d: number): void {
  analysisDepth.set(d);
}

/// 销毁引擎（释放 Worker）
export function destroyEngine(): void {
  if (engineInstance) {
    engineInstance.terminate();
    engineInstance = null;
    engineStatus.set("unloaded");
    currentInfo.set(null);
    multiPVList.set([]);
    lastBestMove.set(null);
    isAnalyzing.set(false);
    isPaused.set(false);
    searchProgress.set({ depth: 0, seldepth: 0, nodes: 0, nps: 0, time: 0 });
    lastStableDepth = 0;
    stableScore.set(null);
  }
}
