# Checklist

- [x] chat_stream 请求前输出 api_key 掩码、model、thinking、messages 数量
- [x] chat_stream HTTP 响应后输出状态码和 Content-Type
- [x] chat_stream 记录首个 data 行原始内容
- [x] chat_stream 记录流结束方式和累计字节数
- [x] ai_move 输出使用的客户端 api_key 掩码
- [x] SSE 解析正确处理 CRLF（trim_end_matches \r）
- [x] SSE 解析正确处理 data: 和 data: 两种前缀
- [x] 流自然结束（无 DONE）时返回已累积数据
- [x] ping_deepseek 命令实现并发送最小请求
- [x] ping_deepseek 返回 status_code、body_preview、content_len、reasoning_len、first_data_line
- [x] ping_deepseek 在 lib.rs 注册
- [x] api.ts 新增 pingDeepseek 函数
- [x] Settings.svelte 新增自检按钮并显示结果
- [x] cargo check 无错误无警告
- [x] npm run check 无错误无警告
- [ ] 人工自检按钮可正常调用并显示结果
- [ ] 自检结果能帮助定位空响应根因