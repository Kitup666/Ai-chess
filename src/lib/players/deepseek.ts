/// DeepSeek 主体：调用后端 ai_move 命令
///
/// ai_move 内部已应用走法、触发流式思考事件（ai-thinking/ai-pick/ai-usage）、持久化。
/// 前端事件监听仍由 App.svelte 的 onMount 注册，PlayerManager 仅驱动调用。
///
/// 网络/超时错误自动重试 1 次：偶发网络抖动时避免用户手动点"重新请求"。
/// 配置错误（API Key 无效、模型不存在等）和"对局已变更"不重试（重试也无效）。
import type { Player } from "./types";
import type { MoveResult } from "../types";
import { aiMove } from "../api";
import { aiThinking, aiFailed, aiReasoning, aiPick } from "../stores/game";

/// 判断是否为可重试的瞬态错误（网络/超时/服务暂时不可用）
/// 后端在 deepseek.rs 中已将这类错误友好化为中文提示，此处按关键词识别
function isRetryableError(e: unknown): boolean {
  const msg = e instanceof Error ? e.message : String(e);
  // "对局已变更"是预期取消，不重试
  if (msg.includes("对局已变更")) return false;
  // 网络/超时/服务暂时不可用类错误，重试可能成功
  return (
    msg.includes("超时") ||
    msg.includes("网络") ||
    msg.includes("连接") ||
    msg.includes("服务暂时不可用") ||
    msg.includes("限流")
  );
}

/// 延迟工具
function delay(ms: number): Promise<void> {
  return new Promise((r) => setTimeout(r, ms));
}

export function createDeepSeekPlayer(): Player {
  return {
    type: "deepseek",
    isAutomatic: true,
    async requestMove(): Promise<MoveResult> {
      aiThinking.set(true);
      try {
        // 第一次尝试
        return await aiMove();
      } catch (e) {
        // 非可重试错误：直接抛出
        if (!isRetryableError(e)) {
          aiFailed.set(true);
          throw e;
        }
        // 可重试错误：清空上一次的部分思考内容，等待 1 秒后重试 1 次
        aiReasoning.set("");
        aiPick.set(null);
        logRetry(e);
        await delay(1000);
        try {
          return await aiMove();
        } catch (e2) {
          aiFailed.set(true);
          throw e2;
        }
      } finally {
        aiThinking.set(false);
      }
    },
  };
}

/// 重试日志（开发调试用）
function logRetry(e: unknown): void {
  const msg = e instanceof Error ? e.message : String(e);
  console.warn(`[DeepSeek] 瞬态错误，1秒后重试: ${msg}`);
}
