# 修复重复设置面板问题

## 摘要

UI 上出现"俩设置面板"，根因是有**两个入口**都渲染同一个 `<Settings />` 组件：

1. **顶部工具栏"设置"按钮**（[App.svelte:436](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L436) `onclick={toggleDrawer}`）→ 打开底部抽屉，抽屉 body 里是 `<Settings />`（[App.svelte:555](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L555)）
2. **右侧合并面板"设置"标签**（[App.svelte:463](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L463) `onclick={() => selectTab("settings")}`）→ 面板内容区渲染 `<Settings />`（[App.svelte:475](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L475)）

桌面端这两个入口同时存在，用户可从顶部按钮打开抽屉式设置，也可从右侧标签打开内嵌式设置，视觉上像"俩设置面板"。

窄屏下也有重复：顶部工具栏"设置"按钮（无 `narrow-only` 类，全屏显示）+ 底部状态栏"设置"按钮（`narrow-only`，窄屏显示）。

## 当前状态分析

### 桌面端设置入口（重复）

```
顶部工具栏:
  [悔棋] [重开] [重新请求] | [鳕鱼] [展开/收起面板] [设置] ← 入口1（打开抽屉）

右侧合并面板:
  [走法历史] [引擎分析] [设置] ← 入口2（标签页内嵌）
                                    ↓
                              <Settings /> 组件

底部抽屉（drawerOpen=true 时滑出）:
  ┌─────────────────────┐
  │ 设置            ✕   │
  │ <Settings />        │ ← 同一个组件重复渲染
  └─────────────────────┘
```

### 窄屏设置入口（重复）

```
顶部工具栏: [设置] ← 入口1（无 narrow-only，全屏显示，打开抽屉）
底部状态栏: [设置] ← 入口2（narrow-only，窄屏显示，打开抽屉）
```

两个按钮都调用 `toggleDrawer`，功能完全相同。

### 用户偏好回顾

- 用户要求右侧合并面板整合三标签（走法历史/引擎分析/设置）→ 面板内"设置"标签是期望的主入口
- 用户要求"开始对弈/继续对局"常驻主页面 → 设置入口不应阻碍主操作
- 桌面端右侧面板已提供"展开/收起"按钮，收起后可恢复 → 无需额外的设置入口

## 拟议修改

### 修改 1：顶部工具栏"设置"按钮加 `narrow-only` 类

**文件**：[src/App.svelte:436](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L436)

**改什么**：给顶部工具栏的"设置"按钮加 `narrow-only` 类，使其桌面端隐藏、窄屏显示。

**为什么**：
- 桌面端已有右侧合并面板"设置"标签作为主入口，顶部按钮多余
- 窄屏下右侧面板不渲染（`{#if started && !narrowScreen}`），需要保留顶部按钮打开抽屉
- `narrow-only` 类的 CSS 已存在（[App.svelte:603-606](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L603-L606) 桌面 `display: none`，[App.svelte:607-610](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L607-L610) 窄屏 `display: inline-flex`），无需新增样式

**怎么改**：
```svelte
<!-- 原 -->
<button class="bar-btn" onclick={toggleDrawer} title="打开设置抽屉">设置</button>

<!-- 改为 -->
<button class="bar-btn narrow-only" onclick={toggleDrawer} title="打开设置抽屉">设置</button>
```

### 修改 2：移除底部状态栏"设置"按钮

**文件**：[src/App.svelte:532-535](file:///c:/Users/24453/Desktop/AI国象/src/App.svelte#L532-L535)

**改什么**：移除底部状态栏 `.actions-cell` 里的"设置"按钮（`narrow-only` 那个）。

**为什么**：
- 窄屏下顶部工具栏已有"设置"按钮（修改 1 后），底部按钮重复
- 移除后窄屏只有一个设置入口（顶部），桌面端也只有一个（右侧面板标签）
- 底部状态栏空间留给"风的加护"开关和评估显示，更简洁

**怎么改**：
```svelte
<!-- 移除这段 -->
<!-- 窄屏下设置入口（桌面端已在顶部工具栏） -->
<button class="bar-btn settings-btn narrow-only" class:active={drawerOpen} onclick={toggleDrawer}>
  设置
</button>
```

直接删除这 3 行（含注释）。

### 不需要改动的部分

- **右侧合并面板"设置"标签**：保留，是桌面端主入口
- **底部抽屉**：保留，窄屏通过顶部按钮触发
- **`toggleDrawer` 函数**：保留，仍被顶部按钮使用
- **抽屉式 `<Settings />`**：保留，窄屏使用
- **`drawerOpen` 状态**：保留

## 假设与决策

| # | 假设/决策 | 说明 |
|---|----------|------|
| 1 | 桌面端主入口是右侧面板"设置"标签 | 符合用户偏好（三标签整合面板） |
| 2 | 窄屏主入口是顶部工具栏"设置"按钮 | 顶部按钮位置显眼，复用现有 UI |
| 3 | 移除底部状态栏"设置"按钮 | 避免窄屏重复，简化底部状态栏 |
| 4 | 不移除底部抽屉本身 | 窄屏仍需要抽屉承载 Settings 组件 |
| 5 | 桌面端收起面板后仍可通过"展开面板"恢复 | 已有按钮支持，无需额外设置入口 |

## 验证步骤

1. **构建验证**：`npm run build` 通过，无 error
2. **视觉验证**（启动 `npm run tauri dev`）：
   - **桌面端**（≥900px）：
     - 顶部工具栏**无**"设置"按钮（隐藏）
     - 右侧合并面板"设置"标签可见，点击显示 `<Settings />`
     - 底部状态栏**无**"设置"按钮
     - 只有一个设置入口
   - **窄屏**（<900px）：
     - 顶部工具栏"设置"按钮可见，点击打开底部抽屉
     - 底部状态栏**无**"设置"按钮
     - 只有一个设置入口
3. **功能验证**：
   - 桌面端在"设置"标签修改 API Key，应用设置后对局正常
   - 窄屏在抽屉里修改 API Key，应用设置后对局正常
   - 桌面端收起右侧面板 → 点"展开面板"恢复 → "设置"标签可用

## 执行顺序

1. 修改 1：顶部工具栏"设置"按钮加 `narrow-only` 类
2. 修改 2：移除底部状态栏"设置"按钮
3. 运行 `npm run build` 验证
4. 询问用户是否需要启动 `npm run tauri dev` 进行视觉验证
