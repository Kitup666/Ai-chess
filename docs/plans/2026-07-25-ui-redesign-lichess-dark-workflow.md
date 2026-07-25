---
intent: 将 UI 从暖纸墨色三栏布局重塑为 Lichess 深色专业风全屏聚焦可折叠布局，统一视觉、布局、交互与信息层次
success_criteria:
  - 启动后直接显示棋盘（无 Welcome 页），无对局时显示"开始对弈"浮层
  - 深色主题统一应用，Lichess 经典棕色棋盘 + #629924 绿色强调
  - 右侧合并面板默认显示（走法历史/分析/设置三标签），可一键收起为纯棋盘
  - 顶部工具栏一键可达所有核心操作（开始/继续/重开/悔棋/引擎/面板收起）
  - 走法历史在面板中实时更新
risk_level: medium
auto_approve: true
---

## Steps

- [ ] **Step 1: CSS 变量层 — 全局色板替换为 Lichess 深色主题**
action: 编辑 `c:\Users\24453\Desktop\AI国象\src\app.css`（或全局样式入口），将 CSS 变量替换为 Lichess 深色主题值：`--bg: #161512`、`--surface: #262421`、`--surface-2: #302e2b`、`--ink: #e8e6e3`、`--ink-muted: #9a9893`、`--ink-faint: #62625a`、`--line: #3d3a37`、`--accent: #629924`、`--accent-soft: rgba(98,153,36,0.15)`、`--danger: #b8442e`、`--highlight: rgba(98,153,36,0.4)`、`--board-light: #f0d9b5`、`--board-dark: #b58863`。保留字体变量 `--font-display`/`--font-sans` 与间距/圆角变量 `--sp-*`/`--r-*` 不变。
loop: false
verify:
  type: shell
  command: npm run build
gate: auto

- [ ] **Step 2: Board.svelte — 棋盘配色与高亮适配深色主题**
action: 编辑 `c:\Users\24453\Desktop\AI国象\src\lib\components\Board.svelte` 的 `<style>` 部分，将棋盘格子配色引用改为 `var(--board-light)` / `var(--board-dark)`，高亮色引用 `var(--highlight)`，选中格/上一步/将军格/合法目标格的背景色适配深色主题与 `--accent` 绿色强调。思考条、举棋标记、举棋箭头配色改为深色主题表面色 + `--accent` 强调。确认棋子 SVG（Cburnett）在深色背景下白棋描边清晰。
loop: false
verify:
  type: shell
  command: npm run build
gate: auto

- [ ] **Step 3: 新建 MoveHistory.svelte — 走法历史组件**
action: 创建 `c:\Users\24453\Desktop\AI国象\src\lib\components\MoveHistory.svelte`。从 `$gameState.move_history`（UCI 字符串数组，如 ["e2e4","e7e5"]）读取，用 chess.js（已安装依赖）从初始 FEN `rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1` 逐步重放转换为 SAN。布局：两列（白方/黑方），左侧回合编号，右侧 SAN 走法。当前走法（最后一手）用 `var(--highlight)` 高亮。容器 `overflow-y: auto`。初始 FEN 应从 gameState 推导或使用标准开局 FEN。
loop: false
verify:
  type: artifact
  path: c:\Users\24453\Desktop\AI国象\src\lib\components\MoveHistory.svelte
  assert:
    kind: exists
gate: auto

- [ ] **Step 4: App.svelte — 移除 Welcome 页，启动直入棋盘 + 浮层按钮**
action: 编辑 `c:\Users\24453\Desktop\AI国象\src\App.svelte`。移除 `.welcome` 欢迎页结构（title/sub/deco）与对应 CSS。启动后无论 `started` 真假都渲染 `.board-area`（棋盘 + 评估柱）。当 `!started` 时，在棋盘上叠加绝对定位居中的"开始对弈"浮层按钮（调用 `handleStart`）。当 `isResumedGame && currentIsAuto` 时，叠加"继续对局"浮层按钮（调用 `handleResume`）。移除底部 `actions-cell` 中之前加的"开始对弈/继续对局"按钮（改由浮层 + 顶部工具栏承载）。
loop: false
verify:
  type: shell
  command: npm run build
