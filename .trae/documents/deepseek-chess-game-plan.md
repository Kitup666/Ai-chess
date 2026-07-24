# DeepSeek 国际象棋对弈程序 - 实施计划

## 1. 概述

开发一个基于 **Tauri 2.x + Rust + Svelte 5** 的桌面应用，接入 DeepSeek 官方 API 实现人机国际象棋对弈。程序需要：
- 向 DeepSeek 提供完整的当前棋子状态
- 向 DeepSeek 提供前 N 步的走法历史
- 向 DeepSeek 注入完整的国际象棋规则
- 强制走法合法性校验（DeepSeek 走非法步时拒绝并重试）

---

## 2. 当前状态分析

- 工作目录 `c:\Users\24453\Desktop\AI国象` 为空，全新项目
- 需要从零搭建 Tauri + Svelte 项目骨架
- 用户环境：Windows，Unity 安装目录 `D:\Unity Hub\Unity2022\2022.3.62f3`（本项目不涉及 Unity）

---

## 3. 技术栈

| 层 | 技术 | 版本 | 用途 |
|---|---|---|---|
| 桌面框架 | Tauri | 2.x | 跨平台桌面应用壳，Rust 后端 + WebView 前端 |
| 后端语言 | Rust | stable (1.75+) | 棋局逻辑、DeepSeek API、走法验证 |
| 前端框架 | Svelte | 5.x (runes) | 响应式 UI |
| 前端构建 | Vite | 5.x | 开发服务器与打包 |
| 象棋库 | `chess` crate | 3.x | 走法生成、合法性校验、FEN/PGN |
| HTTP 客户端 | `reqwest` | 0.12 | 调用 DeepSeek API |
| 序列化 | `serde` / `serde_json` | 1.x | JSON 处理 |
| 异步运行时 | `tokio` | 1.x | 异步任务 |
| UI 样式 | 原生 CSS | - | 扁平化设计，符合用户偏好 |

---

## 4. 项目结构

```
AI国象/
├── src-tauri/                    # Rust 后端
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── build.rs
│   └── src/
│       ├── main.rs               # Tauri 入口
│       ├── lib.rs                # 模块导出
│       ├── chess_engine.rs       # 棋局状态管理（chess crate 封装）
│       ├── deepseek.rs           # DeepSeek API 客户端
│       ├── prompt.rs             # 提示词构建（棋盘状态、规则、历史）
│       ├── move_parser.rs        # DeepSeek 响应解析与走法验证
│       ├── game_state.rs         # 全局游戏状态（AppState）
│       └── commands.rs           # Tauri 命令（前端调用接口）
├── src/                          # Svelte 前端
│   ├── main.ts                   # 前端入口
│   ├── App.svelte                # 根组件
│   ├── app.css                   # 全局样式
│   ├── lib/
│   │   ├── components/
│   │   │   ├── Board.svelte       # 棋盘组件（8x8 格子）
│   │   │   ├── Square.svelte      # 单格组件
│   │   │   ├── Piece.svelte       # 棋子组件
│   │   │   ├── MoveHistory.svelte # 走法历史面板
│   │   │   ├── GameStatus.svelte  # 游戏状态显示
│   │   │   ├── SettingsPanel.svelte # 设置面板（API Key、执方、N步）
│   │   │   └── Sidebar.svelte     # 左侧栏容器
│   │   ├── stores/
│   │   │   ├── game.ts            # 游戏状态 store
│   │   │   └── settings.ts        # 设置 store
│   │   ├── types.ts               # TypeScript 类型定义
│   │   └── api.ts                 # Tauri invoke 封装
│   └── assets/
│       └── pieces/                # 棋子 SVG 图标
├── index.html
├── package.json
├── vite.config.ts
├── svelte.config.js
└── tsconfig.json
```

---

## 5. 后端实现 (Rust)

### 5.1 棋局状态管理 (`chess_engine.rs`)

使用 `chess` crate 封装棋局逻辑：

