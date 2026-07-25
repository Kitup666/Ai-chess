/// Human 主体：等待玩家点击棋盘走棋，不主动请求
import type { Player } from "./types";

export function createHumanPlayer(): Player {
  return { type: "human", isAutomatic: false };
}
