/// DeepSeek 主体：调用后端 ai_move 命令
///
/// ai_move 内部已应用走法、触发流式思考事件（ai-thinking/ai-pick/ai-usage）、持久化。
/// 前端事件监听仍由 App.svelte 的 onMount 注册，PlayerManager 仅驱动调用。
import type { Player } from "./types";
import type { MoveResult } from "../types";
import { aiMove } from "../api";
import { aiThinking, aiFailed } from "../stores/game";

export function createDeepSeekPlayer(): Player {
  return {
    type: "deepseek",
    isAutomatic: true,
    async requestMove(): Promise<MoveResult> {
      aiThinking.set(true);
      try {
        // ai_move 内部已应用走法、触发流式思考事件、持久化
        return await aiMove();
      } catch (e) {
        aiFailed.set(true);
        throw e;
      } finally {
        aiThinking.set(false);
      }
    },
  };
}