```rust
use chess::{ChessMove, Color, GameResult, MoveGen, Board, Square};
use std::sync::Mutex;

pub struct ChessGame {
    pub board: Board,
    pub move_history: Vec<ChessMove>,  // 完整走法历史
    pub player_side: Color,            // 玩家执方
}

impl ChessGame {
    pub fn new(player_side: Color) -> Self { ... }
    
    // 获取当前 FEN 字符串
    pub fn to_fen(&self) -> String { ... }
    
    // 获取可视化的棋盘（用于 DeepSeek 提示）
    pub fn to_visual_string(&self) -> String { ... }
    
    // 获取最近 N 步走法历史
    pub fn recent_moves(&self, n: usize) -> Vec<String> { ... }
    
    // 获取所有合法走法
    pub fn legal_moves(&self) -> Vec<ChessMove> { ... }
    
    // 验证走法是否合法
    pub fn is_legal(&self, mv: &ChessMove) -> bool { ... }
    
    // 应用走法
    pub fn make_move(&mut self, mv: ChessMove) -> Result<(), String> { ... }
    
    // 游戏是否结束
    pub fn is_game_over(&self) -> Option<GameResult> { ... }
    
    // 当前轮到谁
    pub fn side_to_move(&self) -> Color { ... }
}
```

### 5.2 DeepSeek API 客户端 (`deepseek.rs`)

调用 DeepSeek 官方 API（OpenAI 兼容）：

```rust
use reqwest::Client;

const DEEPSEEK_API_URL: &str = "https://api.deepseek.com/v1/chat/completions";

pub struct DeepSeekClient {
    client: Client,
    api_key: String,
    model: String,  // "deepseek-chat" 或 "deepseek-reasoner"
}

impl DeepSeekClient {
    pub async fn chat(&self, messages: Vec<Message>) -> Result<String, String> {
        // POST 请求到 DeepSeek API
        // 返回 assistant 消息内容
    }
    
    pub async fn get_move(&self, prompt: String) -> Result<String, String> {
        // 请求 DeepSeek 返回走法
    }
}
```

### 5.3 提示词构建 (`prompt.rs`)

构建发送给 DeepSeek 的完整提示词，包含：

1. **角色设定**：你是一个国际象棋大师，正在与人类对弈
2. **完整国际象棋规则**：棋子走法、特殊规则（王车易位、吃过路兵、升变）、将军/将杀/和棋
3. **当前棋盘状态**：
   - FEN 表示法
   - 可视化棋盘（8x8 字符矩阵，大小写区分黑白）
4. **走法历史**：最近 N 步（可配置，默认 10 步）
5. **执方信息**：你执黑/白
6. **响应格式要求**：必须返回标准代数记号（SAN）或坐标记号（如 `e2e4`），只返回走法不要解释

```rust
pub fn build_prompt(game: &ChessGame, history_n: usize) -> String {
    // 拼接：角色 + 规则 + 棋盘状态 + 历史 + 格式要求
}

// 完整规则文本（常量）
const CHESS_RULES: &str = r#"
完整国际象棋规则：
1. 棋盘：8x8，列 a-h，行 1-8
2. 棋子：王(K)、后(Q)、车(R)、象(B)、马(N)、兵(P)
3. 走法规则：
   - 王：横竖斜一格
   - 后：横竖斜任意格
   - 车：横竖任意格
   - 象：斜任意格
   - 马：L 形（2+1）
   - 兵：前进一格（首次可两格），斜吃，到底升变
4. 特殊规则：
   - 王车易位（短/长）
   - 吃过路兵
   - 兵升变（默认升后，可选车/象/马）
5. 将军：王被攻击，必须解除将军
6. 将杀：王被攻击且无法解除，游戏结束
7. 和棋：逼和、50步规则、三次重复局面、双方同意
8. 记号：使用坐标记号如 e2e4，升变如 e7e8q
"#;
```

### 5.4 走法解析与验证 (`move_parser.rs`)

```rust
pub fn parse_and_validate(
    response: &str,
    game: &ChessGame,
) -> Result<ChessMove, ParseError> {
    // 1. 从响应中提取走法字符串（支持 e2e4 / e2-e4 / SAN）
    // 2. 尝试解析为 ChessMove
    // 3. 验证是否在合法走法列表中
    // 4. 非法时返回错误信息（包含合法走法列表供重试）
}

pub enum ParseError {
    InvalidFormat(String),
    IllegalMove(String, Vec<String>),  // (走法, 合法走法列表)
}
```

### 5.5 Tauri 命令 (`commands.rs`)

前端调用的接口：

