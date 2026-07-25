# 修复右侧合并面板被遮挡问题

## 摘要

UI 重塑后右侧合并面板（走法历史/引擎分析/设置）出现遮挡现象。经探索发现两个根本原因：

1. **Grid 列数与子元素数量不匹配**：`.stage` 定义了 3 列 `1fr auto 340px`，但只有 2 个子元素（`.board-area` 和 `.side-panel`）。导致 board-area 被放到第 1 列 `1fr`（弹性宽度，占满左侧），side-panel 被放到第 2 列 `auto`（无显式宽度会塌缩或错位），第 3 列 `340px` 空着但占空间。side-panel 在 `auto` 列里宽度不确定，可能被挤到错误位置，再被 `.stage` 的 `overflow: hidden` 裁剪，视觉上像"被遮住"。

2. **Settings 组件作为面板内容时无滚动处理**：`.settings` 样式是 `display: flex; flex-direction: column; gap: var(--sp-5); max-width: 560px; margin: 0 auto;`，没有 `height` 和 `overflow-y`。当它作为 `.panel-content`（`flex: 1; overflow: hidden`）的子元素时，内容超出视口会被 `overflow: hidden` 裁剪而无法滚动，设置项被遮挡。

## 当前状态分析

### 布局结构（[App.svelte:441-486](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L441-L486)）

```
.stage (grid: 1fr auto 340px, align-items: center, overflow: hidden)
├── .board-area (第1列 1fr，弹性宽度)
│   ├── EvalBar
│   └── Board (.board-frame width:100% 撑满 1fr 列)
└── .side-panel (第2列 auto，无显式宽度 → 塌缩)
    ├── .panel-tabs
    └── .panel-content (flex:1, overflow:hidden)
        └── Settings (无 overflow-y:auto → 内容被裁剪)
```

**问题 1 详细分析**：
- grid 定义 3 列，template 只有 2 个子元素
- board-area 进第 1 列 `1fr`，board-frame `width: 100%` 撑满这列，导致 board-area 很宽
- side-panel 进第 2 列 `auto`，无显式 width，宽度由内容决定（panel-tabs 文字宽度）
- 第 3 列 `340px` 空置但占空间，把 side-panel 挤压
- side-panel 实际宽度远小于 340px，内容溢出被 `overflow: hidden` 裁剪
- 收起态 `.stage:has(.side-panel.collapsed)` 改为 `1fr auto 0` 也是 3 列，同样错位

**问题 2 详细分析**：
- `.panel-content` 是 `flex: 1; min-height: 0; overflow: hidden; display: flex; flex-direction: column;`
- Settings 作为子元素，自身 `display: flex; flex-direction: column; gap: var(--sp-5); max-width: 560px; margin: 0 auto;`
- Settings 无 `height: 100%` 也无 `overflow-y: auto`，内容超出 panel-content 高度时被裁剪
- Settings 内容很长（API Key + 模型 + 思考模式 + 伪思考 + 思考语言 + 思考强度 + 最少token + 自洽采样 + 白方主体 + 黑方主体 + 音效 + 鳕鱼难度 + 执方 + 按钮），必然超出

## 拟议修改

### 修改 1：修复 grid 列数与子元素匹配

**文件**：[src/App.svelte](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L638-L653)

**改什么**：将 `.stage` 的 grid 从 3 列改为 2 列，移除空的 `auto` 中间列。

**为什么**：当前 3 列只有 2 个子元素，导致布局错位。改为 2 列后 board-area 在 `1fr` 列居中，side-panel 在 `340px` 列固定宽度。

**怎么改**：
```css
.stage {
  grid-template-columns: 1fr 340px;  /* 原: 1fr auto 340px */
}
.stage:has(.side-panel.collapsed) {
  grid-template-columns: 1fr 0;  /* 原: 1fr auto 0 */
}
```

board-area 仍需在 `1fr` 列居中（flex + align-items: center 已实现），side-panel 在 `340px` 列固定宽度。窄屏 `.stage.narrow` 保持 `1fr` 单列不变。

### 修改 2：给 side-panel 显式宽度兜底

**文件**：[src/App.svelte](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L661-L675)

