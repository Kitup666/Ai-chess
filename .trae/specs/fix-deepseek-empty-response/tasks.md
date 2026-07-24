# Tasks

- [x] Task 1: 全链路诊断日志（deepseek.rs）
  - [x] 1.1: 在 chat_stream 请求前输出 api_key 掩码、model、thinking、messages 数量
  - [x] 1.2: 在 HTTP 响应后输出状态码和 Content-Type 头
  - [x] 1.3: 记录首个 data 行原始内容（前 300 字符）
  - [x] 1.4: 记录流结束方式（DONE / 自然结束）和累计字节数
  - [x] 1.5: 在 ai_move 中输出使用的客户端 api_key 掩码，验证 Key 即时生效

- [x] Task 2: SSE 解析健壮性修复（deepseek.rs）
  - [x] 2.1: 参照 Kitode extract_sse_lines，用 trim_end_matches('\r') 处理 CRLF
  - [x] 2.2: 确认 data: 和 data: 两种前缀都处理（当前已处理）
  - [x] 2.3: 流自然结束（无 DONE）时返回已累积数据（当前已处理）
  - [x] 2.4: 编译验证 cargo check 通过

- [x] Task 3: 新增自检命令 ping_deepseek（commands.rs + lib.rs）
  - [x] 3.1: 在 commands.rs 实现 ping_deepseek 命令，发送最小请求 "Hi"
  - [x] 3.2: 返回结构包含 status_code、body_preview、content_len、reasoning_len、first_data_line
  - [x] 3.3: 在 lib.rs 注册 ping_deepseek 命令
  - [x] 3.4: 编译验证 cargo check 通过

- [x] Task 4: 前端自检按钮（api.ts + Settings.svelte）
  - [x] 4.1: 在 api.ts 新增 pingDeepseek 函数
  - [x] 4.2: 在 Settings.svelte 新增自检按钮，调用 ping_deepseek
  - [x] 4.3: 显示自检结果（状态码、字节数、首行内容）
  - [x] 4.4: 编译验证 npm run check 通过

- [x] Task 5: 全量编译验证
  - [x] 5.1: cargo check 无错误无警告
  - [x] 5.2: npm run check 无错误无警告

- [ ] Task 6: 人工验证自检流程
  - [ ] 6.1: 打开设置，点击自检按钮，观察返回结果
  - [ ] 6.2: 根据自检结果判断 Key/模型/URL 是否可用
  - [ ] 6.3: 若自检正常，走一步棋验证 AI 走棋是否正常
  - [ ] 6.4: 若自检异常，根据返回的状态码和首行内容定位问题

# Task Dependencies
- Task 2 依赖 Task 1（诊断日志先行，便于验证修复效果）
- Task 3 独立于 Task 1/2，可并行
- Task 4 依赖 Task 3（前端调用后端命令）
- Task 5 依赖 Task 1/2/3/4 全部完成
- Task 6 依赖 Task 5