# OpenCode App UI State Inventory

Research source: `anomalyco/opencode` `dev` branch, focused on `packages/app/src` with SDK endpoint names from `packages/sdk/js/src/v2/gen/sdk.gen.ts`.

## Shared Data Flow

The app uses a generated SDK client (`useGlobalSDK`, `useSDK`) and a single global SSE stream. `globalSDK.event.start()` calls `global.event()` (`GET /global/event`) and redispatches events by `directory`; `useSDK().event` scopes those events to the current directory. `useGlobalSync` bootstraps global data with `global.config.get`, `provider.list`, `path.get`, and `project.list`, then bootstraps per-directory stores with session, config, provider, agent, status, permission, question, MCP, LSP, VCS, and command data.

Global SSE events consumed include `sync` heartbeats, `server.connected`, `global.disposed`, and `project.updated`. Directory SSE events consumed include `server.instance.disposed`, `session.created`, `session.updated`, `session.deleted`, `session.diff`, `session.status`, `todo.updated`, `message.updated`, `message.removed`, `message.part.updated`, `message.part.removed`, `message.part.delta`, `vcs.branch.updated`, `permission.asked`, `permission.replied`, `question.asked`, `question.replied`, `question.rejected`, and `lsp.updated`. Terminal state also listens to `pty.exited` through the scoped SDK event bus.

## 1. Session List Sidebar

State sources: global/project layout state, per-directory child stores, notification state, permission state, and prefetched session/message caches. Rows are derived from root sessions plus child-session paths; row indicators show active work, pending approvals, unseen messages, and unseen errors.

API calls:

- Bootstrap: `session.list` (`GET /session`), with fallback/session limits in `loadRootSessionsWithFallback`.
- Hover/focus/pointer prefetch: `session.messages` (`GET /session/{sessionID}/message`) and sometimes `session.get` (`GET /session/{sessionID}`) for warm neighboring sessions.
- Open project/workspace setup: `project.list`, `project.current`, `path.get`, `config.get`, `provider.list`, `app.agents`, `session.status`, `vcs.get`, `command.list`, `permission.list`, `question.list`, `mcp.status`, `lsp.status`.
- Archive row action: `session.update` (`PATCH /session/{sessionID}`) with `time.archived`.
- Project/workspace actions in the same layout shell can call `project.update`, `worktree.list`, `worktree.create`, and session restore/init paths.

SSE events consumed:

- `session.created`, `session.updated`, `session.deleted` mutate the sidebar list and counts.
- `message.updated`, `session.status`, `permission.asked`, `permission.replied`, `question.asked`, `question.replied`, `question.rejected` drive row badges/spinners.
- `worktree.ready`, `worktree.failed`, `project.updated`, `server.connected`, and `global.disposed` refresh workspace/project state.

User interactions that trigger API calls:

- Hover/focus/pointer down on a session warms that session and adjacent sessions via `session.messages` prefetch.
- Clicking a session navigates; route activation triggers `sync.session.sync`, which may call `session.get` and `session.messages`.
- Clicking archive calls `session.update`.
- Sidebar project/workspace management actions call project/worktree/session APIs depending on the selected action.

## 2. Chat Message Thread States

State sources: `sync.data.message[sessionID]`, `sync.data.part[messageID]`, `sync.data.session_status[sessionID]`, `sync.data.session_diff[sessionID]`, `sync.data.todo[sessionID]`, pending optimistic messages, local prompt state, and settings controlling reasoning/tool expansion.

API calls:

- Session entry/load: `session.get` (`GET /session/{sessionID}`) and `session.messages` (`GET /session/{sessionID}/message`) with `limit`/`before` cursor pagination.
- Load earlier/history: more `session.messages` calls with `before` cursor.
- Send normal user prompt: `session.create` (`POST /session`) for new sessions, then `session.promptAsync` (`POST /session/{sessionID}/prompt_async`) with generated `messageID`, model, agent, variant, and parts.
- Send shell-mode prompt: `session.shell` (`POST /session/{sessionID}/shell`).
- Slash/custom command prompt: `session.command` (`POST /session/{sessionID}/command`).
- Abort active work: `session.abort` (`POST /session/{sessionID}/abort`).
- Share/unshare: `session.share` and `session.unshare` (`POST`/`DELETE /session/{sessionID}/share`).
- Rename/archive/delete: `session.update` (`PATCH /session/{sessionID}`) and `session.delete` (`DELETE /session/{sessionID}`).
- Undo/redo/revert: `session.revert` and `session.unrevert`.
- Compact: `session.summarize`.
- Review/diff/file side panel: `session.diff`, `session.todo`, and `file.content`/`file.status` as files are opened.

