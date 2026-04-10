<template>
  <aside class="todo-panel" :class="{ 'is-collapsed': collapsed }">
    <button
      type="button"
      class="todo-toggle"
      :aria-expanded="!collapsed"
      :aria-label="collapsed ? 'Expand TODO panel' : 'Collapse TODO panel'"
      @click="emit('toggle-collapse')"
    >
      {{ collapsed ? '<' : '>' }}
    </button>
    <div v-if="!collapsed" class="todo-body">
      <div class="todo-header">
        <div class="todo-title">TODO</div>
        <div class="todo-count">{{ totalCount }}</div>
      </div>
      <div v-if="sessions.length === 0" class="todo-empty">No todos yet.</div>
      <div v-else class="todo-groups">
        <section v-for="session in sessions" :key="session.sessionId" class="todo-group">
          <header class="todo-group-header">
            <span class="todo-group-title">{{ session.title }}</span>
            <span v-if="session.isSubagent" class="todo-badge">subagent</span>
          </header>
          <div v-if="session.error" class="todo-error">{{ session.error }}</div>
          <ul v-else class="todo-list">
            <li
              v-for="(todo, index) in session.todos"
              :key="index"
              class="todo-item"
              :class="`is-${todo.status}`"
            >
              <span class="todo-status" :title="todo.status">{{ statusIcon(todo.status) }}</span>
              <span class="todo-text">{{ todo.content }}</span>
              <span class="todo-priority" :class="`is-${todo.priority}`">{{ todo.priority }}</span>
            </li>
          </ul>
        </section>
      </div>
    </div>
  </aside>
</template>

<script setup lang="ts">
type TodoEntry = {
  content: string;
  status: string;
  priority: string;
};

type TodoSession = {
  sessionId: string;
  title: string;
  isSubagent: boolean;
  todos: TodoEntry[];
  loading: boolean;
  error: string | undefined;
};

defineProps<{
  collapsed: boolean;
  sessions: TodoSession[];
  totalCount: number;
}>();

const emit = defineEmits<{
  (event: 'toggle-collapse'): void;
}>();

function statusIcon(status: string) {
  if (status === 'completed') return '✓';
  if (status === 'in_progress') return '◐';
  if (status === 'cancelled') return '✕';
  return '○';
}
</script>

<style scoped>
.todo-panel {
  position: relative;
  width: 100%;
  height: 100%;
  min-height: 0;
  display: flex;
  flex-direction: row;
  border: 1px solid var(--border-color);
  border-radius: 12px;
  background-clip: padding-box;
  background: rgba(12, 18, 30, 0.95);
  box-shadow: 0 10px 24px color-mix(in srgb, var(--bg-surface-0) 35%, transparent);
  overflow: hidden;
}

.todo-toggle {
  width: 30px;
  border: 0;
  border-right: 1px solid color-mix(in srgb, var(--text-faint) 45%, transparent);
  background: color-mix(in srgb, var(--bg-surface-4) 92%, transparent);
  color: var(--text-secondary);
  cursor: pointer;
  font-size: 14px;
}

.todo-toggle:hover {
  background: color-mix(in srgb, var(--border-color) 95%, transparent);
}

.todo-body {
  flex: 1;
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.todo-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 10px 8px;
  border-bottom: 1px solid color-mix(in srgb, var(--text-faint) 28%, transparent);
}

.todo-title {
  font-size: 12px;
  font-weight: 700;
  letter-spacing: 0.08em;
  color: var(--text-primary);
}

.todo-count {
  font-size: 11px;
  color: var(--text-muted);
}

.todo-empty {
  margin: auto;
  color: color-mix(in srgb, var(--text-muted) 90%, transparent);
  font-size: 12px;
}

.todo-groups {
  flex: 1;
  min-height: 0;
  overflow: auto;
  padding: 8px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.todo-group {
  border: 1px solid color-mix(in srgb, var(--border-hover) 55%, transparent);
  border-radius: 8px;
  background: color-mix(in srgb, var(--bg-surface-2) 60%, transparent);
}

.todo-group-header {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 7px 8px;
  border-bottom: 1px solid color-mix(in srgb, var(--border-hover) 42%, transparent);
}

.todo-group-title {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 12px;
  font-weight: 600;
  color: var(--text-primary);
}

.todo-badge {
  margin-left: auto;
  padding: 1px 5px;
  border-radius: 999px;
  border: 1px solid rgba(59, 130, 246, 0.5);
  color: var(--color-info);
  background: rgba(30, 64, 175, 0.25);
  font-size: 10px;
}

.todo-error {
  padding: 8px;
  color: var(--color-error-light);
  font-size: 11px;
}

.todo-list {
  list-style: none;
  margin: 0;
  padding: 6px 8px 8px;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.todo-item {
  display: flex;
  align-items: flex-start;
  gap: 6px;
  font-size: 12px;
  color: var(--color-info);
}

.todo-status {
  width: 14px;
  text-align: center;
  color: var(--text-primary);
}

.todo-item.is-completed .todo-status {
  color: var(--color-success);
}

.todo-item.is-in_progress .todo-status {
  color: var(--color-warning);
}

.todo-item.is-cancelled .todo-status {
  color: var(--color-error-light);
}

.todo-text {
  flex: 1;
  min-width: 0;
  overflow-wrap: anywhere;
}

.todo-priority {
  flex: 0 0 auto;
  text-transform: uppercase;
  font-size: 9px;
  letter-spacing: 0.07em;
  color: var(--text-secondary);
  border: 1px solid color-mix(in srgb, var(--text-muted) 45%, transparent);
  border-radius: 999px;
  padding: 2px 5px;
}

.todo-priority.is-high {
  color: var(--color-error-light);
  border-color: rgba(248, 113, 113, 0.6);
}

.todo-priority.is-medium {
  color: var(--color-warning-light);
  border-color: rgba(250, 204, 21, 0.6);
}

.todo-priority.is-low {
  color: var(--color-success);
  border-color: rgba(74, 222, 128, 0.6);
}

.todo-panel.is-collapsed {
  border-color: color-mix(in srgb, var(--text-faint) 45%, transparent);
}

.todo-panel.is-collapsed .todo-toggle {
  width: 100%;
  border-right: 0;
}
</style>
