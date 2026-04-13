# Исследование: мгновенное переключение сессий (KeepAlive)

**Дата:** 2025-07-14
**Статус:** Исследование завершено, реализация ожидает отдельной сессии
**Приоритет:** High — прямое влияние на UX

---

## Проблема

При переключении между сессиями весь список сообщений **пересоздаётся с нуля**: Vue уничтожает все ThreadBlock для старой сессии и создаёт новые для текущей. Это вызывает видимый скролл сверху вниз и мерцание.

Пользователь ожидает поведение как вкладки в браузере: переключился — контент мгновенно на месте, без перерендера.

## Корневая причина

### Архитектура «один OutputPanel + proxy»

```
App.vue
  └─ OutputPanel (ОДИН экземпляр)
       └─ v-for root in visibleRoots
            └─ ThreadBlock :key="root.id"
```

`visibleRoots` берётся из глобального proxy (`useMessages()`), который переключает источник данных через `setActiveSession(sessionId)`. При переключении:

1. `_activeSessionId` меняется с "A" на "B"
2. `_proxyRoots` пересчитывается → массив с другими `root.id`
3. Vue видит новые `:key` → **уничтожает** все ThreadBlock сессии A, **создаёт** заново для B
4. Markdown перерендеривается, DOM строится сверху вниз
5. MutationObserver видит новый DOM → smooth scroll вниз → видимое мерцание

Это эквивалент перезагрузки страницы вместо переключения вкладки.

### Почему `reset()` маскировал проблему, но не решал

Оригинальный код делал `sessionStore.reset()` + `await fetchHistory()` при каждом переключении. Это стирало данные и загружало заново — скролл был **всегда**, но воспринимался как «загрузка». Без reset данные показывались из кэша мгновенно, но Vue всё равно пересоздавал DOM.

### Дополнительный баг: SSE scope leak (ИСПРАВЛЕН)

Все session stores биндились к одному `mainSessionScope`, фильтрующему по реактивному `selectedSessionId`. SSE-события текущей сессии протекали во все stores. **Исправлено:** каждый store теперь имеет фиксированный scope через `ge.mainSession(ref(sessionId))`.

## Решение: KeepAlive + session-bound wrapper

### Концепция

Вместо одного OutputPanel с переключаемым proxy — **отдельный OutputPanel-subtree на каждую сессию**, кэшируемый через `<KeepAlive>`.

### Архитектура (целевая)

```
App.vue
  └─ <KeepAlive :max="10">
       └─ <SessionOutputHost :key="selectedSessionId" :session-id="selectedSessionId">
            └─ provide(MessagesStoreKey, useSessionMessages(sessionId))
            └─ OutputPanel
                 └─ v-for root in visibleRoots  ← берёт из provided store
                      └─ ThreadBlock :key="root.id"
```

### План реализации

#### 1. Создать `SessionOutputHost.vue`

```vue
<template>
  <OutputPanel v-bind="$attrs" />
</template>

<script setup lang="ts">
import { provide } from 'vue';
import OutputPanel from './OutputPanel.vue';
import { useSessionMessages, MESSAGES_STORE_KEY } from '../composables/useMessages';

const props = defineProps<{ sessionId: string }>();
provide(MESSAGES_STORE_KEY, useSessionMessages(props.sessionId));
</script>
```

#### 2. Добавить provide/inject в `useMessages.ts`

```typescript
import { type InjectionKey, inject } from 'vue';

export const MESSAGES_STORE_KEY: InjectionKey<UseMessages> = Symbol('messages-store');

export function useMessages(): UseMessages;
export function useMessages(sessionId: string): UseMessages;
export function useMessages(sessionId?: string): UseMessages {
  if (sessionId !== undefined) {
    return useSessionMessages(sessionId);
  }
  // Попытка взять store из provide (для KeepAlive wrapper)
  const provided = inject(MESSAGES_STORE_KEY, null);
  if (provided) return provided;
  // Fallback: глобальный proxy (обратная совместимость)
  return activeProxy;
}
```

#### 3. Обернуть в App.vue

```vue
<KeepAlive :max="10">
  <SessionOutputHost
    :key="selectedSessionId"
    :session-id="selectedSessionId"
    :theme="shikiTheme"
    ...остальные пропсы...
    @message-rendered="handleOutputPanelMessageRendered"
    ...остальные эмиты...
  />
</KeepAlive>
```

#### 4. Убрать принудительный scroll-to-bottom при переключении

В `reloadSelectedSessionState()`:
- Убрать `resetFollow()` при переключении на закэшированную сессию
- Scroll state должен сохраняться per-session (пользователь был на середине — вернётся на середину)

#### 5. Per-session scroll state (опционально)

Если нужно сохранять позицию скролла per-session:
- `Map<sessionId, { scrollTop: number, isFollowing: boolean }>`
- Сохранять при deactivate, восстанавливать при activate

### Что это даёт

| Сценарий | Сейчас | После |
|----------|--------|-------|
| Переключение на посещённую сессию | Reset → fetch → rerender → scroll (~500ms+) | Мгновенно: KeepAlive показывает кэшированный DOM |
| Переключение на новую сессию | Fetch → render → scroll | Fetch → render (один раз, потом кэш) |
| Возврат к сессии | Полный перерендер | DOM из кэша, scroll position сохранена |

### Оценка трудозатрат

**Short (2-4 часа)**

1. `SessionOutputHost.vue` — 15 минут
2. Provide/inject в `useMessages.ts` — 30 минут
3. KeepAlive в `App.vue` — 30 минут
4. Убрать принудительный scroll при switch — 30 минут
5. Тестирование multi-session — 1-2 часа

### Watch out

1. **Auto-scroller привязан к одному DOM-элементу** — при KeepAlive нужно убедиться что MutationObserver переключается на активную панель
2. **`useMessages()` вызывается в ~15 компонентах** — provide/inject с fallback обеспечит обратную совместимость, но нужно проверить все use-cases
3. **Память** — 10 кэшированных сессий × ~100 ThreadBlock = ~1000 DOM-узлов в памяти. Приемлемо для десктопа.

---

## Сопутствующий фикс: SSE scope isolation (ВЫПОЛНЕН)

В ходе исследования обнаружен и исправлен реальный баг: все session stores биндились к одному `mainSessionScope`, что вызывало утечку SSE-событий между сессиями.

**Фикс:** per-session fixed scopes через `perSessionScopes` Map + `ge.mainSession(ref(sessionId))`.

Этот фикс остаётся независимо от реализации KeepAlive.
