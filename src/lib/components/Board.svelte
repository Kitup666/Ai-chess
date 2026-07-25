<script lang="ts">
  import { tick } from "svelte";
  import {
    gameState,
    selectedSquare,
    aiThinking,
    aiReasoning,
    aiPick,
    aiFailed,
    retrySignal,
    showThinking,
    isPlayerTurn,
    showError,
    updateGameState,
  } from "../stores/game";
  import { playerMove } from "../api";
  import { continueAfterHumanMove, driveTurn } from "../stores/playerManager";
  import { settings } from "../stores/settings";
  import { parseFen, squareName, type PieceType } from "../types";
  import { throttledMultiPVList, isAnalyzing } from "../stockfish/store";
  import { highlightedPV } from "../stockfish/highlight";
  import { boardEffectiveFlipped } from "../stores/boardOrientation";
  import { playMoveSounds, playSound } from "../sounds/player";
  // 棋子 SVG 资源（Cburnett 风格，公共域）
  import wK from "../assets/pieces/wK.svg";
  import wQ from "../assets/pieces/wQ.svg";
  import wR from "../assets/pieces/wR.svg";
  import wB from "../assets/pieces/wB.svg";
  import wN from "../assets/pieces/wN.svg";
  import wP from "../assets/pieces/wP.svg";
  import bK from "../assets/pieces/bK.svg";
  import bQ from "../assets/pieces/bQ.svg";
  import bR from "../assets/pieces/bR.svg";
  import bB from "../assets/pieces/bB.svg";
  import bN from "../assets/pieces/bN.svg";
  import bP from "../assets/pieces/bP.svg";

  // 思考内容：提取最新一句用于轮换显示
  // 按标点/换行分割取最后一段，流式输出时实时更新，形成"每秒蹦十句"的效果
  let currentSentence = $derived.by(() => {
    const text = $aiReasoning;
    if (!text) return "";
    const parts = text.split(/[。！？.!?\n]+/).map((s) => s.trim()).filter(Boolean);
    return parts[parts.length - 1] ?? "";
  });

  // 思考条显示文案：风的加护开启时显示具体思考句子，关闭时只显示"正在思考…"占位
  let thinkingText = $derived(($showThinking && currentSentence) || "正在思考…");

  // 思考条位置：AI 在棋盘哪一侧
  // 玩家执白(flipped=false)→AI(黑)在棋盘上方→思考条放上方
  // 玩家执黑(flipped=true)→AI(白)在棋盘下方→思考条放下方
  let aiOnTop = $derived($gameState.player_side === "white");

  // 是否显示思考条：AI 思考时且位置为"棋盘旁"时显示
  let showThinkingOverlay = $derived($aiThinking && $settings.thinkingPosition === "board");

  // 玩家举棋状态：点击合法目标格后短暂抬起再走棋
  // 声明在 liftingSquare 之前，确保响应式依赖正确追踪
  let playerPick = $state<string | null>(null);

  // 玩家选中棋子时立即举起的源格（点击棋子即抬起，不等点击目标格）
  let playerLiftSquare = $state<string | null>(null);

  // 当前举棋的源格（UCI 前2字符，如 "e2"）
  // 优先级：AI 思考（需风的加护开启） > 玩家走棋(playerPick) > 玩家选中(playerLiftSquare)
  // 风的加护关闭时不显示 AI 举棋，只保留玩家举棋
  let liftingSquare = $derived.by(() => {
    if ($aiThinking && $showThinking) {
      const uci = $aiPick;
      if (!uci || uci.length < 4) return null;
      return uci.slice(0, 2);
    }
    if (playerPick) {
      return playerPick.slice(0, 2);
    }
    if (playerLiftSquare) {
      return playerLiftSquare;
    }
    return null;
  });

  // 举棋的目标格（UCI 后2字符，如 "e4"）
  // 在目标格显示脉冲圆圈，直观看出棋子想去哪
  // 风的加护关闭时不显示 AI 举棋目标
  let pickTargetSquare = $derived.by(() => {
    if ($aiThinking && $showThinking) {
      const uci = $aiPick;
      if (!uci || uci.length < 4) return null;
      return uci.slice(2, 4);
    }
    if (playerPick) {
      return playerPick.slice(2, 4);
    }
    return null;
  });

  // AI 举棋时，棋子到目标格的箭头坐标（百分比，相对棋盘）
  // 仅 AI 思考 + 风的加护开启时显示箭头；玩家举棋不画箭头（已有目标格圆圈指示）
  let arrowCoords = $derived.by(() => {
    if (!$aiThinking || !$showThinking || !$aiPick || $aiPick.length < 4) return null;
    const uci = $aiPick;
    const fromFile = uci.charCodeAt(0) - 97;
    const fromRank = Number(uci[1]) - 1;
    const toFile = uci.charCodeAt(2) - 97;
    const toRank = Number(uci[3]) - 1;
    // 棋盘百分比坐标：file 0-7 → 6.25%~93.75%；rank 0(底)-7(顶) → 93.75%~6.25%
    const toPct = (file: number, rank: number) => ({
      x: (file + 0.5) * 12.5,
      y: (7 - rank + 0.5) * 12.5,
    });
    // 翻转时坐标镜像
    const f = flipped ? { x: (7 - fromFile + 0.5) * 12.5, y: (7 - (7 - fromRank) + 0.5) * 12.5 } : toPct(fromFile, fromRank);
    const t = flipped ? { x: (7 - toFile + 0.5) * 12.5, y: (7 - (7 - toRank) + 0.5) * 12.5 } : toPct(toFile, toRank);
    return { from: f, to: t };
  });

  // 棋子 SVG 映射
  const PIECE_IMG: Record<PieceType, { white: string; black: string }> = {
    k: { white: wK, black: bK },
    q: { white: wQ, black: bQ },
    r: { white: wR, black: bR },
    b: { white: wB, black: bB },
    n: { white: wN, black: bN },
    p: { white: wP, black: bP },
  };

  // 棋子中文名（用于 alt 无障碍）
  const PIECE_NAME: Record<PieceType, string> = {
    k: "王",
    q: "后",
    r: "车",
    b: "象",
    n: "马",
    p: "兵",
  };

  // ===== 音效 =====
  // 使用 Lichess 官方 MP3 音效（见 src/lib/sounds/player.ts）
  // 走棋音效通过 playMoveSounds() 统一处理，AI/鳕鱼走棋在 playerManager.ts 中处理

  // ===== 动画状态 =====
  let isAnimating = $state(false);

  // FLIP 动画：棋子从源位置滑动到目标位置，起手抬起，落子回正
  async function runFlipAnimation(
    fromEl: HTMLElement,
    fromRect: DOMRect,
    toRect: DOMRect
  ) {
    isAnimating = true;
    const dx = fromRect.left - toRect.left;
    const dy = fromRect.top - toRect.top;
    // 初始：偏移到源位置 + 抬起
    fromEl.style.transition = "none";
    fromEl.style.transform = `translate(${dx}px, ${dy}px) scale(1.08)`;
    fromEl.style.zIndex = "10";
    fromEl.style.filter = "drop-shadow(0 8px 16px rgba(0,0,0,0.28))";
    // 强制重排
    void fromEl.offsetHeight;
    // 动画到目标位置 + 回正
    fromEl.style.transition =
      "transform 0.32s cubic-bezier(0.16,1,0.3,1), filter 0.32s cubic-bezier(0.16,1,0.3,1)";
    fromEl.style.transform = "translate(0, 0) scale(1)";
    fromEl.style.filter = "drop-shadow(0 1px 2px rgba(0,0,0,0.18))";
    await new Promise((r) => setTimeout(r, 340));
    fromEl.style.zIndex = "";
    isAnimating = false;
  }

  // 棋盘数据（从 FEN 解析为 8x8，board[rank][file]，rank=0 表示 1 路）
  let board = $derived(parseFen($gameState.fen));

  // 棋盘方向：基础翻转（玩家执黑时自动翻转）与用户手动翻转做 XOR
  // 玩家执白+不翻转=白在底；玩家执白+翻转=黑在底
  // 玩家执黑+不翻转=黑在底（自动看自己方）；玩家执黑+翻转=白在底
  // 使用共享 derived store，与 EvalBar 等组件保持一致
  let flipped = $derived($boardEffectiveFlipped);

  // UCI 走法 → 棋盘百分比坐标（用于引擎评估箭头）
  // 复用 arrowCoords 的坐标系：file 0-7 → 6.25%~93.75%；rank 0(底)-7(顶) → 93.75%~6.25%
  function uciToCoords(uci: string, flip: boolean): { from: { x: number; y: number }; to: { x: number; y: number } } | null {
    if (!uci || uci.length < 4) return null;
    const fromFile = uci.charCodeAt(0) - 97;
    const fromRank = Number(uci[1]) - 1;
    const toFile = uci.charCodeAt(2) - 97;
    const toRank = Number(uci[3]) - 1;
    if (fromFile < 0 || fromFile > 7 || fromRank < 0 || fromRank > 7 || toFile < 0 || toFile > 7 || toRank < 0 || toRank > 7) return null;
    const toPct = (file: number, rank: number) => ({
      x: (file + 0.5) * 12.5,
      y: (7 - rank + 0.5) * 12.5,
    });
    if (flip) {
      return {
        from: { x: (7 - fromFile + 0.5) * 12.5, y: (7 - (7 - fromRank) + 0.5) * 12.5 },
        to: { x: (7 - toFile + 0.5) * 12.5, y: (7 - (7 - toRank) + 0.5) * 12.5 },
      };
    }
    return { from: toPct(fromFile, fromRank), to: toPct(toFile, toRank) };
  }

  // 箭头 path 生成：杆 + 三角头（fill 填充，不用 marker）
  function arrowPath(from: { x: number; y: number }, to: { x: number; y: number }, headSize: number = 4, shaftWidth: number = 1.4): string {
    const dx = to.x - from.x;
    const dy = to.y - from.y;
    const len = Math.sqrt(dx * dx + dy * dy);
    if (len < 0.5) return "";
    const ux = dx / len;
    const uy = dy / len;
    const px = -uy; // 垂直向量
    const py = ux;
    const headLen = headSize;
    const headHalf = headSize * 0.7;
    // 杆终点（头开始处）
    const shaftEndX = to.x - ux * headLen;
    const shaftEndY = to.y - uy * headLen;
    // 杆两侧
    const sLeftX = from.x + px * shaftWidth / 2;
    const sLeftY = from.y + py * shaftWidth / 2;
    const sRightX = from.x - px * shaftWidth / 2;
    const sRightY = from.y - py * shaftWidth / 2;
    const eLeftX = shaftEndX + px * shaftWidth / 2;
    const eLeftY = shaftEndY + py * shaftWidth / 2;
    const eRightX = shaftEndX - px * shaftWidth / 2;
    const eRightY = shaftEndY - py * shaftWidth / 2;
    // 头两侧
    const hLeftX = shaftEndX + px * headHalf;
    const hLeftY = shaftEndY + py * headHalf;
    const hRightX = shaftEndX - px * headHalf;
    const hRightY = shaftEndY - py * headHalf;
    return `M ${sLeftX} ${sLeftY} L ${eLeftX} ${eLeftY} L ${hLeftX} ${hLeftY} L ${to.x} ${to.y} L ${hRightX} ${hRightY} L ${eRightX} ${eRightY} L ${sRightX} ${sRightY} Z`;
  }

  // 引擎评估箭头：多 PV 的第一步走法（订阅节流版，避免高频更新卡顿）
  // PV1=绿色，PV2=黄色，PV3+=琥珀色；高亮的加粗+更亮
  let evalArrows = $derived.by(() => {
    const list = $throttledMultiPVList;
    if (list.length === 0 || (!$isAnalyzing && $highlightedPV === null)) return [];
    return list
      .map((info, idx) => {
        const uci = info.pv?.[0];
        if (!uci) return null;
        const coords = uciToCoords(uci, flipped);
        if (!coords) return null;
        const pvIdx = idx + 1;
        const highlighted = $highlightedPV === pvIdx;
        return { pvIdx, coords, highlighted };
      })
      .filter((x): x is { pvIdx: number; coords: { from: { x: number; y: number }; to: { x: number; y: number } }; highlighted: boolean } => x !== null);
  });

  // 渲染顺序：从上到下的 rank 索引
  let ranks = $derived(flipped ? [0, 1, 2, 3, 4, 5, 6, 7] : [7, 6, 5, 4, 3, 2, 1, 0]);
  let files = $derived(flipped ? [7, 6, 5, 4, 3, 2, 1, 0] : [0, 1, 2, 3, 4, 5, 6, 7]);

  // 当前选中棋子的合法目标格集合
  let legalTargets = $derived.by(() => {
    const sel = $selectedSquare;
    if (!sel) return new Set<string>();
    return new Set(
      $gameState.legal_moves
        .filter((m) => m.startsWith(sel))
        .map((m) => m.substring(2, 4))
    );
  });

  const fileLabels = ["a", "b", "c", "d", "e", "f", "g", "h"];
  const rankLabels = ["1", "2", "3", "4", "5", "6", "7", "8"];

  function isLight(file: number, rank: number): boolean {
    return (file + rank) % 2 === 1;
  }

  function isLastMove(square: string): boolean {
    const lm = $gameState.last_move;
    return lm !== null && (lm.from === square || lm.to === square);
  }

  function isSelected(square: string): boolean {
    return $selectedSquare === square;
  }

  function isLegalTarget(square: string): boolean {
    return legalTargets.has(square);
  }

  // 升变选择：当玩家移动兵到最后一格时弹出选择
  let pendingPromo = $state<{ from: string; to: string } | null>(null);

  function isKingInCheck(square: string): boolean {
    if (!$gameState.in_check) return false;
    // 找到当前轮到方国王所在格
    for (let r = 0; r < 8; r++) {
      for (let f = 0; f < 8; f++) {
        const p = board[7 - r][f]?.piece;
        if (p && p.type === "k" && p.color === $gameState.turn) {
          return squareName(f, r) === square;
        }
      }
    }
    return false;
  }

  async function handleClick(file: number, rank: number) {
    if ($aiThinking || !$isPlayerTurn || isAnimating) return;

    const square = squareName(file, rank);
    const sel = $selectedSquare;

    // 已选中棋子且点击合法目标 → 举棋动画后走棋
    if (sel && isLegalTarget(square)) {
      const moveStr = sel + square;
      // 解析选中格的棋子，判断是否兵升变
      const selFile = sel.charCodeAt(0) - 97;
      const selRank = Number(sel[1]) - 1;
      const movingPiece = board[7 - selRank][selFile]?.piece;
      const isPromo =
        movingPiece?.type === "p" &&
        (square[1] === "8" || square[1] === "1");
      if (isPromo) {
        pendingPromo = { from: sel, to: square };
        return;
      }
      // 直接走棋（移除举棋动画延迟，让人走棋立即响应）
      playerPick = null;
      playerLiftSquare = null;
      await executeMove(moveStr);
      return;
    }

    // 点击己方棋子 → 选中并立即举起
    // 三方主体架构下：当前轮到方=Human 时，Human 的执方就是 turn
    const cellPiece = board[7 - rank][file]?.piece;
    if (cellPiece && cellPiece.color === $gameState.turn) {
      selectedSquare.set(square);
      playerLiftSquare = square;
      // 选中音效（Lichess 无专门选中音效，复用 Move 音效作为轻反馈）
      playSound("move");
    } else {
      selectedSquare.set(null);
      playerLiftSquare = null;
    }
  }

  async function executeMove(moveStr: string) {
    selectedSquare.set(null);
    const fromSq = moveStr.substring(0, 2);
    const toSq = moveStr.substring(2, 4);
    // 走棋前记录源格棋子 DOM 和目标格是否有棋子（吃子判断）
    const fromEl = document.querySelector(`[aria-label="${fromSq}"] .piece`) as HTMLElement | null;
    const fromRect = fromEl?.getBoundingClientRect();
    const toFile = toSq.charCodeAt(0) - 97;
    const toRank = Number(toSq[1]) - 1;
    const targetCell = board[7 - toRank][toFile];
    const isCapture = !!targetCell?.piece;

    try {
      // 记录走棋前的 FEN，用于音效判断吃子
      const oldFen = $gameState.fen;
      const result = await playerMove(moveStr);
      // 更新状态触发 DOM 重渲染（from 棋子移到 to 位置）
      updateGameState(result.state);
      await tick();
      // 获取 to 位置的新棋子 DOM
      const toEl = document.querySelector(`[aria-label="${toSq}"]`) as HTMLElement | null;
      const toRect = toEl?.getBoundingClientRect();
      const newPieceEl = toEl?.querySelector(".piece") as HTMLElement | null;

      // 音效与动画并行播放（落子瞬间发声，与 Lichess 行为一致）
      // 旧实现把 playMoveSounds 放在 runFlipAnimation 之后，导致声音延迟 340ms
      playMoveSounds({
        uci: moveStr,
        oldFen: oldFen,
        inCheck: result.state.in_check,
        gameOver: result.game_over,
        status: result.state.status,
      });

      if (fromRect && toRect && newPieceEl) {
        await runFlipAnimation(newPieceEl, fromRect, toRect);
      }

      if (!result.game_over) {
        // Human 走棋后，驱动对方（自动主体）走棋
        // PlayerManager 内部会处理：若对方是自动主体则走棋，若对方也是 Human 则停止
        continueAfterHumanMove(result.state).catch((e) => {
          showError(String(e));
          aiFailed.set(true);
        });
      }
    } catch (e) {
      showError(String(e));
      selectedSquare.set(null);
      playerLiftSquare = null;
    }
  }

  async function choosePromo(piece: PieceType) {
    if (!pendingPromo) return;
    const moveStr = pendingPromo.from + pendingPromo.to + piece;
    pendingPromo = null;
    await executeMove(moveStr);
  }

  function handleKey(e: KeyboardEvent, file: number, rank: number) {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      handleClick(file, rank);
    }
  }

  // 监听重试信号：底部「重新请求」按钮点击时 retrySignal +1，重新驱动当前轮到方走棋
  // 用 lastRetry 避免初始挂载时误触发
  let lastRetry = 0;
  $effect(() => {
    const sig = $retrySignal;
    if (sig > lastRetry) {
      lastRetry = sig;
      // 重新驱动当前轮到方（自动主体）走棋
      driveTurn($gameState).catch((e) => {
        showError(String(e));
        aiFailed.set(true);
      });
    }
  });

  // 游戏结束状态
  let gameEnded = $derived(
    $gameState.status === "checkmate" ||
    $gameState.status === "stalemate" ||
    $gameState.status === "draw"
  );
  let endText = $derived.by(() => {
    const g = $gameState;
    if (g.status === "checkmate") {
      // 将死时 g.turn 是被将死方，赢家是对方
      const winnerType = g.turn === "white" ? g.black_player : g.white_player;
      const name = (p: string) => p === "human" ? "人" : p === "stockfish" ? "鳕鱼" : "DeepSeek";
      return `${name(winnerType)} 胜`;
    }
    if (g.status === "stalemate") return "和棋 · 逼和";
    if (g.status === "draw") return "和棋";
    return "";
  });
