<script setup lang="ts">
import { useOpenSessions } from '../composables/useOpenSessions';
import SessionTab from './SessionTab.vue';

const { tabs, activeSessionId } = useOpenSessions();

const emit = defineEmits<{
  select: [id: string];
  close: [id: string];
}>();
</script>

<template>
  <div class="session-tabs-row">
    <div class="session-tabs-scroll">
      <SessionTab
        v-for="tab in tabs"
        :key="tab.id"
        :session="tab"
        :is-active="tab.id === activeSessionId"
        @select="emit('select', $event)"
        @close="emit('close', $event)"
      />
    </div>
  </div>
</template>

<style scoped>
.session-tabs-row {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 6px 12px 4px;
  border-top: 1px solid rgba(255, 255, 255, 0.04);
}

.session-tabs-scroll {
  display: flex;
  align-items: center;
  gap: 4px;
  overflow-x: auto;
  flex: 1;
  min-width: 0;
  scrollbar-width: none;
  -ms-overflow-style: none;
}
.session-tabs-scroll::-webkit-scrollbar {
  display: none;
}
</style>
