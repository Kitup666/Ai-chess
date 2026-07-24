---
design_type: initiative
created_at: 2026-07-24
topic: chess-experience-overhaul
---

# 对弈体验全面升级

## Problem

当前 DeepSeek 国际象棋对弈程序已完成核心对弈闭环（流式思考 + 底部抽屉布局 + 棋盘中央思考浮层），但存在三类体验短板：

1. **功能缺失** — 对局进度无法持久化，重启应用即丢失；API Key 与设置同样每次重启需重新填写。用户无法跨会话延续对局。
2. **视觉质感不足** — 现有棋盘/棋子为 Unicode 字符 + 纯色格子，缺乏质感与辨识度，与「Editorial Minimalism」设计语言的精致定位存在落差。
3. **对弈反馈单薄** — 棋子瞬间出现/消失无过渡，走子无声效，将军/将杀无强调，沉浸感弱。

## Vision

将程序从「能用」提升为「想反复把玩」的精致单机对弈应用：

- **Phase 1 功能扩展**：后端统一持久化对局进度与设置，重启自动恢复，零配置延续对局。
- **Phase 2 UI 视觉升级**：重塑棋盘视觉语言（棋子、格子、坐标、氛围），与暖纸墨色设计系统深度契合。
- **Phase 3 对弈体验增强**：棋子平滑移动动画 + 走子音效 + 将军/将杀视觉特效，构建有节奏的对弈反馈。

三个 phase 串行推进，视觉先行于动效以避免返工。

## Non-goals

为控制范围，本期明确不做：

- 难度调节（temperature / 系统提示词分档）
- PGN 导入导出
- 计时器 / 倒计时
- 多存档槽（仅单存档自动恢复）
- 在线对战 / 联网功能
- 账号系统 / 云同步
- API Key 加密存储（明文本地存储，桌面单机可接受；如需可后续加 keyring）
- 棋子拖拽走棋（保持点击选择 + 点击目标格的交互模式）

## Stakeholders

- **用户（棋手）**：唯一使用者，关注对局连续性、视觉舒适度、反馈手感。
- **开发者**：维护者，关注架构整洁、phase 间低耦合、可独立验证。

## Architecture

### 持久化基础（Phase 1 奠定，服务全 initiative）

采用**后端 Rust 统一持久化**：所有可持久化状态由后端管理，写入 Tauri `app_data_dir` 下的 JSON 文件。

- **存储位置**：`<app_data_dir>/chess_state.json`（单文件，单存档）。
- **存储内容**：
  - 对局状态：FEN、move_history、player_side、board_history（用于悔棋恢复）、status
  - 设置：api_key、model、thinking、side、started
- **触发时机**：启动时加载；每步走完（player_move / ai_move / undo）后自动保存；reset 时覆盖保存。
- **前端同步**：启动时前端 settings store 从后端拉取初始值；设置变更通过现有命令同步落盘。

选择后端统一而非前端 localStorage 的理由：对局核心状态（ChessGame）本就在 Rust 端，统一存储避免双源真相；API Key 不暴露在前端 webview 的 localStorage。

### Phase 间解耦

- Phase 1 纯数据层 + 命令层改动，不动 UI 组件结构。
- Phase 2 纯前端视觉层（CSS + 棋子渲染），不动数据/命令。
- Phase 3 在 Phase 2 的视觉基础上加动画/音效，依赖 Phase 2 的棋子渲染结构。

## Phase Breakdown

### Phase 1 — 功能扩展：持久化与恢复

- 后端新增持久化模块：save / load / clear。
- `start_game` / `player_move` / `ai_move` / `undo_move` / `reset_game` 命令在状态变更后触发保存。
- 新增 `load_state` 命令，前端启动时调用恢复对局与设置。
- 前端 settings store 启动时从后端同步。
- 退出/重开应用后：未结束的对局自动恢复到上次局面；设置与 API Key 保留。

### Phase 2 — UI 视觉升级

- 棋子视觉：从 Unicode 字符升级为更精致的渲染（SVG 或精心调校的字体 + 描边/阴影）。
- 棋盘格子：优化配色对比、增加细微纹理或边框处理，提升质感。
- 坐标、高亮、选中、合法目标格的视觉语言统一打磨。
- 配色主题保持暖纸墨色沉绿基底，强化层次。
- 不改变交互结构（仍是点击选子 + 点击目标）。

### Phase 3 — 对弈体验增强

- 棋子移动动画：走子时棋子从源格平滑过渡到目标格（CSS transform / FLIP 技术）。
- 走子音效：走子、吃子、将军、将杀各有音效（本地音频资源，非网络）。
- 将军/将杀视觉特效：将军时国王格高亮脉冲强化；将杀时全屏收束动效。
- AI 思考时的视觉节奏（与现有思考浮层协同）。

## Risks & Open Questions

| # | 风险/问题 | 影响 | 缓解 |
|---|---|---|---|
| 1 | board_history 全量持久化可能导致存档膨胀（长对局） | 中 | 限制保存最近 N 个 board 快照，或仅存 FEN + move_history 并重放 |
| 2 | Phase 2 棋子渲染方案（SVG vs 字体）未定，影响 Phase 3 动画实现 | 中 | Phase 2 brainstorming 时确定，优先选支持 transform 动画的方案 |
| 3 | 音效资源来源（自制 vs 免费库）未定 | 低 | Phase 3 brainstorming 时确定，优先 CC0 免费音效 |
| 4 | 持久化文件损坏时的恢复策略 | 中 | 加载失败时回退默认初始状态并提示用户，不阻塞启动 |
| 5 | 三个 phase 串行周期较长 | 低 | 每个 phase 独立可交付、可验证，用户可随时止步 |

## HOTL Contracts

### Intent Contract

```
intent: 将 DeepSeek 国际象棋对弈程序从「能用」升级为「精致可反复把玩」的单机对弈应用，覆盖持久化、视觉、反馈三层。
constraints:
  - 不破坏现有对弈闭环（流式思考、走法合法性、底部抽屉布局）
  - 不引入联网/账号/云同步
  - 不改变点击选子+点击目标的基础交互模式
  - API Key 明文本地存储（桌面单机可接受）
success_criteria:
  - 重启应用后对局进度与设置自动恢复
  - 棋盘视觉质感显著提升且与设计语言一致
  - 走子有平滑动画与音效，将军/将杀有明确视觉强调
risk_level: medium
```

### Verification Contract

```
verify_steps:
  - phase 1: 重启应用后未结束对局自动恢复到上次局面，API Key/设置保留
  - phase 1: cargo check + npm run check 通过，无 error/warning
  - phase 2: 棋子/棋盘视觉重塑后，走棋、选中、将军、将杀状态显示正确
  - phase 3: 走子动画平滑无闪烁，音效在走子/吃子/将军/将杀时正确触发
  - 全程: 现有流式思考浮层、底部抽屉、悔棋功能不被破坏
```

### Governance Contract

```
approval_gates:
  - 每个 phase 的 design doc 批准后方可 writing-plans
  - Phase 1 持久化方案实现后需人工验证恢复流程
  - Phase 2 视觉方案需人工确认质感再进入 Phase 3
  - Phase 3 动画/音效需人工手感验证
rollback:
  - Phase 1: 持久化为增量功能，回滚即恢复纯内存模式（删除 save 调用）
  - Phase 2/3: 视觉/动效为前端层，可单独回退 CSS/组件
ownership: 用户（产品决策）+ 开发者（实现）
```
