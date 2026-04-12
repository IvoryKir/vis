# utils/ — Core Utilities

13 files. API client, state management, rendering, and helpers.

## FILE MAP

| File | Lines | Role |
|------|-------|------|
| opencode.ts | 451 | **API client** — 40+ endpoint functions, `setBaseUrl()`, `setAuthorization()` |
| stateBuilder.ts | 790 | **State engine** — `ServerState → Project → Sandbox → Session` tree ops |
| toolRenderers.ts | 699 | **Tool extraction** — parses tool output into `{ content, variant, title }` |
| sseConnection.ts | 214 | SSE stream with auto-reconnect (1s backoff) |
| theme.ts | 303 | Theme color resolution, agent color cycling (7 colors) |
| notificationManager.ts | 144 | Permission/question/idle notification state tracking |
| path.ts | 78 | Path normalization, sandbox-aware splitting for API calls |
| useCodeRender.ts | 71 | Vue composable wrapping render-worker for syntax highlighting |
| storageKeys.ts | 68 | Namespaced localStorage keys + safe get/set helpers |
| workerRenderer.ts | 55 | Web Worker pool for async code rendering |
| formatters.ts | 44 | Display: token counts (1K/10K), elapsed time, error messages |
| waitForState.ts | 37 | `waitForState(source, predicate, timeout)` — Vue watch + Promise |
| eventEmitter.ts | 30 | Generic `TypedEmitter<EventMap>` with `on()`, `emit()`, `dispose()` |

## KEY ARCHITECTURE

**stateBuilder.ts** — most complex utility:
- Session tree: all child sessions stored flat under root's sandbox
- `rootSessions: string[]` maintains display order (sorted by `timeUpdated` desc)
- Ephemeral child pruning: sessions idle >20min auto-removed (CHILD_SESSION_PRUNE_TTL_MS)
- Processes 12 SSE event types → state mutations

**opencode.ts** — endpoint categories:
- Sessions: list, get, create, delete, fork, revert, update
- Projects: list, get, update
- Files: list, read content, get diffs
- Messages: list, get, patch parts
- Permissions/Questions: list pending, reply, reject
- PTYs: list, create, update size, delete
- Worktrees: list, create, delete

**toolRenderers.ts** — handles 12+ tool types:
- bash, read, grep, glob, edit, multiedit, write, webfetch, websearch, codesearch, task, batch, apply_patch

## PATTERNS

- **API client is stateless** — `setBaseUrl()`/`setAuthorization()` set module-level globals
- **stateBuilder uses immutable copies** — `new Map()` for updates, never mutates in place
- **Request serialization** — `opencodeQueue` in sse-shared-worker serializes API calls to prevent races
- **LRU caching** — render-worker caches (512 max), pruned on overflow
