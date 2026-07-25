---
design_type: phase
created_at: 2026-07-25
---

# Phase A — Lichess 风格布局重构

## Intent Contract

**intent**: 把鳕鱼分析面板从右侧抽屉改为常驻侧栏，评估柱加宽且数值清晰，让用户无需点击即可一眼看到评估值、深度、PV 变着和引擎控制。

**constraints**:
- 不破坏现有三方主体对弈流程（9 种主体组合必须正常工作）
- 不改变后端 API 与持久化结构（纯前端布局重构）
- 保留现有"风的加护"思考显示开关、成本统计、悔棋/重开等功能
- 保持纸质墨水设计语言（warm paper-ink palette），不引入 Lichess 的深色主题
- 窄屏（<900px）保留抽屉模式作为 fallback

**success_criteria**:
- 启动应用进入对局后，右侧分析面板常驻可见，无需点击"鳕鱼"按钮
- 评估柱宽度 ≥24px，数值标签字号 ≥11px，从远处可读
- 引擎未加载时右侧显示精简占位（"点击启用引擎"按钮），加载后自动显示评估/PV
- 桌面端三栏布局：左侧留白/折叠区 + 中央棋盘（评估柱+棋盘）+ 右侧分析面板（固定 320px）
- 走棋后评估值、PV 列表实时刷新，无闪烁

**risk_level**: medium

## Verification Contract

**verify_steps**:
  - run tests: `npm run check`（svelte-check 0 errors / 0 warnings）
  - run tests: `cd src-tauri && cargo check`（Rust 编译通过，本次应无变更）
  - check: 启动应用 `npm run tauri dev`，进入对局后右侧分析面板常驻可见
  - check: 点击"鳕鱼"按钮加载引擎，评估柱与分析面板同步显示评估值
  - check: 走一步棋，评估值与 PV 列表实时更新
  - check: 切换主体组合（人vs鳕鱼/鳕鱼vs鳕鱼等），布局不破坏
  - check: 窗口缩窄至 <900px，分析面板退化为抽屉模式（点击按钮打开）
  - confirm: 不再需要点击"鳕鱼"按钮才能看到分析，启动后右侧即占位可见

## Governance Contract

**approval_gates**:
  - 设计文档审批（本阶段）
  - 写完 workflow 后执行前审批

**rollback**:
  - 布局改动集中在 App.svelte / AnalysisPanel.svelte / EvalBar.svelte 三个文件
  - git revert 这三个文件的改动即可恢复抽屉式布局
  - 不涉及持久化迁移，无需数据回滚

**ownership**: 前端布局重构，由开发者在 TRAE IDE 中执行

## Scope

| 范围内 | 范围外 |
|---|---|
| 分析面板从抽屉改为常驻侧栏 | 走法评价图标（?/??/?!/!）— Phase C |
| 评估柱加宽 + 数值标签放大 | 走法历史列表带评估 — Phase C |
| 桌面端三栏布局重构 | 鼠标悬停候选走法提示 — Phase B |
| 引擎未加载时的精简占位 | 评估曲线图 — Phase C |
| "鳕鱼"按钮语义改为"启用/关闭引擎" | 开局/残局库 — Phase D |
| 窄屏抽屉 fallback | 后端 API 变更 |
| AnalysisPanel 内部紧凑化适配常驻 | 持久化字段变更 |

## Decisions

| # | 决策 | 选择 | 拒绝的替代 |
|---|---|---|---|
| 1 | 布局方案 | 桌面三栏（左留白 + 中棋盘 + 右常驻面板 320px），窄屏退化为抽屉 | 始终抽屉（用户已抱怨）/ 始终三栏（窄屏不可用） |
| 2 | 分析面板触发 | 引擎首次加载后右侧常驻可见；未加载时显示"启用引擎"占位 | 保留按钮切换显隐（与"常驻"目标冲突） |
| 3 | 评估柱宽度 | 24px（从 18px 加宽），数值标签 11px | 维持 18px（用户已抱怨看不清）/ 加宽到 32px（侵占棋盘空间） |
| 4 | "鳕鱼"按钮语义 | 改为"启用引擎 / 关闭引擎"切换（控制引擎 Worker 生命周期），不再控制面板显隐 | 移除按钮（用户失去关闭引擎的能力） |
| 5 | AnalysisPanel 紧凑化 | 移除面板标题栏（"引擎分析 ✕"），保留控制+进度+PV 三段；关闭按钮移到面板顶部右上角小图标 | 保留完整标题栏（常驻时浪费垂直空间） |
| 6 | 左侧留白区 | Phase A 暂留空（仅占位），Phase C 走法历史填入 | 现在就做走法历史（超出 Phase A 范围） |
| 7 | 窄屏断点 | 900px（与 tauri.conf.json minWidth 820px 兼容，留 80px 缓冲） | 768px（与 minWidth 太接近）/ 1024px（桌面端过早退化） |

## Surface

**App.svelte** — 主布局改造：
- `.stage` 从单列居中改为 CSS Grid 三栏：`grid-template-columns: 1fr auto 320px`（左留白 / 中棋盘 / 右面板）
- 移除 `.analysis-drawer` 抽屉相关 CSS 与 `analysisOpen` 状态切换
- `AnalysisPanel` 直接作为 `.stage` 的第三列子元素渲染，常驻可见
- 窄屏 media query `<900px` 退化为现有抽屉模式（保留 `analysisOpen` 状态与抽屉 CSS，仅在窄屏激活）
- "鳕鱼"按钮文案改为 "启用引擎" / "关闭引擎"，调用 `loadEngine()` 或 `destroyEngine()`，不再切换 `analysisOpen`

**AnalysisPanel.svelte** — 适配常驻：
- 移除顶部 `.panel-header`（"引擎分析"标题栏），改为右上角小关闭按钮（仅窄屏抽屉模式可见，桌面常驻模式隐藏）
- 引擎未加载时显示居中"启用引擎"按钮（调用 `loadEngine()` 后自动 `analyzePosition(currentFen)`）
- 控制区紧凑化：深度滑块、MultiPV、按钮一行内排列（移除竖直堆叠）
- PV 列表占满剩余高度，`overflow-y: auto`

**EvalBar.svelte** — 加宽与数值放大：
- `.eval-bar` 宽度从 18px → 24px
- `.eval-label` 字号从 10px → 11px，padding 加大
- 数值标签在评估柱为空时显示 "—"，避免空白

**新增**：无新文件，全部为现有文件改造

## Risks & Open Questions

| # | 风险/问题 | 缓解/方向 |
|---|---|---|
| 1 | 三栏布局在小屏（820px minWidth）下右侧 320px 会挤压棋盘 | 900px 断点退化抽屉；棋盘 `min-size` 保护 |
| 2 | 引擎未加载时常驻面板占位可能显空 | 显示"启用引擎"按钮 + 引擎简介文案，引导用户启用 |
| 3 | 双 AI 对弈（如鳕鱼vs鳕鱼）时分析面板同时显示评估，可能干扰 | 评估值始终基于当前局面，与谁走棋无关；若用户觉得干扰可关闭引擎 |
| 4 | AnalysisPanel 移除标题栏后关闭入口 | 桌面常驻无需关闭；窄屏抽屉模式保留右上角关闭按钮 |
| 5 | 评估柱加宽后棋盘整体右移，可能影响视觉居中 | 棋盘区 `flex-shrink: 0`，评估柱+棋盘作为整体居中 |
