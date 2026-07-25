<script lang="ts">
  import { settings, updateSettings } from "../stores/settings";
  import { startGame, updateSettingsApi, resetGame } from "../api";
  import { gameState, updateGameState, showError, aiReasoning, aiFailed } from "../stores/game";
  import { resetManager, driveTurn, stopAutoPlay } from "../stores/playerManager";
  import type { PlayerType } from "../types";
  import {
    getSoundVolume,
    setSoundVolume,
    getSoundEnabled,
    setSoundEnabled,
    playSound,
  } from "../sounds/player";

  // 音效设置（本地 state，与 localStorage 同步）
  let soundEnabled = $state(getSoundEnabled());
  let soundVolume = $state(getSoundVolume());

  function toggleSound() {
    soundEnabled = !soundEnabled;
    setSoundEnabled(soundEnabled);
    // 开启时立即试听一次
    if (soundEnabled) playSound("move");
  }

  function onVolumeChange(e: Event) {
    const v = Number((e.currentTarget as HTMLInputElement).value);
    soundVolume = v;
    setSoundVolume(v);
  }

  // 拖动滑块时实时试听
  function onVolumeInput(e: Event) {
    const v = Number((e.currentTarget as HTMLInputElement).value);
    soundVolume = v;
    setSoundVolume(v);
    playSound("move");
  }

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
  let selfConsistencySamples = $derived($settings.selfConsistencySamples);
  let started = $derived($settings.started);
  let whitePlayer = $derived($settings.whitePlayer);
  let blackPlayer = $derived($settings.blackPlayer);
  let stockfishElo = $derived($settings.stockfishElo);
  let stockfishSkill = $derived($settings.stockfishSkill);
  let useStockfishElo = $derived($settings.useStockfishElo);
  let starting = $state(false);
  let saving = $state(false);

  const models = [
    { id: "deepseek-v4-flash", label: "V4 Flash · 快速" },
    { id: "deepseek-v4-pro", label: "V4 Pro · 强力" },
  ];

  const playerOptions: { id: PlayerType; label: string }[] = [
    { id: "human", label: "人" },
    { id: "stockfish", label: "鳕鱼" },
    { id: "deepseek", label: "DeepSeek" },
  ];

  /// 任一方为 DeepSeek 时需要 API Key
  let needApiKey = $derived(whitePlayer === "deepseek" || blackPlayer === "deepseek");
  /// 任一方为 Stockfish 时显示鳕鱼难度
  let showStockfishOpts = $derived(whitePlayer === "stockfish" || blackPlayer === "stockfish");

  // 切换模型时不自动改变思考模式开关（由用户独立控制）
  function selectModel(id: string) {
    updateSettings({ model: id });
  }

  function selectWhitePlayer(p: PlayerType) {
    updateSettings({ whitePlayer: p });
  }
  function selectBlackPlayer(p: PlayerType) {
    updateSettings({ blackPlayer: p });
  }

  async function handleStart() {
    if (needApiKey && !apiKey.trim()) {
      showError("请填入 API Key");
      return;
    }
    starting = true;
    try {
      aiReasoning.set("");
      aiFailed.set(false);
      const state = await startGame({
        side: "white",
        api_key: apiKey,
        model,
        thinking,
        pseudo_thinking: pseudoThinking,
        thinking_language: thinkingLanguage,
        reasoning_effort: reasoningEffort,
        min_thinking_tokens: minThinkingTokens,
        self_consistency_samples: selfConsistencySamples,
        white_player: whitePlayer,
        black_player: blackPlayer,
      });
      // 重置 PlayerManager（按新主体组合构造）
      resetManager(whitePlayer, blackPlayer);
      updateSettings({ started: true });
      updateGameState(state);
      onStarted();
      // 对局开始：播放 Lichess 通知音
      playSound("gameStart");
      // 驱动当前轮到方走棋（若白方是自动主体则开始走，若白方是人则等待点击）
      // 用 setTimeout 确保 UI 先渲染
      setTimeout(() => {
        driveTurn(state).catch((e) => {
          showError(String(e));
          aiFailed.set(true);
        });
      }, 0);
    } catch (e) {
      showError(String(e));
    } finally {
      starting = false;
    }
  }

  // 游戏中修改设置后，即时应用到后端
  // 主体变更时自动重开游戏（避免 PlayerManager 与后端状态不一致导致双人模式误触发 AI）
  async function handleApplySettings() {
    if (needApiKey && !apiKey.trim()) {
      showError("请填入 API Key");
      return;
    }
    saving = true;
    try {
      // 先更新 DeepSeek 设置（API Key/模型/思考模式等）
      await updateSettingsApi(apiKey, model, thinking, pseudoThinking, thinkingLanguage, reasoningEffort, minThinkingTokens, selfConsistencySamples);
      // 检测主体是否变更：与当前对局的主体比较
      const g = $gameState;
      const playerChanged = g.white_player !== whitePlayer || g.black_player !== blackPlayer;
      if (playerChanged) {
        // 主体变更需重开游戏，重建 PlayerManager 和后端 game state
        stopAutoPlay();
        aiReasoning.set("");
        aiFailed.set(false);
        const state = await resetGame("white", whitePlayer, blackPlayer);
        resetManager(whitePlayer, blackPlayer);
        updateGameState(state);
        showError("主体已变更，对局已重开");
        // 驱动当前轮到方走棋（若白方是自动主体则开始走，若白方是人则等待点击）
        setTimeout(() => {
          driveTurn(state).catch((e) => {
            showError(String(e));
            aiFailed.set(true);
          });
        }, 0);
      } else {
        showError("设置已应用");
      }
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
    <div class="label">白方主体</div>
    <div class="seg-group">
      {#each playerOptions as opt}
        <button
          class="seg-btn"
          class:active={whitePlayer === opt.id}
          onclick={() => selectWhitePlayer(opt.id)}
        >{opt.label}</button>
      {/each}
    </div>
  </div>

  <div class="section">
    <div class="label">黑方主体</div>
    <div class="seg-group">
      {#each playerOptions as opt}
        <button
          class="seg-btn"
          class:active={blackPlayer === opt.id}
          onclick={() => selectBlackPlayer(opt.id)}
        >{opt.label}</button>
      {/each}
    </div>
    <p class="hint">
      三方主体可任意排列组合，亦可自对弈（鳕鱼 vs 鳕鱼、DeepSeek vs DeepSeek 等）。
      {whitePlayer === "human" && blackPlayer === "human" ? "双人模式：双方均手动走棋。" : ""}
      {whitePlayer === blackPlayer && whitePlayer !== "human" ? "自对弈模式：步间自动延迟，可随时停止。" : ""}
    </p>
  </div>

  <div class="section">
    <div class="label">音效</div>
    <div class="sound-row">
      <button
        class="seg-btn"
        class:active={!soundEnabled}
        onclick={toggleSound}
        aria-label={soundEnabled ? "静音" : "取消静音"}
      >{soundEnabled ? "🔊 开启" : "🔇 静音"}</button>
      <input
        type="range"
        min="0"
        max="100"
        step="1"
        bind:value={soundVolume}
        oninput={onVolumeInput}
        onchange={onVolumeChange}
        class="slider"
        aria-label="音量"
        disabled={!soundEnabled}
      />
      <span class="slider-value">{soundVolume}</span>
    </div>
    <p class="hint">音效来自 Lichess 官方（AGPL）。走子/吃子/将军/将杀/升变/易位各有独立音效。</p>
  </div>

  {#if showStockfishOpts}
    <div class="section">
      <div class="label">鳕鱼难度</div>
      <div class="seg-group">
        <button
          class="seg-btn"
          class:active={useStockfishElo}
          onclick={() => updateSettings({ useStockfishElo: true })}
        >ELO 等级</button>
        <button
          class="seg-btn"
          class:active={!useStockfishElo}
          onclick={() => updateSettings({ useStockfishElo: false })}
        >Skill Level</button>
      </div>
      {#if useStockfishElo}
        <div class="slider-row">
          <input
            type="range"
            min="1320"
            max="3190"
            step="20"
            bind:value={$settings.stockfishElo}
            class="slider"
            aria-label="鳕鱼 ELO 等级"
          />
          <span class="slider-value">{$settings.stockfishElo}</span>
        </div>
        <p class="hint">ELO 1320-3190（UCI_Elo 限制思考时间）。1500=业余，2400=大师，3000+=特级大师。</p>
      {:else}
        <div class="slider-row">
          <input
            type="range"
            min="0"
            max="20"
            step="1"
            bind:value={$settings.stockfishSkill}
            class="slider"
            aria-label="鳕鱼 Skill Level"
          />
          <span class="slider-value">{$settings.stockfishSkill}</span>
        </div>
        <p class="hint">Skill 0-20。10=中等人类，20=最强，0=最弱（随机走法）。</p>
      {/if}
    </div>
  {/if}

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
    height: 100%;
    overflow-y: auto;
    padding: var(--sp-4);
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
  /* 鳕鱼难度滑块 */
  .slider-row {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    margin-top: var(--sp-2);
  }
  .slider {
    flex: 1;
    -webkit-appearance: none;
    appearance: none;
    height: 2px;
    background: var(--line);
    outline: none;
    cursor: pointer;
  }
  .slider::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 14px;
    height: 14px;
    background: var(--accent);
    border: none;
    cursor: pointer;
    transition: transform 0.15s var(--ease);
  }
  .slider::-webkit-slider-thumb:hover {
    transform: scale(1.2);
  }
  .slider::-moz-range-thumb {
    width: 14px;
    height: 14px;
    background: var(--accent);
    border: none;
    cursor: pointer;
  }
  .slider-value {
    font-family: var(--font-mono);
    font-size: 13px;
    color: var(--ink);
    min-width: 48px;
    text-align: right;
  }
  /* 音效设置行：静音按钮 + 音量滑块 */
  .sound-row {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    margin-top: var(--sp-2);
  }
  .sound-row .seg-btn {
    flex-shrink: 0;
    min-width: 92px;
  }
  .sound-row .slider:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
</style>