</script>

<div class="board-frame" class:ai-top={aiOnTop}>
  <div class="board-wrap">
    <!-- 思考条（绝对定位在棋盘外侧 AI 一侧，不占布局空间，避免颤动） -->
    {#if showThinkingOverlay}
      <div class="thinking-strip">
        <span class="thinking-pulse"></span>
        <span class="thinking-text">{thinkingText}</span>
      </div>
    {/if}

    <div class="board" class:flipped>
      {#each ranks as rank, ri}
        {#each files as file, fi}
          {@const sq = squareName(file, rank)}
          {@const cell = board[7 - rank][file]}
          {@const piece = cell?.piece}
          <div
            class="sq"
            class:light={isLight(file, rank)}
            class:dark={!isLight(file, rank)}
            class:sel={isSelected(sq)}
            class:target={isLegalTarget(sq)}
            class:last={isLastMove(sq)}
            class:check={isKingInCheck(sq)}
            onclick={() => handleClick(file, rank)}
            onkeydown={(e) => handleKey(e, file, rank)}
            role="button"
            tabindex="0"
            aria-label={sq}
          >
            {#if piece}
              <img
                class="piece"
                class:pw={piece.color === "white"}
                class:pb={piece.color === "black"}
                class:lifting={liftingSquare === sq}
                src={PIECE_IMG[piece.type][piece.color]}
                alt={`${piece.color === "white" ? "白" : "黑"}${PIECE_NAME[piece.type]}`}
                draggable="false"
              />
            {/if}
            {#if isLegalTarget(sq) && !piece}
              <span class="dot"></span>
            {/if}
            {#if isLegalTarget(sq) && piece}
              <span class="ring"></span>
            {/if}
            {#if pickTargetSquare === sq}
              <span class="pick-marker"></span>
            {/if}
            {#if fi === 0}
              <span class="coord rank-coord">{rankLabels[rank]}</span>
            {/if}
            {#if ri === 7}
              <span class="coord file-coord">{fileLabels[file]}</span>
            {/if}
          </div>
        {/each}
      {/each}

      <!-- AI 举棋路径箭头：从源棋子指向目标格，落点用圆形 -->
      {#if arrowCoords}
        <svg class="pick-arrow-layer" viewBox="0 0 100 100" preserveAspectRatio="none">
          <defs>
            <marker id="arrowhead" markerWidth="3" markerHeight="3" refX="1.5" refY="1.5" orient="auto">
              <path d="M0,0 L3,1.5 L0,3 Z" fill="var(--highlight)" />
            </marker>
          </defs>
          <line
            x1={arrowCoords.from.x}
            y1={arrowCoords.from.y}
            x2={arrowCoords.to.x}
            y2={arrowCoords.to.y}
            stroke="var(--highlight)"
            stroke-width="1.2"
            stroke-linecap="round"
            marker-end="url(#arrowhead)"
            opacity="0.85"
          />
        </svg>
      {/if}

      <!-- 引擎评估箭头：多 PV 第一步走法（绿色最佳/黄色次佳/高亮加粗） -->
      {#if evalArrows.length > 0}
        <svg class="eval-arrow-layer" viewBox="0 0 100 100" preserveAspectRatio="none" aria-hidden="true">
          {#each evalArrows as arrow (arrow.pvIdx)}
            <path
              d={arrowPath(arrow.coords.from, arrow.coords.to, arrow.highlighted ? 5 : 4, arrow.highlighted ? 2 : 1.4)}
              fill={arrow.pvIdx === 1
                ? (arrow.highlighted ? "rgba(98, 153, 36, 0.85)" : "rgba(98, 153, 36, 0.55)")
                : arrow.pvIdx === 2
                ? (arrow.highlighted ? "rgba(201, 169, 97, 0.85)" : "rgba(201, 169, 97, 0.5)")
                : (arrow.highlighted ? "rgba(201, 169, 97, 0.7)" : "rgba(201, 169, 97, 0.35)")}
            />
          {/each}
        </svg>
      {/if}
    </div>
  </div>

  <!-- 升变选择器 -->
  {#if pendingPromo}
    <div class="overlay">
      <div class="promo-card rise">
        <div class="promo-title">选择升变棋子</div>
        <div class="promo-pieces">
          {#each ["q", "r", "b", "n"] as p}
            <button
              class="promo-btn"
              onclick={() => choosePromo(p as PieceType)}
              aria-label={p}
            >
              <img
                class="piece"
                class:pw={$gameState.player_side === "white"}
                class:pb={$gameState.player_side === "black"}
                src={PIECE_IMG[p as PieceType][$gameState.player_side]}
                alt={PIECE_NAME[p as PieceType]}
                draggable="false"
              />
            </button>
          {/each}
        </div>
      </div>
    </div>
  {/if}

  <!-- 游戏结束遮罩 -->
  {#if gameEnded}
    <div class="overlay end-overlay" class:mate={$gameState.status === "checkmate"}>
      <div class="end-card rise">
        <div class="end-title">{endText}</div>
        <div class="end-sub">
          {#if $gameState.status === "checkmate"}将杀{/if}
          {#if $gameState.status === "stalemate"}无合法走法{/if}
          {#if $gameState.status === "draw"}平局{/if}
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .board-frame {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    height: 100%;
    padding: var(--sp-6);
  }

  .board-wrap {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    /* 棋盘尺寸：取 vh 和 vw 的较小值，确保窄屏不溢出 */
    width: min(72vh, 92vw, 560px);
    height: min(72vh, 92vw, 560px);
    box-shadow: 0 4px 24px rgba(0, 0, 0, 0.4), 0 1px 0 var(--board-frame);
  }

  .board {
    display: grid;
    grid-template-columns: repeat(8, 1fr);
    grid-template-rows: repeat(8, 1fr);
    width: 100%;
    height: 100%;
    aspect-ratio: 1;
    animation: rise 0.7s var(--ease) both;
    animation-delay: 0.15s;
  }

  .sq {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: default;
    transition: background 0.18s var(--ease);
  }

  .light {
    background-color: var(--board-light);
    background-image: radial-gradient(rgba(0, 0, 0, 0.015) 1px, transparent 1px);
    background-size: 3px 3px;
  }
  .dark {
    background-color: var(--board-dark);
    background-image: radial-gradient(rgba(255, 255, 255, 0.02) 1px, transparent 1px);
    background-size: 3px 3px;
  }

  .sq.sel {
    background-color: var(--highlight) !important;
    background-image: none;
  }
  .sq.sel::after {
    content: "";
    position: absolute;
    inset: 0;
    background: var(--highlight);
    opacity: 0.35;
    pointer-events: none;
    z-index: 0;
  }

  .sq.last::before {
    content: "";
    position: absolute;
    inset: 0;
    background: var(--highlight);
    opacity: 0.35;
    pointer-events: none;
    z-index: 0;
    box-shadow: inset 0 0 0 2px rgba(201, 169, 97, 0.5);
  }

  .sq.target {
    cursor: pointer;
  }
  .sq.target:hover {
    background: var(--highlight);
  }

  .sq.check {
    background-color: var(--danger) !important;
    background-image: none;
    animation: pulse-check-strong 1s var(--ease) infinite;
  }
  @keyframes pulse-check-strong {
    0%, 100% {
      filter: brightness(1);
      box-shadow: inset 0 0 0 4px rgba(184, 68, 46, 0.6);
    }
    50% {
      filter: brightness(1.3);
      box-shadow: inset 0 0 0 8px rgba(184, 68, 46, 0.8), 0 0 20px rgba(184, 68, 46, 0.4);
    }
  }

  .piece {
    width: min(8vh, 60px);
    height: min(8vh, 60px);
    object-fit: contain;
    user-select: none;
    z-index: 2;
    transition: transform 0.2s var(--ease);
    pointer-events: none;
    filter: drop-shadow(0 1px 2px rgba(0, 0, 0, 0.18));
  }

  .sq:hover .piece {
    transform: scale(1.06);
  }

  /* 举棋呼吸：棋子抬起 + 放大缩小循环，AI 和玩家通用 */
  .piece.lifting {
    z-index: 10;
    /* 描边：多层 drop-shadow 模拟金色描边环绕棋子 */
    filter:
      drop-shadow(0 0 1px var(--highlight))
      drop-shadow(0 0 1px var(--highlight))
      drop-shadow(0 8px 14px rgba(0, 0, 0, 0.3));
    /* 呼吸动画优先于 hover 的 transform 和 piece 的 transition */
    transition: none !important;
    animation: piece-breathe 1.1s var(--ease) infinite;
    pointer-events: none;
  }
  @keyframes piece-breathe {
    0%, 100% { transform: translateY(-10px) scale(1.15); }
    50%      { transform: translateY(-14px) scale(1.30); }
  }

  /* 举棋落点圆圈：脉冲提示目标格 */
  .pick-marker {
    position: absolute;
    width: 34%;
    height: 34%;
    border-radius: 50%;
    border: 3px solid var(--accent);
    background: var(--accent-soft);
    z-index: 1;
    pointer-events: none;
    animation: pulse-pick 1.1s var(--ease) infinite;
  }
  @keyframes pulse-pick {
    0%, 100% { transform: scale(0.85); opacity: 0.6; }
    50% { transform: scale(1.15); opacity: 1; }
  }

  /* AI 举棋路径箭头层：覆盖在棋盘上，不拦截点击 */
  .pick-arrow-layer {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    pointer-events: none;
    z-index: 6;
  }

  .eval-arrow-layer {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    pointer-events: none;
    z-index: 7;
  }
  .eval-arrow-layer path {
    transition: fill 0.2s var(--ease);
  }

  .dot {
    position: absolute;
    width: 26%;
    height: 26%;
    border-radius: 50%;
    background: var(--accent);
    opacity: 0.45;
    z-index: 1;
    pointer-events: none;
  }

  .ring {
    position: absolute;
    inset: 6%;
    border-radius: 50%;
    border: 4px solid var(--accent);
    opacity: 0.55;
    z-index: 1;
    pointer-events: none;
  }

  .coord {
    position: absolute;
    font-family: var(--font-display);
    font-size: 10px;
    font-weight: 500;
    pointer-events: none;
    opacity: 0.75;
  }
  .rank-coord {
    top: 3px;
    left: 4px;
  }
  .file-coord {
    bottom: 2px;
    right: 5px;
  }
  .light .coord {
    color: var(--board-dark);
  }
  .dark .coord {
    color: var(--board-light);
  }

  /* ===== 思考条（AI 对面侧，单行轮换） ===== */
  /* 绝对定位：脱离 board-wrap 的 flex 布局流，出现/消失时不挤压棋盘，避免颤动 */
  .thinking-strip {
    position: absolute;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    width: min(72vh, 92vw, 560px);
    padding: var(--sp-2) var(--sp-4);
    background: var(--surface);
    border: 1px solid var(--line);
    border-radius: var(--r-md);
    box-shadow: 0 2px 12px rgba(0, 0, 0, 0.3);
    animation: fadeIn 0.3s var(--ease) both;
    z-index: 5;
  }
  /* AI 在上方时，思考条放在棋盘上方 */
  .board-frame.ai-top .thinking-strip {
    bottom: 100%;
    margin-bottom: var(--sp-3);
  }
  /* AI 在下方时，思考条放在棋盘下方 */
  .board-frame:not(.ai-top) .thinking-strip {
    top: 100%;
    margin-top: var(--sp-3);
  }
  .thinking-pulse {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--highlight);
    animation: breathe 1.4s var(--ease) infinite;
    flex-shrink: 0;
  }
  .thinking-text {
    flex: 1;
    font-family: var(--font-display);
    font-size: 13px;
    color: var(--ink-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    letter-spacing: 0.01em;
  }

  /* ===== 遮罩层 ===== */
  .overlay {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.65);
    backdrop-filter: blur(2px);
    z-index: 10;
    animation: fadeIn 0.25s var(--ease) both;
  }

  /* 升变选择器 */
  .promo-card {
    background: var(--surface);
    padding: var(--sp-6) var(--sp-5);
    border-radius: var(--r-lg);
    display: flex;
    flex-direction: column;
    gap: var(--sp-4);
    align-items: center;
  }
  .promo-title {
    font-family: var(--font-display);
    font-size: 15px;
    font-weight: 500;
    color: var(--ink);
    letter-spacing: 0.02em;
  }
  .promo-pieces {
    display: flex;
    gap: var(--sp-2);
  }
  .promo-btn {
    width: 56px;
    height: 56px;
    border: 1px solid var(--line);
    background: var(--bg);
    border-radius: var(--r-md);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.2s var(--ease);
  }
  .promo-btn:hover {
    border-color: var(--accent);
    background: var(--accent-soft);
    transform: translateY(-2px);
  }
  .promo-btn .piece {
    width: 36px;
    height: 36px;
  }

  /* 游戏结束遮罩 */
  .end-overlay {
    background: rgba(0, 0, 0, 0.75);
  }
  .end-overlay.mate {
    animation: mate-converge 0.8s var(--ease) both;
  }
  @keyframes mate-converge {
    0% {
      backdrop-filter: blur(0px);
      background: rgba(184, 68, 46, 0);
    }
    100% {
      backdrop-filter: blur(4px);
      background: rgba(0, 0, 0, 0.8);
    }
  }
  .end-overlay.mate .end-card {
    animation: rise 0.6s var(--ease) 0.2s both;
  }
  .end-card {
    text-align: center;
    padding: var(--sp-7) var(--sp-8);
  }
  .end-title {
    font-family: var(--font-display);
    font-size: 42px;
    font-weight: 500;
    color: var(--ink);
    letter-spacing: -0.02em;
    line-height: 1.1;
  }
  .end-sub {
    font-size: 13px;
    color: var(--ink-muted);
    margin-top: var(--sp-2);
    letter-spacing: 0.05em;
    text-transform: uppercase;
  }
</style>
