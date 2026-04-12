# components/ — Vue 3 SFC Layer

50 components across 5 subdirectories + 18 root-level components.

## STRUCTURE

```
components/
├── ToolWindow/     # 13 SFCs — one per OpenCode tool type (see ToolWindow/AGENTS.md)
├── renderers/      # 5 SFCs — CodeRenderer, DiffRenderer, MarkdownRenderer, ImageRenderer, HexRenderer
├── viewers/        # 2 SFCs — ContentViewer (dispatches to renderer), DiffViewer (multi-file diffs)
├── Dropdown/       # 3+1 SFCs — Dropdown, Item, Search, Label (provide/inject API)
└── (root)          # 18 SFCs — layout panels, threads, modals
```

## COMPONENT HIERARCHY (App.vue children)

```
App.vue (6022 lines — orchestrator)
├── TopPanel          # Session tabs, project picker, notifications, controls
├── SidePanel         # File tree tab + todo list tab
│   ├── TreeView      # Git-aware file tree (1492 lines)
│   └── TodoPanel → TodoList
├── OutputPanel       # Message conversation area
│   ├── ThreadBlock   # User message + assistant response pair
│   │   ├── MessageViewer (user)
│   │   ├── ThreadTarget (agent/model badge)
│   │   └── MessageViewer (assistant)
│   └── StatusBar     # Thinking/streaming indicator
├── InputPanel        # User input with history, favorites, agent selector (1665 lines)
├── FloatingWindow[]  # Review-first tool output windows (736 lines)
│   └── ToolWindow/*  # Dynamic component per tool type
├── SessionTabBar → SessionTab[]
├── Welcome           # New session screen
├── ProjectPicker     # Project selection modal
├── SettingsModal     # App settings
└── ProjectSettingsDialog
```

## RENDERING PIPELINE

```
Tool output (HTML) → FloatingWindow → ToolWindow/* → CodeContent
Message (markdown) → MessageViewer → MarkdownRenderer or CodeRenderer → CodeContent
File viewer         → ContentViewer → CodeRenderer/ImageRenderer/HexRenderer
Diff viewer         → DiffViewer → DiffRenderer (with file tabs)
```

## LARGE COMPONENTS (>500 lines)

| Component | Lines | Known debt |
|-----------|-------|------------|
| App.vue | 6022 | Monolith — 184 functions, all state orchestration |
| InputPanel | 1665 | Mixes history, commands, file attachment |
| TreeView | 1492 | Mixes branch mgmt, file tree, git ops |
| TopPanel | 1236 | Mixes session tabs, notifications |
| FloatingWindow | 736 | Drag/resize could extract to composable |
| ThreadBlock | 671 | Content rendering mixed with actions |
| ProjectPicker | 653 | Project list + worktree ops combined |

## CONVENTIONS

- **renderers/** are stateless display primitives — props in, `rendered` event out
- **viewers/** select and switch between renderers based on file type
- **ToolWindow/** components receive `html` prop (pre-rendered by render-worker)
- **Dropdown/** uses provide/inject for parent-child communication
- Interactive components (Question, Permission) emit `reply`/`reject`; all others are read-only
