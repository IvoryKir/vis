import { ref, computed } from 'vue';
import {
  StorageKeys,
  storageGet,
  storageKey,
  storageRemove,
  storageSet,
} from '../utils/storageKeys';

type Credentials = {
  url: string;
  username: string;
  password: string;
};

const url = ref('');
const username = ref('');
const password = ref('');
const launcherManaged = ref(false);

/**
 * Read the same-origin API base injected by the Vis launcher (`server.js`).
 *
 * The launcher rewrites the served `index.html` to set
 * `<meta name="vis-api-base" content="/api" />`. When that meta tag is
 * non-empty we know we are running under the launcher and can skip the
 * login screen entirely — the API is reachable on the same origin and
 * needs no CORS configuration or hand-typed URL.
 */
function readApiBase(): string | null {
  if (typeof document === 'undefined' || typeof window === 'undefined') return null;
  // 1. Direct window global (set by the Tauri shell via window.eval on boot).
  //    Checked first because the Tauri shell can set it before the meta tag
  //    exists, and we'd rather pick up the eventual URL over stale HTML.
  const direct = (window as unknown as { __VIS_API_BASE__?: unknown }).__VIS_API_BASE__;
  if (typeof direct === 'string' && direct.trim()) {
    return resolveBase(direct.trim());
  }
  // 2. <meta name="vis-api-base" content="..."> injected either by the Node
  //    launcher (server.js rewriting dist/index.html on serve) or hand-written.
  const meta = document.querySelector('meta[name="vis-api-base"]');
  if (meta) {
    const content = meta.getAttribute('content')?.trim();
    if (content) return resolveBase(content);
  }
  return null;
}

function resolveBase(content: string): string | null {
  try {
    return new URL(content, window.location.origin).toString().replace(/\/+$/, '');
  } catch {
    return null;
  }
}

export function useCredentials() {
  // Pick up the launcher-provided base URL if present. Under the Tauri shell
  // this value may not exist yet at first call (the shell injects it via
  // window.eval only after opencode has finished booting) — we also subscribe
  // to the 'vis:api-base-changed' event below to catch that late update.
  const launcherBaseUrl = readApiBase();
  if (launcherBaseUrl && !launcherManaged.value) {
    url.value = launcherBaseUrl;
    launcherManaged.value = true;
  }

  const authHeader = computed(() => {
    const u = username.value.trim();
    const p = password.value.trim();
    if (!u && !p) return undefined;
    const credentials = `${u}:${p}`;
    return `Basic ${btoa(credentials)}`;
  });

  const baseUrl = computed(() => {
    return url.value.replace(/\/+$/, '');
  });

  const isConfigured = computed(() => {
    return url.value.trim().length > 0;
  });

  function save(newUrl: string, newUsername: string, newPassword: string) {
    // When the launcher injected an API base, ignore manual URL overrides
    // — the user shouldn't be allowed to point Vis at a different server.
    if (launcherManaged.value) {
      username.value = newUsername;
      password.value = newPassword;
      // Persist credentials only (not URL) so refreshes keep auth.
      if (typeof window !== 'undefined') {
        try {
          const data: Credentials = { url: '', username: newUsername, password: newPassword };
          storageSet(StorageKeys.auth.credentials, JSON.stringify(data));
        } catch {
          /* ignore */
        }
      }
      return;
    }

    url.value = newUrl;
    username.value = newUsername;
    password.value = newPassword;

    if (typeof window === 'undefined') return;

    try {
      const data: Credentials = {
        url: newUrl,
        username: newUsername,
        password: newPassword,
      };
      storageSet(StorageKeys.auth.credentials, JSON.stringify(data));
    } catch {
      return;
    }
  }

  function load() {
    if (typeof window === 'undefined') return;

    try {
      const raw = storageGet(StorageKeys.auth.credentials);
      if (!raw) return;

      const data = JSON.parse(raw) as unknown;
      if (!data || typeof data !== 'object') return;

      const record = data as Record<string, unknown>;
      const loadedUrl = typeof record.url === 'string' ? record.url : '';
      const loadedUsername = typeof record.username === 'string' ? record.username : '';
      const loadedPassword = typeof record.password === 'string' ? record.password : '';

      // Under the launcher we keep the injected URL and only restore auth.
      if (!launcherManaged.value) {
        url.value = loadedUrl;
      }
      username.value = loadedUsername;
      password.value = loadedPassword;
    } catch {
      return;
    }
  }

  function clear() {
    // Under the launcher we never wipe the URL — the meta tag is the source
    // of truth and a logout simply forgets HTTP auth.
    if (!launcherManaged.value) {
      url.value = '';
    }
    username.value = '';
    password.value = '';

    if (typeof window === 'undefined') return;

    try {
      storageRemove(StorageKeys.auth.credentials);
    } catch {
      return;
    }
  }

  if (typeof window !== 'undefined') {
    window.addEventListener('storage', (event) => {
      if (event.key !== storageKey(StorageKeys.auth.credentials)) return;

      if (!event.newValue) {
        url.value = '';
        username.value = '';
        password.value = '';
        return;
      }

      try {
        const data = JSON.parse(event.newValue) as unknown;
        if (!data || typeof data !== 'object') return;

        const record = data as Record<string, unknown>;
        const loadedUrl = typeof record.url === 'string' ? record.url : '';
        const loadedUsername = typeof record.username === 'string' ? record.username : '';
        const loadedPassword = typeof record.password === 'string' ? record.password : '';

        url.value = loadedUrl;
        username.value = loadedUsername;
        password.value = loadedPassword;
      } catch {
        return;
      }
    });
  }

  // Late injection support: the Tauri shell waits until opencode is up
  // before writing window.__VIS_API_BASE__ and firing this event. If the
  // SPA booted first with an empty meta tag, we latch on when it arrives.
  if (typeof window !== 'undefined') {
    window.addEventListener('vis:api-base-changed', () => {
      const late = readApiBase();
      if (!late) return;
      url.value = late;
      launcherManaged.value = true;
    });
  }

  return {
    url,
    username,
    password,
    authHeader,
    baseUrl,
    isConfigured,
    launcherManaged,
    save,
    load,
    clear,
  };
}
