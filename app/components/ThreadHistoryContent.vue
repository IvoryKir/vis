<template>
  <div class="history-content">
    <div class="history-list">
      <template v-for="entry in props.entries" :key="entry.key">
        <div v-if="entry.kind === 'message'" class="history-item">
          <div class="history-meta">
            <span class="history-index">💬</span>
            <span v-if="entry.agent" class="history-agent">{{ entry.agent }}</span>
            <span class="history-time">{{ formatMessageTime(entry.time) }}</span>
          </div>
          <div class="history-content-wrapper">
            <MessageViewer
              class="message-viewer-context-history"
              :code="entry.content"
              :lang="'markdown'"
              :theme="theme"
              copy-button
              @rendered="handleRendered"
            />
          </div>
        </div>
        <div
          v-else-if="entry.kind === 'reasoning'"
          class="history-item history-item-reasoning"
          @click="handleReasoningClick(entry.part)"
        >
          <div class="history-meta">
            <span class="history-index">🤔</span>
            <span class="history-reasoning-badge">THOUGHT</span>
            <span class="history-time">{{ formatMessageTime(entry.time) }}</span>
          </div>
        </div>
        <div v-else-if="entry.kind === 'question'" class="history-item history-item-question">
          <div class="history-meta history-meta-question">
            <span class="history-index">❓</span>
            <span class="history-question-badge">QUESTION</span>
            <span class="history-question-status" :class="`is-${entry.status}`">{{
              entry.status
            }}</span>
            <span class="history-time">{{ formatMessageTime(entry.time) }}</span>
          </div>
          <div class="history-question-body">
            <div v-for="(item, qi) in entry.questions" :key="qi" class="history-question-section">
              <div class="history-question-header">{{ item.header }}</div>
              <div class="history-question-text">{{ item.question }}</div>
              <div class="history-question-options">
                <div
                  v-for="(opt, oi) in item.options"
                  :key="oi"
                  class="history-question-option"
                  :class="{ 'is-selected': isOptionSelected(entry, qi, opt.label) }"
                >
                  <span class="option-check">{{
                    isOptionSelected(entry, qi, opt.label) ? '☑' : '☐'
                  }}</span>
                  <span class="option-label">{{ opt.label }}</span>
                  <span v-if="opt.description" class="option-desc">{{ opt.description }}</span>
                </div>
              </div>
              <div v-if="getCustomAnswer(entry, qi)" class="history-question-custom">
                {{ getCustomAnswer(entry, qi) }}
              </div>
            </div>
          </div>
        </div>
        <div
          v-else
          class="history-item history-item-tool"
          :style="{ '--tool-color': toolHeaderColor(entry.part.tool) }"
          @click="handleToolClick(entry.part)"
        >
          <div class="history-meta">
            <span class="history-index">🔧</span>
            <span class="history-tool-badge" :class="`history-tool-${entry.part.tool}`">{{
              toolBadgeLabel(entry.part.tool)
            }}</span>
            <span class="history-tool-status" :class="`is-${toolStatusLabel(entry.part)}`">{{
              toolStatusLabel(entry.part)
            }}</span>
            <span class="history-time">{{ formatMessageTime(entry.time) }}</span>
          </div>
          <div class="history-tool-content">{{ toolSummary(entry.part) }}</div>
        </div>
      </template>
    </div>
  </div>
</template>

<script setup lang="ts">
import MessageViewer from './MessageViewer.vue';
import { useFloatingWindowOptional } from '../composables/useFloatingWindow';
import type { QuestionInfo, ReasoningPart, ToolPart } from '../types/sse';

type QuestionHistoryEntry = {
  key: string;
  kind: 'question';
  questions: QuestionInfo[];
  status: 'pending' | 'replied' | 'rejected';
  answers?: string[][];
  time: number;
};

type HistoryEntry =
  | { key: string; kind: 'message'; content: string; time: number; agent?: string }
  | { key: string; kind: 'tool'; part: ToolPart; time: number }
  | { key: string; kind: 'reasoning'; part: ReasoningPart; time: number }
  | QuestionHistoryEntry;

const props = withDefaults(
  defineProps<{
    entries: HistoryEntry[];
    theme?: string;
    onToolClick?: (part: ToolPart) => void;
    onReasoningClick?: (part: ReasoningPart) => void;
  }>(),
  {
    theme: 'github-dark',
  },
);

const floatingWindow = useFloatingWindowOptional();

function handleRendered() {
  floatingWindow?.notifyContentChange();
}

function handleToolClick(part: ToolPart) {
  props.onToolClick?.(part);
}

