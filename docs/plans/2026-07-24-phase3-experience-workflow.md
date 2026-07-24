---
intent: 为棋子走棋添加 FLIP 滑动动画（抬起+落子微弹）+ CC0 走子音效 + 将军/将杀视觉特效
success_criteria:
  - 走棋时棋子从源格平滑滑动到目标格，起手微抬，落子微弹
  - 走子/吃子/将军/将杀各有正确音效触发
  - 将军时国王格强化脉冲，将杀时全屏收束动效
  - cargo check 与 npm run check 均通过 0 errors 0 warnings
risk_level: medium
auto_approve: false
---

## Steps

- [ ] **Step 1: 创建音效资源目录与占位 ogg 文件**
action: |
  创建目录 `src/lib/assets/sounds/`。由于无法联网下载，执行阶段用 Web Audio API 离线渲染生成 4 个简单 ogg 占位音效文件（后续可替换为真实 CC0 音效）：
  - `move.ogg` — 短促木头敲击声（200Hz 正弦波 80ms 衰减）
  - `capture.ogg` — 略重的敲击（150Hz + 噪声 120ms）
  - `check.ogg` — 提示音（440Hz→660Hz 滑音 200ms）
  - `mate.ogg` — 低沉收束（110Hz 长音 600ms 衰减）
  
  执行时用 Node.js 脚本生成纯音 ogg，或用 base64 内联最小化 ogg。若生成困难，退化为 Web Audio API 实时合成（在 Step 3 实现）。
loop: false
verify:
  type: artifact
  path: src/lib/assets/sounds
  assert:
    kind: matches-glob
    value: "*.ogg"
max_iterations: 3

- [ ] **Step 2: Board.svelte 新增音效播放工具与 last_move 动画状态**
action: |
  修改 `src/lib/components/Board.svelte`：
  1. 顶部新增音效工具：
     ```typescript
     // 音效缓存（首次播放时实例化）
     let audioCtx: AudioContext | null = null;
     const soundCache: Record<string, HTMLAudioElement> = {};
     
     function preloadSounds() {
       for (const s of ["move", "capture", "check", "mate"]) {
         const audio = new Audio();
         audio.src = `${import.meta.env.BASE_URL}sounds/${s}.ogg`;
         soundCache[s] = audio;
       }
     }
     
     function playSound(name: "move" | "capture" | "check" | "mate") {
       const audio = soundCache[name];
       if (audio) {
         audio.currentTime = 0;
         audio.play().catch(() => {}); // 静默失败（autoplay policy）
       }
     }
     ```
  2. 在 onMount 或组件初始化时调用 preloadSounds()
  3. 新增动画状态：
     ```typescript
     let isAnimating = $state(false);
     let animatingPiece = $state<{ src: string; fromRect: DOMRect; toRect: DOMRect } | null>(null);
     ```
loop: false
verify: npm run check
max_iterations: 3

- [ ] **Step 3: Board.svelte 实现 FLIP 滑动动画 + 抬起落子**
action: |
  修改 `src/lib/components/Board.svelte` 的 executeMove 函数（第 149-161 行区域）：
  1. 走棋前记录源格棋子 DOM 的 getBoundingClientRect
  2. 调用 playerMove 成功后，在 updateGameState 之前记录目标格 DOM 的 getBoundingClientRect
  3. 创建临时浮动 img 元素（absolute 定位），从源位置 transform 到目标位置：
     ```typescript
     async function executeMove(moveStr: string) {
       selectedSquare.set(null);
       const fromSq = moveStr.substring(0, 2);
       const toSq = moveStr.substring(2, 4);
       const fromEl = document.querySelector(`[aria-label="${fromSq}"] .piece`) as HTMLElement | null;
       const fromRect = fromEl?.getBoundingClientRect();
       
       try {
         const result = await playerMove(moveStr);
         // 等 DOM 更新后获取目标格位置
         await tick();
         const toEl = document.querySelector(`[aria-label="${toSq}"]`) as HTMLElement | null;
         const toRect = toEl?.getBoundingClientRect();
         
         if (fromRect && toRect) {
           await runFlipAnimation(fromEl, fromRect, toRect);
         }
         
         updateGameState(result.state);
         playSound(result.state.last_move?.captured ? "capture" : "move");
         
         if (result.state.in_check) playSound("check");
         if (result.game_over && result.state.status === "checkmate") playSound("mate");
         
         if (!result.game_over) {
           await triggerAiMove();
         }
       } catch (e) {
         showError(String(e));
         selectedSquare.set(null);
       }
     }
     ```
  4. runFlipAnimation 实现：
     ```typescript
     async function runFlipAnimation(el: HTMLElement, fromRect: DOMRect, toRect: DOMRect) {
       isAnimating = true;
       const dx = fromRect.left - toRect.left;
       const dy = fromRect.top - toRect.top;
       el.style.transition = "none";
       el.style.transform = `translate(${dx}px, ${dy}px) scale(1.05)`;
       el.style.zIndex = "10";
       el.style.filter = "drop-shadow(0 6px 12px rgba(0,0,0,0.25))";
       // 强制重排
       el.offsetHeight;
       el.style.transition = "transform 0.3s cubic-bezier(0.16,1,0.3,1), filter 0.3s";
       el.style.transform = "translate(0, 0) scale(1)";
       el.style.filter = "drop-shadow(0 1px 2px rgba(0,0,0,0.18))";
       await new Promise(r => setTimeout(r, 320));
       el.style.zIndex = "";
       isAnimating = false;
     }
     ```
  5. handleClick 开头加 `if (isAnimating) return;` 防误触
loop: false
verify: npm run check
max_iterations: 3

