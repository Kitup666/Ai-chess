<script lang="ts">
  import { onMount } from "svelte";
  import { settings, updateSettings } from "./lib/stores/settings";
  import {
    gameState,
    aiThinking,
    aiReasoning,
    aiPick,
    aiFailed,
    retrySignal,
    showThinking,
    isPlayerTurn,
    showError,
    updateGameState,
    errorMsg,
  } from "./lib/stores/game";
  import {
    costStats,
    updateCostStats,
    lastCacheHitRate,
  } from "./lib/stores/cost";
  import { resetGame, undoMove, aiMove, onAiThinking, onAiThinkingReset, onAiPick, onAiPickReset, onAiUsage, loadState } from "./lib/api";
  import Board from "./lib/components/Board.svelte";
  import Settings from "./lib/components/Settings.svelte";

  // 抽屉展开状态
  let drawerOpen = $state(false);

  let started = $derived($settings.started);
  let gameEnded = $derived(
    $gameState.status === "checkmate" ||
    $gameState.status === "stalemate" ||
    $gameState.status === "draw"
  );
  let canUndo = $derived(
    started &&
    !$aiThinking &&
    $gameState.move_history.length > 0 &&
    !gameEnded
  );
  // 重新请求按钮可点击条件：游戏进行中 + AI 未思考 + 当前是 AI 回合（用户已走完）
  // 正常流程下用户走完会自动触发 AI，此按钮主要在 AI 失败后用于手动重发
  let canRetry = $derived(
    started &&
    !$aiThinking &&
    !$isPlayerTurn &&
    !gameEnded
  );
  let playerTurn = $derived($isPlayerTurn);

  // 状态栏文案
  let statusText = $derived.by(() => {
    const g = $gameState;
    if ($aiThinking) return "DeepSeek 推演中";
    if (g.status === "checkmate") {
      return g.winner === g.player_side ? "你赢了" : "DeepSeek 获胜";
    }
    if (g.status === "stalemate") return "和棋 · 逼和";
    if (g.status === "draw") return "和棋";
    if (g.in_check) return g.turn === g.player_side ? "你被将军" : "DeepSeek 被将军";
    if (!started) return "准备开始";
    return playerTurn ? "你的回合" : "DeepSeek 回合";
  });

  let statusKind = $derived.by(() => {
    const g = $gameState;
    if (g.status === "checkmate") {
      return g.winner === g.player_side ? "win" : "lose";
    }
    if (g.status === "stalemate" || g.status === "draw") return "draw";
    if ($aiThinking) return "thinking";
    if (g.in_check) return "check";
    return "normal";
  });

  // 本轮用量与成本显示
  let lastUsage = $derived($costStats.lastUsage);
  let cacheHitRate = $derived(lastCacheHitRate(lastUsage));
  let dailyCost = $derived($costStats.dailyCost);

  // 监听 AI 思考事件（流式增量 + 重置）+ 启动加载持久化状态
  onMount(() => {
    // 启动加载持久化状态（异步执行，不阻塞 onMount cleanup 返回）
    (async () => {
      try {
        const loaded = await loadState();
        if (loaded) {
          // 恢复设置
          updateSettings({
            apiKey: loaded.settings.api_key,
            model: loaded.settings.model,
            thinking: loaded.settings.thinking,
            pseudoThinking: loaded.settings.pseudo_thinking,
            thinkingLanguage: (loaded.settings.thinking_language === "en" ? "en" : "zh"),
            reasoningEffort: (loaded.settings.reasoning_effort === "max" ? "max" : "high"),
          });
          // 恢复对局
          if (loaded.game) {
            updateGameState(loaded.game);
            updateSettings({ started: true });
          }
        }
      } catch (e) {
        showError(String(e));
      }
    })();

    // 监听 AI 思考事件
    let unlistenThinking: (() => void) | null = null;
    let unlistenReset: (() => void) | null = null;
    let unlistenPick: (() => void) | null = null;
    let unlistenPickReset: (() => void) | null = null;
    let unlistenUsage: (() => void) | null = null;

    onAiThinking((chunk) => {
      aiReasoning.update((s) => s + chunk);
    }).then((fn) => (unlistenThinking = fn));

    onAiThinkingReset(() => {
      aiReasoning.set("");
    }).then((fn) => (unlistenReset = fn));

    // 监听举棋事件：更新当前举起的走法（UCI）
    onAiPick((uci) => {
      aiPick.set(uci);
    }).then((fn) => (unlistenPick = fn));

    onAiPickReset(() => {
      aiPick.set(null);
    }).then((fn) => (unlistenPickReset = fn));

    // 监听本轮 token 用量（底部状态栏显示），传入当前模型选择正确定价
    onAiUsage((usage) => {
      updateCostStats(usage, $settings.model);
    }).then((fn) => (unlistenUsage = fn));

    return () => {
      unlistenThinking?.();
      unlistenReset?.();
      unlistenPick?.();
      unlistenPickReset?.();
      unlistenUsage?.();
    };
  });

  async function handleUndo() {
    if (!canUndo) return;
    try {
      let state = await undoMove();
      if (state.turn !== state.player_side && state.move_history.length > 0) {
        state = await undoMove();
      }
      updateGameState(state);
    } catch (e) {
      showError(String(e));
    }
  }

  async function handleReset() {
    drawerOpen = false;
    try {
      aiReasoning.set("");
      aiPick.set(null);
      aiFailed.set(false);
      const state = await resetGame($settings.side);
      updateGameState(state);
      if ($settings.side === "black") {
        aiThinking.set(true);
        try {
          const result = await aiMove();
          updateGameState(result.state);
        } catch (e) {
          showError(String(e));
          aiFailed.set(true);
        } finally {
          aiThinking.set(false);
          aiPick.set(null);
        }
      }
    } catch (e) {
      showError(String(e));
    }
  }

  function toggleDrawer() {
    drawerOpen = !drawerOpen;
  }

  function handleExit() {
    updateSettings({ started: false });
    drawerOpen = true;
  }

  // AI 回复失败时重新请求：发重试信号给 Board，由 Board 调用 triggerAiMove
  function handleRetry() {
    aiFailed.set(false);
    retrySignal.update((n) => n + 1);
  }