function handleReasoningClick(part: ReasoningPart) {
  props.onReasoningClick?.(part);
}

function isOptionSelected(
  entry: QuestionHistoryEntry,
  questionIndex: number,
  label: string,
): boolean {
  if (entry.status !== 'replied' || !entry.answers) return false;
  const answer = entry.answers[questionIndex];
  return Array.isArray(answer) && answer.includes(label);
}

function getCustomAnswer(entry: QuestionHistoryEntry, questionIndex: number): string {
  if (entry.status !== 'replied' || !entry.answers) return '';
  const answer = entry.answers[questionIndex];
  if (!Array.isArray(answer)) return '';
  const question = entry.questions[questionIndex];
  if (!question) return '';
  const optionLabels = new Set(question.options.map((o) => o.label));
  return answer.filter((v) => !optionLabels.has(v)).join(', ');
}

function toolBadgeLabel(tool: string): string {
  switch (tool) {
    case 'bash':
      return 'SHELL';
    case 'write':
      return 'WRITE';
    case 'edit':
      return 'EDIT';
    case 'multiedit':
      return 'EDIT';
    case 'apply_patch':
      return 'PATCH';
    default:
      return tool.toUpperCase();
  }
}

function toolSummary(part: ToolPart): string {
  const input = part.state.input;
  switch (part.tool) {
    case 'bash': {
      const cmd = typeof input?.command === 'string' ? input.command.trim() : '';
      return cmd ? `$ ${cmd.split('\n')[0].slice(0, 120)}` : '$ ...';
    }
    case 'write': {
      const path = typeof input?.filePath === 'string' ? input.filePath : '';
      return path || 'write';
    }
    case 'edit': {
      const path = typeof input?.filePath === 'string' ? input.filePath : '';
      return path || 'edit';
    }
    case 'multiedit': {
      const path = typeof input?.filePath === 'string' ? input.filePath : '';
      return path || 'multiedit';
    }
    case 'apply_patch': {
      const state = part.state;
      const metadata =
        state.status === 'completed' || state.status === 'error' || state.status === 'running'
          ? state.metadata
          : undefined;
      const files = Array.isArray(metadata?.files) ? metadata.files : [];
      const paths = files
        .map((f: unknown) => {
          if (!f || typeof f !== 'object') return null;
          const r = f as Record<string, unknown>;
          return typeof r.relativePath === 'string'
            ? r.relativePath
            : typeof r.filePath === 'string'
              ? r.filePath
              : typeof r.file === 'string'
                ? r.file
                : null;
        })
        .filter(Boolean) as string[];
      return paths.length > 0 ? paths.join(', ') : 'patch';
    }
    default:
      return part.tool;
  }
}

function toolStatusLabel(part: ToolPart): string {
  return part.state.status;
}

function toolHeaderColor(tool: string): string {
  switch (tool) {
    case 'bash':
      return 'var(--color-purple)';
    case 'edit':
    case 'multiedit':
    case 'apply_patch':
      return 'var(--color-orange)';
    case 'write':
      return 'var(--color-orange)';
    default:
      return 'var(--text-faint)';
  }
}

function formatMessageTime(value?: number) {
  if (typeof value !== 'number') return '';
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return '';
  const year = String(date.getFullYear());
  const month = String(date.getMonth() + 1).padStart(2, '0');
  const day = String(date.getDate()).padStart(2, '0');
  const hours = String(date.getHours()).padStart(2, '0');
  const minutes = String(date.getMinutes()).padStart(2, '0');
  return `${year}-${month}-${day} ${hours}:${minutes}`;
}
</script>

<style scoped>
.history-content {
}

.history-list {
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.history-item {
  border: 1px solid var(--border-color);
  border-radius: 8px;
  background: var(--bg-surface-0);
}

.history-meta {
  padding: 6px 10px;
  background: color-mix(in srgb, var(--accent-primary) 12%, var(--bg-surface-2));
  border: 1px solid var(--border-faint);
  border-radius: 7px 7px 0 0;
  display: flex;
  gap: 8px;
  align-items: center;
  font-size: 11px;
  color: var(--text-muted);
}

.history-index {
  font-weight: 600;
  color: var(--text-primary);
}

.history-time {
  margin-left: auto;
}

.history-agent {
  padding: 2px 6px;
  background: var(--bg-surface-4);
  border-radius: 4px;
  color: var(--text-secondary);
}

.history-content-wrapper {
  padding: 10px;
  font-size: var(--vis-font-size, 13px);
  line-height: 1.4;
}

.history-item-reasoning {
  cursor: pointer;
  border: 1px solid var(--border-faint));
  transition:
    border-color 0.15s,
    background 0.15s;
}

