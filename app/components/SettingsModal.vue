<template>
  <dialog
    ref="dialogRef"
    class="modal-backdrop"
    @close="$emit('close')"
    @cancel.prevent
    @click.self="dialogRef?.close()"
  >
    <div class="modal">
      <header class="modal-header">
        <div class="modal-title">Settings</div>
        <button type="button" class="modal-close-button" @click="dialogRef?.close()">
          <Icon icon="lucide:x" :width="14" :height="14" />
        </button>
      </header>
      <div class="modal-body">
        <div class="setting-row">
          <div class="setting-info">
            <div class="setting-label">Enter to send</div>
            <div class="setting-description">
              Send messages by pressing Enter. When off, use Ctrl+Enter.
            </div>
          </div>
          <label class="toggle-switch">
            <input v-model="enterToSend" type="checkbox" class="toggle-input" />
            <span class="toggle-track" />
          </label>
        </div>

        <!-- ── Font Family ── -->
        <div class="setting-row setting-row-vertical">
          <div class="setting-info" style="width: 100%;">
            <div class="setting-label">Font</div>
            <div class="setting-description" style="margin-bottom: 10px;">
              Monospace font used across the application.
            </div>
            <div class="font-grid">
              <button
                v-for="font in FONT_OPTIONS"
                :key="font.id"
                class="font-button"
                :class="{ 'font-button-active': fontFamily === font.id }"
                @click="fontFamily = font.id"
              >
                <div class="font-preview" :style="{ fontFamily: font.family }">
                  <span class="font-preview-latin">Aa Bb 01</span>
                  <span class="font-preview-cyrillic">Аа Бб Яя</span>
                  <span v-if="font.hasLigatures" class="font-preview-ligatures">=> != ===</span>
                </div>
                <div class="font-meta">
                  <span class="font-name">{{ font.name }}</span>
                  <span v-if="font.hasLigatures" class="font-badge">ligatures</span>
                </div>
              </button>
            </div>
          </div>
        </div>

        <!-- ── Font Size ── -->
        <div class="setting-row">
          <div class="setting-info">
            <div class="setting-label">Font size</div>
            <div class="setting-description">
              Base font size ({{ FONT_SIZE_MIN }}–{{ FONT_SIZE_MAX }}px).
            </div>
          </div>
          <div class="font-size-control">
            <button
              class="font-size-btn"
              :disabled="fontSize <= FONT_SIZE_MIN"
              @click="fontSize = Math.max(FONT_SIZE_MIN, fontSize - 1)"
            >
              <Icon icon="lucide:minus" :width="12" :height="12" />
            </button>
            <span class="font-size-value">{{ fontSize }}px</span>
            <button
              class="font-size-btn"
              :disabled="fontSize >= FONT_SIZE_MAX"
              @click="fontSize = Math.min(FONT_SIZE_MAX, fontSize + 1)"
            >
              <Icon icon="lucide:plus" :width="12" :height="12" />
            </button>
          </div>
        </div>

        <!-- ── Theme ── -->
        <div class="setting-row setting-row-vertical">
          <div class="setting-info" style="width: 100%;">
            <div class="setting-label">Theme</div>
            <div class="setting-description" style="margin-bottom: 12px;">
              Select the visual style for the application.
            </div>
            <div class="theme-grid">
              <button
                v-for="theme in themes"
                :key="theme.id"
                class="theme-button"
                :class="{ 'theme-button-active': currentTheme === theme.id }"
                @click="setTheme(theme.id)"
              >
                <div class="theme-preview" :style="{
                  background: theme.colors['bg-surface-0'],
                  borderColor: theme.colors['border-color']
                }">
                  <div class="theme-preview-header" :style="{ background: theme.colors['bg-surface-2'], borderBottomColor: theme.colors['border-faint'] }">
                    <div class="theme-preview-dot" :style="{ background: theme.colors['color-error'] }"></div>
                    <div class="theme-preview-dot" :style="{ background: theme.colors['color-warning'] }"></div>
                    <div class="theme-preview-dot" :style="{ background: theme.colors['color-success'] }"></div>
                  </div>
                  <div class="theme-preview-body" :style="{ background: theme.colors['bg-base'] }">
                    <div class="theme-preview-sidebar" :style="{ background: theme.colors['bg-surface-1'], borderRightColor: theme.colors['border-faint'] }"></div>
                    <div class="theme-preview-content">
                      <div class="theme-preview-line" :style="{ background: theme.colors['text-primary'] }"></div>
                      <div class="theme-preview-line w-half" :style="{ background: theme.colors['accent-primary'] }"></div>
                    </div>
                  </div>
                </div>
                <span class="theme-name" :style="{ color: theme.colors['text-primary'] }">{{ theme.name }}</span>
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  </dialog>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue';
import { Icon } from '@iconify/vue';
import { useSettings, FONT_OPTIONS, FONT_SIZE_MIN, FONT_SIZE_MAX } from '../composables/useSettings';
import { useTheme } from '../composables/useTheme';