```rust
#[tauri::command]
async fn start_game(side: String, api_key: String, model: String) -> Result<GameState, String>;

#[tauri::command]
async fn player_move(move_str: String) -> Result<MoveResult, String>;

#[tauri::command]
async fn ai_move() -> Result<MoveResult, String>;  // 触发 DeepSeek 走棋

#[tauri::command]
async fn get_game_state() -> Result<GameState, String>;

#[tauri::command]
async fn reset_game() -> Result<GameState, String>;

#[tauri::command]
async fn undo_move() -> Result<GameState, String>;
```

### 5.6 全局状态 (`game_state.rs`)

```rust
use std::sync::Mutex;
use std::sync::Arc;

pub struct AppState {
    pub game: Mutex<Option<ChessGame>>,
    pub deepseek: Mutex<Option<DeepSeekClient>>,
    pub settings: Mutex<Settings>,
}

pub struct Settings {
    pub api_key: String,
    pub model: String,
    pub history_n: usize,  // 前N步历史，默认10
    pub max_retries: u32,  // 非法走法最大重试次数，默认3
}
```

### 5.7 AI 走棋流程（核心逻辑）

```rust
async fn ai_move_internal(state: &AppState) -> Result<MoveResult, String> {
    let game = state.game.lock().unwrap();
    let game = game.as_ref().unwrap();
    
    // 1. 构建提示词
    let prompt = build_prompt(game, settings.history_n);
    
    // 2. 调用 DeepSeek，带重试机制
    for attempt in 0..settings.max_retries {
        let response = deepseek.get_move(prompt.clone()).await?;
        
        // 3. 解析并验证走法
        match parse_and_validate(&response, game) {
            Ok(mv) => {
                // 4. 应用走法
                game.make_move(mv)?;
                return Ok(MoveResult { move_san, new_state, game_over });
            }
            Err(ParseError::IllegalMove(bad, legal)) => {
                // 5. 非法走法：带反馈重试
                let retry_prompt = format!(
                    "你上一步 {} 是非法的。合法走法：{}。请重新选择。",
                    bad, legal.join(", ")
                );
                // 下次循环使用带反馈的提示词
            }
            Err(ParseError::InvalidFormat(s)) => {
                // 格式错误，重试
            }
        }
    }
    
    Err("DeepSeek 多次返回非法走法".into())
}
```

---

## 6. 前端实现 (Svelte) — UI 设计系统

### 6.0 设计方向：「棋谱编辑部」

**美学定位**：精致的编辑式极简主义 (Editorial Minimalism)。
融合现代应用与高级棋类期刊的安静质感——温暖的纸张色调、有性格的衬线展示字体、克制的留白、单一自信的强调色。整体克制、安静、有呼吸感，**绝对简洁但每一处细节都耐看**。

避免一切通用 AI 审美：不用 Inter/Roboto、不用紫色渐变、不用卡片堆叠、不用浮夸阴影。

### 6.1 设计 Tokens (CSS 变量)

```css
:root {
  /* 色彩 —— 温暖纸张 + 墨色 + 单一沉绿强调 */
  --bg: #F6F2EA;              /* 暖纸底色 */
  --surface: #FCFAF5;         /* 卡片/面板表面 */
  --surface-2: #EFEAE0;       /* 次级表面（棋盘浅格） */
  --ink: #1B1A17;             /* 主文字（近黑墨色） */
  --ink-muted: #8C857A;       /* 次级文字 */
  --ink-faint: #B8B0A2;       /* 弱化文字/分隔 */
  --line: #E3DDD0;            /* 分隔线 */
  --accent: #2E4A3E;          /* 沉绿强调（象棋桌呢） */
  --accent-soft: #E8EDE7;     /* 强调色浅底 */
  --board-light: #EDE4D3;     /* 棋盘浅格 */
  --board-dark: #6B7F6E;      /* 棋盘深格（柔灰绿） */
  --highlight: #C9A961;       /* 上一步/选中（赭金） */
  --danger: #A8453A;          /* 将军/错误（砖红） */

  /* 字体 */
  --font-display: 'Fraunces', Georgia, serif;       /* 展示衬线，有光学尺寸 */
  --font-sans: 'Geist', system-ui, sans-serif;       /* UI 无衬线 */
  --font-mono: 'Geist Mono', ui-monospace, monospace;/* 走法记号 */

  /* 间距节奏 */
  --sp-1: 4px; --sp-2: 8px; --sp-3: 12px; --sp-4: 16px;
  --sp-5: 24px; --sp-6: 32px; --sp-7: 48px; --sp-8: 64px;

  /* 圆角 —— 极小，近乎方正，符合偏好 */
  --r-sm: 4px; --r-md: 6px; --r-lg: 10px;

  /* 缓动 */
  --ease: cubic-bezier(0.16, 1, 0.3, 1);  /* ease-out-expo */
}
```

