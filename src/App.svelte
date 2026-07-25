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
  import { resetGame, undoMove, startGame, onAiThinking, onAiThinkingReset, onAiPick, onAiPickReset, onAiUsage, loadState } from "./lib/api";
  import { resetManager, driveTurn, stopAutoPlay, resetStopFlag } from "./lib/stores/playerManager";
  import { boardFlipped, toggleBoardFlipped } from "./lib/stores/boardOrientation";
  import { playSound, preloadSounds } from "./lib/sounds/player";
  import type { PlayerType } from "./lib/types";
  import Board from "./lib/components/Board.svelte";
  import Settings from "./lib/components/Settings.svelte";
  import EvalBar from "./lib/components/EvalBar.svelte";
  import ThinkingPanel from "./lib/components/ThinkingPanel.svelte";
  import AnalysisPanel from "./lib/components/AnalysisPanel.svelte";
  import MoveHistory from "./lib/components/MoveHistory.svelte";
  import {
    engineStatus,
    currentInfo,
    loadEngine,
    analyzePosition,
    stopAnalysis,
    destroyEngine,
    scoreFromWhitePerspective,
  } from "./lib/stockfish/store";

  // 抽屉展开状态（仅窄屏 <900px 使用抽屉模式承载设置/分析）
  let drawerOpen = $state(false);
  // 分析抽屉展开状态（仅窄屏 <900px 使用抽屉模式）
  let analysisOpen = $state(false);
  // 窄屏检测：true 时分析面板用抽屉，false 时常驻侧栏
  let narrowScreen = $state(typeof window !== "undefined" && window.innerWidth < 900);
  // 是否显示评估柱（引擎加载过即显示）
  let showEvalBar = $derived($engineStatus !== "unloaded");
  // 思考链位置：左边栏还是棋盘旁
  let thinkingPanelPos = $derived($settings.thinkingPosition);

  // 右侧合并面板：标签页切换 + 收起状态
  type TabId = "history" | "analysis" | "settings";
  let activeTab = $state<TabId>("history");
  let panelCollapsed = $state(false);
  // 标记当前对局是否来自持久化加载（应用启动时恢复的对局）
  // 仅此时显示"继续对局"按钮让用户手动驱动 AI；正常对局中 AI 自动驱动
  let resumedFromPersist = $state(false);
  // 标记对局已开始但第一步 AI 未驱动（用户点"开始对弈"后，若白方是自动主体则等待"开始对局"触发）
  let pendingStart = $state(false);
  // 标记用户手动暂停（区别于恢复持久化对局的 resumedFromPersist）
  let isPaused = $state(false);
  function selectTab(tab: TabId) {
    activeTab = tab;
    // 收起状态下点击标签自动展开
    if (panelCollapsed) panelCollapsed = false;
  }
  function togglePanel() {
    panelCollapsed = !panelCollapsed;
  }

  let started = $derived($settings.started);
  let whitePlayer = $derived($settings.whitePlayer);
  let blackPlayer = $derived($settings.blackPlayer);
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

  /// 当前轮到方主体类型
  let currentPlayerType = $derived.by(() => {
    const g = $gameState;
    return g.turn === "white" ? whitePlayer : blackPlayer;
  });

  /// 主体名称映射
  function playerName(p: string): string {
    if (p === "human") return "人";
    if (p === "stockfish") return "鳕鱼";
    if (p === "deepseek") return "DeepSeek";
    return p;
  }

  // 状态栏文案
  let statusText = $derived.by(() => {
    const g = $gameState;
    if ($aiThinking) {
      return `${playerName(currentPlayerType)} 推演中`;
    }
    if (g.status === "checkmate") {
      // 将死时 g.turn 是被将死方，赢家是对方
      const winnerType = g.turn === "white" ? blackPlayer : whitePlayer;
      const loserType = g.turn === "white" ? whitePlayer : blackPlayer;
      return `${playerName(winnerType)} 胜 · ${playerName(loserType)} 被将死`;
    }
    if (g.status === "stalemate") return "和棋 · 逼和";
    if (g.status === "draw") return "和棋";
    if (g.in_check) {
      return `${playerName(currentPlayerType)} 被将军`;
    }
    if (!started) return "准备开始";
    return `${playerName(currentPlayerType)} 回合`;
  });

  let statusKind = $derived.by(() => {
    const g = $gameState;
    if (g.status === "checkmate") return "lose";
    if (g.status === "stalemate" || g.status === "draw") return "draw";
    if ($aiThinking) return "thinking";
    if (g.in_check) return "check";
    return "normal";
  });

  // 本轮用量与成本显示
  let lastUsage = $derived($costStats.lastUsage);
  let cacheHitRate = $derived(lastCacheHitRate(lastUsage));
  let dailyCost = $derived($costStats.dailyCost);

  // Stockfish 引擎评估显示
  // 评估值转换为白方视角（正=白优，负=黑优）
  let sfEvalText = $derived.by(() => {
    const info = $currentInfo;
    if (!info || !info.score) return "";
    const whiteScore = scoreFromWhitePerspective(info.score, $gameState.turn as "white" | "black");
    if (whiteScore === null) return "";
    if (Math.abs(whiteScore) >= 100000) {
      const mateMoves = Math.ceil((Math.abs(whiteScore) - 99999) / 1);
      return whiteScore > 0 ? `+M${mateMoves}` : `-M${mateMoves}`;
    }
    const val = whiteScore / 100;
    return (val >= 0 ? "+" : "") + val.toFixed(2);
  });
  let sfDepthText = $derived.by(() => {
    const info = $currentInfo;
    if (!info || !info.depth) return "";
    return `d${info.depth}`;
  });
  let sfLoading = $derived($engineStatus === "loading");

  // 走棋后自动重新分析当前局面（Lichess 风格）
  // 条件：引擎已加载 且 当前轮到方不是 stockfish（避免与对弈搜索冲突）
  // 用 lastAutoAnalyzedFen 去重，防止同一局面重复触发
  let lastAutoAnalyzedFen = "";
  $effect(() => {
    const currentFen = $gameState.fen;
    const currentTurn = $gameState.turn;
    const playerType = currentTurn === "white" ? $settings.whitePlayer : $settings.blackPlayer;
    // 引擎未加载时不分析
    if ($engineStatus === "unloaded") return;
    // 当前轮到方是 stockfish 时引擎在对弈，不自动分析避免冲突
    if (playerType === "stockfish") return;
    // 去重：同一局面不重复分析
    if (currentFen === lastAutoAnalyzedFen) return;
    lastAutoAnalyzedFen = currentFen;
    // 自动分析当前局面（静默失败，不打扰用户）
    analyzePosition(currentFen).catch(() => {});
  });

  // 游戏结束音效监听（Lichess 风格：将杀时播放胜利/失败音效）
  // 用 lastEndStatus 去重，避免同一终局状态重复触发
  let lastEndStatus = "";
  $effect(() => {
    const status = $gameState.status;
    const started = $settings.started;
    // 仅在游戏进行中且状态为终局时触发
    if (!started) return;
    if (status !== "checkmate" && status !== "stalemate" && status !== "draw") return;
    // 去重
    const sig = `${status}-${$gameState.fen}`;
    if (sig === lastEndStatus) return;
    lastEndStatus = sig;

    // 将杀时播放 victory/defeat（仅玩家参与的对局）
    if (status === "checkmate") {
      const playerSide = $gameState.player_side;
      // 将杀时 g.turn 是被将死方，赢家是对方
      const loserSide = $gameState.turn;
      const winnerSide = loserSide === "white" ? "black" : "white";
      // 仅当玩家参与对局时播放胜利/失败音效
      const hasHuman = $settings.whitePlayer === "human" || $settings.blackPlayer === "human";
      if (hasHuman) {
        if (playerSide === winnerSide) {
          playSound("victory");
        } else {
          playSound("defeat");
        }
      }
    }
  });

  // 切换 Stockfish 引擎开关（启用/关闭引擎，控制 Worker 生命周期）
  async function toggleStockfish() {
    if ($engineStatus === "unloaded") {
      // 启用引擎：加载后由 $effect 自动触发首次分析
      try {
        await loadEngine();
        // 窄屏模式下打开抽屉显示分析面板
        if (narrowScreen) analysisOpen = true;
      } catch (e) {
        showError(`鳕鱼引擎错误: ${e}`);
      }
    } else {
      // 关闭引擎（释放 Worker）
      stopAnalysis();
      destroyEngine();
      // 重置去重标记，下次启用引擎时可重新分析
      lastAutoAnalyzedFen = "";
    }
  }

  // 监听 AI 思考事件（流式增量 + 重置）+ 启动加载持久化状态
  onMount(() => {
    // 预加载所有音效（加速首次播放，避免 autoplay 限制）
    preloadSounds();
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
            minThinkingTokens: loaded.settings.min_thinking_tokens,
            selfConsistencySamples: loaded.settings.self_consistency_samples,
          });
          // 恢复对局
          if (loaded.game) {
            updateGameState(loaded.game);
            // 从游戏状态恢复主体类型到 settings（旧存档迁移时后端会填默认值）
            updateSettings({
              whitePlayer: loaded.game.white_player as PlayerType,
              blackPlayer: loaded.game.black_player as PlayerType,
              started: true,
            });
            // 重建 PlayerManager（按恢复的主体组合）
            resetManager(loaded.game.white_player as PlayerType, loaded.game.black_player as PlayerType);
            // 标记为恢复的对局：显示"继续对局"按钮让用户手动驱动 AI
            resumedFromPersist = true;
            // 不自动驱动 AI：用户需手动点「开始对弈/继续对局」按钮触发
            // 防止应用启动时 AI 自行输出，确保"点开局后才开始对局"
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

    // 窄屏检测：监听窗口大小变化
    const onResize = () => { narrowScreen = window.innerWidth < 900; };
    window.addEventListener("resize", onResize);

    return () => {
      unlistenThinking?.();
      unlistenReset?.();
      unlistenPick?.();
      unlistenPickReset?.();
      unlistenUsage?.();
      window.removeEventListener("resize", onResize);
    };
  });

  async function handleUndo() {
    if (!canUndo) return;
    try {
      let state = await undoMove();
      // 旧逻辑：若悔棋后仍轮到 AI，再悔一步（人 vs AI 场景）
      // 新架构下需根据主体判断：若当前轮到方仍是自动主体，再悔一步
      const currentPlayer = state.turn === "white" ? state.white_player : state.black_player;
      if (currentPlayer !== "human" && state.move_history.length > 0) {
        state = await undoMove();
      }
      updateGameState(state);
    } catch (e) {
      showError(String(e));
    }
  }

  async function handleReset() {
    drawerOpen = false;
    // 停止正在进行的自对弈
    stopAutoPlay();
    // 重开对局不是恢复的对局，清除标志
    resumedFromPersist = false;
    pendingStart = false;
    isPaused = false;
    try {
      aiReasoning.set("");
      aiPick.set(null);
      aiFailed.set(false);
      const white = $settings.whitePlayer;
      const black = $settings.blackPlayer;
      const state = await resetGame("white", white, black);
      // 重置 PlayerManager（按新主体组合构造）
      resetManager(white, black);
      updateGameState(state);
      // 若白方是自动主体，设置 pendingStart 等待用户点"开始对局"
      if (white !== "human") {
        pendingStart = true;
      }
    } catch (e) {
      showError(String(e));
    }
  }

  function toggleDrawer() {
    drawerOpen = !drawerOpen;
  }

  /// 是否为恢复的对局（应用启动时从持久化加载，且有走法历史且游戏进行中）
  /// 用于在主页面显示"继续对局"按钮，点击后只驱动 AI 不新开对局
  /// 正常对局中 resumedFromPersist=false，AI 由 continueAfterHumanMove 自动驱动
  let isResumedGame = $derived(
    resumedFromPersist && started && $gameState.move_history.length > 0 && $gameState.status === "playing"
  );
  /// 当前轮到方是否为自动主体（需要用户手动点"继续对局"驱动）
  let currentIsAuto = $derived.by(() => {
    const g = $gameState;
    const p = g.turn === "white" ? whitePlayer : blackPlayer;
    return p !== "human";
  });
  /// 任一方为 DeepSeek 时需要 API Key
  let needApiKey = $derived(whitePlayer === "deepseek" || blackPlayer === "deepseek");

  /// 开始新对局（统一入口：Welcome 页面 + Settings 面板的"开始对弈"按钮都走这里）
  /// 处理 startGame API + resetManager + pendingStart 标记，避免入口重复逻辑
  async function handleStart() {
    // 新开对局不是恢复的对局，清除标志
    resumedFromPersist = false;
    pendingStart = false;
    isPaused = false;
    if (needApiKey && !$settings.apiKey.trim()) {
      showError("请填入 API Key");
      drawerOpen = true;
      return;
    }
    try {
      aiReasoning.set("");
      aiPick.set(null);
      aiFailed.set(false);
      const white = $settings.whitePlayer;
      const black = $settings.blackPlayer;
      const state = await startGame({
        side: "white",
        api_key: $settings.apiKey,
        model: $settings.model,
        thinking: $settings.thinking,
        pseudo_thinking: $settings.pseudoThinking,
        thinking_language: $settings.thinkingLanguage,
        reasoning_effort: $settings.reasoningEffort,
        min_thinking_tokens: $settings.minThinkingTokens,
        self_consistency_samples: $settings.selfConsistencySamples,
        white_player: white,
        black_player: black,
      });
      resetManager(white, black);
      updateSettings({ started: true });
      updateGameState(state);
      // 关闭设置抽屉（窄屏模式下从 Settings 点开始对弈时）
      drawerOpen = false;
      // 对局开始：播放 Lichess 通知音
      playSound("gameStart");
      // 若第一步轮到自动主体（白方是 AI/鳕鱼），设置 pendingStart 等待用户点"开始对局"
      // 若白方是人，pendingStart 保持 false，用户直接点棋盘走棋
      if (white !== "human") {
        pendingStart = true;
      }
    } catch (e) {
      showError(String(e));
    }
  }

  /// 用户点"开始对局"按钮触发 AI 第一步
  async function handleStartGame() {
    pendingStart = false;
    isPaused = false;
    if (needApiKey && !$settings.apiKey.trim()) {
      showError("请填入 API Key");
      drawerOpen = true;
      return;
    }
    try {
      aiReasoning.set("");
      aiFailed.set(false);
      const state = $gameState;
      driveTurn(state).catch((e) => {
        showError(String(e));
        aiFailed.set(true);
      });
    } catch (e) {
      showError(String(e));
    }
  }

  /// 继续恢复的对局（从棋盘页面点击，不新开对局）
  async function handleResume() {
    // 重置标志：用户触发继续后，对局进入正常流程，后续 AI 由 continueAfterHumanMove 自动驱动
    resumedFromPersist = false;
    pendingStart = false;
    isPaused = false;
    if (needApiKey && !$settings.apiKey.trim()) {
      showError("请填入 API Key");
      drawerOpen = true;
      return;
    }
    try {
      aiReasoning.set("");
      aiFailed.set(false);
      const state = $gameState;
      driveTurn(state).catch((e) => {
        showError(String(e));
        aiFailed.set(true);
      });
    } catch (e) {
      showError(String(e));
    }
  }

  /// 暂停/继续对局（单按钮切换文案）
  function handlePauseResume() {
    if (isPaused) {
      // 继续：重置停止标志并驱动当前轮到方走棋
      isPaused = false;
      resetStopFlag();
      driveTurn($gameState).catch((e) => {
        showError(String(e));
        aiFailed.set(true);
      });
    } else {
      // 暂停：停止自对弈循环
      isPaused = true;
      stopAutoPlay();
    }
  }

  function handleExit() {
    // 退出当前对局时停止自对弈
    stopAutoPlay();
    updateSettings({ started: false });
    // 退出后清除恢复标志，避免下次新对局误触发
    resumedFromPersist = false;
    pendingStart = false;
    isPaused = false;
    drawerOpen = true;
  }

  // AI 回复失败时重新请求：发重试信号给 Board，由 Board 调用 triggerAiMove
  function handleRetry() {
    aiFailed.set(false);
    retrySignal.update((n) => n + 1);
  }
