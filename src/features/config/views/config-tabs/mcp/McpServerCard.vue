<template>
  <div class="card bg-base-100 border border-base-300">
    <div class="card-body p-3 gap-2">
      <div class="flex items-center gap-2">
        <input v-model.trim="draft.name" class="input input-bordered input-xs flex-1" placeholder="显示名称" />
        <button class="btn btn-xs" type="button" :disabled="disabled" @click="emitSave">保存</button>
        <button class="btn btn-xs btn-ghost" type="button" :disabled="disabled" @click="$emit('remove', draft.id)">删除</button>
      </div>

      <details class="collapse bg-base-200 border-base-300 border">
        <summary class="collapse-title min-h-0 py-1 px-2 text-xs font-medium">MCP 配置 JSON</summary>
        <div class="collapse-content py-1 px-2">
          <textarea
            v-model="draft.definitionJson"
            class="textarea textarea-bordered textarea-xs font-mono min-h-40 w-full"
            placeholder='输入 MCP JSON，例如 {"transport":"stdio","command":"npx","args":["-y","@upstash/context7-mcp"]}'
          ></textarea>
        </div>
      </details>

      <div class="flex items-center justify-between gap-2">
        <div class="text-[11px] opacity-70 break-all">
          状态: {{ draft.lastStatus || "-" }}
          <span v-if="draft.lastError" class="text-error"> | {{ draft.lastError }}</span>
        </div>
        <div class="flex items-center gap-2">
          <button class="btn btn-xs" type="button" :disabled="disabled" @click="emitValidate">校验</button>
          <button
            class="btn btn-xs"
            :class="draft.enabled ? 'btn-warning' : 'btn-success'"
            type="button"
            :disabled="disabled"
            @click="emitToggleDeploy"
          >
            {{ draft.enabled ? "停止" : "部署" }}
          </button>
        </div>
      </div>

      <McpToolList
        :tools="draft.toolItems"
        :elapsed-ms="draft.lastElapsedMs"
        :disabled="disabled"
        @toggle-tool="(payload) => $emit('toggleTool', { serverId: draft.id, ...payload })"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { reactive, watch } from "vue";
import type { McpServerConfig, McpToolDescriptor } from "../../../../../types/app";
import McpToolList from "./McpToolList.vue";

type McpServerView = McpServerConfig & {
  toolItems: McpToolDescriptor[];
  lastElapsedMs: number;
};

const props = defineProps<{
  server: McpServerView;
  disabled?: boolean;
}>();

const emit = defineEmits<{
  (e: "save", server: McpServerView): void;
  (e: "remove", serverId: string): void;
  (e: "validate", server: McpServerView): void;
  (e: "toggleDeploy", server: McpServerView): void;
  (e: "toggleTool", payload: { serverId: string; toolName: string; enabled: boolean }): void;
}>();

const draft = reactive<McpServerView>({ ...props.server });

watch(
  () => props.server,
  (next) => {
    Object.assign(draft, next);
  },
  { deep: true },
);

function emitSave() {
  emit("save", { ...draft });
}

function emitValidate() {
  emit("validate", { ...draft });
}

function emitToggleDeploy() {
  emit("toggleDeploy", { ...draft });
}
</script>
