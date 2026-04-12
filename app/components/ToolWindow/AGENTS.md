# ToolWindow/ — Tool Output Viewers

8 SFCs for OpenCode tool types. Displayed inside FloatingWindow containers.
Most simple tools (bash, read, edit, task) render directly via CodeContent — no dedicated component.

## TOOL → COMPONENT MAP

| Tool name | Component | Interactive? | Key prop |
|-----------|-----------|-------------|----------|
| grep, codesearch | SearchResult.vue | no | `html`, `pattern`, `status`, `variant='code'` |
| glob, list | SearchResult.vue | no | `html`, `pattern`, `status`, `variant='term'` |
| webfetch, websearch | Web.vue | no | `html`, `url`/`query`, `status` |
| shell | Shell.vue | no | `shellId` (xterm.js) |
| permission | Permission.vue | **yes** | `request` → emits `reply` |
| question | Question.vue | **yes** | `request` → emits `reply`, `reject` |
| reasoning | MarkdownEntries.vue | no | `entries[]` (markdown) |
| subagent | MarkdownEntries.vue | no | `entries[]` (markdown) |
| bash, read, edit, task, etc. | *(CodeContent via FloatingWindow)* | no | `html` + `variant` |

## PATTERNS

- **Running status**: components with `status` prop show spinner/placeholder when `status === 'running'`
- **CodeContent**: most non-interactive components render pre-highlighted HTML via `<CodeContent :html="html" />`
- **Question drafts**: Question.vue persists answers to localStorage with 400ms debounce, restores on reopen
- **Permission metadata**: Permission.vue displays file patterns, path info, command details from request metadata
- **Shell terminal**: Shell.vue creates xterm.js instance, connects to PTY via WebSocket (`/api/pty/<id>/connect`)

## ADDING A NEW TOOL VIEWER

1. Create `app/components/ToolWindow/YourTool.vue`
2. Add extraction logic in `app/utils/toolRenderers.ts` → `extractFileRead()` switch
3. Register in `app/App.vue` where tool windows are opened (search for `ToolWindow`)
4. Pre-render HTML in `app/workers/render-worker.ts` if syntax highlighting needed