- [ ] **Step 4: Board.svelte AI 走棋同样触发动画与音效**
action: |
  修改 `src/lib/components/Board.svelte` 的 triggerAiMove 函数（第 177-187 行区域）：
  ```typescript
  async function triggerAiMove() {
    aiThinking.set(true);
    try {
      const result = await aiMove();
      // AI 走棋的 from/to 在 result.state.last_move
      const lm = result.state.last_move;
      if (lm) {
        await tick();
        const fromEl = document.querySelector(`[aria-label="${lm.from}"] .piece`) as HTMLElement | null;
        const toEl = document.querySelector(`[aria-label="${lm.to}"]`) as HTMLElement | null;
        if (fromEl && toEl) {
          const fromRect = fromEl.getBoundingClientRect();
          const toRect = toEl.getBoundingClientRect();
          await runFlipAnimation(fromEl, fromRect, toRect);
        }
      }
      updateGameState(result.state);
      playSound(lm?.captured ? "capture" : "move");
      if (result.state.in_check) playSound("check");
      if (result.game_over && result.state.status === "checkmate") playSound("mate");
    } catch (e) {
      showError(String(e));
    } finally {
      aiThinking.set(false);
    }
  }
  ```
  注意：需从 svelte import tick。AI 走棋时 DOM 已更新（updateGameState 后），但 FLIP 需要在更新前获取 from 位置——此处 AI 走棋的 from 位置是走棋前的棋子，updateGameState 后该棋子已在 to 位置。因此 AI 动画需在 updateGameState 之前获取 from 位置，但 to 位置需在 updateGameState 之后。调整：先获取当前 from 棋子位置，updateGameState 后获取 to 位置，from 棋子已移走所以用 from 格的 rect 作为起点。
loop: false
verify: npm run check
max_iterations: 3

- [ ] **Step 5: Board.svelte 将军/将杀视觉特效强化**
action: |
  修改 `src/lib/components/Board.svelte` 的 CSS：
  1. 将军时国王格脉冲强化（现有 .sq.check 加强化动画）：
     ```css
     .sq.check {
       background-color: var(--danger) !important;
       background-image: none;
       animation: pulse-check-strong 1s var(--ease) infinite;
     }
     @keyframes pulse-check-strong {
       0%, 100% { 
         filter: brightness(1);
         box-shadow: inset 0 0 0 4px rgba(168, 69, 58, 0.6);
       }
       50% { 
         filter: brightness(1.3);
         box-shadow: inset 0 0 0 8px rgba(168, 69, 58, 0.8), 0 0 20px rgba(168, 69, 58, 0.4);
       }
     }
     ```
  2. 将杀时全屏收束动效（新增 .mate-overlay）：
     ```css
     .end-overlay.mate {
       animation: mate-converge 0.8s var(--ease) both;
     }
     @keyframes mate-converge {
       0% { 
         backdrop-filter: blur(0px);
         background: rgba(168, 69, 58, 0);
       }
       100% { 
         backdrop-filter: blur(4px);
         background: rgba(27, 26, 23, 0.75);
       }
     }
     .end-overlay.mate .end-card {
       animation: rise 0.6s var(--ease) 0.2s both;
     }
     ```
  3. 在模板中给将杀遮罩加 mate class：
     ```svelte
     {#if gameEnded}
       <div class="overlay end-overlay" class:mate={$gameState.status === "checkmate"}>
     ```
loop: false
verify: npm run check
max_iterations: 3

- [ ] **Step 6: 确认 last_move 类型含 captured 字段（用于区分走子/吃子音效）**
action: |
  检查 `src/lib/types.ts` 的 last_move 类型。若不含 captured 字段，则改为通过 FEN 对比判断吃子（走棋前记录被吃棋子）。简化方案：在 executeMove/triggerAiMove 中，走棋前先从 board 数据查找目标格是否有棋子：
  ```typescript
  // 走棋前检查目标格是否有敌方棋子（吃子）
  const toFile = parseInt(toSq.charCodeAt(0)) - 97;
  const toRank = parseInt(toSq[1]) - 1;
  const targetCell = board[7 - toRank][toFile];
  const isCapture = !!targetCell?.piece;
  ```
  此判断在 playerMove 调用前执行，board 还是走棋前的状态。
loop: false
verify: npm run check
max_iterations: 3

- [ ] **Step 7: 全量编译验证**
action: |
  运行后端与前端检查：
  - 后端：cargo check --manifest-path src-tauri/Cargo.toml（本 phase 不动后端，预期通过）
  - 前端：npm run check
  修复所有 error 和 warning。
loop: until both pass with 0 errors 0 warnings
verify:
  - type: shell
    command: cargo check --manifest-path src-tauri/Cargo.toml
  - type: shell
    command: npm run check
max_iterations: 3

- [ ] **Step 8: 人工验证动画与音效手感**
action: |
  启动 dev 应用（npm run tauri dev），人工验证：
  1. 玩家走棋：棋子从源格平滑滑到目标格，起手微抬（scale 1.05 + 阴影），落子回正
  2. 吃子：被吃方消失，走子方滑入，播放 capture 音效
  3. 普通走子：播放 move 音效
  4. 将军：国王格强化红色脉冲 + check 音效
  5. 将杀：全屏收束动效 + mate 音效 + 结束遮罩
  6. AI 走棋：同样有滑动动画与音效
  7. 动画期间点击棋盘无响应（防误触）
  8. 悔棋/重开：不触发动画与音效
gate: human
loop: false
verify:
  type: human-review
  check: 动画流畅、音效正确触发、将军将杀特效到位、动画期间无误触
