# 流式输出 + 布局重构 + 思考浮层 计划

## 概述

将 DeepSeek 国际象棋对弈程序重构为：默认流式输出、移除历史步数、设置移至底部可展开抽屉、棋盘中央大字叠加显示 AI 思考内容（带开关）。

## 当前状态分析

**布局**（`src/App.svelte`）：CSS Grid 双栏 `300px 1fr`，左侧面板放 Settings/GameStatus/MoveHistory，右侧棋盘。用户反馈"左侧太丑"。

**流式**（`src-tauri/src/deepseek.rs:81`）：`stream: false` 写死，一次性 JSON 解析，思考内容（`reasoning_content`）仅作兜底，不推送给前端。

**历史步数**（`history_n`）：散布在 6 个文件中（game_state.rs / commands.rs / prompt.rs / types.ts / settings.ts / Settings.svelte）。

**思考展示**：前端无任何思考内容存储，GameStatus 仅显示静态"DeepSeek 思考中..."文案。

**Tauri 版本**：v2.11.3，支持 `AppHandle::emit()` 推送事件，`core:default` 已含事件权限。

## 改动计划

### 步骤 1：后端移除 history_n

**`src-tauri/src/game_state.rs`**
- 删除 `Settings.history_n` 字段（行 20）及默认值（行 30）
- 删除 `StartGameArgs.history_n` 字段（行 84）

**`src-tauri/src/commands.rs`**
- `start_game`：删除 `if let Some(n) = args.history_n` 块（行 24-25）
- `ai_move`：`build_messages(game, settings.history_n, &ai_side)` 改为 `build_messages(game, &ai_side)`（行 80）

**`src-tauri/src/prompt.rs`**
- `user_message(game, history_n, ai_side)` → `user_message(game, ai_side)`
- `build_messages(game, history_n, ai_side)` → `build_messages(game, ai_side)`
- 删除 `let history = game.recent_moves(history_n);` 及 `History(last ...)` 行
- 提示词中移除历史上下文（FEN 已含完整状态）

**`src/lib/types.ts`**
- 删除 `StartGameArgs.history_n`（行 37）

**`src/lib/stores/settings.ts`**
- 删除 `historyN` 字段（行 10）及默认值（行 19）

**`src/lib/components/Settings.svelte`**
- 删除 `historyN` 派生（行 11）、传参（行 37）、整个"历史步数 N" section（行 130-144）及相关样式

### 步骤 2：后端流式输出 + 思考推送

**`src-tauri/src/deepseek.rs`**
- `ChatRequest.stream` 改为构造时传入 `true`
- 新增 `StreamChunk` 反序列化结构：
  ```rust
  #[derive(Deserialize)]
  struct StreamChunk { choices: Vec<StreamChoice> }
  #[derive(Deserialize)]
  struct StreamChoice { delta: StreamDelta }
  #[derive(Deserialize)]
  struct StreamDelta {
      #[serde(default)] content: String,
      #[serde(default)] reasoning_content: String,
  }
  ```
- 新方法 `chat_stream(&self, messages, app: &AppHandle) -> Result<String, String>`：
  - `stream: true` 发起请求
  - `resp.bytes_stream()` 获取字节流
  - 用 `tokio_stream` 逐块读取，按行拆分 SSE（`data: {...}\n\n`）
  - 解析每个 `data: {...}` JSON 为 `StreamChunk`
  - 增量 `delta.reasoning_content` → `app.emit("ai-thinking", &chunk_text)`
  - 增量 `delta.content` → 累积到最终 content
  - 遇到 `data: [DONE]` 结束
  - 返回完整 content（用于走法解析）
- 保留原 `chat()` 方法作为非流式回退（重试时可用）
- `Cargo.toml`：确认 `tokio` 带 `stream` feature，新增 `futures-util`（用于 `StreamExt`）

**`src-tauri/src/commands.rs`**
- `ai_move` 签名增加 `app: tauri::AppHandle` 参数（Tauri 自动注入）
- 首次请求调用 `client.chat_stream(messages.clone(), &app)` 推送思考
- 重试请求仍用 `client.chat_stream`（继续推送，前端覆盖上一次思考）
- 解析逻辑不变（`parse_and_validate` 解析完整 content）

**`src-tauri/src/lib.rs`**
- 无需改动（AppHandle 由 Tauri 自动注入命令参数）

### 步骤 3：前端监听思考事件 + Store

**`src/lib/stores/game.ts`**
- 新增 `aiReasoning: writable<string>("")` — AI 当前思考内容（流式增量）
- 新增 `showThinking: writable<boolean>(false)` — 是否显示思考浮层（开关）

**`src/lib/api.ts`**
- 新增 `listenAiThinking(callback): Promise<UnlistenFn>` — 封装 `listen("ai-thinking", ...)`

**`src/App.svelte`**
- `onMount` 中调用 `listenAiThinking`，回调里 `aiReasoning.update(s => s + chunk)`
- `aiMove` 调用前清空 `aiReasoning`
- `aiMove` 完成后保留 `aiReasoning`（用户可查看）或清空（按开关关闭时清）

### 步骤 4：布局重构 — 去左栏，底部抽屉

