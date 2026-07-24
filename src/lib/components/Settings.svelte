<script lang="ts">
  import { settings, updateSettings } from "../stores/settings";
  import { startGame, aiMove, updateSettingsApi } from "../api";
  import { updateGameState, showError, aiThinking, aiReasoning, aiFailed } from "../stores/game";
  import type { Side } from "../types";

  let {
    onStarted = () => {},
    onExit = () => {},
  }: {
    onStarted?: () => void;
    onExit?: () => void;
  } = $props();

  let apiKey = $derived($settings.apiKey);
  let model = $derived($settings.model);
  let thinking = $derived($settings.thinking);
  let pseudoThinking = $derived($settings.pseudoThinking);
  let thinkingLanguage = $derived($settings.thinkingLanguage);
  let reasoningEffort = $derived($settings.reasoningEffort);
  let minThinkingTokens = $derived($settings.minThinkingTokens);
  let side = $derived($settings.side);
  let started = $derived($settings.started);
  let starting = $state(false);
  let saving = $state(false);

  const models = [
    { id: "deepseek-v4-flash", label: "V4 Flash · 快速" },
    { id: "deepseek-v4-pro", label: "V4 Pro · 强力" },
  ];

  // 切换模型时不自动改变思考模式开关（由用户独立控制）
  function selectModel(id: string) {
    updateSettings({ model: id });
  }

  async function handleStart() {
    if (!apiKey.trim()) {
      showError("请填入 API Key");
      return;
    }
    starting = true;
    try {
      aiReasoning.set("");
      aiFailed.set(false);
      const state = await startGame({
        side,
        api_key: apiKey,
        model,
        thinking,
        pseudo_thinking: pseudoThinking,
        thinking_language: thinkingLanguage,
        reasoning_effort: reasoningEffort,
        min_thinking_tokens: minThinkingTokens,
      });
      updateSettings({ started: true });
      updateGameState(state);
      onStarted();
      // 若玩家执黑，AI 先走
      if (side === "black") {
        triggerAiFirst();
      }
    } catch (e) {
      showError(String(e));
    } finally {
      starting = false;
    }
  }

  async function triggerAiFirst() {
    aiThinking.set(true);
    try {
      const result = await aiMove();
      updateGameState(result.state);
    } catch (e) {
      showError(String(e));
      aiFailed.set(true);
    } finally {
      aiThinking.set(false);
    }
  }

  // 游戏中修改设置后，即时应用到后端（不重开游戏）
  async function handleApplySettings() {
    if (!apiKey.trim()) {
      showError("请填入 API Key");
      return;
    }
    saving = true;
    try {
      await updateSettingsApi(apiKey, model, thinking, pseudoThinking, thinkingLanguage, reasoningEffort, minThinkingTokens);
      showError("设置已应用");
    } catch (e) {
      showError(String(e));
    } finally {
      saving = false;
    }
  }

</script>

