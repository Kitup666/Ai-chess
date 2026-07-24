# Phase 2 — UI 视觉升级计划

## Summary

将 DeepSeek 国际象棋对弈程序从「能用」提升到「精致可反复把玩」的视觉层级。棋子从 Unicode 字符升级为经典 SVG 棋子集（Cburnett 风格，公共域），棋盘、顶部标题、底部状态栏、欢迎页、设置抽屉全场景质感统一打磨，保持「棋谱编辑部」暖纸墨色沉绿设计语言。为 Phase 3 动画/音效奠定可 transform 的渲染基础。

## Current State Analysis

**棋子渲染**（[Board.svelte:48-56](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/Board.svelte#L48-L56)）：
- Unicode 字符（`\u265A` 等），CSS 控制颜色
- 白棋 `#fcfaf5` + text-shadow，黑棋 `#1b1a17`
- 问题：Unicode 字符在不同字体下渲染不一致，质感单薄，辨识度依赖系统字体

**棋盘格子**（[Board.svelte:343-348](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/Board.svelte#L343-L348)）：
- 纯色（浅 `#ede4d3` / 深 `#6b7f6e`），无纹理无层次
- 高亮状态（选中/最后一步/合法目标/将军）已有但视觉语言不统一

**设计系统**（[app.css](file:///c:/Users/24453/Desktop/AI国象/src/app.css)）：
- token 完整（色彩/字体/间距/圆角/缓动）
- Fraunces / Geist / Geist Mono 已加载
- 通用组件（input-line / seg-btn / btn-primary / label）已定义

**布局**（[App.svelte](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte)）：
- 单栏：顶部标题 + 中央棋盘 + 底部状态栏 + 抽屉
- welcome 页用大字 "Welcome" + 装饰字符 ♞
- 抽屉向上展开

## Assumptions & Decisions

| # | 决策 | 选择 | 理由 |
|---|---|---|---|
| 1 | 棋子渲染方案 | SVG 棋子集（Cburnett 风格） | 用户选定位图棋子集；Cburnett 是公共域 SVG，经典国际象棋图标，支持 CSS transform 为 Phase 3 动画奠基 |
| 2 | SVG 引入方式 | 本地文件 + Vite import URL | 12 个 SVG 文件放 `src/lib/assets/pieces/`，Vite 原生支持 `import url from '*.svg'`，用 `<img>` 渲染 |
| 3 | 棋子配色 | 保留 SVG 原始配色（白棋浅填充+深描边，黑棋深填充+浅描边） | Cburnett 风格自带描边，与编辑部设计语言中"暖纸墨色"契合；不强制改色避免破坏辨识度 |
| 4 | 棋盘格子质感 | 加细微噪点纹理 + 边缘微阴影 | 纯色过于扁平，加 CSS 生成的微纹理提升纸质质感，不引入图片资源 |
| 5 | 升级范围 | 全场景统一 | 用户选定；棋盘+标题+状态栏+欢迎页+抽屉 |
| 6 | 配色微调 | 棋盘深色格 `#6b7f6e` → `#5a7159`（略深沉绿） | 当前略灰，加深后与 `--accent #2e4a3e` 同色系更协调 |
| 7 | 字体角色 | Fraunces 用于标题/坐标/思考浮层；Geist 用于 UI 文本 | 保持现状，强化 editorial 气质 |

## Proposed Changes

### 1. 棋子 SVG 资源（新建）

**文件**：`src/lib/assets/pieces/{wK,wQ,wR,wB,wN,wP,bK,bQ,bR,bB,bN,bP}.svg`

**what**: 12 个 SVG 棋子文件，采用 Wikimedia Commons Cburnett 风格（公共域）。

**how**: 执行阶段从 `https://commons.wikimedia.org/wiki/Category:SVG_chess_pieces` 下载 Cburnett 系列棋子 SVG，重命名为统一命名（w=white, b=black, K/Q/R/B/N/P=King/Queen/Rook/Bishop/Knight/Pawn）。每个 SVG 约置入 `src/lib/assets/pieces/`。

**why**: Unicode 棋子依赖系统字体渲染，跨平台不一致且质感单薄；SVG 矢量图保证一致渲染、支持 CSS transform 动画（Phase 3 依赖）。

### 2. Board.svelte 棋子渲染重写

**文件**：[src/lib/components/Board.svelte](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/Board.svelte)

**what**: 
- 删除 `PIECE_CHAR` Unicode 映射（第 48-56 行）
- 新增 `import` 12 个 SVG URL：`import wK from "../assets/pieces/wK.svg"` 等
- 新增 `PIECE_IMG` 映射：`Record<PieceType, { white: string; black: string }>`
- 棋子渲染从 `<span class="piece">{PIECE_CHAR[...]}</span>` 改为 `<img class="piece" src={PIECE_IMG[type][color]} alt={...} />`
- 升变选择器（第 280 行）同步改为 `<img>`
- 思考浮层不受影响

**why**: 切换到 SVG 渲染，支持 transform 动画且渲染一致。

**how**: 
```typescript
import wK from "../assets/pieces/wK.svg";
// ... 11 个
const PIECE_IMG: Record<PieceType, { white: string; black: string }> = {
  k: { white: wK, black: bK },
  // ...
};
```
棋子 CSS（第 379-397 行）调整：`.piece` 从 `font-size` 改为 `width/height: min(8vh, 60px)`，删除 `color/text-shadow`，保留 `transition: transform` 和 hover scale。

### 3. Board.svelte 棋盘格子质感

**文件**：[src/lib/components/Board.svelte](file:///c:/Users/24453/Desktop/AI国象/src/lib/components/Board.svelte#L334-L348)

**what**:
- `.light` / `.dark` 背景加 CSS `background-image` 微噪点（`radial-gradient` 或 `repeating-linear-gradient` 生成，无外部图片）
- 棋盘外框加细描边 + 微阴影（`box-shadow: 0 4px 24px rgba(27,26,23,0.08)`）
- 高亮状态统一为「半透明叠加层」语言：
  - 选中：金色 `--highlight` 0.35 叠加
  - 最后一步：金色 0.18 叠加
  - 合法目标空格：中心圆点（保留）
  - 合法目标吃子：边框 ring（保留）
  - 将军：红色脉冲（保留）

**why**: 纯色格子过于扁平，加微纹理提升纸质质感；高亮语言统一减少视觉杂讯。

**how**: 
```css
.light {
  background-color: var(--board-light);
  background-image: radial-gradient(rgba(0,0,0,0.015) 1px, transparent 1px);
  background-size: 3px 3px;
}
.dark {
  background-color: var(--board-dark);
  background-image: radial-gradient(rgba(255,255,255,0.02) 1px, transparent 1px);
  background-size: 3px 3px;
}
.board-wrap {
  /* 新增外框质感 */
  box-shadow: 0 4px 24px rgba(27,26,23,0.08), 0 1px 0 rgba(27,26,23,0.04);
}
```

### 4. app.css 设计 token 微调

**文件**：[src/app.css](file:///c:/Users/24453/Desktop/AI国象/src/app.css#L19-L20)

**what**:
- `--board-dark: #6b7f6e` → `#5a7159`（略深沉绿，与 accent 同色系）
- 新增 `--board-frame: rgba(27,26,23,0.04)` 用于棋盘外框阴影

**why**: 当前棋盘深色格偏灰，与沉绿强调色不协调。

### 5. App.svelte 顶部标题区质感

**文件**：[src/App.svelte:245-271](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L245-L271)

**what**:
- `.top-bar` 去掉 `border-bottom`，改为底部 1px 渐变线（`background: linear-gradient(to right, transparent, var(--line), transparent)`）
- `.brand-mark` ♟ 改为小 SVG 图标或保留但调色（`color: var(--accent)` + `opacity: 0.7`）
- `.brand-title` 字距微调 `letter-spacing: 0.04em`，加 `font-style: italic` 强化 editorial 气质

**why**: 硬边框过于生硬，渐变线更精致；标题斜体呼应 Fraunces 的 editorial 属性。

### 6. App.svelte 欢迎页质感

**文件**：[src/App.svelte:284-313](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L284-L313)

**what**:
- `.welcome-title` "Welcome" 字号 72px → 88px，字重 400 → 300，加 `font-style: italic`
- `.welcome-deco` ♞ 换为更大尺寸（96px → 120px）+ 旋转动画（缓慢 `transform: rotate` 来回）
- `.welcome-sub` 行距加宽 `line-height: 2`

**why**: 欢迎页是第一印象，放大字号+斜体强化 editorial 气质，装饰字符加动画增加生气。

### 7. App.svelte 底部状态栏质感

**文件**：[src/App.svelte:315-408](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L315-L408)

**what**:
- `.bottom-bar` 高度 56px → 60px，`background` 从 `--surface` 改为 `--bg`（与顶部呼应）
- `.status-dot` 加 `box-shadow: 0 0 0 3px rgba(46,74,62,0.08)` 光晕（normal/thinking 状态）
- `.status-meta` 字号 11px → 10px，`letter-spacing: 0.05em` 强化 mono 气质
- `.bar-btn` hover 加 `transform: translateY(-1px)` 微动效
- `.divider` 高度 18px → 20px，颜色 `--line` → `--ink-faint` opacity 0.3

**why**: 状态栏是常驻元素，微调提升精致度；按钮微动效增加反馈感。

### 8. App.svelte 设置抽屉质感

**文件**：[src/App.svelte:461-527](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L461-L527)

**what**:
- `.drawer` 顶部圆角（新增 `border-radius: var(--r-lg) var(--r-lg) 0 0`）
- `.drawer` 阴影加重 `box-shadow: 0 -12px 40px rgba(27,26,23,0.12)`
- `.drawer-header` 加 grab handle 视觉（顶部 36px 宽 4px 高圆角条，`background: var(--line)` 居中）
- `.drawer-title` 字号 16px → 18px

**why**: 抽屉圆角+阴影强化浮层质感；grab handle 是现代移动端/桌面端抽屉的视觉约定。

## Verification

1. **编译验证**：
   - `npm run check` — 0 errors 0 warnings
   - `cargo check --manifest-path src-tauri/Cargo.toml` — 0 errors 0 warnings（本 phase 不动后端，预期无需改动）

2. **视觉验证**（人工，启动 `npm run tauri dev`）：
   - 棋子：12 种棋子 SVG 正确渲染，黑白辨识清晰，大小自适应棋盘
   - 棋盘：格子有微纹理质感，外框有阴影，配色协调
   - 高亮：选中/最后一步/合法目标/将军 状态显示正确且视觉统一
   - 顶部：标题区渐变线 + 斜体字 + 图标配色协调
   - 欢迎页：大字斜体 "Welcome" + 旋转装饰字符
   - 状态栏：光晕圆点 + 按钮微动效 + mono 字号
   - 抽屉：圆角 + 阴影 + grab handle
   - 升变选择器：棋子图标正确显示
   - 思考浮层：不受影响，仍正常显示流式内容

3. **回归验证**：
   - 走棋/悔棋/重开流程不受影响
   - 持久化恢复不受影响（Phase 1 功能）
   - 流式思考浮层显示不受影响

## Out of Scope

- Phase 3 内容：棋子移动动画、走子音效、将军/将杀特效（下一个 phase）
- 多主题切换（深夜模式等）
- 棋子拖拽走棋
- 后端改动
