/**
 * Application-wide constants.
 * Centralizes magic numbers and config values that were scattered across App.vue.
 */

// --- Scrolling ---
export const FOLLOW_THRESHOLD_PX = 24;

// --- File viewer floating window ---
export const FILE_VIEWER_WINDOW_WIDTH = 840;
export const FILE_VIEWER_WINDOW_HEIGHT = 520;

// --- Terminal defaults ---
export const TERM_COLUMNS = 80;
export const TERM_ROWS = 25;
export const TERM_LINE_HEIGHT = 1.1;
export const TERM_TITLEBAR_HEIGHT_PX = 22;
export const TERM_WINDOW_BORDER_PX = 2;
export const TERM_INNER_PADDING_X_PX = 4;
export const TERM_INNER_PADDING_Y_PX = 4;
export const TERM_GUTTER_WIDTH_EM = 3.2;

// --- Shell ---
export const SHELL_LINGER_MS = 1000;

// --- Floating windows ---
export const REASONING_CLOSE_DELAY_MS = 3000;
export const SUBAGENT_CLOSE_DELAY_MS = 3000;

// --- Attachments ---
export const ATTACHMENT_MIME_ALLOWLIST = new Set([
  'image/png',
  'image/jpeg',
  'image/gif',
  'image/webp',
]);

// --- Session navigation ---
export const NAVIGABLE_MAX_SESSIONS = 5;

// --- Keyboard ---
export const DOUBLE_ESC_THRESHOLD = 500;
export const DOUBLE_CTRL_G_THRESHOLD = 500;

// --- Tool window visibility ---
export const TOOL_WINDOW_HIDDEN = new Set([
  'question',
  'todoread',
  'todowrite',
  'lsp',
  'plan_enter',
  'plan_exit',
  'task',
]);

export const TOOL_WINDOW_SUPPORTED = new Set([
  'apply_patch',
  'bash',
  'codesearch',
  'edit',
  'glob',
  'grep',
  'list',
  'multiedit',
  'read',
  'task',
  'webfetch',
  'websearch',
  'write',
]);
