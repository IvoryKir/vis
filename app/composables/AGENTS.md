# composables/ — Vue 3 Composition API Layer

25 composables organized in 5 architectural layers.

## LAYERS

| Layer | Composables | Role |
|-------|-------------|------|
| **Transport** | useGlobalEvents | SSE connection via SharedWorker or direct |
| **State** | useMessages, useServerState, useDeltaAccumulator, useOpenSessions | Reactive stores |
| **Selection** | useSessionSelection, useCredentials | Session/project/auth state |
| **API** | useOpenCodeApi, useTodos, usePtyOneshot | REST calls to OpenCode |
| **UI** | useFloatingWindows, useFloatingWindow, usePermissions, useQuestions, useReasoningWindows, useSubagentWindows, useAutoScroller, useContentSearch, useThinkingAnimation, useInitialRenderTracking | Window/scroll/search management |
| **Persistence** | useSettings, useTheme, useFavoriteMessages | localStorage with cross-tab sync |

## KEY COMPOSABLES

| Composable | Lines | Singleton? | Purpose |
|------------|-------|------------|---------|
| useGlobalEvents | 500 | yes | SSE routing, SessionScope/MainSessionScope creation |
| useMessages | 612 | yes (per-session Map) | Message store with active-session proxy pattern |
| useFloatingWindows | 468 | yes | Window lifecycle, z-index tiers, TTL expiry |
| useFileTree | 1119 | yes | File tree + git status + branch management |
| useOpenCodeApi | 315 | yes | REST wrapper with `withPending()` and `waitWithRetry()` |
| usePermissions | 242 | yes | Permission request → floating window pipeline |
| useQuestions | 311 | yes | Question request → floating window pipeline |

## PATTERNS

- **Module-level singletons**: most composables cache state at module scope, return same refs on every call
- **Active-session proxy**: `useMessages()` delegates all methods to `_activeStore` ref — switch session = switch proxy target
- **Request dedup**: generation counters (`++counter; if (counter !== saved) return`) cancel stale async ops
- **Flexible parsing**: usePermissions/useQuestions handle multiple field name variants (id/permissionID/requestID)
- **Computed filtering**: `allowedSessionIds` ComputedRef gates permission/question visibility

## DEPENDENCY FLOW

```
useGlobalEvents → useMessages, useDeltaAccumulator, useServerState
  → useSessionSelection, useOpenSessions
    → usePermissions, useQuestions → useFloatingWindows
      → useReasoningWindows, useSubagentWindows
```

Utility composables (useCredentials, useSettings, useTheme, useFileTree) are leaf nodes — no downstream dependents.
