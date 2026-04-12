# app/ — Vue 3 SPA

Frontend application. Vite root points here (`vite.config.ts: root: 'app'`).

## STRUCTURE

```
app/
├── components/     # 50 SFCs — panels, threads, floating windows, tool viewers
├── composables/    # 25 composables — state, transport, UI layers
├── utils/          # API client, state builder, rendering, storage
├── types/          # SSE events (30+), message types, worker protocol
├── workers/        # SharedWorker (SSE+state), Web Worker (rendering)
├── themes/         # 10 themes, 25 semantic colors each
├── styles/         # Global CSS
├── public/         # Static assets (favicon, logo)
├── main.ts         # createApp(App).mount('#app') — 6 lines
├── App.vue         # Root component (6022 lines — monolith, known debt)
└── index.html      # HTML entry with <meta name="vis-api-base"> for URL injection
```

## DATA FLOW

```
SSE stream → SharedWorker (validates, builds state) → broadcast to tabs
  → useGlobalEvents (routes packets) → composables (update reactive state)
  → Vue components (render UI)

User action → composable → opencode.ts (REST API) → SSE event confirms
```

## KEY PATTERNS

- **Per-session stores**: `useMessages()` returns proxy delegating to active session's store
- **Delta accumulation**: streaming tokens via `useDeltaAccumulator`, not full re-renders
- **Floating windows**: tool output in draggable windows managed by `useFloatingWindows`
- **Worker rendering**: Shiki + markdown-it run in Web Worker to avoid main-thread jank
- **localStorage sync**: settings/credentials/tabs persist and sync across browser tabs via `storage` events

## TYPES (app/types/)

- `sse.ts` (567 lines): 30+ SSE event types, 12 message part types. **SSOT for wire format**
- `worker-state.ts`: `ServerState → ProjectState[] → SandboxState[] → SessionState[]`
- `message.ts`: Message, tokens, usage, attachments, diffs, history entries
- `sse-worker.ts`: TabToWorker / WorkerToTab message protocol

## WORKERS (app/workers/)

- `sse-shared-worker.ts` (1029 lines): SSE connection, state building, packet validation, multi-tab broadcast
- `render-worker.ts` (1031 lines): Shiki highlighting, markdown-it, Myers diff, LRU cache (512 max)

## THEMES (app/themes/)

10 themes: Default, Matrix, Amber CRT, Phosphor Green, Dracula, Nord, Monokai, Cyberpunk, Solarized Dark, Retro Amber. Each defines 25 CSS custom properties.
