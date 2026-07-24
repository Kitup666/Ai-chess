# DeepSeek 空响应全量排查 Spec

## Why
用户已更换新的 DeepSeek API Key，但 AI 走棋时多次重试均返回空内容，最终使用兜底走法。请求未报 401 错误，说明 HTTP 层成功但响应体为空。需要全量排查从 API Key 传递到 SSE 解析的整条链路，定位空响应根因。

## What Changes
- 新增端到端诊断日志
- 修复 API Key 可能未真正生效的问题
- 修复 SSE 解析可能漏读数据的边界问题
- 验证请求体构造符合官方文档
- 新增自检命令用于离线验证配置连通性

## Impact
- src-tauri/src/deepseek.rs
- src-tauri/src/commands.rs
- src-tauri/src/lib.rs
- src/lib/api.ts
- src/lib/components/Settings.svelte

## ADDED Requirements

### Requirement: 全链路诊断日志
系统 SHALL 在 chat_stream 的关键节点输出结构化日志：
1. 请求发起前：api_key 掩码（前3后4）、模型名、thinking 开关、messages 数量
2. HTTP 响应：状态码、响应头 Content-Type
3. SSE 流：首个 data 行原始内容、累计 chunk 数、累计 content/reasoning 字节数
4. 流结束方式（DONE 标志 / 流自然结束 / 超时）

#### Scenario: 正常流式响应
- WHEN AI 走棋触发 chat_stream
- THEN 日志输出 api_key 掩码、HTTP 200、Content-Type text/event-stream、累计字节数大于 0

#### Scenario: 空响应诊断
- WHEN chat_stream 返回空 content 和空 reasoning
- THEN 日志输出首个 data 行原始内容（若存在）和流结束方式，便于定位是 API 返回空还是解析漏读

### Requirement: API Key 即时生效验证
系统 SHALL 确保 update_settings 命令重建的 DeepSeekClient 被 ai_move 正确使用，不残留旧客户端。

#### Scenario: 更换 Key 后立即走棋
- WHEN 用户在设置中输入新 Key 并点击应用设置
- AND 用户走一步棋触发 ai_move
- THEN ai_move 使用的客户端 api_key 必须是新 Key（通过日志掩码验证）

### Requirement: SSE 解析健壮性
系统 SHALL 参照 Kitode 的 extract_sse_lines 实现，正确处理：
1. \r\n 和 \n 两种行尾
2. data: 和 data: 两种前缀（带/不带空格）
3. 空行跳过
4. 流自然结束（无 DONE 标志）的情况

#### Scenario: 流自然结束无 DONE
- WHEN 服务端关闭连接但未发送 DONE 标志
- THEN 系统应返回已累积的 content/reasoning，不丢失数据

### Requirement: 自检命令 ping_deepseek
系统 SHALL 提供新命令 ping_deepseek，用当前配置发送一个最小请求（如 "Hi"），返回：
1. HTTP 状态码
2. 响应体前 500 字符
3. content 和 reasoning 字节数
4. 首个 data 行原始内容

#### Scenario: 用户自检配置
- WHEN 用户在设置面板点击自检按钮
- THEN 调用 ping_deepseek 并在前端显示结果，帮助用户快速判断 Key/模型/URL 是否可用