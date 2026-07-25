<script lang="ts">
  /// 引擎分析面板（多 PV 变着列表 + 控制）
  ///
  /// 作为右侧抽屉内容，显示：
  /// - 控制区：深度滑块、MultiPV 数量、暂停/继续、重新分析、停止
  /// - 进度区：当前深度、节点数、NPS、搜索时间
  /// - 多 PV 列表：前 N 条主要变着（SAN 走法序列 + 评估值 + 深度）
  ///
  /// 点击某条 PV 可高亮对应走法箭头（通过 highlightedPV store）

  import {
    multiPVList,
    searchProgress,
    isAnalyzing,
    isPaused,
    multiPVCount,
    analysisDepth,
    engineStatus,
    scoreFromWhitePerspective,
    pauseAnalysis,
    resumeAnalysis,
    reanalyze,
    stopAnalysis,
    applyMultiPV,
    applyAnalysisDepth,
    loadEngine,
    analyzePosition,
  } from "../stockfish/store";
  import { gameState } from "../stores/game";
  import { uciMovesToSanString } from "../stockfish/san";
  import { highlightedPV } from "../stockfish/highlight";

  // 当前局面
  let fen = $derived($gameState.fen);
  let turn = $derived($gameState.turn as "white" | "black");

  // 每条 PV 的显示数据
  let pvLines = $derived.by(() => {
    const list = $multiPVList;
    if (list.length === 0) return [];
    return list.map((info, idx) => {
      const score = scoreFromWhitePerspective(info.score, turn);
      let evalText = "";
      let isMate = false;
      if (score !== null) {
        if (score >= 100000) {
          isMate = true;
          evalText = `+M${100000 - score + 1}`;
        } else if (score <= -100000) {
          isMate = true;
          evalText = `-M${100000 - Math.abs(score) + 1}`;
        } else {
          const val = score / 100;
          evalText = (val >= 0 ? "+" : "") + val.toFixed(2);
        }
      }
      const sanStr = info.pv ? uciMovesToSanString(fen, info.pv) : "";
      return {
        idx: idx + 1,
        evalText,
        isMate,
        score,
        depth: info.depth,
        seldepth: info.seldepth ?? 0,
        sanStr,
        pv: info.pv ?? [],
      };
    });
  });

  let progress = $derived($searchProgress);
  let analyzing = $derived($isAnalyzing);
  let paused = $derived($isPaused);
  let loading = $derived($engineStatus === "loading");
  let canControl = $derived($engineStatus === "ready" || $engineStatus === "searching");

  // 深度滑块
  let depthValue = $derived($analysisDepth);
  function onDepthChange(e: Event) {
    const v = parseInt((e.target as HTMLInputElement).value, 10);
    applyAnalysisDepth(v);
    if (analyzing || paused) reanalyze();
  }

  // MultiPV 选择
  let mpvValue = $derived($multiPVCount);
  function onMultiPVChange(e: Event) {
    const v = parseInt((e.target as HTMLSelectElement).value, 10);
    applyMultiPV(v);
  }

  function handlePauseResume() {
    if (analyzing) pauseAnalysis();
    else if (paused) resumeAnalysis();
  }

  function handleStop() {
    stopAnalysis();
  }

  function handleReanalyze() {
    reanalyze();
  }

  // 高亮某条 PV（鼠标 hover 或点击）
  function highlightPV(idx: number | null) {
    highlightedPV.set(idx);
  }

  // 格式化节点数
  function fmtNodes(n: number): string {
    if (n >= 1_000_000) return (n / 1_000_000).toFixed(2) + "M";
    if (n >= 1_000) return (n / 1_000).toFixed(1) + "K";
    return String(n);
  }

  // 格式化时间
  function fmtTime(ms: number): string {
    if (ms >= 1000) return (ms / 1000).toFixed(1) + "s";
    return ms + "ms";
  }

  // 格式化 NPS
  function fmtNps(nps: number): string {
    if (nps >= 1_000_000) return (nps / 1_000_000).toFixed(2) + "M nps";
    if (nps >= 1_000) return (nps / 1_000).toFixed(0) + "K nps";
    return nps + " nps";
  }

  // 导出关闭回调（仅窄屏抽屉模式可见）
  let { onClose = () => {}, closable = false }: { onClose?: () => void; closable?: boolean } = $props();

  // 引擎未加载时启用引擎
  async function handleEnableEngine() {
    try {
      await loadEngine();
      await analyzePosition($gameState.fen);
    } catch (e) {
      console.error("启用引擎失败:", e);
    }
  }
</script>

