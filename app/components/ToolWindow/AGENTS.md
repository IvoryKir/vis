# ToolWindow/ — Tool Output Viewers

13 SFCs, one per OpenCode tool type. Displayed inside FloatingWindow containers.

## TOOL → COMPONENT MAP

| Tool name | Component | Interactive? | Key prop |
|-----------|-----------|-------------|----------|
| bash | Bash.vue | no | `html`, `command`, `status` |
| read | Read.vue | no | `html` |
| edit, multiedit | Edit.vue | no | `html` (diff) |
| write | Edit.vue | no | `html` (diff) |
| grep, codesearch | Grep.vue | no | `html`, `pattern`, `status` |
| glob, list | Glob.vue | no | `html`, `pattern`, `status` |
| webfetch, websearch | Web.vue | no | `html`, `url`/`query`, `status` |
| task | Task.vue | no | `html` (terminal style) |
| shell | Shell.vue | no | `shellId` (xterm.js) |
| permission | Permission.vue | **yes** | `request` → emits `reply` |
| question | Question.vue | **yes** | `request` → emits `reply`, `reject` |
| reasoning | Reasoning.vue | no | `entries[]` (markdown) |
| subagent | Subagent.vue | no | `entries[]` (markdown) |
| (unknown) | Default.vue | no | `html` (fallback) |

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