gate: auto

- [ ] **Step 5: App.svelte — 顶部工具栏**
action: 编辑 `c:\Users\24453\Desktop\AI国象\src\App.svelte`。在 `<main>` 顶部新增 `.toolbar` 工具栏，包含按钮：开始/继续（复用 handleStart/handleResume 逻辑，根据 started 与 isResumedGame 切换文案）、重开（handleReset）、悔棋（handleUndo，disabled={!canUndo}）、引擎切换（toggleStockfish）、面板收起/展开（切换 panelCollapsed）。工具栏左侧显示品牌标识"♟ AI Chess · DeepSeek"。移除底部状态栏中原有重复的"重开/悔棋/重新请求/引擎"按钮（改由工具栏承载），底部状态栏只保留状态文案 + 用量 + 成本统计 + "风的加护"开关。
loop: false
verify:
  type: shell
  command: npm run build
gate: auto

- [ ] **Step 6: App.svelte — 右侧合并面板 + 标签页切换 + 收起功能**
action: 编辑 `c:\Users\24453\Desktop\AI国象\src\App.svelte`。新增状态 `activeTab: "history" | "analysis" | "settings"`（默认 "history"）与 `panelCollapsed: boolean`（默认 false）。右侧 `.side-panel` 常驻渲染：顶部标签页切换栏（走法历史/引擎分析/设置 三个 tab），下方渲染对应组件（MoveHistory/AnalysisPanel/Settings）。当 `panelCollapsed` 为 true 时隐藏 `.side-panel`，棋盘纯全屏；工具栏"面板收起"按钮切换为"展开面板"。移除原有桌面三栏布局的 `.left-spacer` 与 `.analysis-side` 结构，改为 `.board-area` + `.side-panel` 两栏。移除 `.drawer` 底部设置抽屉结构与 CSS（`drawerOpen` 状态仅用于窄屏退化）。
loop: false
verify:
  type: shell
  command: npm run build
gate: auto

- [ ] **Step 7: Settings.svelte — 适配面板标签**
action: 编辑 `c:\Users\24453\Desktop\AI国象\src\lib\components\Settings.svelte`。移除抽屉容器相关样式（作为右侧面板的"设置"标签页内容渲染，不再需要独立容器背景/圆角）。保留所有设置项：API Key / 模型 / 思考模式 / 伪思考 / 思考语言 / 思考强度 / 最少思考token / 自洽采样 / 白方主体 / 黑方主体 / 音效 / 鳕鱼难度 / 执方。保留"开始对弈 / 应用设置 / 退出对局"按钮逻辑。"继续对局"按钮逻辑保留但不再在设置标签内显示（已由浮层 + 工具栏承载）。配色适配深色主题（输入框/按钮/滑块用 `--surface`/`--ink`/`--accent`）。
loop: false
verify:
  type: shell
  command: npm run build
gate: auto

- [ ] **Step 8: AnalysisPanel.svelte — 适配面板标签**
action: 编辑 `c:\Users\24453\Desktop\AI国象\src\lib\components\AnalysisPanel.svelte`。移除独立标题栏（"引擎分析"标题 + 关闭按钮），作为右侧面板的"分析"标签页内容渲染。引擎未加载时显示居中"启用引擎"按钮（调用 loadEngine）。加载后显示深度/MultiPV 控制 + PV 列表，PV 列表 `overflow-y: auto` 占满剩余高度。配色适配深色主题（`--surface`/`--ink`/`--accent`）。窄屏抽屉模式保留右上角关闭按钮。
loop: false
verify:
  type: shell
  command: npm run build
gate: auto

