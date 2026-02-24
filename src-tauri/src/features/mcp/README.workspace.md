# MCP Workspace

MCP 使用三层模型：

- `servers/`：只放 MCP 连接定义（用户/LLM 可编辑）
- `policies/`：只放工具开关（用户/LLM 可编辑）
- 运行状态（部署中/已部署/失败/错误信息）：只在内存，不落盘

## 目录

- `servers/<serverId>.json`
- `policies/<serverId>.json`

## servers 最简格式（推荐）

规则：`servers/<serverId>.json` 文件只放“单个 server 的裸定义”；`<serverId>` 来自文件名，不需要在 JSON 内再包一层同名 key。

```json
{
  "transport": "stdio",
  "command": "npx",
  "args": ["-y", "@upstash/context7-mcp"]
}
```

支持的接入方式最简写法：

1. `stdio` 本地进程
2. `streamable_http` 直连 URL
3. `stdio + mcp-remote` 远程桥接

## policies 格式

成功部署后，系统会为同名 server 自动创建（若不存在）：

```json
{
  "serverId": "context7",
  "enabled": true,
  "tools": [
    { "toolName": "resolve-library-id", "enabled": true },
    { "toolName": "get-library-docs", "enabled": true }
  ]
}
```

规则：

- `tools[].enabled` 是工具级开关（单个工具开/关），并且跨重部署保持
  - 例：某工具 `enabled=false`，重新部署后仍为关闭
- `enabled` 是服务级总开关（master switch），控制该 MCP 是否参与全量重部署
  - 例：`enabled=false` 时，即使 `tools[].enabled=true` 也不会参与全量重部署
- 重新部署时只补新增工具
- 已有工具的 `tools[].enabled` 不会被覆盖
- 因此用户关闭某个工具后，重新部署仍保持关闭

## 说明

- 前端“刷新”是轻量读取（内存状态 + policies），不会自动部署
- 要获得最新工具名，先部署一次
- `refresh_mcp_and_skills`（LLM 工具）会先停止全部 MCP，再按 `policies.enabled=true` 全量重部署