字体加载：Fraunces + Geist + Geist Mono，通过 Google Fonts / Fontsource 本地引入。

### 6.2 整体布局

**双栏**，左侧栏固定 `280px`，右侧棋盘区自适应居中沉浸：

```
┌─────────────┬──────────────────────────────────┐
│             │                                  │
│  侧栏 280px │         棋盘居中区                │
│             │       （暖纸大背景 + 留白）       │
│  - 标题     │                                  │
│  - 状态     │         [ 8x8 棋盘 ]             │
│  - 设置     │                                  │
│  - 历史     │         回合指示 / 计时           │
│             │                                  │
└─────────────┴──────────────────────────────────┘
```

- 侧栏背景 `--surface`，右侧 `--bg`，二者仅靠极细分隔线 `--line` 区分（无阴影）
- 侧栏内容垂直排列，各区块间用 `--sp-6` 留白分隔
- 棋盘区大留白，棋盘居中，最大尺寸约 `min(70vh, 560px)`，保持正方形

### 6.3 入场动画（遵循用户偏好：交错 + ease-out-expo）

页面加载时侧栏各区块与棋盘**交错淡入上移**：

```css
@keyframes rise {
  from { opacity: 0; transform: translateY(12px); }
  to   { opacity: 1; transform: translateY(0); }
}
.rise { animation: rise 0.7s var(--ease) both; }
/* 各元素 animation-delay 递增 80ms：0, 0.08s, 0.16s, 0.24s... */
```

棋盘格子可做极轻微的波纹式显色入场（格子按对角线顺序淡入深浅色）。

### 6.4 棋盘组件 (`Board.svelte`)