- [ ] **Step 9: EvalBar.svelte — 配色适配深色主题**
action: 编辑 `c:\Users\24453\Desktop\AI国象\src\lib\components\EvalBar.svelte`。评估柱背景改为 `var(--surface)`，白方/黑方优势色块保持白/黑但边框改为 `var(--line)`。数值标签字号 ≥11px，颜色 `var(--ink)`。评估柱为空时显示"—"。
loop: false
verify:
  type: shell
  command: npm run build
gate: auto

- [ ] **Step 10: 走法历史实时更新 + 自动滚动**
action: 编辑 `c:\Users\24453\Desktop\AI国象\src\lib\components\MoveHistory.svelte`。用 `$effect` 监听 `$gameState.move_history` 与 `$gameState.ply`，走法变化时自动滚动到当前走法（用 `bind:this` 获取当前走法 DOM 元素，调用 scrollIntoView({behavior:"smooth", block:"nearest"})）。检测用户手动滚动事件（scroll），用户上滚时暂停自动滚动，回到底部时恢复（与 reasoning 容器同策略，用 `userScrolledUp` 标记）。
loop: false
verify:
  type: shell
  command: npm run build
gate: auto

- [ ] **Step 11: 窄屏退化（<900px 面板改抽屉）**
action: 编辑 `c:\Users\24453\Desktop\AI国象\src\App.svelte` 的 `<style>` 部分。添加 media query `@media (max-width: 900px)`：隐藏 `.side-panel` 常驻显示，改为抽屉模式（从右侧滑入），用 `panelCollapsed` 控制（窄屏下 panelCollapsed=true 时隐藏，false 时滑入）。窄屏下工具栏"面板收起"按钮在抽屉打开时显示"关闭面板"。确保 `narrowScreen` 状态检测（window.innerWidth < 900）与 resize 监听保留。
loop: false
verify:
  type: shell
  command: npm run build
gate: auto

- [ ] **Step 12: 构建与类型检查验证**
action: 运行 `npm run build` 与 `npm run check`，修复所有 error 与 warning。确认 svelte-check 输出 "0 errors, 0 warnings"。
loop: until npm run check 输出 0 errors 0 warnings
max_iterations: 5
verify:
  type: shell
  command: npm run check
gate: auto

- [ ] **Step 13: 人工视觉与交互确认**
action: 启动 `npm run tauri dev`，人工确认以下要点：1) 启动后直接显示棋盘（无 Welcome 页），无对局时显示"开始对弈"浮层；2) 深色主题统一，无残留暖纸墨色，棋盘为 Lichess 棕色；3) 点"开始对弈"进入对局，右侧面板默认显示走法历史标签；4) 切换标签页到引擎分析/设置，内容正确；5) 点工具栏"面板收起"按钮，右侧面板隐藏，棋盘纯全屏，按钮变"展开面板"；6) 走一步棋，走法历史实时更新，评估柱/分析面板同步刷新；7) 切换主体组合（人vs鳕鱼/鳕鱼vs鳕鱼等）布局不破坏；8) 恢复对局后显示"继续对局"浮层，点击后 AI 驱动；9) 窗口缩窄至 <900px，面板退化为抽屉模式。
loop: false
verify:
  type: human-review
  check: 用户确认上述 9 项视觉与交互要点全部通过
gate: human

- [ ] **Step 14: 设定下一个 PLAN 并询问用户**
action: 本次 UI 重塑完成后，向用户呈现下一个 PLAN 候选方向并询问选择：1) 走法评价图标（?/??/!?/!，基于引擎评估差值）；2) 评估曲线图（走法历史下方显示评估变化曲线）；3) PGN 导入导出；4) 计时器/倒计时；5) 开局/残局库；6) 生产版本打包（Windows .exe/.msi）；7) 其他用户指定方向。必须询问用户后再继续，不得自行停止或跳过。
loop: false
verify:
  type: human-review
  check: 用户已选择下一个 PLAN 方向
gate: human