<div class="analysis-panel">
  <!-- 窄屏抽屉模式：右上角关闭按钮 -->
  {#if closable}
    <button class="close-btn" onclick={onClose} aria-label="关闭分析面板">✕</button>
  {/if}

  <!-- 引擎未加载占位 -->
  {#if $engineStatus === "unloaded"}
    <div class="empty-placeholder">
      <div class="placeholder-icon">♞</div>
      <div class="placeholder-text">启用鳕鱼引擎获取实时评估</div>
      <button class="enable-btn" onclick={handleEnableEngine}>启用引擎</button>
    </div>
  {:else}
    <!-- 控制区（紧凑一行） -->
  <section class="control-section">
    <div class="control-row">
      <label class="control-label" for="sf-depth-slider">
        <span class="label-text">深度</span>
        <span class="label-value">{depthValue}</span>
      </label>
      <input
        id="sf-depth-slider"
        type="range"
        min="1"
        max="30"
        step="1"
        value={depthValue}
        oninput={onDepthChange}
        class="depth-slider"
        disabled={!canControl}
      />
      <select
        aria-label="多 PV 数量"
        value={mpvValue}
        onchange={onMultiPVChange}
        class="mpv-select"
        disabled={!canControl}
      >
        <option value={1}>1 PV</option>
        <option value={2}>2 PV</option>
        <option value={3}>3 PV</option>
        <option value={4}>4 PV</option>
        <option value={5}>5 PV</option>
      </select>
      <div class="button-row">
        <button
          class="ctrl-btn icon-btn"
          onclick={handlePauseResume}
          disabled={!canControl || loading}
          title={analyzing ? "暂停" : paused ? "继续" : "开始"}
          aria-label={analyzing ? "暂停" : paused ? "继续" : "开始"}
        >
          {analyzing ? "⏸" : paused ? "▶" : "▶"}
        </button>
        <button
          class="ctrl-btn icon-btn"
          onclick={handleReanalyze}
          disabled={!canControl || loading}
          title="重新分析"
          aria-label="重新分析"
        >
          ↻
        </button>
        <button
          class="ctrl-btn icon-btn danger"
          onclick={handleStop}
          disabled={!analyzing && !paused}
          title="停止"
          aria-label="停止"
        >
          ⏹
        </button>
      </div>
    </div>
  </section>

  <!-- 进度区 -->
  {#if progress.depth > 0 || analyzing}
    <section class="progress-section">
      <div class="progress-grid">
        <div class="prog-cell">
          <span class="prog-label">深度</span>
          <span class="prog-value">{progress.depth}{#if progress.seldepth > progress.depth}/{progress.seldepth}{/if}</span>
        </div>
        <div class="prog-cell">
          <span class="prog-label">节点</span>
          <span class="prog-value">{fmtNodes(progress.nodes)}</span>
        </div>
        <div class="prog-cell">
          <span class="prog-label">速度</span>
          <span class="prog-value">{fmtNps(progress.nps)}</span>
        </div>
        <div class="prog-cell">
          <span class="prog-label">时间</span>
          <span class="prog-value">{fmtTime(progress.time)}</span>
        </div>
      </div>
    </section>
  {/if}

  <!-- 多 PV 列表 -->
  <section class="pv-list-section">
    {#if loading}
      <div class="empty-state">引擎加载中…</div>
    {:else if pvLines.length === 0}
      <div class="empty-state">
        {analyzing ? "搜索中…" : "点击「开始」分析当前局面"}
      </div>
    {:else}
      <div class="pv-list">
        {#each pvLines as line (line.idx)}
          <div
            class="pv-card"
            class:highlighted={$highlightedPV === line.idx}
            onmouseenter={() => highlightPV(line.idx)}
            onmouseleave={() => highlightPV(null)}
            onclick={() => highlightPV($highlightedPV === line.idx ? null : line.idx)}
            onkeydown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); highlightPV($highlightedPV === line.idx ? null : line.idx); } }}
            role="button"
            tabindex="0"
          >
            <div class="pv-header">
              <span class="pv-index">{line.idx}</span>
              <span
                class="pv-eval"
                class:positive={line.score !== null && line.score > 0}
                class:negative={line.score !== null && line.score < 0}
                class:mate={line.isMate}
              >
                {line.evalText || "—"}
              </span>
              <span class="pv-depth">d{line.depth}</span>
            </div>
            <div class="pv-moves">{line.sanStr || "（无走法）"}</div>
          </div>
        {/each}
      </div>
    {/if}
  </section>
  {/if}
</div>

<style>
  .analysis-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg);
    overflow: hidden;
    position: relative;
  }

  /* 窄屏抽屉模式关闭按钮 */
  .close-btn {
    position: absolute;
    top: var(--sp-2);
    right: var(--sp-2);
    width: 28px;
    height: 28px;
    border: none;
    background: transparent;
    color: var(--ink-muted);
    font-size: 14px;
    cursor: pointer;
    border-radius: var(--r-sm);
    z-index: 10;
    transition: background 0.2s var(--ease), color 0.2s var(--ease);
  }
  .close-btn:hover {
    background: var(--surface-2);
    color: var(--ink);
  }

  /* 引擎未加载占位 */
  .empty-placeholder {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--sp-3);
    padding: var(--sp-5);
    text-align: center;
  }
  .placeholder-icon {
    font-size: 48px;
    color: var(--ink-faint);
    line-height: 1;
  }
  .placeholder-text {
    font-size: 13px;
    color: var(--ink-muted);
    line-height: 1.6;
  }
  .enable-btn {
    margin-top: var(--sp-2);
    padding: 8px 20px;
    border: 1px solid var(--accent);
    border-radius: var(--r-md);
    background: var(--accent);
    color: #fff;
    font-size: 13px;
    font-family: inherit;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s var(--ease);
  }
  .enable-btn:hover {
    background: var(--accent);
    filter: brightness(1.1);
    transform: translateY(-1px);
  }

  /* 控制区（紧凑一行） */
  .control-section {
    padding: var(--sp-2) var(--sp-3);
    border-bottom: 1px solid var(--line);
    flex-shrink: 0;
  }
  .control-row {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
  }
  .control-label {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 11px;
    color: var(--ink-muted);
    flex-shrink: 0;
  }
  .label-text {
    font-weight: 500;
  }
  .label-value {
    font-family: var(--font-mono);
    color: var(--ink);
    font-weight: 600;
    min-width: 18px;
    text-align: right;
  }

  .depth-slider {
    flex: 1;
    min-width: 60px;
    height: 4px;
    -webkit-appearance: none;
    appearance: none;
    background: var(--surface-2);
    border-radius: 2px;
    outline: none;
    cursor: pointer;
  }
  .depth-slider::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--accent);
    cursor: pointer;
    border: 2px solid var(--bg);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
  }
  .depth-slider::-moz-range-thumb {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--accent);
    cursor: pointer;
    border: 2px solid var(--bg);
  }
  .depth-slider:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .mpv-select {
    width: auto;
    padding: 3px 6px;
    border: 1px solid var(--line);
    border-radius: var(--r-sm);
    background: var(--surface);
    color: var(--ink);
    font-size: 11px;
    font-family: inherit;
    cursor: pointer;
    flex-shrink: 0;
  }
  .mpv-select:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .button-row {
    display: flex;
    gap: 4px;
    flex-shrink: 0;
  }
  .ctrl-btn {
    padding: 5px 8px;
    border: 1px solid var(--line);
    border-radius: var(--r-sm);
    background: var(--surface);
    color: var(--ink);
    font-size: 12px;
    font-family: inherit;
    cursor: pointer;
    transition: all 0.2s var(--ease);
  }
  .ctrl-btn.icon-btn {
    width: 28px;
    height: 28px;
    padding: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 13px;
  }
  .ctrl-btn:hover:not(:disabled) {
    background: var(--surface-2);
    border-color: var(--accent);
    color: var(--accent);
  }
  .ctrl-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .ctrl-btn.danger:hover:not(:disabled) {
    border-color: var(--danger);
    color: var(--danger);
  }

  /* 进度区 */
  .progress-section {
    padding: var(--sp-2) var(--sp-3);
    border-bottom: 1px solid var(--line);
    flex-shrink: 0;
  }
  .progress-grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: var(--sp-2);
  }
  .prog-cell {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .prog-label {
    font-size: 10px;
    color: var(--ink-faint);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .prog-value {
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: 600;
    color: var(--ink);
  }

  /* 多 PV 列表 */
  .pv-list-section {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: var(--sp-2) var(--sp-3);
  }
  .empty-state {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    min-height: 120px;
    color: var(--ink-faint);
    font-size: 13px;
    text-align: center;
  }

  .pv-list {
    display: flex;
    flex-direction: column;
    gap: var(--sp-2);
  }
  .pv-card {
    padding: var(--sp-2) var(--sp-3);
    border: 1px solid var(--line);
    border-radius: var(--r-sm);
    background: var(--surface);
    cursor: pointer;
    transition: all 0.15s var(--ease);
  }
  .pv-card:hover,
  .pv-card.highlighted {
    border-color: var(--accent);
    background: rgba(46, 74, 62, 0.04);
  }
  .pv-card.highlighted {
    box-shadow: 0 0 0 1px var(--accent);
  }

  .pv-header {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    margin-bottom: 4px;
  }
  .pv-index {
    font-size: 10px;
    font-weight: 600;
    color: var(--ink-faint);
    background: var(--surface-2);
    padding: 1px 5px;
    border-radius: 2px;
    min-width: 16px;
    text-align: center;
  }
  .pv-eval {
    font-family: var(--font-mono);
    font-size: 12px;
    font-weight: 700;
    color: var(--ink);
  }
  .pv-eval.positive {
    color: var(--accent);
  }
  .pv-eval.negative {
    color: var(--danger);
  }
  .pv-eval.mate {
    color: var(--highlight);
  }
  .pv-depth {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--ink-faint);
    margin-left: auto;
  }

  .pv-moves {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--ink-muted);
    line-height: 1.5;
    word-break: break-word;
  }
</style>