**视觉**：
- 8x8 CSS Grid，无边框、无外阴影
- 浅格 `--board-light` (#EDE4D3)，深格 `--board-dark` (#6B7F6E 柔灰绿)——非俗套棕褐
- 格子间无缝拼接
- 棋盘外侧极简坐标：底部 `a-h`（衬线小写），左侧 `1-8`，颜色 `--ink-faint`

**棋子**：
- SVG 矢量图标（CBurnett 公共领域），黑白两套
- 棋子尺寸约为格子的 80%，居中
- 选中棋子：格子叠加 `--highlight` 半透明层（非边框）
- 合法走法提示：目标格中心一个 `10px` 圆点，`--accent` 色 30% 透明；有吃子时改为空心环
- 上一步走法：起止两格叠加 `--highlight` 15% 透明
- 将军：王所在格叠加 `--danger` 20% 透明脉动

**交互**：
- 点击己方棋子 → 高亮 + 显示合法走法点
- 再点目标格 → 移动；点其他己方棋子 → 切换选中；点空白 → 取消
- 鼠标悬停可走棋子：光标 `pointer`，格子极轻微提亮
- 棋盘根据玩家执方翻转（玩家执黑时黑方在下）

**走子动画**：棋子用 CSS `transform` 平移过渡 `0.25s var(--ease)`，吃子时被吃方淡出缩小。

### 6.5 侧栏各区块

#### 标题区
- 小标题「CHESS」用 `--font-display` Fraunces，字重 500，字间距宽松，下方一行 `--ink-muted` 小字「对弈 DeepSeek」
- 极简，无 logo 图标干扰

#### 游戏状态 (`GameStatus.svelte`)
- 大号当前回合指示：用 `--font-display` 显示「白方思考中」/「黑方思考中」
- DeepSeek 思考时：文字旁三个小圆点呼吸式跳动动画（错开延迟）
- 步数 / 用时：`--font-mono` 小字，`--ink-muted`
- 游戏结束：中央覆盖一个极简结果条（胜/负/和），`--accent` 或 `--danger` 底，白字

#### 设置面板 (`SettingsPanel.svelte`)
- 标签用 `--font-sans` 小写大写混合，`--ink-muted`
- **API Key**：底线输入框（无边框，仅底部 1px `--line`，聚焦变 `--accent`），右侧小眼睛图标切换显隐
- **模型选择**：极简下拉，或两个方形分段按钮（chat / reasoner）
- **执方选择**：两个方形按钮「白」「黑」，选中态为 `--accent` 底白字，未选中为透明 `--ink` 字——**方形开关**符合用户偏好
- **历史步数 N**：底线数字输入，`--font-mono`
- **操作按钮**：开始（`--accent` 底白字）、重置、悔棋（透明底 `--ink` 字，悬停 `--surface-2` 底）——全部扁平，无阴影无圆角至多 `--r-sm`

#### 走法历史 (`MoveHistory.svelte`)
- 双列：左白方 / 右黑方，行号 `--font-mono` 小字 `--ink-faint`
- 走法用 `--font-mono`，`--ink`
- 最新一步 `--accent` 色高亮
- 自动滚动到底（流式输出时强制滚动，用户手动上滚则停止强制，回到底部恢复——遵循用户偏好）
- 空状态：`--ink-faint` 斜体「尚无走法」

### 6.6 升变选择 UI

玩家兵到底时，弹出极简浮层：四个方形按钮（后/车/象/马），`--surface` 底，`--font-display` 标注，选中后应用。无遮罩或仅极淡遮罩。

### 6.7 错误提示

- 非法走法 / API 失败：侧栏状态区一行 `--danger` 小字，淡入显示 3 秒后淡出
- 不用 alert/弹窗

### 6.8 状态管理 (`stores/game.ts`)

```typescript
import { writable } from 'svelte/store';

export interface GameState {
  board: string;          // FEN
  playerSide: 'white' | 'black';
  turn: 'white' | 'black';
  moveHistory: string[];
  status: 'playing' | 'thinking' | 'checkmate' | 'stalemate' | 'draw';
  winner?: 'white' | 'black';
  lastMove?: { from: string; to: string };
  legalMoves: string[];   // 当前选中棋子的合法走法
  selectedSquare?: string;
  inCheck?: boolean;
}

export const gameState = writable<GameState>(initialState);
```

### 6.9 前端 API 封装 (`lib/api.ts`)

```typescript
import { invoke } from '@tauri-apps/api/core';

export async function startGame(side, apiKey, model) {
  return invoke('start_game', { side, apiKey, model });
}
export async function playerMove(moveStr) {
  return invoke('player_move', { moveStr });
}
export async function aiMove() {
  return invoke('ai_move');
}
// ...
```

### 6.10 设计要点总结（确保简洁好看）

1. **克制**：通篇无 box-shadow（除棋盘极淡投影可选）、无渐变、无 3D
2. **呼吸感**：大量留白，元素间距遵循 `--sp-5`/`--sp-6` 节奏
3. **字体性格**：Fraunces 衬线承担「格调」，Geist 承担「清晰」，Geist Mono 承担「棋谱」
4. **单一强调**：沉绿 `--accent` 只用在关键状态（选中、当前步、主按钮），其余靠灰阶层级
5. **暖色基底**：纸张色 `--bg` 让整体不冷硬，有温度
6. **动画安静**：只做淡入上移与走子平移，时长 0.25–0.7s，统一 ease-out-expo，不花哨
7. **棋盘是主角**：棋盘居中、最大、留白包裹，其余元素退让

---

## 7. DeepSeek 走法数据流

```
玩家走棋 → 更新棋盘 → 轮到 AI →
  构建提示词（FEN + 可视化棋盘 + 规则 + 历史N步）→
  调用 DeepSeek API →
  解析响应 → 验证合法性 →
    合法：应用走法，更新棋盘，返回前端
    非法：带反馈重试（最多 max_retries 次）→
      仍非法：返回错误，让玩家处理
```

---

## 8. 棋子 SVG 资源

使用开源国际象棋棋子 SVG（如 Wikimedia Commons 的 CBurnett 棋子集，公共领域），放在 `src/assets/pieces/`：
- `wK.svg` `wQ.svg` `wR.svg` `wB.svg` `wN.svg` `wP.svg`
- `bK.svg` `bQ.svg` `bR.svg` `bB.svg` `bN.svg` `bP.svg`

---

## 9. 实施步骤（按顺序）

### 步骤 1：项目初始化
- 使用 `npm create tauri-app@latest` 创建 Tauri + Svelte 项目
- 配置 `Cargo.toml` 添加依赖：`chess`, `reqwest`, `serde`, `serde_json`, `tokio`, `tauri`
- 验证：`cargo build` 和 `npm run dev` 均可运行

### 步骤 2：后端棋局核心
- 实现 `chess_engine.rs`：封装 chess crate，提供 FEN、可视化、走法、历史
- 实现 `game_state.rs`：AppState 结构
- 单元测试：验证走法、FEN 生成、合法走法列表

### 步骤 3：DeepSeek 集成
- 实现 `deepseek.rs`：HTTP 客户端，chat 接口
- 实现 `prompt.rs`：提示词构建（规则 + 棋盘 + 历史）
- 实现 `move_parser.rs`：响应解析与验证
- 测试：用 mock API 测试解析和验证逻辑

### 步骤 4：Tauri 命令层
- 实现 `commands.rs`：start_game, player_move, ai_move, reset, undo
- 实现 AI 走棋流程（含重试机制）
- 在 `main.rs` 注册命令

### 步骤 5：前端棋盘 UI
- 实现 `Board.svelte` + `Square.svelte` + `Piece.svelte`
- 棋盘渲染、棋子放置、点击交互
- 合法走法提示、上一步高亮
- 导入棋子 SVG 资源

### 步骤 6：前端控制面板
- 实现 `SettingsPanel.svelte`：API Key、模型、执方、N步设置
- 实现 `GameStatus.svelte`：状态显示、思考动画
- 实现 `MoveHistory.svelte`：走法列表
- 实现 `Sidebar.svelte`：280px 左栏容器
- 实现双栏布局 + 扁平化样式 + 入场动画

### 步骤 7：前后端联调
- 前端调用 Tauri 命令，打通完整对弈流程
- 玩家走棋 → 棋盘更新 → AI 思考 → AI 走棋 → 棋盘更新
- 测试：完整对局、非法走法处理、游戏结束判定

### 步骤 8：边界情况与打磨
- 升变 UI（选择升变棋子）
- 王车易位、吃过路兵交互
- 游戏结束弹窗（胜/负/和）
- 悔棋功能
- 错误提示（API 失败、网络错误）

### 步骤 9：验证与测试
- 完整对局测试（人 vs DeepSeek）
- 各种特殊走法测试
- 非法走法重试测试
- 不同执方测试
- 性能与响应速度评估

### 步骤 10：设定下一个 PLAN 和 SPEC（需要询问用户）
- 本计划完成后，询问用户下一步要做什么
- 可能的后续方向：
  - 难度调节（调整 DeepSeek 提示词策略）
  - PGN 棋谱导出
  - 复盘功能
  - 多局对弈记录
  - 自定义规则变体
- **需要询问用户确认下一个计划方向，不允许在未获用户同意前停止输出**

---

## 10. 假设与决策

| 项 | 决策 | 原因 |
|---|---|---|
| 走法记号 | 坐标记号 `e2e4` | 解析简单，DeepSeek 易遵循；升变用 `e7e8q` |
| 历史步数 N | 默认 10，可配置 | 平衡 token 用量与上下文 |
| 非法走法重试 | 最多 3 次，带合法走法反馈 | 避免 API 浪费，提升成功率 |
| 棋子图标 | CBurnett SVG（公共领域） | 经典美观，免费可用 |
| 模型 | 默认 deepseek-chat，可选 reasoner | chat 速度快，reasoner 推理强 |
| API Key 存储 | 仅内存，不持久化 | 安全优先，重启需重输 |
| 棋盘渲染 | CSS Grid + SVG | 轻量，无需额外依赖 |
| 棋库 | `chess` crate | 成熟稳定，API 清晰 |

---

## 11. 验证步骤

1. **项目构建**：`cargo build` 和 `npm run build` 无错误
2. **开发运行**：`npm run tauri dev` 启动应用
3. **棋盘渲染**：8x8 棋盘正确显示，棋子位置正确（初始局面）
4. **玩家走棋**：点击棋子→显示合法走法→点击目标→棋子移动
5. **DeepSeek 走棋**：轮到 AI 时正确调用 API 并返回合法走法
6. **非法走法处理**：DeepSeek 返回非法走法时重试，日志可见
7. **特殊走法**：王车易位、吃过路兵、升变均可用
8. **游戏结束**：将杀/逼和/和棋正确判定并显示
9. **执方切换**：可选执白或执黑，棋盘正确翻转
10. **UI 一致性**：扁平化、双栏布局、动画符合用户偏好