</script>

<main class="app">
  <!-- 顶部极简标题 -->
  <header class="top-bar">
    <div class="brand">
      <span class="brand-mark">♟</span>
      <span class="brand-title">AI Chess · DeepSeek</span>
    </div>
  </header>

  <!-- 中央棋盘区 -->
  <section class="stage">
    {#if started}
      <Board />
    {:else}
      <div class="welcome rise">
        <div class="welcome-title">Welcome</div>
        <div class="welcome-sub">
          点击底部「设置」填入 DeepSeek API Key，<br />
          选择执方后开始对弈。
        </div>
        <div class="welcome-deco">♞</div>
      </div>
    {/if}

    <!-- 全局错误提示 -->
    {#if $errorMsg}
      <div class="toast">{$errorMsg}</div>
    {/if}
  </section>

  <!-- 底部状态栏 -->
  <footer class="bottom-bar">
    <div class="status-cell" data-kind={statusKind}>
      <span class="status-dot"></span>
      <span class="status-text">{statusText}</span>
      {#if started}
        <span class="status-meta">· 第 {Math.floor($gameState.move_history.length / 2) + 1} 回合 · {$gameState.ply} 步</span>
      {/if}
      {#if lastUsage}
        <span class="cost-meta">
          · 入 {lastUsage.prompt_tokens}
          · 出 {lastUsage.completion_tokens}
          · 缓存命中 {lastUsage.prompt_cache_hit_tokens} ({cacheHitRate}%)
          · 今日 ¥{dailyCost.toFixed(4)}
        </span>
      {:else if started && dailyCost > 0}
        <span class="cost-meta">· 今日 ¥{dailyCost.toFixed(4)}</span>
      {/if}
    </div>

    <div class="actions-cell">
      {#if started}
        <button class="bar-btn" onclick={handleUndo} disabled={!canUndo}>悔棋</button>
        <button class="bar-btn" onclick={handleReset}>重开</button>
        <button class="bar-btn retry-btn" class:failed={$aiFailed} onclick={handleRetry} disabled={!canRetry}>重新请求</button>
        <span class="divider"></span>
      {/if}

      <!-- 风的加护开关（显示 AI 思考内容） -->
      <label class="thinking-toggle" class:on={$showThinking}>
        <span class="toggle-label">风的加护</span>
        <button
          class="toggle"
          class:on={$showThinking}
          onclick={() => showThinking.set(!$showThinking)}
          role="switch"
          aria-checked={$showThinking}
          aria-label="显示 AI 思考内容"
        >
          <span class="toggle-thumb"></span>
        </button>
      </label>

      <button class="bar-btn settings-btn" class:active={drawerOpen} onclick={toggleDrawer}>
        设置
      </button>
    </div>
  </footer>

  <!-- 设置抽屉（向上展开） -->
  <div
    class="drawer-mask"
    class:show={drawerOpen}
    role="button"
    tabindex="-1"
    aria-label="关闭设置"
    onclick={() => (drawerOpen = false)}
    onkeydown={(e) => { if (e.key === "Escape" || e.key === "Enter") drawerOpen = false; }}
  ></div>
  <aside class="drawer" class:open={drawerOpen}>
    <div class="drawer-header">
      <span class="drawer-title">设置</span>
      <button class="drawer-close" onclick={() => (drawerOpen = false)}>✕</button>
    </div>
    <div class="drawer-body">
      <Settings onStarted={() => (drawerOpen = false)} onExit={handleExit} />
    </div>
  </aside>
</main>

<style>
  .app {
    display: flex;
    flex-direction: column;
    height: 100vh;
    width: 100vw;
    overflow: hidden;
  }

  /* ===== 顶部标题栏 ===== */
  .top-bar {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--sp-3) var(--sp-5);
    border-bottom: 1px solid transparent;
    background:
      linear-gradient(to right, transparent, var(--line), transparent) bottom / 100% 1px no-repeat,
      var(--bg);
    flex-shrink: 0;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
  }
  .brand-mark {
    font-size: 18px;
    color: var(--accent);
    opacity: 0.7;
    line-height: 1;
  }
  .brand-title {
    font-family: var(--font-display);
    font-size: 14px;
    font-weight: 500;
    font-style: italic;
    color: var(--ink);
    letter-spacing: 0.04em;
  }

  /* ===== 中央舞台 ===== */
  .stage {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    position: relative;
    min-height: 0;
    overflow: hidden;
  }

  /* ===== 欢迎页 ===== */
  .welcome {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--sp-3);
    text-align: center;
    animation-delay: 0.2s;
  }
  .welcome-title {
    font-family: var(--font-display);
    font-size: 88px;
    font-weight: 300;
    font-style: italic;
    color: var(--ink);
    letter-spacing: -0.03em;
  }
  .welcome-sub {
    font-size: 13px;
    color: var(--ink-muted);
    line-height: 2;
  }
  .welcome-deco {
    font-size: 120px;
    color: var(--accent);
    opacity: 0.18;
    margin-top: var(--sp-5);
    line-height: 1;
    animation: sway 6s var(--ease) infinite;
  }
  @keyframes sway {
    0%, 100% { transform: rotate(-8deg); }
    50% { transform: rotate(8deg); }
  }

  /* ===== 底部状态栏 ===== */
  .bottom-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 var(--sp-5);
    height: 60px;
    background: var(--bg);
    border-top: 1px solid var(--line);
    flex-shrink: 0;
    z-index: 10;
  }

  .status-cell {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    font-size: 13px;
    color: var(--ink);
  }
  .status-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--ink-faint);
    transition: background 0.3s var(--ease), box-shadow 0.3s var(--ease);
  }
  .status-cell[data-kind="normal"] .status-dot {
    background: var(--accent);
    box-shadow: 0 0 0 3px rgba(46, 74, 62, 0.08);
  }
  .status-cell[data-kind="thinking"] .status-dot {
    background: var(--highlight);
    box-shadow: 0 0 0 3px rgba(201, 169, 97, 0.12);
    animation: breathe 1.4s var(--ease) infinite;
  }
  .status-cell[data-kind="check"] {
    color: var(--danger);
  }
  .status-cell[data-kind="check"] .status-dot {
    background: var(--danger);
  }
  .status-cell[data-kind="win"] {
    color: var(--accent);
    font-weight: 500;
  }
  .status-cell[data-kind="win"] .status-dot {
    background: var(--accent);
    box-shadow: 0 0 0 3px rgba(46, 74, 62, 0.12);
  }
  .status-cell[data-kind="lose"] {
    color: var(--danger);
    font-weight: 500;
  }
  .status-cell[data-kind="lose"] .status-dot {
    background: var(--danger);
  }
  .status-meta {
    color: var(--ink-muted);
    font-family: var(--font-mono);
    font-size: 10px;
    letter-spacing: 0.05em;
  }
  .cost-meta {
    color: var(--ink-faint);
    font-family: var(--font-mono);
    font-size: 10px;
    letter-spacing: 0.03em;
    margin-left: var(--sp-1);
  }

  .actions-cell {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
  }
  .bar-btn {
    padding: var(--sp-2) var(--sp-3);
    background: transparent;
    color: var(--ink);
    border: none;
    font-family: var(--font-sans);
    font-size: 12px;
    cursor: pointer;
    transition: background 0.2s var(--ease), transform 0.2s var(--ease);
    border-radius: var(--r-sm);
  }
  .bar-btn:hover:not(:disabled) {
    background: var(--surface-2);
    transform: translateY(-1px);
  }
  .bar-btn:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }
  .bar-btn.active {
    background: var(--accent);
    color: #fff;
  }
  .bar-btn.retry-btn {
    color: var(--ink-muted);
  }
  .bar-btn.retry-btn.failed {
    color: #b8442e;
    font-weight: 600;
  }
  .bar-btn.retry-btn.failed:hover:not(:disabled) {
    background: rgba(184, 68, 46, 0.1);
  }
  .bar-btn.retry-btn:hover:not(:disabled):not(.failed) {
    background: var(--surface-2);
  }
  .divider {
    width: 1px;
    height: 20px;
    background: var(--ink-faint);
    opacity: 0.3;
    margin: 0 var(--sp-1);
  }

  /* ===== 思考开关 ===== */
  .thinking-toggle {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    cursor: pointer;
    padding: var(--sp-1) var(--sp-2);
    border-radius: var(--r-sm);
    transition: background 0.2s var(--ease);
  }
  .thinking-toggle:hover {
    background: var(--surface-2);
  }
  .toggle-label {
    font-size: 12px;
    color: var(--ink-muted);
    transition: color 0.2s var(--ease);
  }
  .thinking-toggle.on .toggle-label {
    color: var(--accent);
  }
  .toggle {
    width: 36px;
    height: 20px;
    border: 1px solid var(--line);
    background: var(--bg-soft);
    position: relative;
    cursor: pointer;
    padding: 0;
    transition: background 0.2s var(--ease), border-color 0.2s var(--ease);
    border-radius: 0;
  }
  .toggle.on {
    background: var(--accent);
    border-color: var(--accent);
  }
  .toggle-thumb {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 14px;
    height: 14px;
    background: var(--ink);
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.15);
    transition: transform 0.2s var(--ease);
  }
  .toggle.on .toggle-thumb {
    transform: translateX(16px);
    background: var(--board-light);
  }

  /* ===== 设置抽屉 ===== */
  .drawer-mask {
    position: fixed;
    inset: 0;
    background: rgba(27, 26, 23, 0.35);
    opacity: 0;
    pointer-events: none;
    transition: opacity 0.3s var(--ease);
    z-index: 20;
  }
  .drawer-mask.show {
    opacity: 1;
    pointer-events: auto;
  }

  .drawer {
    position: fixed;
    left: 0;
    right: 0;
    bottom: 0;
    background: var(--surface);
    border-top: 1px solid var(--line);
    border-radius: var(--r-lg) var(--r-lg) 0 0;
    box-shadow: 0 -12px 40px rgba(27, 26, 23, 0.12);
    transform: translateY(100%);
    transition: transform 0.4s var(--ease);
    z-index: 21;
    max-height: 75vh;
    display: flex;
    flex-direction: column;
  }
  .drawer.open {
    transform: translateY(0);
  }
  .drawer-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--sp-4) var(--sp-5) var(--sp-3);
    border-bottom: 1px solid var(--line);
    flex-shrink: 0;
    position: relative;
  }
  .drawer-header::before {
    content: "";
    position: absolute;
    top: 8px;
    left: 50%;
    transform: translateX(-50%);
    width: 36px;
    height: 4px;
    background: var(--line);
    border-radius: 2px;
  }
  .drawer-title {
    font-family: var(--font-display);
    font-size: 18px;
    font-weight: 500;
    color: var(--ink);
    letter-spacing: 0.01em;
  }
  .drawer-close {
    background: transparent;
    border: none;
    color: var(--ink-muted);
    cursor: pointer;
    font-size: 16px;
    padding: var(--sp-1) var(--sp-2);
    border-radius: var(--r-sm);
    transition: background 0.2s var(--ease), color 0.2s var(--ease);
  }
  .drawer-close:hover {
    background: var(--surface-2);
    color: var(--ink);
  }
  .drawer-body {
    flex: 1;
    overflow-y: auto;
    padding: var(--sp-5);
  }

  /* ===== Toast ===== */
  .toast {
    position: absolute;
    bottom: var(--sp-6);
    left: 50%;
    transform: translateX(-50%);
    padding: var(--sp-3) var(--sp-5);
    background: var(--danger);
    color: #fff;
    font-size: 13px;
    border-radius: var(--r-md);
    animation: rise 0.3s var(--ease) both;
    z-index: 100;
    box-shadow: 0 4px 12px rgba(168, 69, 58, 0.25);
  }
</style>