<div class="settings">
  <div class="section">
    <div class="label">对弈设置</div>
    <p class="hint">与 DeepSeek 对局。填入你的 API Key 即可开始。</p>
  </div>

  <div class="section">
    <div class="label">API Key</div>
    <input
      class="input-line"
      type="password"
      placeholder="sk-..."
      value={apiKey}
      oninput={(e) => updateSettings({ apiKey: e.currentTarget.value })}
    />
  </div>

  <div class="section">
    <div class="label">模型</div>
    <div class="seg-group">
      {#each models as m}
        <button
          class="seg-btn"
          class:active={model === m.id}
          onclick={() => selectModel(m.id)}
        >{m.label}</button>
      {/each}
    </div>
  </div>

  <div class="section">
    <div class="label-row">
      <span class="label">思考模式</span>
      <button
        class="toggle"
        class:on={thinking && !pseudoThinking}
        onclick={() => { updateSettings({ thinking: !thinking, pseudoThinking: false }); }}
        role="switch"
        aria-checked={thinking && !pseudoThinking}
        aria-label="思考模式开关"
      >
        <span class="toggle-thumb"></span>
      </button>
    </div>
    <p class="hint">
      {thinking && !pseudoThinking ? "开启：模型先思考再走棋，更强但更慢更贵。" : "关闭：直接走棋，快速省 token。"}
    </p>
  </div>

  {#if thinking && !pseudoThinking}
    <div class="section">
      <div class="label">思考强度</div>
      <div class="seg-group">
        <button
          class="seg-btn"
          class:active={reasoningEffort === "high"}
          onclick={() => updateSettings({ reasoningEffort: "high" })}
        >High · 省钱</button>
        <button
          class="seg-btn"
          class:active={reasoningEffort === "max"}
          onclick={() => updateSettings({ reasoningEffort: "max" })}
        >Max · 最强</button>
      </div>
      <p class="hint">High 平衡强度与成本（推荐）；Max 思考最深但输出 token 更多更贵。</p>
    </div>
  {/if}

  <div class="section">
    <div class="label-row">
      <span class="label">伪思考模式</span>
      <button
        class="toggle"
        class:on={pseudoThinking}
        onclick={() => { updateSettings({ pseudoThinking: !pseudoThinking, thinking: false }); }}
        role="switch"
        aria-checked={pseudoThinking}
        aria-label="伪思考模式开关"
      >
        <span class="toggle-thumb"></span>
      </button>
    </div>
    <p class="hint">
      {pseudoThinking
        ? "开启：关闭 API thinking，用提示词在 content 中模拟思考输出。完全由提示词控制格式，省思考模式输出 token 成本。"
        : "关闭 API thinking，改用提示词模拟思考流。提示词完全控制输出格式，更省钱且可控。"}
    </p>
  </div>

  <div class="section">
    <div class="label">思考语言</div>
    <div class="seg-group">
      <button
        class="seg-btn"
        class:active={thinkingLanguage === "zh"}
        onclick={() => updateSettings({ thinkingLanguage: "zh" })}
      >中文</button>
      <button
        class="seg-btn"
        class:active={thinkingLanguage === "en"}
        onclick={() => updateSettings({ thinkingLanguage: "en" })}
      >English</button>
    </div>
    <p class="hint">AI 思考过程的语言。English 更省 token（推荐省钱）。思考使用精简关键词。</p>
  </div>

  <div class="section">
    <div class="label-row">
      <span class="label">最少思考 token 数</span>
      <input
        type="number"
        bind:value={$settings.minThinkingTokens}
        min="0"
        max="4096"
        step="50"
        class="number-input"
        aria-label="最少思考token数"
      />
    </div>
    <p class="hint">
      强制 AI 输出至少这么多 token 的思考内容（0=不限制）。伪思考模式默认至少 300 token。数值越大思考越深入但成本越高。推荐 300-800。
    </p>
  </div>

  <div class="section">
    <div class="label">执方</div>
    <div class="seg-group">
      <button
        class="seg-btn"
        class:active={side === "white"}
        onclick={() => updateSettings({ side: "white" as Side })}
      >白方 先手</button>
      <button
        class="seg-btn"
        class:active={side === "black"}
        onclick={() => updateSettings({ side: "black" as Side })}
      >黑方 后手</button>
    </div>
  </div>

  <div class="actions">
    {#if started}
      <button class="btn-primary" onclick={handleApplySettings} disabled={saving}>
        {saving ? "保存中..." : "应用设置"}
      </button>
      <button class="btn-ghost" onclick={onExit} disabled={starting} style="margin-top: 8px;">
        退出当前对局
      </button>
    {:else}
      <button class="btn-primary" onclick={handleStart} disabled={starting}>
        {starting ? "正在开局..." : "开始对弈"}
      </button>
    {/if}
  </div>
</div>

<style>
  .settings {
    display: flex;
    flex-direction: column;
    gap: var(--sp-5);
    max-width: 560px;
    margin: 0 auto;
  }
  .hint {
    font-size: 12px;
    color: var(--ink-muted);
    margin-top: var(--sp-1);
    line-height: 1.5;
  }
  .label-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .label {
    margin-bottom: 0;
  }
  /* 方形拨动开关（符合用户偏好：方形、内高亮） */
  .toggle {
    width: 40px;
    height: 22px;
    border: 1px solid var(--border);
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
    width: 16px;
    height: 16px;
    background: var(--ink);
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.15);
    transition: transform 0.2s var(--ease);
  }
  .toggle.on .toggle-thumb {
    transform: translateX(18px);
    background: var(--board-light);
  }
  .actions {
    padding-top: var(--sp-3);
  }
  .number-input {
    width: 100px;
    padding: 6px 10px;
    border: 1px solid var(--border);
    background: var(--bg-soft);
    color: var(--ink);
    font-size: 14px;
    font-family: inherit;
    border-radius: 0;
    text-align: right;
  }
  .number-input:focus {
    outline: none;
    border-color: var(--accent);
  }
</style>
