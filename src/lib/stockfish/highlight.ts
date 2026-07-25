/// 引擎分析高亮状态
///
/// 跟踪当前高亮的 PV 索引（鼠标 hover 或点击 AnalysisPanel 某条 PV 时）。
/// Board.svelte 据此绘制对应走法的箭头（最佳=绿色，次佳=黄色，高亮=加粗）。

import { writable } from "svelte/store";

/// 当前高亮的 PV 索引（1-based，null=无高亮，0=最佳走法）
export const highlightedPV = writable<number | null>(null);
