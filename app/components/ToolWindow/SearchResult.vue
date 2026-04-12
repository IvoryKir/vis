<script setup lang="ts">
import CodeContent from '../CodeContent.vue';

defineProps<{
  html: string;
  status?: string;
  pattern?: string;
  path?: string;
  include?: string;
  /** CodeContent variant: 'code' for grep, 'term' for glob */
  variant?: 'code' | 'term' | 'plain';
}>();
</script>

<template>
  <div v-if="status === 'running'" class="tool-placeholder">
    <div v-if="pattern">Pattern: {{ pattern }}</div>
    <div v-if="path">Path: {{ path }}</div>
    <div v-if="include">Include: {{ include }}</div>
    <div v-if="!pattern && !path && !include">Running...</div>
  </div>
  <CodeContent v-else :html="html" :variant="variant || 'code'" />
</template>

<style scoped>
.tool-placeholder {
  font-family: inherit;
  font-size: var(--vis-font-size, 13px);
  line-height: 1.5;
  color: var(--text-muted);
  padding: 4px;
  white-space: pre-wrap;
}
</style>
