<script lang="ts">
  import { gameState } from "../stores/game";
  import { Chess } from "chess.js";

  /// 标准开局 FEN（用于重放 move_history 转 SAN）
  const INITIAL_FEN = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

  /// 走法历史项：回合号 + 白方 SAN + 黑方 SAN
  interface HistoryRow {
    turn: number;
    white?: string;
    black?: string;
  }

  /// 将 UCI 走法数组转换为 SAN 走法行（两列：白/黑）
  /// 用 chess.js 从初始 FEN 逐步走棋并记录 SAN
  let rows = $derived.by<HistoryRow[]>(() => {
    const uciList = $gameState.move_history;
    if (!uciList || uciList.length === 0) return [];
    const chess = new Chess(INITIAL_FEN);
    const result: HistoryRow[] = [];
    let currentTurn = 1;
    let currentRow: HistoryRow = { turn: currentTurn };
    for (const uci of uciList) {
      try {
        // chess.js 的 move 接受 { from, to, promotion } 对象
        const from = uci.slice(0, 2);
        const to = uci.slice(2, 4);
        const promotion = uci.length > 4 ? uci.slice(4, 5) : undefined;
        const moveRes = chess.move({ from, to, promotion });
        if (!moveRes) continue;
        const san = moveRes.san;
        // 判断当前是白方还是黑方：chess.js move 后 turn 已切换，需用 moveRes.color 判断走子方
        if (moveRes.color === "w") {
          currentRow.white = san;
        } else {
          currentRow.black = san;
          result.push(currentRow);
          currentTurn++;
          currentRow = { turn: currentTurn };
        }
      } catch {
        // 单步转换失败时跳过，保证后续走法仍可显示
        continue;
      }
    }
    // 若最后是白方走（黑方未走），补一行
    if (currentRow.white !== undefined) {
      result.push(currentRow);
    }
    return result;
  });

  /// 当前走法 ply（用于高亮最后一手）
  let currentPly = $derived($gameState.ply);

  /// 自动滚动到当前走法
  let listEl: HTMLDivElement | null = null;
  let userScrolledUp = false;

  function onScroll() {
    if (!listEl) return;
    const atBottom = listEl.scrollHeight - listEl.scrollTop - listEl.clientHeight < 20;
    userScrolledUp = !atBottom;
  }

  $effect(() => {
    // 依赖 rows 与 currentPly，走法变化时触发
    const _rows = rows;
    const _ply = currentPly;
    if (!listEl || userScrolledUp) return;
    // 滚动到底部
    requestAnimationFrame(() => {
      if (listEl) listEl.scrollTop = listEl.scrollHeight;
    });
  });
</script>

<div class="move-history" bind:this={listEl} onscroll={onScroll}>
  {#if rows.length === 0}
    <div class="empty">尚未走棋</div>
  {:else}
    <table class="move-table">
      <tbody>
        {#each rows as row, i}
          <tr class:active={i === rows.length - 1 && row.black === undefined}>
            <td class="turn">{row.turn}.</td>
            <td class="white" class:active={i === rows.length - 1 && row.black === undefined}>{row.white ?? ""}</td>
            <td class="black">{row.black ?? ""}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>

<style>
  .move-history {
    height: 100%;
    overflow-y: auto;
    padding: var(--sp-2);
    font-family: var(--font-mono);
    font-size: 13px;
    color: var(--ink);
  }
  .empty {
    color: var(--ink-faint);
    text-align: center;
    padding: var(--sp-5);
    font-family: var(--font-sans);
    font-size: 12px;
  }
  .move-table {
    width: 100%;
    border-collapse: collapse;
  }
  .move-table td {
    padding: var(--sp-1) var(--sp-2);
    border-bottom: 1px solid var(--line);
  }
  .move-table tr:last-child td {
    border-bottom: none;
  }
  .turn {
    color: var(--ink-faint);
    width: 36px;
    text-align: right;
    user-select: none;
  }
  .white, .black {
    color: var(--ink);
  }
  .move-table tr.active td,
  .move-table td.active {
    background: var(--highlight);
    color: var(--ink);
  }
  .move-table tr:hover td {
    background: var(--surface-2);
  }
  .move-table tr.active:hover td {
    background: var(--highlight);
  }
</style>
