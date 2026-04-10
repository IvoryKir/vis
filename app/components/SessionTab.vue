<script setup lang="ts">
import { computed } from 'vue';
import { Icon } from '@iconify/vue';
import type { OpenSession } from '../composables/useOpenSessions';

const props = defineProps<{
  session: OpenSession;
  isActive: boolean;
}>();

const emit = defineEmits<{
  select: [id: string];
  close: [id: string];
}>();

const tabClasses = computed(() => {
  const base = 'session-tab';
  const active = props.isActive ? 'is-active' : '';
  const error = props.session.status === 'error' ? 'is-error' : '';
  return [base, active, error].filter(Boolean).join(' ');
});

function onClose(e: MouseEvent) {
  e.stopPropagation();
  emit('close', props.session.id);
}

function onMiddleClick(e: MouseEvent) {
  if (e.button === 1) {
    e.preventDefault();
    emit('close', props.session.id);
  }
}
</script>

<template>
  <button
    :class="tabClasses"
    :title="session.title"
    @click="emit('select', session.id)"
    @mousedown="onMiddleClick"
  >
    <!-- Status indicator: spinner for busy, dot for unread, icon for idle -->
    <span v-if="session.status === 'busy'" class="tab-spinner" />
    <span v-else-if="session.status === 'error'" class="tab-error-icon">!</span>
    <span v-else-if="session.hasUnread" class="tab-unread-dot" />

    <span class="tab-title">{{ session.title }}</span>

    <Icon
      icon="lucide:x"
      :width="12"
      class="tab-close"
      @click="onClose"
    />
  </button>
</template>

<style scoped>
.session-tab {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 28px;
  padding: 0 10px;
  border-radius: 6px;
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.04em;
  cursor: pointer;
  flex-shrink: 0;
  max-width: 280px;
  min-width: 120px;
  user-select: none;
  transition: background 0.15s, border-color 0.15s, color 0.15s;
  /* Default state — matches .side-tab from SidePanel.vue */
  background: rgba(15, 23, 42, 0.7);
  border: 1px solid rgba(100, 116, 139, 0.35);
  color: #94a3b8;
}

.session-tab:hover {
  background: rgba(30, 41, 59, 0.92);
  color: #cbd5e1;
}

.session-tab.is-active {
  background: rgba(30, 64, 175, 0.45);
  border-color: rgba(96, 165, 250, 0.6);
  color: #e2e8f0;
}

.session-tab.is-error {
  border-color: rgba(248, 113, 113, 0.5);
  color: #f87171;
}

.session-tab.is-error.is-active {
  background: rgba(248, 113, 113, 0.15);
}

/* Title truncation — matches .selected-title from TopPanel.vue */
.tab-title {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* Busy spinner — reuses app-loading-spin keyframe from App.vue */
.tab-spinner {
  width: 12px;
  height: 12px;
  border: 2px solid transparent;
  border-top-color: #60a5fa;
  border-radius: 50%;
  animation: app-loading-spin 1s linear infinite;
  flex-shrink: 0;
}

/* Error icon */
.tab-error-icon {
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: rgba(248, 113, 113, 0.2);
  color: #f87171;
  font-size: 10px;
  font-weight: 800;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

/* Unread dot */
.tab-unread-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: #60a5fa;
  flex-shrink: 0;
}

/* Close button — hidden by default, visible on hover and always on active */
.tab-close {
  opacity: 0;
  color: #64748b;
  transition: opacity 0.15s, color 0.15s;
  flex-shrink: 0;
  cursor: pointer;
  border-radius: 3px;
}

.session-tab:hover .tab-close {
  opacity: 1;
}

.session-tab.is-active .tab-close {
  opacity: 0.6;
}

.session-tab.is-active:hover .tab-close,
.tab-close:hover {
  opacity: 1;
  color: #e2e8f0;
}
</style>
