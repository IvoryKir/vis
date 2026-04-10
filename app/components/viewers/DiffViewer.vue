<template>
  <div class="diff-viewer-root">
    <div v-if="hasFileTabs" class="viewer-tabs">
      <button
        v-for="(tab, i) in diffTabs"
        :key="tab.file"
        type="button"
        class="viewer-tab"
        :class="{ active: i === activeFileIndex }"
        @click="activeFileIndex = i"
      >
        {{ basename(tab.file) }}
      </button>
    </div>

    <div class="viewer-tabs">
      <button
        v-for="mode in primaryModes"
        :key="mode.id"
        type="button"
        class="viewer-tab"
        :class="{ active: mode.id === primaryMode }"
        @click="primaryMode = mode.id"
      >
        {{ mode.label }}
      </button>
    </div>

    <div class="viewer-body">
      <!-- Side-by-side via diff2html -->
      <div
        v-if="primaryMode === 'side-by-side'"
        class="diff2html-wrapper"
        v-html="sideBySideHtml"
      />
      <DiffRenderer
        v-else-if="primaryMode === 'diff'"
        :path="activeFilePath"
        :diff-code="activeBefore"
        :diff-after="activeAfter"
        :diff-patch="activeDiffPatch"
        :gutter-mode="diffGutterMode"
        :lang="lang"
        :theme="theme"
        @rendered="emit('rendered')"
      />
      <ContentViewer
        v-else
        :path="activeFilePath"
        :file-content="isBitmapFile ? undefined : activeText"
        :binary-base64="isBitmapFile ? activeBase64 : undefined"
        :lang="activeLanguage"
        :theme="theme"
        @rendered="emit('rendered')"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, shallowRef, watch } from 'vue';
import { guessLanguageFromPath } from '../ToolWindow/utils';
import DiffRenderer from '../renderers/DiffRenderer.vue';
import ContentViewer from './ContentViewer.vue';
// diff2html and diff are loaded dynamically to avoid babel parser
// issues with namespace imports in vue/compiler-sfc.
// Using shallowRef so Vue tracks when they become available.
const diff2htmlMod = shallowRef<typeof import('diff2html') | null>(null);
const jsDiffMod = shallowRef<typeof import('diff') | null>(null);
import('diff2html').then(m => { diff2htmlMod.value = m; });
import('diff').then(m => { jsDiffMod.value = m; });

type PrimaryMode = 'original' | 'modified' | 'diff' | 'side-by-side';

const props = defineProps<{
  path?: string;
  diffCode?: string;
  diffAfter?: string;
  diffCodeBase64?: string;
  diffAfterBase64?: string;
  diffPatch?: string;
  diffTabs?: Array<{
    file: string;
    before: string;
    after: string;
    beforeBase64?: string;
    afterBase64?: string;
  }>;
  gutterMode?: 'none' | 'double';
  lang?: string;
  theme?: string;
}>();

const emit = defineEmits<{
  (event: 'rendered'): void;
}>();

const activeFileIndex = ref(0);
const primaryMode = ref<PrimaryMode>('side-by-side');

const hasFileTabs = computed(() => !!props.diffTabs && props.diffTabs.length > 1);
const hasBeforeAfter = computed(() => {
  if (props.diffTabs && props.diffTabs.length > 0) return true;
  return props.diffAfter != null;
});

const activeEntry = computed(() => {
  const tabs = props.diffTabs;
  if (!tabs || tabs.length === 0) {
    return {
      file: props.path ?? '',
      before: props.diffCode ?? '',
      after: props.diffAfter ?? '',
      beforeBase64: props.diffCodeBase64,
      afterBase64: props.diffAfterBase64,
    };
  }
  return tabs[activeFileIndex.value] ?? tabs[0];
});

const BITMAP_EXTENSIONS = new Set(['png', 'jpg', 'jpeg', 'gif', 'webp', 'bmp']);

const isBitmapFile = computed(() => {
  const filepath = activeEntry.value.file || props.path || '';
  const ext = filepath.split('.').pop()?.toLowerCase();
  if (!ext) return false;
  return BITMAP_EXTENSIONS.has(ext);
});

const primaryModes = computed(() => {
  if (!hasBeforeAfter.value) return [{ id: 'diff', label: 'Diff' }];
  if (isBitmapFile.value) {
    return [
      { id: 'modified', label: 'Modified' },
      { id: 'original', label: 'Original' },
    ];
  }
  return [
    { id: 'side-by-side', label: 'Side by Side' },
    { id: 'diff', label: 'Unified' },
    { id: 'original', label: 'Original' },
    { id: 'modified', label: 'Modified' },
  ];
});

watch(
  primaryModes,
  (modes) => {
    const valid = modes.some((mode) => mode.id === primaryMode.value);
    if (!valid && modes[0]) primaryMode.value = modes[0].id;
  },
  { immediate: true },
);

const activeFilePath = computed(() => activeEntry.value.file || props.path || '');
const activeBefore = computed(() => activeEntry.value.before ?? '');
const activeAfter = computed(() => activeEntry.value.after ?? '');
const activeDiffPatch = computed(() =>
  props.diffTabs && props.diffTabs.length > 0 ? undefined : props.diffPatch,
);