const props = defineProps<{
  open: boolean;
}>();

defineEmits<{
  (event: 'close'): void;
}>();

const dialogRef = ref<HTMLDialogElement | null>(null);
const { enterToSend, fontFamily, fontSize } = useSettings();
const { themes, currentTheme, setTheme } = useTheme();

watch(
  () => props.open,
  (open) => {
    const el = dialogRef.value;
    if (!el) return;
    if (open) {
      if (!el.open) el.showModal();
    } else if (el.open) {
      el.close();
    }
  },
);
</script>

<style scoped>
.modal-backdrop {
  border: none;
  padding: 0;
  margin: 0;
  background: transparent;
  color: inherit;
  position: fixed;
  inset: 0;
  width: 100%;
  height: 100%;
  max-width: none;
  max-height: none;
  display: flex;
  align-items: center;
  justify-content: center;
}

.modal-backdrop:not([open]) {
  display: none;
}

.modal-backdrop::backdrop {
  background: color-mix(in srgb, var(--bg-surface-0) 65%, transparent);
}

.modal {
  width: min(520px, 95vw);
  max-height: 85vh;
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 16px;
  background: color-mix(in srgb, var(--bg-surface-2) 98%, transparent);
  border: 1px solid var(--border-color);
  border-radius: 12px;
  box-shadow: 0 12px 32px color-mix(in srgb, var(--bg-surface-0) 45%, transparent);
  color: var(--text-primary);
  font-family: inherit;
}

.modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  flex-shrink: 0;
}

.modal-body {
  display: flex;
  flex-direction: column;
  gap: 12px;
  overflow-y: auto;
}

.modal-title {
  font-size: 14px;
  font-weight: 600;
}

.modal-close-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border: 1px solid var(--border-color);
  border-radius: 6px;
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
}

.modal-close-button:hover {
  background: var(--bg-surface-4);
  color: var(--text-primary);
}

.setting-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 10px 12px;
  border: 1px solid var(--border-faint);
  border-radius: 8px;
  background: color-mix(in srgb, var(--bg-surface-0) 45%, transparent);
}

.setting-row-vertical {
  align-items: stretch;
}

.setting-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.setting-label {
  font-size: 13px;
  font-weight: 500;
  color: var(--text-primary);
}

.setting-description {
  font-size: 11px;
  color: var(--text-faint);
}

/* ── Toggle ── */
.toggle-switch {
  position: relative;
  display: inline-flex;
  align-items: center;
  flex-shrink: 0;
  cursor: pointer;
}

.toggle-input {
  position: absolute;
  opacity: 0;
  width: 0;
  height: 0;
}

.toggle-track {
  width: 36px;
  height: 20px;
  background: var(--bg-surface-5);
  border-radius: 10px;
  position: relative;
  transition: background 0.2s;
}