</script>

<main class="app">
  <!-- 顶部工具栏（品牌 + 核心操作按钮） -->
  <header class="top-bar">
    <div class="brand">
      <span class="brand-mark">♟</span>
      <span class="brand-title">AI Chess · DeepSeek</span>
    </div>
    <div class="toolbar-actions">
      {#if started}
        <button class="bar-btn" onclick={handleUndo} disabled={!canUndo} aria-label="悔棋">悔棋</button>
        <button class="bar-btn" onclick={handleReset} aria-label="重开对局">重开</button>
        <button class="bar-btn retry-btn" class:failed={$aiFailed} onclick={handleRetry} disabled={!canRetry} aria-label="重新请求 AI 走棋">重新请求</button>
        <span class="divider" aria-hidden="true"></span>
        <button
          class="bar-btn sf-btn"
          class:on={$engineStatus !== "unloaded"}
          class:loading={sfLoading}
          onclick={toggleStockfish}
          disabled={sfLoading}
          title="启用/关闭鳕鱼引擎"
          aria-label="启用或关闭鳕鱼引擎"
          aria-pressed={$engineStatus !== "unloaded"}
        >鳕鱼{$engineStatus !== "unloaded" ? " · 开" : ""}</button>
        <button class="bar-btn narrow-hide" onclick={togglePanel} title={panelCollapsed ? "展开面板" : "收起面板"} aria-label={panelCollapsed ? "展开面板" : "收起面板"}>
          {panelCollapsed ? "展开面板" : "收起面板"}
        </button>
        <button class="bar-btn narrow-only" onclick={toggleDrawer} title="打开设置抽屉" aria-label="打开设置抽屉">设置</button>
      {/if}
    </div>
  </header>

  <!-- 中央棋盘区（桌面双栏：棋盘 + 右侧合并面板 / 窄屏单列） -->
  <section class="stage" class:narrow={narrowScreen}>
    <!-- 第一列：棋盘 + 评估柱 + 思考链面板（启动直入棋盘，无 Welcome 页） -->
    <div class="board-area" class:with-eval={showEvalBar}>
      {#if showEvalBar}
        <EvalBar />
      {/if}
      {#if thinkingPanelPos === "left"}
        <ThinkingPanel />
      {/if}
      <Board />
    </div>

    <!-- 第二列：桌面端右侧合并面板（走法历史/分析/设置三标签）；窄屏不渲染（用抽屉） -->
    {#if started && !narrowScreen}
      <aside class="side-panel" class:collapsed={panelCollapsed}>
        <div class="panel-tabs">
          <button class="tab-btn" class:active={activeTab === "history"} onclick={() => selectTab("history")}>走法历史</button>
          <button class="tab-btn" class:active={activeTab === "analysis"} onclick={() => selectTab("analysis")}>引擎分析</button>
          <button class="tab-btn" class:active={activeTab === "settings"} onclick={() => selectTab("settings")}>设置</button>
          <button class="panel-toggle" onclick={togglePanel} title={panelCollapsed ? "展开面板" : "收起面板"}>
            {panelCollapsed ? "≪" : "≫"}
          </button>
        </div>
        {#if !panelCollapsed}
          <div class="panel-content">
            {#if activeTab === "history"}
              <MoveHistory />
            {:else if activeTab === "analysis"}
              <AnalysisPanel />
            {:else}
              <Settings onStarted={handleStart} onExit={handleExit} />
            {/if}
          </div>
        {/if}
      </aside>
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
      <!-- 对局控制按钮（开始对弈/开始对局/继续对局/暂停/继续） -->
      <div class="game-controls" role="group" aria-label="对局控制">
        {#if !started}
          <button class="ctrl-btn primary" onclick={handleStart} aria-label="开始对弈">开始对弈</button>
        {:else if pendingStart && currentIsAuto}
          <button class="ctrl-btn primary" onclick={handleStartGame} aria-label="开始对局，让 AI 走第一步">开始对局</button>
        {:else if isResumedGame && currentIsAuto}
          <button class="ctrl-btn primary" onclick={handleResume} aria-label="继续对局，让 AI 走棋">继续对局</button>
        {:else if isPaused}
          <button class="ctrl-btn" onclick={handlePauseResume} aria-label="继续对局">继续</button>
        {:else}
          <button class="ctrl-btn" onclick={handlePauseResume} disabled={!currentIsAuto} aria-label="暂停对局">暂停</button>
        {/if}
      </div>

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

      <!-- 反转棋盘按钮 -->
      <button
        class="flip-toggle"
        class:active={$boardFlipped}
        onclick={toggleBoardFlipped}
        title={$boardFlipped ? "恢复默认方向" : "反转棋盘"}
        aria-label="反转棋盘"
      >
        <span class="flip-icon">⇅</span>
        <span class="flip-label">反转</span>
      </button>

      <span class="divider"></span>

      {#if sfEvalText}
        <span class="sf-eval-text" class:positive={sfEvalText.startsWith("+")} class:negative={sfEvalText.startsWith("-")}>
          {sfEvalText} <span class="sf-depth">{sfDepthText}</span>
        </span>
      {/if}

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
      <Settings onStarted={handleStart} onExit={handleExit} />
    </div>
  </aside>

  <!-- 分析抽屉（右侧滑出） -->
  <div
    class="drawer-mask analysis-mask"
    class:show={analysisOpen}
    role="button"
    tabindex="-1"
    aria-label="关闭分析"
    onclick={() => (analysisOpen = false)}
    onkeydown={(e) => { if (e.key === "Escape" || e.key === "Enter") analysisOpen = false; }}
  ></div>
  <aside class="drawer analysis-drawer" class:open={analysisOpen}>
    <AnalysisPanel onClose={() => (analysisOpen = false)} closable={true} />
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
    justify-content: space-between;
    padding: var(--sp-3) var(--sp-5);
    border-bottom: 1px solid var(--line);
    background: var(--bg);
    flex-shrink: 0;
  }
  .toolbar-actions {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
  }
  .toolbar-actions .bar-btn {
    padding: var(--sp-1) var(--sp-3);
    font-size: 12px;
  }
  /* 窄屏下隐藏底部设置按钮（桌面端已在工具栏），窄屏显示 */
  .bar-btn.narrow-only {
    display: none;
  }
  /* 窄屏（<900px）：底部设置按钮显示，顶部工具栏部分按钮可隐藏 */
  @media (max-width: 900px) {
    .bar-btn.narrow-only {
      display: inline-flex;
    }
    .bar-btn.narrow-hide {
      display: none;
    }
    .toolbar-actions {
      gap: var(--sp-1);
    }
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
    display: grid;
    grid-template-columns: 1fr 340px;
    align-items: center;
    gap: var(--sp-4);
    padding: 0 var(--sp-4);
    position: relative;
    min-height: 0;
    overflow: hidden;
  }
  /* 面板收起时，右侧列塌缩为 0，棋盘居中占满 */
  .stage:has(.side-panel.collapsed) {
    grid-template-columns: 1fr 0;
  }
  /* 窄屏：退化为单列居中（面板走抽屉） */
  .stage.narrow {
    grid-template-columns: 1fr;
    justify-items: center;
    padding: 0 var(--sp-3);
  }

  /* 右侧合并面板（桌面常驻，三标签 + 收起） */
  .side-panel {
    width: 100%;
    height: 100%;
    min-height: 0;
    display: flex;
    flex-direction: column;
    border-left: 1px solid var(--line);
    background: var(--surface);
    overflow: hidden;
  }
  .side-panel.collapsed {
    width: 0;
    border-left: none;
    overflow: hidden;
  }
  .panel-tabs {
    display: flex;
    align-items: center;
    gap: var(--sp-1);
    padding: var(--sp-2) var(--sp-2) 0;
    border-bottom: 1px solid var(--line);
    flex-shrink: 0;
  }
  .tab-btn {
    padding: var(--sp-2) var(--sp-3);
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    color: var(--ink-muted);
    font-family: var(--font-sans);
    font-size: 12px;
    cursor: pointer;
    transition: color 0.2s var(--ease), border-color 0.2s var(--ease);
  }
  .tab-btn:hover {
    color: var(--ink);
  }
  .tab-btn.active {
    color: var(--ink);
    border-bottom-color: var(--accent);
  }
  .panel-toggle {
    margin-left: auto;
    padding: var(--sp-1) var(--sp-2);
    background: transparent;
    border: none;
    color: var(--ink-muted);
    font-size: 16px;
    cursor: pointer;
    border-radius: var(--r-sm);
    transition: background 0.2s var(--ease), color 0.2s var(--ease);
  }
  .panel-toggle:hover {
    background: var(--surface-2);
    color: var(--ink);
  }
  .panel-content {
    flex: 1;
    min-height: 0;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  /* ===== 棋盘区域（评估柱 + 棋盘横向并排） ===== */
  .board-area {
    position: relative;
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    flex-shrink: 0;
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

  /* 对局控制按钮（开始对弈/开始对局/继续对局/暂停/继续） */
  .game-controls {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
  }
  .ctrl-btn {
    padding: var(--sp-1) var(--sp-3);
    border-radius: var(--r-sm);
    border: 1px solid var(--line);
    background: var(--bg-soft);
    color: var(--ink-muted);
    font-size: 12px;
    cursor: pointer;
    transition: background 0.2s var(--ease), color 0.2s var(--ease), border-color 0.2s var(--ease), opacity 0.15s var(--ease);
  }
  .ctrl-btn:hover:not(:disabled) {
    background: var(--surface-2);
  }
  .ctrl-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
  .ctrl-btn.primary {
    background: var(--accent);
    color: #fff;
    border-color: var(--accent);
  }
  .ctrl-btn.primary:hover {
    opacity: 0.9;
  }

  /* 反转棋盘按钮：与风的加护同风格 */
  .flip-toggle {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    cursor: pointer;
    padding: var(--sp-1) var(--sp-2);
    border-radius: var(--r-sm);
    border: 1px solid var(--line);
    background: var(--bg-soft);
    color: var(--ink-muted);
    font-size: 12px;
    transition: background 0.2s var(--ease), color 0.2s var(--ease), border-color 0.2s var(--ease);
  }
  .flip-toggle:hover {
    background: var(--surface-2);
  }
  .flip-toggle .flip-icon {
    font-size: 14px;
    line-height: 1;
  }
  .flip-toggle.active {
    background: var(--accent);
    color: #fff;
    border-color: var(--accent);
  }

  /* ===== 鳕鱼引擎按钮 + 评估 ===== */
  .sf-btn {
    color: var(--ink-muted);
    font-weight: 500;
    transition: color 0.2s var(--ease), background 0.2s var(--ease);
  }
  .sf-btn:hover:not(:disabled) {
    color: var(--accent);
  }
  .sf-btn.on {
    color: var(--accent);
    background: rgba(46, 74, 62, 0.08);
  }
  .sf-btn.loading {
    opacity: 0.6;
    cursor: wait;
  }
  .sf-depth {
    font-size: 10px;
    font-weight: 400;
    color: var(--ink-faint);
    margin-left: 2px;
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

  /* ===== 分析抽屉（右侧滑出，覆盖 .drawer 的底部滑出样式） ===== */
  .analysis-drawer {
    left: auto;
    top: 0;
    bottom: 0;
    width: 340px;
    max-height: none;
    height: 100vh;
    border-top: none;
    border-left: 1px solid var(--line);
    border-radius: var(--r-lg) 0 0 var(--r-lg);
    box-shadow: -12px 0 40px rgba(27, 26, 23, 0.12);
    transform: translateX(100%);
  }
  .analysis-drawer.open {
    transform: translateX(0);
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
