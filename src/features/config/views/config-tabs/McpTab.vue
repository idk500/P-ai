<template>
  <div class="space-y-3">
    <div class="flex items-center justify-between">
      <div class="text-xs opacity-70">MCP Server 列表</div>
      <div class="flex items-center gap-2">
        <button class="btn btn-xs" type="button" @click="reloadServers" :disabled="loading">刷新</button>
        <button class="btn btn-xs btn-primary" type="button" @click="addServer">新增</button>
      </div>
    </div>

    <div v-if="loading" class="text-xs opacity-70">加载中...</div>

    <McpServerCard
      v-for="server in servers"
      :key="server.id"
      :server="server"
      :disabled="loading"
      @save="saveServer"
      @remove="removeServer"
      @validate="validateDefinition"
      @toggle-deploy="toggleDeploy"
      @toggle-tool="onToggleTool"
    />

    <div v-if="statusText" class="text-xs" :class="statusError ? 'text-error' : 'opacity-70'">
      {{ statusText }}
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { invokeTauri } from "../../../../services/tauri-api";
import type {
  McpDefinitionValidateResult,
  McpListServerToolsResult,
  McpServerConfig,
  McpToolDescriptor,
} from "../../../../types/app";
import { toErrorMessage } from "../../../../utils/error";
import McpServerCard from "./mcp/McpServerCard.vue";

type McpServerView = McpServerConfig & {
  toolItems: McpToolDescriptor[];
  lastElapsedMs: number;
};

const loading = ref(false);
const statusText = ref("");
const statusError = ref(false);
const servers = ref<McpServerView[]>([]);

function setStatus(text: string, isError = false) {
  statusText.value = text;
  statusError.value = isError;
}

function toView(server: McpServerConfig): McpServerView {
  return {
    ...server,
    toolItems: [],
    lastElapsedMs: 0,
  };
}

function upsertServer(local: McpServerView) {
  const idx = servers.value.findIndex((s) => s.id === local.id);
  if (idx >= 0) {
    servers.value[idx] = {
      ...servers.value[idx],
      ...local,
    };
    return;
  }
  servers.value.unshift(local);
}

async function reloadServers() {
  loading.value = true;
  try {
    const list = await invokeTauri<McpServerConfig[]>("mcp_list_servers");
    servers.value = list.map(toView);
    const enabledServers = servers.value.filter((s) => s.enabled);
    if (enabledServers.length > 0) {
      const results = await Promise.allSettled(
        enabledServers.map((server) =>
          invokeTauri<McpListServerToolsResult>("mcp_list_server_tools", {
            input: { serverId: server.id },
          }),
        ),
      );
      for (let i = 0; i < enabledServers.length; i++) {
        const target = enabledServers[i];
        const result = results[i];
        if (result.status !== "fulfilled") continue;
        target.toolItems = result.value.tools;
        target.lastElapsedMs = result.value.elapsedMs;
      }
    }
    setStatus(`已加载 ${servers.value.length} 个 MCP 服务`);
  } catch (error) {
    setStatus(`加载 MCP 服务失败: ${toErrorMessage(error)}`, true);
  } finally {
    loading.value = false;
  }
}

async function saveServer(server: McpServerView) {
  loading.value = true;
  try {
    const saved = await _saveServerCore(server);
    upsertServer({ ...server, ...saved });
    setStatus(`已保存: ${saved.name}`);
  } catch (error) {
    setStatus(`保存失败: ${toErrorMessage(error)}`, true);
  } finally {
    loading.value = false;
  }
}

function addServer() {
  const seed = Date.now();
  servers.value.unshift({
    id: `mcp-${seed}`,
    name: `MCP ${servers.value.length + 1}`,
    enabled: false,
    definitionJson: '{\n  "transport": "stdio",\n  "command": "npx",\n  "args": ["-y", "@upstash/context7-mcp"]\n}',
    toolPolicies: [],
    lastStatus: "",
    lastError: "",
    updatedAt: "",
    toolItems: [],
    lastElapsedMs: 0,
  });
}

async function removeServer(serverId: string) {
  loading.value = true;
  try {
    await invokeTauri<boolean>("mcp_remove_server", {
      input: { serverId },
    });
    servers.value = servers.value.filter((s) => s.id !== serverId);
    setStatus(`已删除: ${serverId}`);
  } catch (error) {
    setStatus(`删除失败: ${toErrorMessage(error)}`, true);
  } finally {
    loading.value = false;
  }
}

async function validateDefinition(server: McpServerView) {
  loading.value = true;
  try {
    const result = await invokeTauri<McpDefinitionValidateResult>("mcp_validate_definition", {
      input: { definitionJson: server.definitionJson },
    });
    if (!result.ok) {
      const detailText = Array.isArray(result.details) && result.details.length > 0
        ? ` | ${result.details.join(" ; ")}`
        : "";
      const codeText = result.errorCode ? ` [${result.errorCode}]` : "";
      setStatus(`校验失败${codeText}: ${result.message}${detailText}`, true);
      return;
    }
    if (result.migratedDefinitionJson) {
      server.definitionJson = result.migratedDefinitionJson;
    }
    setStatus(`校验通过: transport=${result.transport || "-"}`);
  } catch (error) {
    setStatus(`校验失败: ${toErrorMessage(error)}`, true);
  } finally {
    loading.value = false;
  }
}

async function toggleDeploy(server: McpServerView) {
  loading.value = true;
  try {
    if (server.enabled) {
      const updated = await invokeTauri<McpServerConfig>("mcp_undeploy_server", {
        input: { serverId: server.id },
      });
      upsertServer({ ...server, ...updated });
      setStatus(`已停止: ${server.name}`);
      return;
    }

    const savedBeforeDeploy = await _saveServerCore(server);
    upsertServer({ ...server, ...savedBeforeDeploy });
    const deployResult = await invokeTauri<McpListServerToolsResult>("mcp_deploy_server", {
      input: { serverId: server.id },
    });
    const saved = await invokeTauri<McpServerConfig[]>("mcp_list_servers");
    const latest = saved.find((s) => s.id === server.id);
    if (latest) {
      upsertServer({
        ...server,
        ...latest,
        toolItems: deployResult.tools,
        lastElapsedMs: deployResult.elapsedMs,
      });
    }
    setStatus(`部署成功: ${server.name}（tools=${deployResult.tools.length}）`);
  } catch (error) {
    setStatus(`部署失败: ${toErrorMessage(error)}`, true);
  } finally {
    loading.value = false;
  }
}

async function _saveServerCore(server: McpServerView): Promise<McpServerConfig> {
  return invokeTauri<McpServerConfig>("mcp_save_server", {
    input: {
      id: server.id,
      name: server.name,
      enabled: server.enabled,
      definitionJson: server.definitionJson,
    },
  });
}

async function onToggleTool(payload: { serverId: string; toolName: string; enabled: boolean }) {
  loading.value = true;
  try {
    await invokeTauri<McpServerConfig>("mcp_set_tool_enabled", {
      input: payload,
    });
    const server = servers.value.find((s) => s.id === payload.serverId);
    if (server) {
      const tool = server.toolItems.find((t) => t.toolName === payload.toolName);
      if (tool) {
        tool.enabled = payload.enabled;
      }
    }
    setStatus(`工具已${payload.enabled ? "启用" : "禁用"}: ${payload.toolName}`);
  } catch (error) {
    setStatus(`工具开关失败: ${toErrorMessage(error)}`, true);
  } finally {
    loading.value = false;
  }
}

void reloadServers();
</script>