Rendered states:

- User messages are shown immediately from fetched or optimistic message records.
- Assistant streaming is represented by an assistant message without `time.completed` plus `session.status` not idle; the active turn shows spinner/progress.
- Tool calls/results/diffs/errors are part records rendered by `SessionTurn`; part data comes from `message.part.updated` snapshots and `message.part.delta` text deltas. Patch/step-start/step-finish parts are intentionally skipped in app stores.
- Diffs are held in `session_diff` from `session.diff` API or `session.diff` SSE.
- Errors are displayed from message/part state and request-failure toasts; unseen error badges are managed by notification state.

SSE events consumed:

- `message.updated`, `message.removed`, `message.part.updated`, `message.part.delta`, `message.part.removed` update the timeline.
- `session.status` controls busy/retry/idle UI, progress bar, and composer state.
- `session.diff` updates review/diff state.
- `todo.updated` updates the todo dock.
- `permission.asked`/`question.asked` can block the composer and add overlays.

User interactions that trigger API calls:

- Submit prompt, shell prompt, or slash command triggers `session.create` when needed and then `session.promptAsync`, `session.shell`, or `session.command`.
- Empty-submit while working triggers `session.abort`.
- Load earlier triggers paginated `session.messages`.
- Rename/share/unshare/archive/delete menu actions trigger their matching session APIs.
- Undo/redo/compact/fork/review actions trigger `session.abort`, `session.revert`, `session.unrevert`, `session.summarize`, `session.fork`, or `session.diff` depending on command.

## 3. Tool Approval Overlay

State sources: `sync.data.permission[sessionID]`, session hierarchy lookup in `session-request-tree`, and auto-accept state in `PermissionProvider`. The UI is `SessionPermissionDock`, shown in the composer dock and blocks the prompt unless the current session is a child session.

API calls:

- Bootstrap/list: `permission.list` (`GET /permission`).
- User decision: `permission.respond` (`POST /session/{sessionID}/permissions/{permissionID}`) with `response: once | always | reject`.
- Auto-accept enabling checks outstanding permissions with `permission.list({ directory })` and responds with `permission.respond`.

SSE events consumed:

- `permission.asked` inserts or updates a pending permission request.
- `permission.replied` removes the request and dismisses notification/toast state.

User interactions that trigger API calls:

- `Deny`, `Allow Always`, and `Allow Once` call `permission.respond`.
- Toggling auto-accept in settings/commands can call `permission.list` and then auto-respond with `permission.respond` for outstanding matching requests.

## 4. Question Prompt Overlay

State sources: `sync.data.question[sessionID]`, request-local cache for partially answered multi-step forms, and `SessionQuestionDock` local tab/answer/editing state. The dock supports single-choice, multi-choice, and custom answers.

API calls:

- Bootstrap/list: `question.list` (`GET /question`).
- Submit answers: `question.reply` (`POST /question/{requestID}/reply`).
- Dismiss/reject: `question.reject` (`POST /question/{requestID}/reject`).

SSE events consumed:

- `question.asked` inserts or updates a request.
- `question.replied` and `question.rejected` remove the request and dismiss notification state.

User interactions that trigger API calls:

- Submit on final question or `Ctrl/Cmd+Enter` calls `question.reply`.
- Dismiss button or `Escape` calls `question.reject`.
- Selecting options, custom answers, progress segments, Back/Next are local until submit/reject.

## 5. PTY Terminal Panel

State sources: workspace-persisted terminal tabs, active terminal ID, terminal buffer/cursor/size snapshot, `view().terminal.opened()`, and `Terminal` WebSocket state. The panel auto-creates a terminal when opened with no terminals.

API calls and sockets:

- Discover shells for settings: `pty.shells` (`GET /pty/shells`).
- Create terminal: `pty.create` (`POST /pty`).
- Update title/size: `pty.update` (`PATCH /pty/{ptyID}`), throttled for terminal resize.
- Inspect missing terminal after WebSocket failure: `pty.get` (`GET /pty/{ptyID}`).
- Clone recovery after connect error: `pty.create` again.
- Terminal I/O: WebSocket to `/pty/{ptyID}/connect?directory=...&cursor=...`; data frames write terminal output, binary control frames update cursor.

SSE events consumed:

- `pty.exited` removes the exited local terminal tab.

User interactions that trigger API calls:

- Opening terminal when empty calls `pty.create`.
- `+` terminal tab button calls `pty.create`.
- Resizing the terminal surface calls throttled `pty.update`.
- Renaming or cleanup persistence calls `pty.update` with title/size.
- Typing sends bytes through the WebSocket, not the REST SDK.
- Connection failures call `pty.get`; if missing/unusable, the panel clones with `pty.create`.

