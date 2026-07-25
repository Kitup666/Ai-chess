---
intent: 把鳕鱼分析面板从右侧抽屉改为常驻侧栏，评估柱加宽且数值清晰，让用户无需点击即可一眼看到评估值、深度、PV 变着和引擎控制。
success_criteria: 启动应用进入对局后右侧分析面板常驻可见；评估柱宽度 ≥24px 数值字号 ≥11px；引擎未加载时显示"启用引擎"占位；桌面三栏布局；走棋后评估/PV 实时刷新无闪烁；窄屏 <900px 退化为抽屉模式。
risk_level: medium
auto_approve: true
---

## Steps

- [ ] **Step 1: EvalBar 加宽与数值放大**
action: 修改 `src/lib/components/EvalBar.svelte`：将 `.eval-bar` 的 `width` 从 `18px` 改为 `24px`；将 `.eval-label` 的 `font-size` 从 `10px` 改为 `11px`，`padding` 从 `2px 5px` 改为 `3px 6px`；在 `evalText` 为空时（`whiteScore === null`）让模板渲染 "—" 而非空字符串（修改模板中的 `{#if evalText}` 块，添加 `{:else}` 分支显示 "—"）。
loop: false
verify: cd "c:\Users\24453\Desktop\AI国象" ; npm run check 2>&1 | Select-String "0 errors"

- [ ] **Step 2: AnalysisPanel 移除标题栏并添加未加载占位**
action: 修改 `src/lib/components/AnalysisPanel.svelte`：删除 `<header class="panel-header">` 整块（包含"引擎分析"标题与关闭按钮）；在组件顶部添加一个窄工具条：右侧保留一个 `✕` 关闭按钮（通过新增 prop `closable: boolean = false` 控制，仅窄屏抽屉模式传入 true，桌面常驻模式默认 false 隐藏）；当 `$engineStatus === "unloaded"` 时，主体区域渲染居中占位卡片，包含文案"启用鳕鱼引擎获取实时评估"和按钮"启用引擎"（onclick 调用 `loadEngine` 后 `analyzePosition($gameState.fen)`）；已加载时按原逻辑渲染控制区+进度区+PV 列表。
loop: false
verify: cd "c:\Users\24453\Desktop\AI国象" ; npm run check 2>&1 | Select-String "0 errors"

- [ ] **Step 3: AnalysisPanel 控制区紧凑化**
action: 修改 `src/lib/components/AnalysisPanel.svelte` 的 `.control-section` 与子元素 CSS：将 `.control-row` 改为 `display: flex; align-items: center; gap: var(--sp-2)`，深度标签+滑块+MultiPV 选择+按钮全部放一行（移除竖直堆叠的 `flex-direction: column`）；`.button-row` 的三个按钮（暂停/继续、重新分析、停止）改为 icon-only 小按钮（文字改为符号：⏸/▶、↻、⏹），宽度固定 32px；`.progress-section` 的四列网格保留但字号缩小（`.prog-value` 从 12px 改为 11px）；`.pv-list-section` 添加 `flex: 1; min-height: 0` 占满剩余高度。
loop: false
verify: cd "c:\Users\24453\Desktop\AI国象" ; npm run check 2>&1 | Select-String "0 errors"

- [ ] **Step 4: App.svelte 主布局改为三栏 Grid**
action: 修改 `src/App.svelte`：将 `.stage` 的 CSS 从 `display: flex; justify-content: center` 改为 `display: grid; grid-template-columns: 1fr auto 320px; align-items: center; gap: var(--sp-4); padding: 0 var(--sp-4)`；在 `<section class="stage">` 内部，当 `started` 为 true 时，新增第三列子元素 `<div class="analysis-side">` 包裹 `<AnalysisPanel />`（桌面常驻渲染，无条件显示）；原 `.board-area` 作为第二列保留；第一列留空 `<div class="left-spacer"></div>`（Phase A 占位，Phase C 填走法历史）。
loop: false
verify: cd "c:\Users\24453\Desktop\AI国象" ; npm run check 2>&1 | Select-String "0 errors"

