# Полное ревью кодовой базы Vis

**Ветка:** `feat/multi-session-launcher`
**Дата:** 2025-07-14
**Общий объём:** ~32,600 строк кода (97 файлов)

---

## ОГЛАВЛЕНИЕ

1. [Общая картина](#1-общая-картина)
2. [Критические проблемы](#2-критические-проблемы)
3. [Ошибки TypeScript (12 ошибок компиляции)](#3-ошибки-typescript)
4. [Неиспользуемый код (мёртвый код)](#4-неиспользуемый-код)
5. [Дублирование кода](#5-дублирование-кода)
6. [Архитектурные проблемы](#6-архитектурные-проблемы)
7. [Ревью компонентов Vue](#7-ревью-компонентов-vue)
8. [Ревью composables](#8-ревью-composables)
9. [Ревью утилит и типов](#9-ревью-утилит-и-типов)
10. [Code Style и Best Practices](#10-code-style-и-best-practices)
11. [Производительность](#11-производительность)
12. [CSS и стили](#12-css-и-стили)
13. [Рекомендации по рефакторингу (план действий)](#13-план-рефакторинга)

---

## 1. Общая картина

### Размеры файлов (топ-20 по строкам)

| Файл | Строк | Оценка |
|------|-------|--------|
| `App.vue` | 6,026 | 🔴 КРИТИЧНО — монолит |
| `InputPanel.vue` | 1,693 | 🔴 Слишком большой |
| `TreeView.vue` | 1,493 | 🔴 Слишком большой |
| `TopPanel.vue` | 1,236 | 🔴 Слишком большой |
| `useFileTree.ts` | 1,119 | 🟠 Большой |
| `render-worker.ts` | 1,031 | 🟠 Большой |
| `sse-shared-worker.ts` | 1,029 | 🟠 Большой |
| `stateBuilder.ts` | 790 | 🟡 Средний |
| `ThreadBlock.vue` | 763 | 🟡 Средний |
| `FloatingWindow.vue` | 749 | 🟡 Средний |
| `toolRenderers.ts` | 699 | 🟡 Средний |
| `useMessages.ts` | 667 | 🟡 Средний |
| `ProjectPicker.vue` | 653 | 🟡 Средний |
| `ThreadHistoryContent.vue` | 627 | 🟡 Средний |
| `server.js` | 567 | ✅ Нормально |
| `sse.ts (types)` | 567 | ✅ Типы — ОК |
| `ProjectSettingsDialog.vue` | 567 | 🟡 Средний |
| `SettingsModal.vue` | 536 | 🟡 Средний |
| `Dropdown.vue` | 522 | 🟡 Средний |
| `MarkdownRenderer.vue` | 506 | 🟡 Средний |

### Здоровье сборки

- **oxlint:** 0 ошибок, 0 предупреждений ✅
- **vue-tsc:** 12 ошибок ❌
- **Консольные логи:** 11 мест (приемлемо, но стоит унифицировать)
- **Хаки типов (`as any`):** 17 мест ⚠️
- **eslint-disable:** 1 место ✅

---

## 2. Критические проблемы

### 2.1 App.vue — монолит на 6,026 строк

**Это самая большая проблема проекта.** App.vue содержит:
- 191 функцию
- 164 top-level переменных
- 22 watcher'а
- 481 обращение к `.value`
- ~337 строк CSS

В одном файле смешаны:
- Управление shell/PTY сессиями (~500 строк)
- Git операции (~200 строк)
- Просмотр файлов (~150 строк)
- Клавиатурные сочетания (~110 строк)
- Управление floating windows (~300 строк)
- Управление провайдерами/моделями (~200 строк)
- SSE события и обработка сообщений (~400 строк)
- UI layout и state management (~500 строк)
- Инициализация и bootstrapping (~200 строк)

**Действие:** Разбить на 8-10 composables (см. раздел 13).

### 2.2 Три компонента-монолита

| Компонент | Строк | Проблема |
|-----------|-------|----------|
| `InputPanel.vue` | 1,693 | Смешаны: история, избранное, команды, вложения, выбор модели. 653 строки CSS! |
| `TreeView.vue` | 1,493 | Смешаны: файловое дерево, branch picker, git операции. 606 строк CSS! |
| `TopPanel.vue` | 1,236 | Смешаны: session picker, уведомления, меню проекта. 572 строки CSS! |

### 2.3 Утечки памяти (потенциальные)

- `useAssistantPreRenderer.ts`: `watchEffect` на module-level без cleanup — Maps растут бесконечно (`assistantHtmlCache`, `deferredKeyCache`, `submitSeqMap`, `appliedSeqMap`, `lastSubmitted`)
- `useMessages.ts`: `messageStores` (Map) — stores создаются, но `disposeMessages()` не вызывается нигде в коде
- `useFileTree.ts`: `scheduledDirectoryReloads` (Map) — таймеры не очищаются при смене проекта
- `useReasoningWindows.ts`: `reasoningCloseTimers`, `lastReasoningMessageIdByKey`, `activeReasoningMessageIdByKey`, `finishedReasoningByKey` — не очищаются при rebind
- `useSubagentWindows.ts`: `closeTimers`, `activeMessageIdBySession` — не очищаются при rebind
- `App.vue`: `shellSessionsByPtyId`, `pendingShellFits`, `shellExitWaiters`, `composerDraftRevisionByContext` — растут без ограничений

---

## 3. Ошибки TypeScript

**12 ошибок компиляции (`vue-tsc --noEmit`):**

| # | Файл | Строка | Ошибка | Серьёзность |
|---|------|--------|--------|-------------|
| 1 | `App.vue` | 1367 | `resolveSessionTitle` declared but never read (TS6133) | 🟡 Мёртвый код |
| 2 | `App.vue` | 1397 | `branch` не существует на типе `SandboxState` (TS2339) | 🔴 Ошибка типов |
| 3 | `App.vue` | 4281 | Несовместимость overload для watch callback (TS2769) | 🔴 Ошибка типов |
| 4 | `InputPanel.vue` | 878 | `__TAURI_INTERNALS__` не существует на `Window` (TS2339) | 🟡 Нужен тип |
| 5 | `ThreadHistoryContent.vue` | 146 | `openSubagentWindow` declared but never read (TS6133) | 🟡 Мёртвый код |
| 6 | `TreeView.vue` | 437 | `upstreamRemote` declared but never read (TS6133) | 🟡 Мёртвый код |
| 7 | `DiffViewer.vue` | 23 | Type `string` not assignable to `PrimaryMode` (TS2322) | 🔴 Ошибка типов |
| 8 | `DiffViewer.vue` | 150 | Type `string` not assignable to `PrimaryMode` (TS2322) | 🔴 Ошибка типов |
| 9 | `DiffViewer.vue` | 191 | `"dark"` not assignable to `ColorSchemeType` (TS2322) | 🔴 Ошибка типов |
| 10 | `useFloatingWindow.ts` | 31 | `null` not assignable to `FloatingWindowAPI` (TS2345) | 🟡 Нужна перегрузка |
| 11 | `useFloatingWindows.ts` | 221 | `closable` не существует на типе `never` (TS2339) | 🔴 Ошибка типов |
| 12 | `useSubagentWindows.ts` | 222 | `"top"` not assignable to scroll mode type (TS2322) | 🟡 Нужна enum |

---

## 4. Неиспользуемый код

### 4.1 Неиспользуемые экспорты из `utils/`

| Файл | Экспорт | Действие |
|------|---------|----------|
| `notificationManager.ts` | `NotificationSnapshotEntry` | Удалить тип |
| `opencode.ts` | `getSessionDiff` | Удалить функцию |
| `opencode.ts` | `getSession` | Удалить функцию |
| `opencode.ts` | `getSessionChildren` | Удалить функцию |
| `opencode.ts` | `listWorktrees` | Удалить функцию |
| `opencode.ts` | `getSessionMessage` | Удалить функцию |
| `path.ts` | `normalizeRelativePathNoParent` | Удалить функцию |
| `path.ts` | `normalizeAbsolutePathNoParent` | Удалить функцию |
| `sseConnection.ts` | `SseConnectionOptions` | Удалить тип |
| `sseConnection.ts` | `SseConnectionCallbacks` | Удалить тип |
| `sseConnection.ts` | `SseConnectionConnectOptions` | Удалить тип |
| `theme.ts` | `ThemeColors` | Удалить тип |
| `theme.ts` | `ThemeJson` | Удалить тип |
| `toolRenderers.ts` | `extractXmlTagContent` | Удалить функцию |
| `toolRenderers.ts` | `ToolRenderersHelpers` | Удалить тип |
| `toolRenderers.ts` | `extractStepFinish` | Удалить функцию |
| `useCodeRender.ts` | `CodeRenderResult` | Удалить тип |

### 4.2 Неиспользуемые экспорты из `composables/`

| Файл | Экспорт | Действие |
|------|---------|----------|
| `useDeltaAccumulator.ts` | `AccumulatedMessage` | Удалить тип |
| `useFavoriteMessages.ts` | `FavoriteMessageEntry` | Удалить тип |
| `useFileTree.ts` | `GitStatus` | Удалить тип |
| `useFloatingWindows.ts` | `Extent` | Удалить тип |
| `useGlobalEvents.ts` | `MainSessionScope` | Удалить тип (alias) |
| `useMessages.ts` | `UseMessages` | Удалить тип |
| `useMessages.ts` | `disposeMessages` | Удалить функцию (никогда не вызывается!) |
| `useOpenCodeApi.ts` | `UseOpenCodeApi` | Удалить тип |
| `useOpenSessions.ts` | `SessionTabStatus` | Удалить тип |
| `useQuestions.ts` | `QuestionAnswer` | Удалить тип |
| `useReasoningWindows.ts` | `ReasoningFinish` | Удалить тип |
| `useReasoningWindows.ts` | `UseReasoningWindowsReturn` | Удалить тип |
| `useServerState.ts` | `UseServerState` | Удалить тип |
| `useSettings.ts` | `FontId` | Удалить тип |
| `useSettings.ts` | `FONT_SIZE_DEFAULT` | Удалить константу |

### 4.3 Неиспользуемые экспорты из `types/`

**57 неиспользуемых экспортов из `types/sse.ts`!**

Большинство — это типы SSE-пакетов (`MessageRemovedPacket`, `SessionUpdatedPacket`, `PtyCreatedPacket`, и т.д.). Они экспортируются, но нигде не импортируются. Скорее всего экспортированы «про запас». 

**Также не используются:**
- `types/message.ts` → `Message`
- `types/worker-state.ts` → `NotificationState`

### 4.4 Мёртвый код в App.vue

| Строка | Код | Описание |
|--------|-----|----------|
| 4221 | `function log(..._args: unknown[]) {}` | Пустая функция-заглушка |
| 4225-4227 | `TOOL_RENDERER_READ_EVENT_TYPES`, `TOOL_RENDERER_WRITE_EVENT_TYPES` | Определены, но не используются |
| 4236-4238 | `toolRendererReadTypesKey`, `toolRendererWriteTypesKey`, `toolRendererMessageTypesKey` | Строковые ключи — нигде не используются |
| 1367 | `resolveSessionTitle` | Объявлена, но не используется |
| 1397 | `.branch` | Обращение к несуществующему свойству |

---

## 5. Дублирование кода

### 5.1 Точные дупликаты

#### `formatMessageTime()` — полная копия
- **Оригинал:** `utils/formatters.ts` (экспортируется)
- **Копия:** `ThreadHistoryContent.vue` строка 322 (локальная функция, 11 строк, посимвольно идентична)
- **Действие:** Удалить копию, импортировать из `formatters.ts`

#### `matchesQuery()` — полная копия
- **Место 1:** `TopPanel.vue` строка 518 (4 строки)
- **Место 2:** `InputPanel.vue` строка 998 (4 строки)
- **Действие:** Вынести в `utils/search.ts`, импортировать в оба

#### `BITMAP_EXTENSIONS` — полная копия
- **Место 1:** `viewers/ContentViewer.vue` строка 67
- **Место 2:** `viewers/DiffViewer.vue` строка 121
- **Действие:** Вынести в `utils/fileTypes.ts`

### 5.2 Структурные дупликаты

#### ToolWindow компоненты — 5 почти идентичных файлов

`Bash.vue`, `Default.vue`, `Edit.vue`, `Read.vue`, `Task.vue` — все имеют одну и ту же структуру:
```vue
<template>
  <CodeContent :html="props.entry.renderedHtml" :status="props.entry.status" />
</template>
<script setup>
const props = defineProps<{ entry: ... }>();
</script>
<style scoped>
/* 1-3 строки стилей */
</style>
```
**Действие:** Заменить на один универсальный `SimpleToolWindow.vue` с пропсом `toolName`.

#### `Reasoning.vue` и `Subagent.vue` — 80% дупликат
Оба отображают markdown-контент с заголовком. Различия минимальны.
**Действие:** Объединить в `MarkdownToolWindow.vue`.

#### `Glob.vue` и `Grep.vue` — структурный дупликат
Оба показывают результаты поиска с подсчётом файлов/строк. Различаются только в формате отображения.
**Действие:** Объединить в `SearchResultWindow.vue`.

#### `usePermissions.ts` и `useQuestions.ts` — 90% дупликат
Оба реализуют: queue → show → answer → cleanup. Различаются только типы данных и API endpoints.
**Действие:** Создать абстрактный `usePromptQueue<T>()` composable.

#### `useReasoningWindows.ts` и `useSubagentWindows.ts` — 80% дупликат
Оба реализуют: entries by session → open/close windows → cleanup timers → rebind.
**Действие:** Создать абстрактный `useSessionWindows<T>()` composable.

### 5.3 Дублирование CSS

#### Стили кнопок `.ib-action`
Дублируются в `ThreadBlock.vue` и `ThreadFooter.vue`.

#### Стили `.todo-*`
Дублируются в `InputPanel.vue` и `TodoPanel.vue`.

#### Стили `.history-target-provider`
Дублируются в `InputPanel.vue` (строка 1513-1525) и `ThreadTarget.vue` (строка 54-66).

---

## 6. Архитектурные проблемы

### 6.1 Отсутствующие composables (код в App.vue, который должен быть вынесен)

| Composable | Строк в App.vue | Описание |
|-----------|-----------------|----------|
| `useShellSessions()` | ~500 | Управление PTY/shell сессиями, WebSocket, terminal fitting |
| `useGitOperations()` | ~200 | Git diff, commit snapshots, worktree scripts |
| `useFileViewer()` | ~150 | Открытие файлов, рендеринг, определение языка |
| `useKeyboardShortcuts()` | ~110 | Все горячие клавиши (Ctrl+A, Ctrl+G, Alt+N, Escape, ...) |
| `useProviders()` | ~200 | Загрузка провайдеров, моделей, thinking options |
| `useMessageSending()` | ~100 | Валидация, отправка, slash-команды |
| `useToolWindows()` | ~300 | Открытие/закрытие tool windows, SSE обработка |
| `useSessionBoot()` | ~200 | Инициализация, bootstrapping, восстановление сессии |

### 6.2 Файл `useCodeRender.ts` в неправильном месте

Файл `app/utils/useCodeRender.ts` — это composable (использует `ref()`, `watch()`, `onBeforeUnmount()`), но находится в `utils/`. 
**Действие:** Переместить в `app/composables/useCodeRender.ts`.

### 6.3 provide/inject без типобезопасности

- `Dropdown.vue` → `provide('x-selectable', api)` — строковый ключ вместо `InjectionKey<T>`
- `App.vue` → `provide('subagentCompletedEntries', ...)` — строковый ключ
- `App.vue` → `provide('openSubagentWindow', ...)` — строковый ключ

**Действие:** Создать `app/injection-keys.ts` с типизированными `InjectionKey<T>`.

### 6.4 Модульные синглтоны без cleanup

Несколько composables используют module-level state:
- `useMessages.ts` — `messageStores = new Map()`
- `useAssistantPreRenderer.ts` — 5 module-level Maps
- `useFileTree.ts` — `scheduledDirectoryReloads = new Map()`

Это нормальный паттерн для shared state, но ни один не имеет механизма очистки при закрытии приложения.

### 6.5 Нет разделения на feature-модули

Текущая структура:
```
app/
  components/     ← все компоненты в одной папке (28 файлов)
  composables/    ← все composables в одной папке (25 файлов)
  utils/          ← все утилиты в одной папке (14 файлов)
```

Рекомендуемая структура (feature-based):
```
app/
  features/
    shell/          ← useShellSessions, Shell.vue
    git/            ← useGitOperations, TreeView, BranchPicker
    messaging/      ← useMessages, useDeltaAccumulator, ThreadBlock
    floating/       ← useFloatingWindows, FloatingWindow
    settings/       ← useSettings, SettingsModal
    providers/      ← useProviders, ModelSelector
  shared/
    components/     ← Dropdown, CodeContent, etc.
    composables/    ← useAutoScroller, useContentSearch
    utils/          ← formatters, path, storageKeys
    types/          ← sse, message, worker-state
```

---

## 7. Ревью компонентов Vue

### 7.1 Компоненты, требующие разделения

#### InputPanel.vue (1,693 строк)
Смешивает 5 ответственностей:
1. Поле ввода сообщения + toolbar → оставить в `InputPanel.vue`
2. История сообщений dropdown → извлечь `HistoryDropdown.vue`
3. Избранные сообщения dropdown → извлечь `FavoritesDropdown.vue` (или объединить с историей)
4. Выбор модели/агента → извлечь `ModelSelector.vue`
5. Управление вложениями → извлечь `AttachmentList.vue`

#### TopPanel.vue (1,236 строк)
Смешивает 4 ответственности:
1. Верхняя панель layout → оставить в `TopPanel.vue`
2. Session/worktree picker dropdown → извлечь `SessionPicker.vue`
3. Кнопка уведомлений → извлечь `NotificationButton.vue`
4. Меню проекта → извлечь `ProjectMenu.vue`

#### TreeView.vue (1,493 строк)
Смешивает 3 ответственности:
1. Файловое дерево → оставить в `TreeView.vue`
2. Branch picker с поиском → извлечь `BranchPicker.vue`
3. Git status bar → извлечь `GitStatusBar.vue`

#### FloatingWindow.vue (749 строк)
Смешивает 3 ответственности:
1. Layout окна → оставить в `FloatingWindow.vue`
2. Drag/resize логика → извлечь `useWindowDrag.ts` composable
3. Search bar → извлечь `WindowSearchBar.vue`

### 7.2 Проблемы в отдельных компонентах

#### ThreadHistoryContent.vue
- Локальная копия `formatMessageTime()` (см. дупликаты)
- `openSubagentWindow` объявлен через inject, но не используется (TS6133)

#### ThreadBlock.vue
- `HISTORY_TOOL_NAMES` Set определён, но используется в одном месте — можно inline
- Дублированная логика маппинга history entries (строки 194-233 и 374-412)

#### Dropdown.vue
- provide по строковому ключу `'x-selectable'` вместо `InjectionKey<T>`
- `defineExpose` экспортирует 4 метода — слишком много для одного dropdown

#### ProjectPicker.vue
- `fetchController`, `fetchRequestId` — non-reactive module-level переменные (возможен race condition при параллельных вызовах)
- Ошибки перехватываются пустым catch (строка 246-254)

#### SessionTab.vue, SessionTabBar.vue, StatusBar.vue, Welcome.vue, ThreadTarget.vue
- Маленькие, чистые, хорошо написанные ✅

### 7.3 ToolWindow компоненты

#### Хорошо написаны:
- `Permission.vue` (315 строк) — сложный, но хорошо структурированный
- `Question.vue` (492 строк) — сложный, можно извлечь sub-components

#### Можно объединить (дупликаты):
- `Bash.vue` + `Default.vue` + `Edit.vue` + `Read.vue` + `Task.vue` → `SimpleToolWindow.vue`
- `Reasoning.vue` + `Subagent.vue` → `MarkdownToolWindow.vue`
- `Glob.vue` + `Grep.vue` → `SearchResultWindow.vue`

#### `utils.ts` (ToolWindow)
- Чистый файл с форматтерами ✅

### 7.4 Renderers

#### MarkdownRenderer.vue (506 строк) — большой
- Миксует логику рендеринга, copy-to-clipboard, подсветку, lifecycle
- `copiedResetTimers` — Map без cleanup (потенциальная утечка)
- **Действие:** Извлечь `useMarkdownCopy.ts`, `useMarkdownHighlight.ts`

#### CodeRenderer.vue, DiffRenderer.vue, HexRenderer.vue, ImageRenderer.vue
- Хорошо написаны, небольшие ✅

### 7.5 Viewers

#### DiffViewer.vue
- 3 ошибки TypeScript (строки 23, 150, 191)
- Hardcoded colors в CSS (12+ мест)
- Unscoped CSS для d2h стилей — может конфликтовать

#### ContentViewer.vue
- Дублирование `BITMAP_EXTENSIONS` с DiffViewer.vue

---

## 8. Ревью composables

### 8.1 Composables, требующие рефакторинга

#### useFileTree.ts (1,119 строк) — слишком большой
- `parseGitStatusOutput()` — 107 строк, должна быть в отдельном парсере
- Много nested функций
- **Действие:** Извлечь `useGitStatus.ts`, `useFileTreeExpansion.ts`, `gitStatusParser.ts`

#### useMessages.ts (667 строк) — большой
- `createMessages()` — 401 строк! Одна функция
- `_proxyMessages as any`, `_proxyRoots as any`, `_proxyStreaming as any` — хаки типов
- `disposeMessages()` экспортирована, но нигде не вызывается
- **Действие:** Разбить `createMessages()` на подфункции

#### useFloatingWindows.ts (468 строк) — средний
- Race condition: async `renderContent()` без cancellation token
- `eslint-disable` комментарий для spread в `closeAll()`
- **Действие:** Добавить AbortController для async операций

#### useGlobalEvents.ts (500 строк) — большой
- Сложная логика SSE routing по сессиям
- Смешивает transport (SSE) и бизнес-логику (session filtering)
- **Действие:** Извлечь `useSseTransport.ts` и `useSessionEventRouter.ts`

### 8.2 Дублирование между composables

| Группа | Файлы | % дупликации | Действие |
|--------|-------|--------------|----------|
| Prompt queue | `usePermissions.ts`, `useQuestions.ts` | ~90% | Создать `usePromptQueue<T>()` |
| Session windows | `useReasoningWindows.ts`, `useSubagentWindows.ts` | ~80% | Создать `useSessionWindows<T>()` |

### 8.3 Composables без проблем ✅

- `useAutoScroller.ts` — хорошо написан, правильный cleanup
- `useContentSearch.ts` — использует CSS Highlights API, корректно
- `useCredentials.ts` — хорошая обработка Tauri/browser/localStorage
- `useDeltaAccumulator.ts` — чистый, простой
- `useFavoriteMessages.ts` — O(n) lookup вместо Map, но при малых N — ОК
- `useInitialRenderTracking.ts` — чистый
- `useOpenCodeApi.ts` — чистый
- `useOpenSessions.ts` — чистый
- `useServerState.ts` — чистый
- `useSessionSelection.ts` — чистый
- `useSettings.ts` — чистый
- `useTheme.ts` — чистый
- `useThinkingAnimation.ts` — чистый, hardcoded animation speed — не критично

---

## 9. Ревью утилит и типов

### 9.1 utils/

#### toolRenderers.ts (699 строк)
- `extractFileRead()` — god-function, обрабатывает все типы tool output
- `extractXmlTagContent()` — не используется нигде
- `extractStepFinish()` — не используется нигде
- **Действие:** Разбить на `bashRenderer.ts`, `editRenderer.ts`, `readRenderer.ts`, etc.

#### stateBuilder.ts (790 строк)
- Хорошо написан, но большой
- Сложная логика построения дерева сессий
- **Действие:** Можно разбить на `sessionTreeBuilder.ts` и `projectStateBuilder.ts`

#### opencode.ts (451 строк)
- API клиент — хорошо структурирован
- 5 неиспользуемых функций (`getSessionDiff`, `getSession`, `getSessionChildren`, `listWorktrees`, `getSessionMessage`)
- **Действие:** Удалить неиспользуемые, остальное оставить

#### sseConnection.ts
- Хорошо написан
- 3 неиспользуемых типа — удалить

#### storageKeys.ts
- Хорошо написан ✅

#### formatters.ts
- Чистый, маленький ✅

#### path.ts
- 2 неиспользуемых функции (`normalizeRelativePathNoParent`, `normalizeAbsolutePathNoParent`)

#### eventEmitter.ts
- Чистый, маленький ✅

#### workerRenderer.ts
- Чистый ✅

#### waitForState.ts
- Чистый ✅

#### theme.ts
- 2 неиспользуемых типа (`ThemeColors`, `ThemeJson`)

#### notificationManager.ts
- 1 неиспользуемый тип (`NotificationSnapshotEntry`)

### 9.2 types/

#### sse.ts (567 строк)
- **57 неиспользуемых экспортов!**
- Большинство — типы пакетов, экспортированные про запас
- **Действие:** Не удалять — они полезны для будущего, но пометить комментарием или перестать экспортировать

#### message.ts
- Тип `Message` не используется

#### worker-state.ts
- Тип `NotificationState` не используется

### 9.3 workers/

#### render-worker.ts (1,031 строк)
- Смешивает markdown рендеринг, code highlighting, diff formatting
- Кэши (`codeHtmlCache`, `mdHighlightCache`) растут без ограничений
- **Действие:** Добавить LRU cache, разбить на модули

#### sse-shared-worker.ts (1,029 строк)
- Смешивает SSE transport, state building, notification management
- Хорошо работает, но трудно поддерживать
- **Действие:** Разбить на `sseTransport.ts`, `workerStateManager.ts`, `portRouter.ts`

### 9.4 server.js (567 строк)
- **Хорошо написан!** ✅
- Чистая структура: parseArgs → helpers → builders → main
- Правильный graceful shutdown с process group kill
- Подробные комментарии с ссылками на issues
- Единственная претензия — нет TypeScript (но как bin-entry это нормально)

---

## 10. Code Style и Best Practices

### 10.1 Хорошие практики в проекте ✅

- Vue 3 Composition API с `<script setup>` везде
- TypeScript strict mode
- Proper `defineProps<T>()` и `defineEmits<T>()`
- `onBeforeUnmount()` cleanup в большинстве компонентов
- CSS variables для темизации (93 использования `--text-muted`)
- Web Workers для тяжёлых операций
- SharedWorker для SSE connections
- oxlint + oxfmt для линтинга и форматирования

### 10.2 Проблемы code style

#### Хаки типов (`as any`) — 17 мест

| Файл | Строка | Код | Проблема |
|------|--------|-----|----------|
| `App.vue` | 37 | `as any` для CSS custom properties | Нужен тип `CSSProperties` |
| `App.vue` | 1844 | `(providers_data[0] as any)?.default` | Нужен тип для provider response |
| `App.vue` | 3084 | `(terminal as any)._core` | Доступ к internal API xterm.js |
| `App.vue` | 4887, 4919 | `toolRendererHelpers as any` | Нужна типизация helpers |
| `useMessages.ts` | 604-606 | `_proxyMessages as any` и т.д. | Нужна generic обёртка |
| `useMessages.ts` | 611 | `as any)) as any` | Двойной каст — плохо |

#### Magic numbers

| Файл | Строка | Значение | Что это |
|------|--------|----------|---------|
| `App.vue` | 378 | `24` | FOLLOW_THRESHOLD_PX |
| `App.vue` | 379 | `840` | FILE_VIEWER_WINDOW_WIDTH |
| `App.vue` | 381-382 | `80`, `25` | TERM_COLUMNS, TERM_ROWS |
| `App.vue` | 391 | `1000` | SHELL_LINGER_MS |
| `App.vue` | 539 | `3000` | REASONING_CLOSE_DELAY_MS |
| `App.vue` | 3855 | `500` | DOUBLE_ESC_THRESHOLD |

Эти определены как const — хорошо. Но разбросаны по файлу, а не в одном месте.

**Действие:** Вынести все в `app/constants.ts`.

#### Inconsistent error handling

- Где-то `console.error()` + игнорирование
- Где-то `sendStatus.value = 'error'`  
- Где-то пустой `catch {}`
- Нет централизованного обработчика ошибок

**Действие:** Создать `useErrorHandler()` composable.

#### Missing JSDoc

200+ функций в App.vue без документации. Сложные функции вроде `parseCommitSnapshotOutput()`, `buildWorktreeSnapshotScript()`, `reloadSelectedSessionState()` не имеют ни строчки описания.

### 10.3 Naming conventions

**Хорошо:**
- `handle*()` для event handlers
- `resolve*()` для lookups
- `build*()` для builders
- `parse*()` для парсеров
- `use*()` для composables

**Проблемы:**
- Некоторые функции не следуют конвенции: `log()`, `fetchHistory()`, `loadAgents()`
- Переменные: mix of `camelCase` и `UPPER_SNAKE_CASE` для констант — OK

---

## 11. Производительность

### 11.1 Отсутствует виртуальный скроллинг

- **TreeView.vue** — рендерит все строки файлового дерева (может быть 1000+)
- **ThreadHistoryContent.vue** — рендерит все entries истории сразу
- **InputPanel.vue** — рендерит все элементы history/favorites в dropdown
- **OutputPanel.vue** — рендерит все ThreadBlock'и (указано в README TODO)

### 11.2 Отсутствует `v-memo`

- `ThreadBlock.vue` — дорогой рендер сообщений при каждом обновлении
- `TreeView.vue` — строки дерева пересоздаются при каждом изменении

### 11.3 Чрезмерные watchers

App.vue имеет 22 watcher'а, многие с overlapping dependencies. Например:
- watch на `[projectDirectory, activeDirectory, selectedSessionId]`
- watch на `selectedSessionId` отдельно
- watchEffect на множество refs одновременно

### 11.4 Unbounded кэши

| Файл | Кэш | Ограничение |
|------|-----|-------------|
| `render-worker.ts` | `codeHtmlCache` | Нет (растёт бесконечно) |
| `render-worker.ts` | `mdHighlightCache` | Нет |
| `useAssistantPreRenderer.ts` | `assistantHtmlCache` | Нет |
| `useAssistantPreRenderer.ts` | `deferredKeyCache` | Нет |
| `useMessages.ts` | `messageStores` | Нет (stores не удаляются) |

**Действие:** Добавить LRU-eviction или очистку при достижении порога.

### 11.5 Вызовы функций в template

- `TopPanel.vue` строка 180: `sessionShareHref(...)` в template — вызывается при каждом рендере
- `TreeView.vue` строка 295: `rowStatusClass(...)` в template — то же
- **Действие:** Перенести в computed properties

---

## 12. CSS и стили

### 12.1 Hardcoded colors (40+ мест)

Проект использует CSS variables для темизации, но во многих местах цвета захардкожены:

| Компонент | Примеры |
|-----------|---------|
| `ToolWindow/Bash.vue` | `color: #a5b4fc` |
| `ToolWindow/Permission.vue` | `rgba(14, 116, 144, 0.25)`, `rgba(34, 197, 94, 0.18)`, `rgba(239, 68, 68, 0.18)` |
| `ToolWindow/Question.vue` | 12 hardcoded rgba() значений |
| `renderers/DiffRenderer.vue` | `rgba(26, 29, 36, 0.95)` |
| `renderers/MarkdownRenderer.vue` | `color: #79b8ff` |
| `viewers/DiffViewer.vue` | 10+ hardcoded цветов для d2h |
| `CodeContent.vue` | `#8a8a8a`, `#2ea043`, `#aff5b4`, `#f85149`, `#ffdcd7` |
| `Dropdown/Item.vue` | `rgba(59, 130, 246, 0.2)` |

**Действие:** Заменить на CSS variables из темы (`--color-success`, `--color-error`, etc.)

### 12.2 Массивные `<style scoped>` блоки

| Компонент | Строк CSS | % от файла |
|-----------|-----------|------------|
| `InputPanel.vue` | 653 | 38% |
| `TreeView.vue` | 606 | 41% |
| `TopPanel.vue` | 572 | 46% |
| `App.vue` | 337 | 6% |
| `ThreadBlock.vue` | 196 | 26% |

**Действие:** При разбиении компонентов CSS уйдёт в свои части. Общие стили вынести в `tailwind.css` или shared CSS files.

### 12.3 Дублирование CSS стилей

- `.ib-action` button стили — дублируются между `ThreadBlock.vue` и `ThreadFooter.vue`
- `.todo-*` стили — дублируются между `InputPanel.vue` и `TodoPanel.vue`
- `.history-target-provider` — дублируется между `InputPanel.vue` и `ThreadTarget.vue`
- Diff line colors (`#2ea043`, `#f85149`) — дублируются между `CodeContent.vue` и `DiffViewer.vue`

### 12.4 Нет responsive дизайна

- Только `TopPanel.vue` имеет `hidden lg:block` — единственный адаптивный элемент
- Dropdown компоненты не адаптируются к мобильным экранам
- ProjectPicker модал не адаптируется к маленьким экранам

### 12.5 Accessibility

- `TreeView.vue`: `role="button"` на `<span>`, но нет keyboard handler
- `StatusBar.vue`: `role="status"` — хорошо ✅
- `InputPanel.vue`: Нет `aria-label` на многих кнопках
- В целом accessibility минимальная

---

## 13. План рефакторинга

### Фаза 1: Быстрые исправления (1-2 дня)

**13.1 Удалить мёртвый код**
- [ ] Удалить `resolveSessionTitle` из App.vue (строка 1367)
- [ ] Удалить `log()` заглушку из App.vue (строка 4221)
- [ ] Удалить `TOOL_RENDERER_READ_EVENT_TYPES`, `TOOL_RENDERER_WRITE_EVENT_TYPES` (App.vue строки 4225-4227)
- [ ] Удалить `toolRendererReadTypesKey`, `toolRendererWriteTypesKey`, `toolRendererMessageTypesKey` (App.vue строки 4236-4238)
- [ ] Удалить `openSubagentWindow` inject из ThreadHistoryContent.vue (строка 146)
- [ ] Удалить `upstreamRemote` из TreeView.vue (строка 437)
- [ ] Удалить 17 неиспользуемых экспортов из `utils/` (см. раздел 4.1)
- [ ] Удалить 15 неиспользуемых экспортов из `composables/` (см. раздел 4.2)

**13.2 Исправить TypeScript ошибки**
- [ ] Исправить `branch` на `SandboxState` (App.vue строка 1397)
- [ ] Исправить watch callback overload (App.vue строка 4281)
- [ ] Добавить `__TAURI_INTERNALS__` в global типы (InputPanel.vue строка 878)
- [ ] Исправить `PrimaryMode` типы в DiffViewer.vue (строки 23, 150, 191)
- [ ] Исправить `null` для `FloatingWindowAPI` (useFloatingWindow.ts строка 31)
- [ ] Исправить `closable` на `never` (useFloatingWindows.ts строка 221)
- [ ] Исправить `"top"` scroll mode (useSubagentWindows.ts строка 222)

**13.3 Устранить точные дупликаты**
- [ ] Удалить локальную `formatMessageTime()` из ThreadHistoryContent.vue, использовать из formatters.ts
- [ ] Вынести `matchesQuery()` в `utils/search.ts`
- [ ] Вынести `BITMAP_EXTENSIONS` и `IMAGE_EXTENSIONS` в `utils/fileTypes.ts`

### Фаза 2: Объединение ToolWindow компонентов (1 день)

- [ ] Создать `SimpleToolWindow.vue` взамен Bash/Default/Edit/Read/Task
- [ ] Создать `MarkdownToolWindow.vue` взамен Reasoning/Subagent
- [ ] Создать `SearchResultWindow.vue` взамен Glob/Grep

### Фаза 3: Разбиение App.vue (3-5 дней)

- [ ] Извлечь `useShellSessions()` composable (~500 строк)
- [ ] Извлечь `useGitOperations()` composable (~200 строк)
- [ ] Извлечь `useKeyboardShortcuts()` composable (~110 строк)
- [ ] Извлечь `useProviders()` composable (~200 строк)
- [ ] Извлечь `useMessageSending()` composable (~100 строк)
- [ ] Извлечь `useToolWindows()` composable (~300 строк)
- [ ] Извлечь `useSessionBoot()` composable (~200 строк)
- [ ] Вынести все magic numbers в `app/constants.ts`
- [ ] Перенести `useCodeRender.ts` из `utils/` в `composables/`

### Фаза 4: Разбиение компонентов-монолитов (3-5 дней)

- [ ] InputPanel.vue → InputPanel + HistoryDropdown + ModelSelector + AttachmentList
- [ ] TopPanel.vue → TopPanel + SessionPicker + NotificationButton + ProjectMenu
- [ ] TreeView.vue → TreeView + BranchPicker + GitStatusBar
- [ ] FloatingWindow.vue → FloatingWindow + useWindowDrag + WindowSearchBar
- [ ] ThreadBlock.vue → ThreadBlock + ThreadActions + ThreadHistory
- [ ] MarkdownRenderer.vue → MarkdownRenderer + useMarkdownCopy + useMarkdownHighlight

### Фаза 5: Абстракции и DRY (2-3 дня)

- [ ] Создать `usePromptQueue<T>()` из usePermissions + useQuestions
- [ ] Создать `useSessionWindows<T>()` из useReasoningWindows + useSubagentWindows
- [ ] Создать `app/injection-keys.ts` для типизированных provide/inject ключей
- [ ] Заменить hardcoded colors на CSS variables (40+ мест)
- [ ] Вынести дублированные CSS стили в shared файлы

### Фаза 6: Производительность (2-3 дня)

- [ ] Добавить LRU-ограничения для кэшей в render-worker, useAssistantPreRenderer
- [ ] Добавить cleanup для messageStores
- [ ] Добавить `v-memo` для дорогих списков (TreeView rows, ThreadBlock)
- [ ] Подготовить виртуальный скроллинг для file tree и message thread

### Фаза 7: Качество (1-2 дня)

- [ ] Создать `useErrorHandler()` composable
- [ ] Добавить JSDoc для ключевых функций (хотя бы 20 самых сложных)
- [ ] Заменить `as any` на proper типы (17 мест)
- [ ] Устранить `console.error/warn` в пользу централизованного логгера

---

## Итоговая оценка

| Область | Оценка | Комментарий |
|---------|--------|-------------|
| **Архитектура** | 5/10 | Composables паттерн правильный, но App.vue — монолит |
| **Code Quality** | 6/10 | TypeScript strict, но 12 ошибок + `as any` + мёртвый код |
| **DRY** | 4/10 | Много дупликатов: функции, компоненты, CSS |
| **Производительность** | 5/10 | Workers — хорошо, но нет virtual scroll и unbounded кэши |
| **CSS/Design** | 5/10 | CSS variables есть, но 40+ hardcoded colors |
| **Тестирование** | 0/10 | Тестов нет вообще |
| **Документация** | 3/10 | README хороший, но 200+ функций без JSDoc |
| **Безопасность** | 7/10 | Нет очевидных проблем, credentials в localStorage — стандарт |
| **Server** | 9/10 | server.js — отлично написан |
| **Общий итог** | **5/10** | Работает, но нуждается в серьёзном рефакторинге |

**Оценка трудозатрат на полный рефакторинг: 14-21 дней.**

---

## ПРОГРЕСС РЕФАКТОРИНГА

### Фаза 1 — ЗАВЕРШЕНА ✅ (2025-07-14)

Результат: **oxlint 0 errors, 0 warnings | vue-tsc 0 errors | vite build OK**

| Задача | Статус | Что сделано |
|--------|--------|------------|
| 1.1 Мёртвый код App.vue | ✅ | Удалён `resolveSessionTitle`; исправлен `sandbox.branch` → `sandbox.name` |
| 1.1 Мёртвый код компоненты | ✅ | Удалены `upstreamRemote` (TreeView), `openSubagentWindow` inject (ThreadHistoryContent) |
| 1.2 Неиспользуемые экспорты utils/ | ✅ | 5 функций удалено из opencode.ts; 3 из toolRenderers.ts; типы де-экспортированы в path, theme, sseConnection, notificationManager, useCodeRender |
| 1.3 Неиспользуемые экспорты composables/ | ✅ | `disposeMessages` удалён; 14 типов де-экспортированы; 3 unused type aliases полностью удалены |
| 1.4 types/sse.ts комментарий | ✅ | Добавлен комментарий «не удалять»; удалены `Message` из message.ts, `NotificationState` из worker-state.ts |
| 1.5 TypeScript ошибки (12→0) | ✅ | Исправлены: watch callback overload, sandbox.branch→name, DiffViewer PrimaryMode/ColorSchemeType, FloatingWindow closable/null, SubagentWindows scroll, Tauri globals |
| 1.6 Дупликаты | ✅ | Создано `utils/search.ts` (matchesQuery), `utils/fileTypes.ts` (BITMAP/IMAGE_EXTENSIONS); удалена копия formatMessageTime из ThreadHistoryContent |

### Новые файлы:
- `app/utils/search.ts` — shared `matchesQuery()` (из TopPanel + InputPanel)
- `app/utils/fileTypes.ts` — shared `BITMAP_EXTENSIONS`, `IMAGE_EXTENSIONS` (из ContentViewer + DiffViewer)