## 6. Provider/Model Switcher

State sources: provider lists from `useProviders`, model visibility/recent/variant persisted by `ModelsProvider`, local current agent/model state, and global/per-directory provider bootstrap state. The popover/dialog lists visible connected models grouped by provider and exposes provider connect/manage actions.

API calls:

- Bootstrap/list: `provider.list` (`GET /provider`) globally and per directory.
- Provider auth methods: `provider.auth` (`GET /provider/auth`).
- OAuth start: `provider.oauth.authorize` (`POST /provider/{providerID}/oauth/authorize`).
- Auth removal: `auth.remove` (`DELETE /auth/{providerID}`).
- After connect/disconnect: `global.dispose` (`POST /global/dispose`) to force server/provider refresh.
- Custom provider/config changes use `global.config.update` (`PATCH /global/config`) through settings/config flows.

SSE events consumed:

- `global.disposed` and `server.connected` trigger global/bootstrap refresh.
- Per-directory provider state refresh is also part of child bootstrap after disposal/reconnect.

User interactions that trigger API calls:

- Selecting a model updates local persisted current/recent model only; no API call.
- Opening connect provider may fetch `provider.auth` if methods are not cached.
- Selecting OAuth/API auth method calls `provider.oauth.authorize`; completion calls `global.dispose`.
- Disconnect provider calls `auth.remove`; for normal providers it then calls `global.dispose`; for config custom providers it disables provider via `global.config.update`.
- Manage/show/hide models toggles local visibility only; no API call.

## 7. Settings Panel

State sources: `settings.v3` persisted UI settings, theme context, platform APIs, permission auto-accept state, global config, provider/model state, and command registry/keybind overrides.

API calls:

- General tab: `pty.shells` loads shell options; changing shell calls `global.config.update` (`PATCH /global/config`). Desktop update check uses platform APIs, not SDK.
- Permissions auto-accept: enabling may call `permission.list` and `permission.respond` for currently pending items.
- Providers tab: `provider.list` via bootstrap, `provider.auth`, `provider.oauth.authorize`, `auth.remove`, `global.dispose`, and `global.config.update` for custom/disabled provider changes.
- Models tab: no SDK calls for visibility; uses provider/model data already loaded.
- Shortcuts tab: no SDK calls; changes are persisted locally in `settings.v3`.

SSE events consumed:

- Settings itself does not subscribe directly beyond shared app contexts, but provider/server changes from `global.disposed`, `server.connected`, and `project.updated` refresh underlying lists.
- Pending permission SSE can affect the auto-accept controls and notification state.

User interactions that trigger API calls:

- Change shell: `global.config.update`.
- Toggle auto-accept on: possible `permission.list` plus `permission.respond` for outstanding matching requests.
- Connect/disconnect provider/custom provider: provider/auth/config/dispose calls listed above.
- Theme, color scheme, language, fonts, sounds, notification switches, feed/tool expansion, model visibility, and keybind capture are local/platform state only.

## 8. Status Bar / Status Popover

State sources: server health (`useServer` plus polling), `sync.data.mcp`, `sync.data.lsp`, `sync.data.config.plugin`, and selected server state. Trigger badge is green for healthy connected server/no MCP issues, red for unhealthy server or MCP issue, gray while unknown/not ready.

API calls:

- Server health polling uses `useCheckServerHealth` against each configured server; this is a direct health request, not the generated SDK client.
- MCP tab toggles: `mcp.connect` (`POST /mcp/{name}/connect`) and `mcp.disconnect` (`POST /mcp/{name}/disconnect`), followed by refetching `mcp.status` (`GET /mcp`).
- Initial/refresh status data: `mcp.status` and `lsp.status` (`GET /lsp`) from child bootstrap/query cache.
- Manage servers dialog uses platform/server persistence, not core SDK calls unless server selection causes global bootstrap against a different server.

SSE events consumed:

- `lsp.updated` refetches `lsp.status`.
- `server.connected` and `global.disposed` trigger refresh/bootstrap.
- MCP status is primarily query/refetch driven; the popover reads `sync.data.mcp`.

User interactions that trigger API calls:

- Opening the popover starts health polling for configured servers and lazy-loads body UI.
- Clicking an MCP row or switch calls `mcp.connect`/`mcp.disconnect` and refetches `mcp.status`.
- Selecting another healthy server switches active server and causes app/global bootstrap against that server.
- Manage Servers opens a dialog; server changes are platform/server-context operations.