- [ ] **Step 5: 移除桌面端分析抽屉，保留窄屏抽屉 fallback**
action: 修改 `src/App.svelte`：保留 `analysisOpen` 状态变量与 `.analysis-drawer` / `.analysis-mask` 的 HTML 与 CSS，但仅在窄屏使用。具体做法：在 `<aside class="drawer analysis-drawer">` 的渲染外层包一个 `{#if narrowScreen}` 条件块（新增 `let narrowScreen = $state(window.innerWidth < 900)`，并在 `onMount` 中添加 `window.addEventListener('resize', ...)` 更新 `narrowScreen`）；桌面端（`!narrowScreen`）渲染常驻的 `<div class="analysis-side"><AnalysisPanel /></div>`；窄屏端渲染原有抽屉与遮罩。同时为 `.analysis-side` 添加 CSS：`width: 320px; height: 100%; overflow: hidden; border-left: 1px solid var(--line)`。
loop: false
verify: cd "c:\Users\24453\Desktop\AI国象" ; npm run check 2>&1 | Select-String "0 errors"

- [ ] **Step 6: "鳕鱼"按钮语义改为引擎开关**
action: 修改 `src/App.svelte` 中 `toggleStockfish` 函数与按钮文案：`toggleStockfish` 改为：若 `$engineStatus === "unloaded"` 则 `await loadEngine(); await analyzePosition($gameState.fen);`（窄屏额外设置 `analysisOpen = true`）；若已加载则 `destroyEngine()`（释放 Worker）并 `stopAnalysis()`。按钮文案从 `"鳕鱼"` 改为根据状态：`$engineStatus === "unloaded"` 时显示 "启用引擎"，否则显示 "关闭引擎"。移除原 `analysisOpen = true` 这一行（窄屏场景改到上面 if 分支内）。删除原 `sf-eval` 评估值按钮（点击打开抽屉），评估值改由常驻面板显示。
loop: false
verify: cd "c:\Users\24453\Desktop\AI国象" ; npm run check 2>&1 | Select-String "0 errors"

- [ ] **Step 7: 走棋后自动重新分析当前局面**
action: 检查 `src/App.svelte` 中 `$gameState` 的订阅逻辑：若已有 `gameState.subscribe` 在 fen 变化时调用 `analyzePosition`，确认逻辑正确；若没有，在 `onMount` 中添加一个 derived effect：当 `$gameState.fen` 变化且 `$engineStatus` 为 ready/searching 且 `$isAnalyzing` 为 true 时，自动调用 `analyzePosition($gameState.fen)` 重新分析新局面。使用 Svelte 5 的 `$effect` 或 `gameState.subscribe` 实现，避免重复触发（用 lastAnalyzedFen 去重，store 内已有该字段）。
loop: false
verify: cd "c:\Users\24453\Desktop\AI国象" ; npm run check 2>&1 | Select-String "0 errors"

- [ ] **Step 8: 类型检查与编译验证**
action: 运行前端类型检查 `npm run check` 与后端编译 `cargo check`，确认 0 errors / 0 warnings。若 svelte-check 报错，根据错误信息修复（常见：未使用的 import、类型不匹配、prop 类型缺失）。
loop: until npm run check 与 cargo check 均通过且无报错
max_iterations: 5
verify: cd "c:\Users\24453\Desktop\AI国象" ; npm run check 2>&1 | Select-String "0 errors and 0 warnings"

- [ ] **Step 9: 启动应用验证布局**
action: 启动 Tauri 开发模式 `npm run tauri dev`（在后台运行），等待应用窗口启动。在应用中点击"设置"→选择"人 vs 鳕鱼"→"开始对弈"，验证：1) 右侧分析面板常驻可见（桌面端，无需点击按钮）；2) 点击"启用引擎"按钮，引擎加载后评估柱与 PV 列表显示；3) 走一步棋，评估值与 PV 实时更新；4) 窗口缩窄至 <900px，分析面板退化为抽屉模式。
loop: false
verify:
  type: human-review
  check: 右侧分析面板常驻可见，评估柱宽度增加且数值清晰，走棋后实时更新，窄屏退化为抽屉

- [ ] **Step 10: 询问下一个 PLAN 方向**
action: Phase A 完成后，向用户汇报成果并询问下一个 PLAN 方向：Phase B（实时走法提示：悬停棋子显示候选走法+评估）、Phase C（完整复盘：走法标色+accuracy%+评估曲线图）、或其他方向。使用 AskUserQuestion 工具提供选项。
loop: false
verify: 已向用户提问并等待回答
gate: human
