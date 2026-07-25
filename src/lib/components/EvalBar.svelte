<script lang="ts">
  /// 评估柱（Eval Bar）- lichess 风格的垂直胜率柱
  ///
  /// 棋盘左侧紧贴的垂直柱状条，白黑两段随评估值变化高度。
  /// 白优→白色段扩大，黑优→黑色段扩大，mate→占满+脉冲。
  ///
  /// 评估值来源：multiPVList[0].score（最佳走法的分数）
  /// 视角转换：UCI score 是当前轮到方视角，转为白方视角后显示

  import { stableScore, scoreFromWhitePerspective } from "../stockfish/store";
  import { gameState } from "../stores/game";
  import { boardEffectiveFlipped } from "../stores/boardOrientation";

  // 白方视角评估值（厘兵，mate 用 ±100000）
  // 使用 stableScore：只在深度递增时更新，避免同深度内多次变化导致跳动
  let whiteScore = $derived.by(() => {
    const stable = $stableScore;
    if (!stable?.score) return null;
    return scoreFromWhitePerspective(stable.score, $gameState.turn as "white" | "black");
  });

  // 棋盘是否翻转：使用共享 derived store，与 Board 保持一致
  let flipped = $derived($boardEffectiveFlipped);

  // 白色段占比（0~1）
  // 线性映射：±5 兵（±500 厘兵）→ 0~1，超出截断
  let whiteRatio = $derived.by(() => {
    if (whiteScore === null) return 0.5;
    if (whiteScore >= 100000) return 1;
    if (whiteScore <= -100000) return 0;
    const clamped = Math.max(-500, Math.min(500, whiteScore));
    return 0.5 + clamped / 1000;
  });

  // 数值标签
  let evalText = $derived.by(() => {
    if (whiteScore === null) return "";
    if (whiteScore >= 100000) {
      const mateMoves = 100000 - whiteScore + 1;
      return `M${mateMoves}`;
    }
    if (whiteScore <= -100000) {
      const mateMoves = 100000 - Math.abs(whiteScore) + 1;
      return `-M${mateMoves}`;
    }
    const val = whiteScore / 100;
    return (val >= 0 ? "+" : "") + val.toFixed(2);
  });

  // 是否将杀
  let isWhiteMate = $derived(whiteScore !== null && whiteScore >= 100000);
  let isBlackMate = $derived(whiteScore !== null && whiteScore <= -100000);
</script>

<div
  class="eval-bar"
  class:flipped
  class:white-mate={isWhiteMate}
  class:black-mate={isBlackMate}
  aria-label="引擎评估柱"
  title={evalText ? `评估 ${evalText}` : "等待引擎评估"}
>
  <!-- 白色段（默认在底部，翻转后在顶部） -->
  <div class="bar-white" style="height: {whiteRatio * 100}%"></div>
  <!-- 黑色段（默认在顶部，翻转后在底部） -->
  <div class="bar-black" style="height: {(1 - whiteRatio) * 100}%"></div>

  <!-- 评估数值标签：固定在黑段所在端 -->
  {#if evalText}
    <span class="eval-label" class:at-bottom={flipped}>{evalText}</span>
  {:else}
    <span class="eval-label" class:at-bottom={flipped}>—</span>
  {/if}
</div>

<style>
  .eval-bar {
    width: 24px;
    /* 高度与棋盘 .board-wrap 一致，确保视觉对齐 */
    height: min(72vh, 92vw, 560px);
    position: relative;
    display: flex;
    flex-direction: column-reverse; /* 白段在底部 */
    border-radius: 3px;
    overflow: hidden;
    box-shadow:
      inset 0 0 0 1px rgba(0, 0, 0, 0.4),
      0 1px 3px rgba(0, 0, 0, 0.3);
    flex-shrink: 0;
    background: #1a1a1a; /* 默认黑色底（无数据时） */
    transition: box-shadow 0.3s var(--ease);
  }

  /* 翻转：白段在顶部 */
  .eval-bar.flipped {
    flex-direction: column;
  }

  .bar-white {
    background: linear-gradient(to top, #e8e6e3, #ffffff);
    transition: height 0.4s cubic-bezier(0.22, 1, 0.36, 1);
    flex-shrink: 0;
  }

  .bar-black {
    background: linear-gradient(to bottom, #2a2a2a, #1a1a1a);
    transition: height 0.4s cubic-bezier(0.22, 1, 0.36, 1);
    flex-shrink: 0;
  }

  /* 评估数值标签 */
  .eval-label {
    position: absolute;
    top: 6px;
    left: 50%;
    transform: translateX(-50%);
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: 700;
    color: #e8e6e3;
    background: rgba(0, 0, 0, 0.65);
    padding: 3px 6px;
    border-radius: 3px;
    white-space: nowrap;
    pointer-events: none;
    letter-spacing: 0.02em;
    line-height: 1;
  }

  /* 翻转时标签在底部（黑段在底部） */
  .eval-label.at-bottom {
    top: auto;
    bottom: 6px;
  }

  /* 将杀时脉冲动画 */
  .eval-bar.white-mate {
    box-shadow:
      inset 0 0 0 1px rgba(98, 153, 36, 0.4),
      0 0 0 2px rgba(98, 153, 36, 0.3);
    animation: mate-pulse-white 1.2s var(--ease) infinite;
  }
  .eval-bar.black-mate {
    box-shadow:
      inset 0 0 0 1px rgba(184, 68, 46, 0.4),
      0 0 0 2px rgba(184, 68, 46, 0.3);
    animation: mate-pulse-black 1.2s var(--ease) infinite;
  }

  @keyframes mate-pulse-white {
    0%, 100% {
      box-shadow:
        inset 0 0 0 1px rgba(98, 153, 36, 0.4),
        0 0 0 2px rgba(98, 153, 36, 0.3);
    }
    50% {
      box-shadow:
        inset 0 0 0 1px rgba(98, 153, 36, 0.7),
        0 0 0 4px rgba(98, 153, 36, 0.15);
    }
  }
  @keyframes mate-pulse-black {
    0%, 100% {
      box-shadow:
        inset 0 0 0 1px rgba(184, 68, 46, 0.4),
        0 0 0 2px rgba(184, 68, 46, 0.3);
    }
    50% {
      box-shadow:
        inset 0 0 0 1px rgba(184, 68, 46, 0.7),
        0 0 0 4px rgba(184, 68, 46, 0.15);
    }
  }
</style>
