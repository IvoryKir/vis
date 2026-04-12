<template>
  <div class="md-entries-content">
    <div
      v-for="(entry, index) in entries"
      :key="entry.id"
      class="md-entry"
      :class="{ 'md-entry-separator': index > 0 }"
    >
      <MessageViewer :code="entry.text" lang="markdown" :theme="theme" @rendered="handleRendered" />
    </div>
  </div>
</template>

<script setup lang="ts">
import MessageViewer from '../MessageViewer.vue';
import { useFloatingWindow } from '../../composables/useFloatingWindow';

export type MarkdownEntry = {
  id: string;
  text: string;
};

withDefaults(
  defineProps<{
    entries: MarkdownEntry[];
    theme?: string;
  }>(),
  {
    theme: 'github-dark',
  },
);

const floatingWindow = useFloatingWindow();

function handleRendered() {
  floatingWindow.notifyContentChange();
}
</script>

<style scoped>
.md-entries-content {
  min-height: 100%;
}

.md-entry-separator {
  margin-top: 0.4em;
  padding-top: 0.4em;
  border-top: 1px solid color-mix(in srgb, var(--text-muted) 15%, transparent);
}
</style>
