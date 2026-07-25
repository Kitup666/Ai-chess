<script lang="ts">
  import { aiReasoning, aiThinking, showThinking } from "../stores/game";

  let reasoningText = $derived($aiReasoning);
  let isThinking = $derived($aiThinking);
  let showDetail = $derived($showThinking);

  let lines = $derived(
    reasoningText.split('\n').map(s => s.trim()).filter(Boolean)
  );

  let panelBody: HTMLDivElement | undefined = $state();

  $effect(() => {
    if (reasoningText && panelBody) {
      panelBody.scrollTop = panelBody.scrollHeight;
    }
  });
</script>

<div class="thinking-panel" class:active={isThinking} class:idle={!isThinking && lines.length === 0}>
  <div class="panel-header">
    <svg class="header-icon" viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.3">
      <path d="M3 2h10M3 6h10M3 10h7M3 14h10" stroke-linecap="round"/>
    </svg>
    <span class="panel-title">思维链</span>
    {#if isThinking}
      <span class="thinking-dot"></span>
    {/if}
    {#if lines.length > 0}
      <span class="line-count">{lines.length}</span>
    {/if}
  </div>
  <div class="panel-body" bind:this={panelBody}>
    {#if lines.length > 0 && showDetail}
      <div class="reasoning-content">
        {#each lines as line, i}
          <p class="reasoning-line" class:latest={i === lines.length - 1 && isThinking}>
            <span class="line-num">{i + 1}</span>
            <span class="line-text">{line}</span>
          </p>
        {/each}
      </div>
    {:else if isThinking}
      <div class="placeholder">
        <svg class="placeholder-icon" viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round">
          <path d="M12 2a4 4 0 0 1 4 4c0 2-2 3-2 5v2h-4v-2c0-2-2-3-2-5a4 4 0 0 1 4-4z"/>
          <path d="M10 18h4v2a2 2 0 0 1-4 0v-2z"/>
        </svg>
        <p class="placeholder-text">思考中…</p>
      </div>
    {:else}
      <div class="placeholder">
        <svg class="placeholder-icon" viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linecap="round">
          <circle cx="12" cy="12" r="9"/>
          <path d="M9 9l6 6M15 9l-6 6"/>
        </svg>
        <p class="placeholder-text">暂无思考</p>
      </div>
    {/if}
  </div>
</div>

<style>
  .thinking-panel {
    width: 220px;
    height: min(72vh, 92vw, 560px);
    display: flex;
    flex-direction: column;
    background: var(--surface);
    border: 1px solid var(--line);
    border-radius: var(--r-md);
    overflow: hidden;
    flex-shrink: 0;
    transition: border-color 0.4s var(--ease), box-shadow 0.4s var(--ease);
  }
  .thinking-panel.active {
    border-color: rgba(98, 153, 36, 0.5);
    box-shadow:
      0 0 0 1px rgba(98, 153, 36, 0.15),
      inset 0 0 20px rgba(98, 153, 36, 0.03);
  }
  .thinking-panel.idle {
    opacity: 0.5;
  }

  .panel-header {
    display: flex;
    align-items: center;
    gap: var(--sp-1);
    padding: var(--sp-2) var(--sp-3);
    border-bottom: 1px solid var(--line);
    font-family: var(--font-sans);
    font-size: 11px;
    font-weight: 500;
    color: var(--ink-faint);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    flex-shrink: 0;
    user-select: none;
  }
  .header-icon {
    opacity: 0.6;
    flex-shrink: 0;
  }
  .panel-title {
    flex: 1;
  }
  .thinking-dot {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--accent);
    animation: breathe 1.2s var(--ease) infinite;
    flex-shrink: 0;
  }
  .line-count {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--ink-faint);
    background: var(--surface-2);
    padding: 1px 6px;
    border-radius: 8px;
    flex-shrink: 0;
  }

  .panel-body {
    flex: 1;
    overflow-y: auto;
    scrollbar-width: thin;
    scrollbar-color: var(--line) transparent;
  }
  .panel-body::-webkit-scrollbar {
    width: 4px;
  }
  .panel-body::-webkit-scrollbar-track {
    background: transparent;
  }
  .panel-body::-webkit-scrollbar-thumb {
    background: var(--line);
    border-radius: 2px;
  }

  .reasoning-content {
    display: flex;
    flex-direction: column;
    padding: var(--sp-1) 0;
  }
  .reasoning-line {
    display: flex;
    margin: 0;
    font-size: 11px;
    line-height: 1.7;
    padding: 0 var(--sp-3);
    transition: background 0.3s var(--ease);
  }
  .reasoning-line.latest {
    background: linear-gradient(90deg, rgba(98, 153, 36, 0.06), transparent);
  }
  .reasoning-line:hover {
    background: var(--surface-2);
  }
  .line-num {
    flex-shrink: 0;
    width: 22px;
    font-family: var(--font-mono);
    font-size: 9px;
    color: var(--ink-faint);
    opacity: 0.4;
    text-align: right;
    padding-right: var(--sp-2);
    user-select: none;
  }
  .line-text {
    flex: 1;
    color: var(--ink-muted);
    font-family: var(--font-mono);
    word-break: break-all;
  }
  .reasoning-line.latest .line-text {
    color: var(--ink);
  }

  .placeholder {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    gap: var(--sp-2);
  }
  .placeholder-icon {
    opacity: 0.2;
  }
  .placeholder-text {
    font-size: 12px;
    color: var(--ink-faint);
    opacity: 0.4;
    margin: 0;
  }

  @keyframes breathe {
    0%, 100% { opacity: 1; transform: scale(1); }
    50% { opacity: 0.5; transform: scale(0.85); }
  }
</style>