.history-item-reasoning:hover {
  border: 1px solid var(--border-faint));
  background: color-mix(in srgb, var(--color-purple) 6%, var(--bg-surface-0));
}

.history-item-reasoning .history-meta {
  background: color-mix(in srgb, var(--color-purple) 18%, color-mix(in srgb, var(--bg-surface-2) 95%, transparent));
  border-bottom: none;
}

.history-reasoning-badge {
  padding: 1px 5px;
  border-radius: 3px;
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.5px;
  background: rgba(88, 28, 135, 0.5);
  color: #d8b4fe;
}

/* Question entry */
.history-item-question {
  border: 1px solid var(--border-faint));
}

.history-meta-question {
  background: color-mix(in srgb, var(--color-success) 18%, color-mix(in srgb, var(--bg-surface-2) 95%, transparent));
  border: 1px solid var(--border-faint));
}

.history-question-badge {
  padding: 1px 5px;
  border-radius: 3px;
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.5px;
  background: rgba(6, 78, 59, 0.6);
  color: var(--color-success);
}

.history-question-status {
  font-size: 10px;
  color: var(--text-faint);
}

.history-question-status.is-replied {
  color: var(--color-success);
}

.history-question-status.is-rejected {
  color: var(--color-error);
}

.history-question-status.is-pending {
  color: var(--color-warning-light);
}

.history-question-body {
  padding: 8px 10px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.history-question-section {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.history-question-header {
  font-size: 11px;
  font-weight: 600;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.3px;
}

.history-question-text {
  font-size: var(--vis-font-size, 13px);
  line-height: 1.4;
  color: var(--text-primary);
}

.history-question-options {
  display: flex;
  flex-direction: column;
  gap: 3px;
  margin-top: 4px;
}

.history-question-option {
  display: flex;
  gap: 6px;
  align-items: baseline;
  font-size: 12px;
  line-height: 1.4;
  color: var(--text-muted);
  padding: 2px 4px;
  border-radius: 3px;
}

.history-question-option.is-selected {
  color: var(--text-primary);
  background: color-mix(in srgb, var(--color-success) 10%, transparent);
}

.option-check {
  flex-shrink: 0;
  font-size: var(--vis-font-size, 13px);
}

.option-label {
  font-weight: 500;
}

.option-desc {
  color: var(--text-faint);
}

.history-question-option.is-selected .option-desc {
  color: var(--text-muted);
}

.history-question-custom {
  margin-top: 4px;
  padding: 4px 8px;
  background: color-mix(in srgb, var(--color-success) 8%, transparent);
  border-left: 2px solid var(--color-success);
  border-radius: 2px;
  font-size: 12px;
  line-height: 1.4;
  color: var(--text-primary);
}

.history-item-tool {
  cursor: pointer;
  border: 1px solid var(--border-faint));
  transition:
    border-color 0.15s,
    background 0.15s;
}

.history-item-tool:hover {
  border: 1px solid var(--border-faint));
  background: color-mix(in srgb, var(--tool-color, var(--text-faint)) 6%, var(--bg-surface-0));
}

.history-item-tool .history-meta {
  background: color-mix(in srgb, var(--tool-color, var(--text-faint)) 18%, color-mix(in srgb, var(--bg-surface-2) 95%, transparent));
  border: 1px solid var(--border-faint));
}

.history-tool-badge {
  padding: 1px 5px;
  border-radius: 3px;
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.5px;
  color: var(--text-primary);
  background: var(--bg-surface-5);
}

.history-tool-badge.history-tool-bash {
  background: rgba(22, 78, 99, 0.7);
  color: #67e8f9;
}

.history-tool-badge.history-tool-write {
  background: rgba(21, 94, 117, 0.5);
  color: #a5f3fc;
}

.history-tool-badge.history-tool-edit,
.history-tool-badge.history-tool-multiedit {
  background: color-mix(in srgb, var(--accent-dark) 50%, transparent);
  color: var(--color-info);
}

.history-tool-badge.history-tool-apply_patch {
  background: color-mix(in srgb, var(--accent-dark) 50%, transparent);
  color: var(--color-info);
}

.history-tool-status {
  font-size: 10px;
  color: var(--text-faint);
}

.history-tool-status.is-completed {
  color: var(--color-success);
}

.history-tool-status.is-error {
  color: var(--color-error);
}

.history-tool-status.is-running {
  color: var(--color-warning-light);
}

.history-tool-content {
  padding: 6px 10px;
  font-family: inherit;
  font-size: 12px;
  line-height: 1.4;
  color: var(--text-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
