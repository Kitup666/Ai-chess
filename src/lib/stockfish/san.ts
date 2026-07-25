/// UCI 走法 ↔ SAN 走法转换（基于 chess.js）
///
/// Stockfish 的 pv（主要变着）输出的是 UCI 走法（如 "e2e4"），
/// 显示时需转成人类可读的 SAN（如 "e4"、"Nf3"、"O-O"、"exd5"）。
///
/// 转换流程：起始局面 FEN → 逐步走子 → 收集每步的 SAN

import { Chess } from "chess.js";

/// UCI 走法字符串解析为 {from, to, promotion}
/// 如 "e2e4" → {from:"e2", to:"e4"}
/// 如 "e7e8q" → {from:"e7", to:"e8", promotion:"q"}
function parseUci(uci: string): { from: string; to: string; promotion?: string } {
  const from = uci.slice(0, 2);
  const to = uci.slice(2, 4);
  const promotion = uci.length > 4 ? uci[4] : undefined;
  return { from, to, promotion };
}

/// 从 FEN 解析当前半步数（ply）
/// 用于格式化 SAN 时正确显示回合数
/// 如起始局面 → 0；走完 1.e4 后 → 1；走完 1.e4 e5 后 → 2
export function fenToPly(fen: string): number {
  const parts = fen.split(" ");
  const fullmove = parseInt(parts[5] ?? "1", 10) || 1;
  const turn = parts[1] ?? "w";
  return (fullmove - 1) * 2 + (turn === "w" ? 0 : 1);
}

/// 把 UCI 走法序列转成 SAN 走法序列
///
/// @param fen 起始局面 FEN
/// @param uciMoves UCI 走法数组（如 ["e2e4", "e7e5", "g1f3"]）
/// @returns SAN 走法数组（如 ["e4", "e5", "Nf3"]），遇到非法走法停止转换
export function uciMovesToSan(fen: string, uciMoves: string[]): string[] {
  try {
    const chess = new Chess(fen);
    const sans: string[] = [];
    for (const uci of uciMoves) {
      try {
        const move = chess.move(parseUci(uci));
        if (move) sans.push(move.san);
      } catch {
        // 非法走法，停止转换后续（避免局面错乱）
        break;
      }
    }
    return sans;
  } catch {
    return [];
  }
}

/// 格式化 SAN 序列为走法记号字符串
///
/// 如 ["e4", "e5", "Nf3"] + startPly=0 → "1. e4 e5 2. Nf3"
/// 如 ["Nf3"] + startPly=2 → "2... Nf3"（黑方续走）
///
/// @param sans SAN 走法数组
/// @param startPly 起始半步数（默认 0）
export function formatSans(sans: string[], startPly: number = 0): string {
  if (sans.length === 0) return "";
  let result = "";
  for (let i = 0; i < sans.length; i++) {
    const ply = startPly + i;
    const moveNumber = Math.floor(ply / 2) + 1;
    const isWhiteMove = ply % 2 === 0;
    if (isWhiteMove) {
      result += `${moveNumber}. ${sans[i]} `;
    } else {
      // 黑方走：若是序列首步（前一步白方未显示），补 "N... "
      if (i === 0) {
        result += `${moveNumber}... ${sans[i]} `;
      } else {
        result += `${sans[i]} `;
      }
    }
  }
  return result.trim();
}

/// 一步 UCI 转 SAN（便捷函数，用于箭头标签）
export function uciToSan(fen: string, uci: string): string {
  const sans = uciMovesToSan(fen, [uci]);
  return sans[0] ?? uci;
}

/// 把 UCI 走法序列转成格式化的 SAN 字符串
///
/// @param fen 起始局面 FEN
/// @param uciMoves UCI 走法数组
/// @param startPly 起始半步数（默认从 FEN 解析）
export function uciMovesToSanString(fen: string, uciMoves: string[], startPly?: number): string {
  const sans = uciMovesToSan(fen, uciMoves);
  const ply = startPly ?? fenToPly(fen);
  return formatSans(sans, ply);
}