**`src/App.svelte`**（核心重写）
- 布局改为单栏 flex column：
  ```
  .app { display: flex; flex-direction: column; height: 100vh; }
  .stage { flex: 1; display: flex; align-items: center; justify-content: center; position: relative; }
  .bottom-bar { 常驻底部状态栏 }
  .settings-drawer { 向上展开的抽屉 }
  ```
- **`.stage`**（flex:1）：棋盘居中大显示 + 思考浮层叠加
- **`.bottom-bar`**（常驻，紧凑）：
  - 左：状态信息（轮次/回合数/将军提示）
  - 右：思考显示开关 + 「设置」按钮
- **`.settings-drawer`**（向上展开）：
  - 默认 `transform: translateY(100%)` 隐藏
  - 点「设置」按钮 → `translateY(0)` 展开
  - 包含所有设置项（API Key / 模型 / 思考模式 / 执方 / 显示思考开关）
  - 内含「开始对弈 / 重新开始」按钮
- 删除原左侧 `<aside class="sidebar">` 及 `.brand`、`.panel-body`、`.actions`
- 顶部可选保留极简标题（居中小字）

**`src/lib/components/Settings.svelte`**
- 改造为抽屉内容组件，移除最外层 `.settings` 的 rise 动画（抽屉自带动画）
- 保留所有设置项（API Key / 模型 / 思考模式 / 执方）
- 「开始对弈」按钮改为根据 `started` 状态切换「开始对弈」/「重新开始」

**`src/lib/components/GameStatus.svelte`**
- 精简为底部状态栏的内容片段（不再独立卡片）
- 显示：轮次圆点 + 文字 + 回合数

**`src/lib/components/MoveHistory.svelte`**
- 移到设置抽屉内，或底部状态栏可展开查看（可选）
- 简化：默认隐藏，点「历史」按钮在抽屉内显示

### 步骤 5：棋盘中央思考浮层

**`src/lib/components/Board.svelte`**
- 棋盘容器 `.board-wrap` 设为 `position: relative`
- 新增 `.thinking-overlay`（绝对定位覆盖棋盘）：
  - 条件渲染：`$showThinking && $aiThinking && $aiReasoning`
  - 半透明深色背景（`rgba(27,26,23,0.85)`）
  - 中央大字显示 `$aiReasoning`（流式逐字）
  - 字体：`var(--font-display)`，大小 `clamp(16px, 2.5vw, 22px)`
  - 文字颜色：`var(--board-light)`，行高 1.6
  - `overflow-y: auto`（思考可能很长）
  - 顶部小标签「DeepSeek 正在思考」
  - 入场动画：淡入 + 轻微上移
- 流式滚动：思考内容增长时自动滚到底部（参考用户偏好：流式输出强制滚底，用户手动上滚则停止）

### 步骤 6：底部状态栏 + 开关组件

**`src/App.svelte`** 底部栏实现
- 状态栏高度约 56px，背景 `var(--bg-soft)`，顶部 1px 边框
- 左侧：轮次指示（白/黑圆点 + 文字）+ 回合数
- 右侧：
  - 「思考」开关（方形拨动，复用 Settings.svelte 的 `.toggle` 样式，绑定 `showThinking`）
  - 「设置」按钮（点击切换抽屉展开状态）
- 抽屉展开时，「设置」按钮高亮

### 步骤 7：编译验证 + 联调

- `cargo check --manifest-path src-tauri/Cargo.toml`
- `npm run check`
- 启动 dev 实际对弈测试：
  - 验证流式思考实时显示在棋盘中央
  - 验证开关可隐藏/显示思考
  - 验证底部抽屉展开/收起
  - 验证设置修改后能重新开局
  - 验证走法解析正常（`<move>` 标签）

### 步骤 8：设定下一个 PLAN 和 SPEC（询问用户）

完成本次改造后，询问用户下一个 PLAN/SPEC 方向。

## 假设与决策

1. **流式推送用 Tauri 事件**：`ai-thinking` 事件携带增量文本，前端增量拼接。理由：Tauri v2 原生支持，无需 WebSocket。
2. **思考浮层覆盖棋盘**：用户选择"棋盘中央大字叠加"，故浮层覆盖整个棋盘区域（不遮挡棋子用半透明背景）。
3. **历史步数完全移除**：FEN 已含完整局面状态，历史步数对 LLM 决策帮助有限且费 token。
4. **设置抽屉默认折叠**：游戏开始后常驻底部状态栏，需要改设置时点「设置」展开抽屉。
5. **MoveHistory 移入抽屉**：保持底部简洁，走法历史在抽屉内查看。
6. **流式时重试仍推送**：重试时继续 emit 思考事件，前端清空后重新拼接（避免残留上一次思考）。
7. **`reasoning_content` 为空时**（非思考模式）：不 emit 事件，浮层不显示，仅正常走棋。

## 验证步骤

1. `cargo check` 通过，无 warning
2. `npm run check` 通过，无 error
3. 启动应用，未开局时显示底部状态栏 + 棋盘欢迎页
4. 点「设置」展开抽屉，填入 API Key，选择模型/执方，点「开始对弈」
5. 棋盘显示，AI 走棋时棋盘中央出现大字思考浮层（流式逐字）
6. 关闭「思考」开关，AI 走棋时浮层不显示
7. 走棋正常，无非法走法（兜底机制生效）
8. 底部状态栏正确显示轮次和回合数
9. 对弈过程中点「设置」可展开抽屉修改设置并重新开始
