---
design_type: phase
created_at: 2026-07-24
---

# Phase 3 — 对弈体验增强

## Intent Contract

```
intent: 为棋子走棋添加 FLIP 滑动动画（抬起+落子微弹）+ CC0 走子音效 + 将军/将杀视觉特效，构建有节奏的对弈反馈
constraints:
  - 不破坏现有走棋/悔棋/持久化/流式思考流程
  - 不改变点击选子+点击目标的基础交互
  - 动画基于 Phase 2 的 SVG <img> 渲染（支持 CSS transform）
  - 音效文件本地加载，不联网
success_criteria:
  - 走棋时棋子从源格平滑滑动到目标格，起手微抬，落子微弹
  - 走子/吃子/将军/将杀各有正确音效触发
  - 将军时国王格强化脉冲，将杀时全屏收束动效
  - cargo check 与 npm run check 均通过
risk_level: medium
```

## Verification Contract

```
verify_steps:
  - cargo check --manifest-path src-tauri/Cargo.toml 通过
  - npm run check 0 errors 0 warnings
  - 走棋：棋子平滑滑动，起手抬起，落子微弹
  - 吃子：被吃方淡出，走子方滑入
  - 将军：国王格红色脉冲强化 + 提示音
  - 将杀：全屏收束动效 + 低沉音效
  - 悔棋/重开：动画不卡顿，音效不误触发
```

## Governance Contract

```
approval_gates:
  - FLIP 动画实现后需人工验证手感
  - 音效触发时机需人工验证
  - 将军/将杀特效需人工验证视觉强度
rollback:
  - 动画为 CSS 层，可单独回退
  - 音效为增量，移除 audio 调用即可
ownership: 用户（手感验证）+ 开发者（实现）
```

## Scope

| In | Out |
|---|---|
| FLIP 滑动动画（抬起+落子微弹） | 棋子拖拽走棋 |
| CC0 音效（走子/吃子/将军/将杀） | 背景音乐 |
| 将军时国王格强化脉冲 | 多套音效主题 |
| 将杀时全屏收束动效 | ELO 评分 |
| 动画期间禁用点击（防误触） | 计时器 |

## Decisions

| # | 决策 | 选择 | 理由 |
|---|---|---|---|
| 1 | 音效来源 | CC0 音效文件 | 用户选定；音质优于合成 |
| 2 | 动画实现 | FLIP + 抬起落子 | 用户选定；质感最佳 |
| 3 | 动画技术 | CSS transform + transition | SVG <img> 支持 transform，FLIP 用 getBoundingClientRect 计算位移 |
| 4 | 音效格式 | .ogg（体积小，Tauri webview 支持） | 比 mp3 体积小，现代浏览器原生支持 |
| 5 | 音效触发 | player_move / ai_move 成功后 | 与走棋状态同步 |
| 6 | 动画期间交互 | 禁用点击（300ms 动画窗口） | 防止快速连点导致状态混乱 |

## Surface

**前端**：
- `src/lib/assets/sounds/` — 新建目录，放 4 个 ogg 音效（move/capture/check/mate）
- `src/lib/components/Board.svelte` — FLIP 动画逻辑 + 音效播放 + 将军/将杀特效
- `src/lib/stores/game.ts` — 可能新增 `lastMoveAnim` 状态供动画消费

**后端**：无改动（走棋命令已返回 MoveResult，前端消费即可）

## Risks & Open Questions

| # | 风险 | 缓解 |
|---|---|---|
| 1 | FLIP 动画在棋盘翻转（玩家执黑）时坐标计算需适配 | 用 getBoundingClientRect 绝对坐标，不依赖 grid 位置 |
| 2 | 音效文件需确保 CC0 许可 | 执行阶段从 freesound.org 筛选 CC0 资源 |
| 3 | 动画期间用户快速点击可能导致状态混乱 | 动画窗口内禁用点击（isAnimating 标志） |
| 4 | AI 走棋后立即触发动画，可能与流式思考浮层切换时机冲突 | 思考浮层在 aiThinking=false 后消失，动画在 updateGameState 后触发，时序正确 |