.toggle-track::after {
  content: '';
  position: absolute;
  top: 2px;
  left: 2px;
  width: 16px;
  height: 16px;
  background: var(--text-muted);
  border-radius: 50%;
  transition:
    transform 0.2s,
    background 0.2s;
}

.toggle-input:checked + .toggle-track {
  background: var(--accent-secondary);
}

.toggle-input:checked + .toggle-track::after {
  transform: translateX(16px);
  background: var(--text-primary);
}

/* ── Font Grid ── */
.font-grid {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.font-button {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 10px;
  background: transparent;
  border: 1px solid var(--border-faint);
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.15s;
  text-align: left;
  color: var(--text-primary);
}

.font-button:hover {
  background: rgba(255, 255, 255, 0.04);
  border-color: var(--border-color);
}

.font-button-active {
  background: rgba(255, 255, 255, 0.06);
  border-color: var(--accent-primary);
}

.font-preview {
  display: flex;
  gap: 10px;
  font-size: 13px;
  line-height: 1.3;
  flex-shrink: 0;
}

.font-preview-latin {
  color: var(--text-primary);
}

.font-preview-cyrillic {
  color: var(--text-muted);
}

.font-preview-ligatures {
  color: var(--text-faint);
  font-feature-settings: 'liga' 1, 'calt' 1;
}

.font-meta {
  margin-left: auto;
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}

.font-name {
  font-size: 11px;
  color: var(--text-muted);
  white-space: nowrap;
}

.font-badge {
  font-size: 9px;
  padding: 1px 5px;
  border-radius: 3px;
  background: color-mix(in srgb, var(--accent-primary) 15%, transparent);
  color: var(--accent-primary);
  white-space: nowrap;
}

/* ── Font Size Control ── */
.font-size-control {
  display: flex;
  align-items: center;
  gap: 4px;
  flex-shrink: 0;
}

.font-size-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  border: 1px solid var(--border-faint);
  border-radius: 5px;
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  transition: all 0.15s;
}

.font-size-btn:hover:not(:disabled) {
  background: var(--bg-surface-4);
  color: var(--text-primary);
  border-color: var(--border-color);
}

.font-size-btn:disabled {
  opacity: 0.3;
  cursor: not-allowed;
}

.font-size-value {
  font-size: 13px;
  font-weight: 500;
  min-width: 38px;
  text-align: center;
  color: var(--text-primary);
  font-variant-numeric: tabular-nums;
}

/* ── Theme Grid ── */
.theme-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
  gap: 12px;
  margin-top: 8px;
}

.theme-button {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  background: transparent;
  border: none;
  cursor: pointer;
  padding: 8px;
  border-radius: 8px;
  transition: background 0.2s;
}

.theme-button:hover {
  background: rgba(255, 255, 255, 0.05);
}

.theme-button-active {
  background: rgba(255, 255, 255, 0.1);
}

.theme-preview {
  width: 100%;
  aspect-ratio: 16 / 10;
  border: 2px solid transparent;
  border-radius: 6px;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.2);
  transition: transform 0.2s, box-shadow 0.2s;
}

.theme-button:hover .theme-preview {
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
}

.theme-button-active .theme-preview {
  outline: 2px solid var(--accent-primary, var(--accent-primary));
  outline-offset: 2px;
}

.theme-preview-header {
  height: 12px;
  display: flex;
  align-items: center;
  padding: 0 6px;
  gap: 4px;
  border-bottom: 1px solid;
}

.theme-preview-dot {
  width: 4px;
  height: 4px;
  border-radius: 50%;
}

.theme-preview-body {
  flex: 1;
  display: flex;
}

.theme-preview-sidebar {
  width: 25%;
  height: 100%;
  border-right: 1px solid;
}

.theme-preview-content {
  flex: 1;
  padding: 6px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.theme-preview-line {
  height: 4px;
  border-radius: 2px;
  opacity: 0.8;
}

.theme-preview-line.w-half {
  width: 50%;
}

.theme-name {
  font-size: 11px;
  font-weight: 500;
}
</style>