**改什么**：给 `.side-panel` 加 `width: 100%`（占满 grid cell 的 340px 列），确保即使 grid 行为异常也能占满列宽。

**为什么**：grid `auto` 列已改为固定 `340px`，但加 `width: 100%` 作为兜底，防止 side-panel 内容塌缩。

**怎么改**：
```css
.side-panel {
  width: 100%;  /* 新增：占满 grid cell */
  height: 100%;
  /* 其余不变 */
}
```

### 修改 3：Settings 加滚动支持

**文件**：[src/lib/components/Settings.svelte](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/Settings.svelte#L442-L448)

**改什么**：给 `.settings` 加 `height: 100%` 和 `overflow-y: auto`，让内容超出时滚动而非被裁剪。

**为什么**：Settings 作为 panel-content 子元素，panel-content 是 `overflow: hidden`，Settings 内容超出会被裁剪。加滚动后用户可上下滑动查看所有设置项。

**怎么改**：
```css
.settings {
  display: flex;
  flex-direction: column;
  gap: var(--sp-5);
  max-width: 560px;
  margin: 0 auto;
  height: 100%;        /* 新增：占满 panel-content */
  overflow-y: auto;    /* 新增：内容超出滚动 */
  padding: var(--sp-4); /* 新增：内边距，避免内容贴边 */
}
```

### 修改 4：确认 AnalysisPanel 与 MoveHistory 的滚动

**文件**：[src/lib/components/AnalysisPanel.svelte](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/AnalysisPanel.svelte)、[src/lib/components/MoveHistory.svelte](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/MoveHistory.svelte)

**改什么**：确认两个组件根元素有 `height: 100%` + `overflow-y: auto`（或内部有滚动容器）。

**为什么**：作为 panel-content 子元素，需自行处理滚动。

**探索结果**：
- MoveHistory 已有 `height: 100%` + `overflow-y: auto`（[MoveHistory.svelte:88-89](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/MoveHistory.svelte#L88-L89)），无需改动
- AnalysisPanel 根元素 `height: 100%` + `overflow: hidden`（[AnalysisPanel.svelte:291-293](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/AnalysisPanel.svelte#L291-L293)），内部 PV 列表已有 `overflow-y: auto`（第 511-512 行），无需改动

**怎么改**：无需改动，仅验证。

## 假设与决策

| # | 假设/决策 | 说明 |
|---|----------|------|
| 1 | 假设遮挡主因是 grid 3列2元素错位 | 探索发现 grid 定义 3 列但 template 只有 2 个子元素，是明确 bug |
| 2 | 假设 Settings 无滚动是次要遮挡原因 | Settings 内容长但无 overflow-y，作为面板内容会被裁剪 |
| 3 | 决策 grid 改 2 列而非补第 3 个子元素 | 2 列更简洁，符合"棋盘 + 右侧面板"的双栏设计意图 |
| 4 | 决策 side-panel 加 width:100% 兜底 | 防御性样式，确保占满 grid cell |
| 5 | 决策 Settings 加 padding | 作为面板标签内容，需内边距避免贴边，与抽屉模式一致（抽屉 body 有 padding: var(--sp-5)） |
| 6 | 不改 stage 的 align-items: center | board-area 垂直居中是期望行为，side-panel 有 height:100% 占满 |

## 验证步骤

1. **构建验证**：`npm run build` 通过，无 error
2. **类型检查**：`npm run check` 输出 0 errors 0 warnings
3. **视觉验证**（启动 `npm run tauri dev`）：
   - 桌面端启动对局后，右侧合并面板完整可见，340px 宽度，不被遮挡
   - 面板标签栏（走法历史/引擎分析/设置）完整显示，不溢出
   - 切换到"设置"标签，所有设置项可见，内容超出时可滚动
   - 切换到"引擎分析"标签，PV 列表可滚动
   - 切换到"走法历史"标签，走法列表可滚动
   - 点"收起面板"，右侧面板隐藏，棋盘居中占满；点"展开面板"恢复
   - 窄屏（<900px）右侧面板不渲染，底部"设置"按钮打开抽屉正常
4. **走棋验证**：走一步棋，走法历史实时更新可见，面板不被遮挡
