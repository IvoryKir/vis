import { computed, ref, shallowReactive } from 'vue';

export type SessionTabStatus = 'idle' | 'busy' | 'error';

export interface OpenSession {
  /** Unique session ID from opencode. */
  id: string;
  /** Project ID this session belongs to. */
  projectId: string;
  /** Human-readable title (session description or slug). */
  title: string;
  /** Current agent status for the tab indicator. */
  status: SessionTabStatus;
  /** Whether the tab has messages the user hasn't seen yet. */
  hasUnread: boolean;
}

/**
 * Manages the set of currently open session tabs.
 *
 * This is a thin state layer — it only tracks which sessions are "open"
 * (visible as tabs) and which one is active. The actual message data for
 * each tab lives in `useMessages(sessionId)` instances managed by the
 * consumer (App.vue).
 *
 * Module-level singleton so every component sees the same tab state.
 */
const tabs = shallowReactive<OpenSession[]>([]);
const activeIndex = ref(0);

const activeTab = computed(() => tabs[activeIndex.value] ?? null);
const activeSessionId = computed(() => activeTab.value?.id ?? '');
const activeProjectId = computed(() => activeTab.value?.projectId ?? '');

function findIndex(sessionId: string): number {
  return tabs.findIndex((t) => t.id === sessionId);
}

/**
 * Open a session as a new tab (or switch to it if already open).
 * Returns the index of the tab.
 */
function open(session: Pick<OpenSession, 'id' | 'projectId' | 'title'>): number {
  const existing = findIndex(session.id);
  if (existing >= 0) {
    activeIndex.value = existing;
    return existing;
  }
  const tab: OpenSession = {
    id: session.id,
    projectId: session.projectId,
    title: session.title || session.id.slice(0, 12),
    status: 'idle',
    hasUnread: false,
  };
  tabs.push(tab);
  activeIndex.value = tabs.length - 1;
  return activeIndex.value;
}

/**
 * Close a tab. If the active tab is closed, switch to the nearest neighbor.
 */
function close(sessionId: string) {
  const idx = findIndex(sessionId);
  if (idx < 0) return;
  tabs.splice(idx, 1);
  if (tabs.length === 0) {
    activeIndex.value = 0;
    return;
  }
  if (activeIndex.value >= tabs.length) {
    activeIndex.value = tabs.length - 1;
  } else if (activeIndex.value > idx) {
    activeIndex.value -= 1;
  }
}

/** Switch to an already-open tab by session ID. */
function activate(sessionId: string) {
  const idx = findIndex(sessionId);
  if (idx >= 0) activeIndex.value = idx;
}

/** Update the status indicator for a specific tab. */
function setStatus(sessionId: string, status: SessionTabStatus) {
  const tab = tabs.find((t) => t.id === sessionId);
  if (tab) tab.status = status;
}

/** Mark a tab as having unread messages (or clear the flag). */
function setUnread(sessionId: string, hasUnread: boolean) {
  const tab = tabs.find((t) => t.id === sessionId);
  if (tab) tab.hasUnread = hasUnread;
}

/** Update the title of a tab (e.g. when session description changes). */
function setTitle(sessionId: string, title: string) {
  const tab = tabs.find((t) => t.id === sessionId);
  if (tab) tab.title = title;
}

/** Reorder tabs (for future drag-and-drop). */
function reorder(fromIndex: number, toIndex: number) {
  if (fromIndex < 0 || fromIndex >= tabs.length) return;
  if (toIndex < 0 || toIndex >= tabs.length) return;
  const [moved] = tabs.splice(fromIndex, 1);
  tabs.splice(toIndex, 0, moved);
  // Keep active tab the same session after reorder.
  const activeId = activeTab.value?.id;
  if (activeId) {
    const newIdx = findIndex(activeId);
    if (newIdx >= 0) activeIndex.value = newIdx;
  }
}

export function useOpenSessions() {
  return {
    tabs,
    activeIndex,
    activeTab,
    activeSessionId,
    activeProjectId,
    open,
    close,
    activate,
    setStatus,
    setUnread,
    setTitle,
    reorder,
  };
}