const activeText = computed(() => {
  if (primaryMode.value === 'original') return activeBefore.value;
  if (primaryMode.value === 'modified') return activeAfter.value;
  return '';
});

const activeBase64 = computed(() => {
  if (primaryMode.value === 'original') return activeEntry.value.beforeBase64;
  if (primaryMode.value === 'modified') return activeEntry.value.afterBase64;
  return undefined;
});

const activeLanguage = computed(() => guessLanguageFromPath(activeFilePath.value));

/** Generate side-by-side diff HTML via diff2html. */
const sideBySideHtml = computed(() => {
  const d2h = diff2htmlMod.value;
  const jsd = jsDiffMod.value;
  if (!d2h || !jsd) return '<div style="padding:12px;color:#94a3b8">Loading diff viewer...</div>';
  const before = activeBefore.value;
  const after = activeAfter.value;
  if (!before && !after) return '';
  const filePath = activeFilePath.value || 'file';
  const patch = jsd.createPatch(filePath, before, after, '', '', { context: 3 });
  return d2h.html(patch, {
    outputFormat: 'side-by-side',
    drawFileList: false,
    matching: 'lines',
    diffStyle: 'word',
    colorScheme: 'dark',
    renderNothingWhenEmpty: false,
  });
});

const diffGutterMode = computed<'none' | 'double'>(() => props.gutterMode ?? 'double');

function basename(filepath: string) {
  return filepath.split('/').pop() ?? filepath;
}
</script>

<style>
@import 'diff2html/bundles/css/diff2html.min.css';
</style>

<style scoped>
.diff-viewer-root {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
}
.viewer-tabs {
  display: flex;
  gap: 0;
  background: var(--bg-surface-2, rgba(26, 29, 36, 0.95));
  border-bottom: 1px solid var(--border-faint, rgba(90, 100, 120, 0.35));
  overflow-x: auto;
  scrollbar-width: none;
  flex-shrink: 0;
}
.viewer-tabs::-webkit-scrollbar { display: none; }
.viewer-tab {
  border: 0;
  background: transparent;
  color: var(--text-muted);
  font-size: 11px;
  font-family: inherit;
  padding: 3px 10px;
  cursor: pointer;
  white-space: nowrap;
  border-bottom: 2px solid transparent;
  transition: color 0.15s, border-color 0.15s;
}
.viewer-tab:hover { color: var(--text-secondary); }
.viewer-tab.active {
  color: var(--text-primary);
  border-bottom-color: var(--accent-primary);
}
.viewer-body {
  flex: 1;
  min-height: 0;
  overflow: auto;
}
.diff2html-wrapper {
  height: 100%;
  overflow: auto;
}
</style>

<style>
/* diff2html dark theme overrides (unscoped so they reach diff2html's DOM) */
.d2h-wrapper { background: var(--bg-base, #0b1320) !important; color: var(--text-primary, #e2e8f0) !important; }
.d2h-file-header { background: var(--bg-surface-2, #1a1d24) !important; border-bottom: 1px solid var(--border-faint, rgba(90,100,120,0.35)) !important; color: var(--text-primary, #e2e8f0) !important; }
.d2h-file-diff { background: var(--bg-base, #0b1320) !important; }
.d2h-code-side-linenumber, .d2h-code-linenumber { background: var(--bg-surface-1, #0f1729) !important; color: var(--text-muted, #64748b) !important; border-color: var(--border-faint, rgba(90,100,120,0.2)) !important; }
.d2h-code-side-line, .d2h-code-line { background: var(--bg-base, #0b1320) !important; color: var(--text-primary, #e2e8f0) !important; }
.d2h-del { background: rgba(239,68,68,0.15) !important; border-color: rgba(239,68,68,0.3) !important; }
.d2h-ins { background: rgba(34,197,94,0.15) !important; border-color: rgba(34,197,94,0.3) !important; }
.d2h-del .d2h-code-side-linenumber, .d2h-del .d2h-code-linenumber { background: rgba(239,68,68,0.2) !important; color: #f87171 !important; }
.d2h-ins .d2h-code-side-linenumber, .d2h-ins .d2h-code-linenumber { background: rgba(34,197,94,0.2) !important; color: #4ade80 !important; }
.d2h-info { background: var(--bg-surface-2, #1a1d24) !important; color: var(--accent-primary, #60a5fa) !important; border-color: var(--border-faint, rgba(90,100,120,0.2)) !important; }
.d2h-cntx { background: var(--bg-base, #0b1320) !important; color: var(--text-muted, #94a3b8) !important; }
.d2h-code-side-emptyplaceholder, .d2h-emptyplaceholder { background: var(--bg-surface-1, #0f1729) !important; border-color: var(--border-faint, rgba(90,100,120,0.2)) !important; }
del.d2h-change { background: rgba(239,68,68,0.35) !important; text-decoration: none !important; border-radius: 2px; }
ins.d2h-change { background: rgba(34,197,94,0.35) !important; text-decoration: none !important; border-radius: 2px; }
.d2h-file-list-wrapper { display: none !important; }
.d2h-code-line-ctn, .d2h-code-side-line { font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, 'Liberation Mono', monospace !important; font-size: 12px !important; }
</style>
