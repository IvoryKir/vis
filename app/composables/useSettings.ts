import { ref, watch, computed } from 'vue';
import { StorageKeys, storageGet, storageKey, storageSet } from '../utils/storageKeys';

export const FONT_OPTIONS = [
  { id: 'jetbrains-mono', name: 'JetBrains Mono', family: "'JetBrains Mono'", hasLigatures: true },
  { id: 'fira-code', name: 'Fira Code', family: "'Fira Code'", hasLigatures: true },
  { id: 'ibm-plex-mono', name: 'IBM Plex Mono', family: "'IBM Plex Mono'", hasLigatures: false },
  { id: 'roboto-mono', name: 'Roboto Mono', family: "'Roboto Mono'", hasLigatures: false },
  { id: 'source-code-pro', name: 'Source Code Pro', family: "'Source Code Pro'", hasLigatures: false },
] as const;

export type FontId = (typeof FONT_OPTIONS)[number]['id'];

export const FONT_SIZE_MIN = 10;
export const FONT_SIZE_MAX = 20;
export const FONT_SIZE_DEFAULT = 13;

const DEFAULT_FONT: FontId = 'jetbrains-mono';

const enterToSend = ref(storageGet(StorageKeys.settings.enterToSend) === 'true');
const suppressAutoWindows = ref(storageGet(StorageKeys.settings.suppressAutoWindows) === 'true');
const fontFamily = ref<FontId>((storageGet(StorageKeys.settings.fontFamily) as FontId) || DEFAULT_FONT);
const fontSize = ref<number>(Number(storageGet(StorageKeys.settings.fontSize)) || FONT_SIZE_DEFAULT);

/* ── derived CSS values ── */
const fontFamilyCSS = computed(() => {
  const font = FONT_OPTIONS.find((f) => f.id === fontFamily.value) || FONT_OPTIONS[0];
  return `${font.family}, ui-monospace, SFMono-Regular, Menlo, Consolas, 'Liberation Mono', monospace`;
});

/* ── persist ── */
watch(enterToSend, (value) => {
  storageSet(StorageKeys.settings.enterToSend, String(value));
});

watch(suppressAutoWindows, (value) => {
  storageSet(StorageKeys.settings.suppressAutoWindows, String(value));
});

watch(fontFamily, (value) => {
  storageSet(StorageKeys.settings.fontFamily, value);
  applyFontToDocument();
});

watch(fontSize, (value) => {
  storageSet(StorageKeys.settings.fontSize, String(value));
  applyFontToDocument();
});

/* ── apply to :root ── */
function applyFontToDocument() {
  if (typeof document === 'undefined') return;
  const root = document.documentElement;
  root.style.setProperty('--vis-font-family', fontFamilyCSS.value);
  root.style.setProperty('--vis-font-size', `${fontSize.value}px`);
}

// Apply on load
applyFontToDocument();

/* ── cross-tab sync ── */
if (typeof window !== 'undefined') {
  window.addEventListener('storage', (event) => {
    if (event.key === storageKey(StorageKeys.settings.enterToSend)) {
      enterToSend.value = event.newValue === 'true';
    }
    if (event.key === storageKey(StorageKeys.settings.suppressAutoWindows)) {
      suppressAutoWindows.value = event.newValue === 'true';
    }
    if (event.key === storageKey(StorageKeys.settings.fontFamily)) {
      fontFamily.value = (event.newValue as FontId) || DEFAULT_FONT;
    }
    if (event.key === storageKey(StorageKeys.settings.fontSize)) {
      fontSize.value = Number(event.newValue) || FONT_SIZE_DEFAULT;
    }
  });
}

export function useSettings() {
  return {
    enterToSend,
    suppressAutoWindows,
    fontFamily,
    fontSize,
    fontFamilyCSS,
  };
}
