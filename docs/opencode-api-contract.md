# opencode API Contract

Source researched: `anomalyco/opencode` at current `main` clone on 2026-04-28. Focus files:

- `packages/opencode/src/server/server.ts`
- `packages/opencode/src/server/routes/global.ts`
- `packages/opencode/src/server/routes/instance/{event,session,tui,pty,permission,question,provider}.ts`
- `packages/opencode/src/{pty,permission,question,provider}/**`

This document describes the legacy Hono HTTP API created by `Server.createHono`. If `OPENCODE_EXPERIMENTAL_HTTPAPI` is set, opencode switches to a separate Effect HttpApi surface; this contract is for the default Hono surface.

## Routing Model

`server.ts` always mounts `GlobalRoutes` at `/global`.

When `OPENCODE_WORKSPACE_ID` is set, the server mounts only the instance API at `/` after `InstanceMiddleware` and `FenceMiddleware`.

When `OPENCODE_WORKSPACE_ID` is not set, the server mounts:

- Control-plane routes at `/`.
- Workspace legacy routes at `/experimental/workspace` plus workspace router middleware.
- Instance routes at `/`.
- Web UI catch-all at `/*`.

Instance routes relevant to this contract are mounted as:

- `/event`
- `/session/*`
- `/permission/*`
- `/question/*`
- `/provider/*`
- `/pty/*`
- `/tui/*`
- Also instance utility routes from `routes/instance/index.ts`: `/instance/dispose`, `/path`, `/vcs`, `/vcs/diff`, `/command`, `/agent`, `/skill`, `/lsp`, `/formatter`.

## Auth

Auth is global middleware in `server.ts` and applies to all Hono routes except `OPTIONS` preflight.

- If `OPENCODE_SERVER_PASSWORD` is unset, the server is unsecured.
- If `OPENCODE_SERVER_PASSWORD` is set, all non-`OPTIONS` requests require HTTP Basic auth.
- Username defaults to `opencode`; override with `OPENCODE_SERVER_USERNAME`.
- Header form: `Authorization: Basic base64("<username>:<password>")`.
- Query-token form: `?auth_token=<basic-token>` where `<basic-token>` is the base64 payload normally placed after `Basic `. The middleware rewrites this query value into `Authorization: Basic <auth_token>`.
- Browser CORS preflight is allowed without auth.
- Compression is disabled for `/event`, `/global/event`, and `POST /session/:sessionID/(message|prompt_async)`.

## Shared Schemas

Identifier-like fields are strings. Generated IDs use typed prefixes internally, but clients should treat them as opaque strings.

Common aliases:

```ts
type SessionID = string
type MessageID = string
type PartID = string
type PermissionID = string
type QuestionID = string
type PtyID = string
type ProviderID = string
type ModelID = string
type WorkspaceID = string
type ProjectID = string
```

Error responses are JSON `NamedError` objects for most handled errors. Known status mappings:

- `404` for storage/not-found errors.
- `400` for provider model not found, provider auth validation failure, worktree errors, session busy, and route validation failures.
- `500` for unknown errors.

### PermissionRule and Ruleset

```ts
type PermissionAction = "allow" | "deny" | "ask"
type PermissionRule = { permission: string; pattern: string; action: PermissionAction }
type PermissionRuleset = PermissionRule[]
```

### Session

```ts
type Session = {
  id: SessionID
  slug: string
  projectID: ProjectID
  workspaceID?: WorkspaceID
  directory: string
  path?: string
  parentID?: SessionID
  summary?: { additions: number; deletions: number; files: number; diffs?: FileDiff[] }
  share?: { url: string }
  title: string
  version: string
  time: { created: number; updated: number; compacting?: number; archived?: number }
  permission?: PermissionRuleset
  revert?: { messageID: MessageID; partID?: PartID; snapshot?: string; diff?: string }
}
```

### Message and Part

Stored messages are returned as `{ info, parts }`.

```ts
type MessageWithParts = { info: UserMessage | AssistantMessage; parts: Part[] }

type UserMessage = {
  id: MessageID
  sessionID: SessionID
  role: "user"
  time: { created: number }
  format?: OutputFormat
  summary?: { title?: string; body?: string; diffs: FileDiff[] }
  agent: string
  model: { providerID: ProviderID; modelID: ModelID; variant?: string }
  system?: string
  tools?: Record<string, boolean>
}

type AssistantMessage = {
  id: MessageID
  sessionID: SessionID
  role: "assistant"
  time: { created: number; completed?: number }
  error?: unknown
  parentID: MessageID
  modelID: ModelID
  providerID: ProviderID
  mode: string
  agent: string
  path: { cwd: string; root: string }
  summary?: boolean
  cost: number
  tokens: { total?: number; input: number; output: number; reasoning: number; cache: { read: number; write: number } }
  structured?: unknown
  variant?: string
  finish?: string
}

type OutputFormat = { type: "text" } | { type: "json_schema"; schema: Record<string, unknown>; retryCount?: number }
```

`Part` is a discriminated union by `type`:

```ts
type Part =
  | { id: PartID; sessionID: SessionID; messageID: MessageID; type: "text"; text: string; synthetic?: boolean; ignored?: boolean; time?: { start: number; end?: number }; metadata?: Record<string, unknown> }
  | { id: PartID; sessionID: SessionID; messageID: MessageID; type: "reasoning"; text: string; metadata?: Record<string, unknown>; time: { start: number; end?: number } }
  | { id: PartID; sessionID: SessionID; messageID: MessageID; type: "file"; mime: string; filename?: string; url: string; source?: FilePartSource }
  | { id: PartID; sessionID: SessionID; messageID: MessageID; type: "agent"; name: string; source?: { value: string; start: number; end: number } }
  | { id: PartID; sessionID: SessionID; messageID: MessageID; type: "subtask"; prompt: string; description: string; agent: string; model?: { providerID: ProviderID; modelID: ModelID }; command?: string }
  | { id: PartID; sessionID: SessionID; messageID: MessageID; type: "tool"; callID: string; tool: string; state: ToolState; metadata?: Record<string, unknown> }
  | { id: PartID; sessionID: SessionID; messageID: MessageID; type: "step-start"; snapshot?: string }
  | { id: PartID; sessionID: SessionID; messageID: MessageID; type: "step-finish"; reason: string; snapshot?: string; cost: number; tokens: TokenUsage }
  | { id: PartID; sessionID: SessionID; messageID: MessageID; type: "snapshot"; snapshot: string }
  | { id: PartID; sessionID: SessionID; messageID: MessageID; type: "patch"; hash: string; files: string[] }
  | { id: PartID; sessionID: SessionID; messageID: MessageID; type: "retry"; attempt: number; error: unknown; time: { created: number } }
  | { id: PartID; sessionID: SessionID; messageID: MessageID; type: "compaction"; auto: boolean; overflow?: boolean; tail_start_id?: MessageID }

type ToolState =
  | { status: "pending"; input: Record<string, unknown>; raw: string }
  | { status: "running"; input: Record<string, unknown>; title?: string; metadata?: Record<string, unknown>; time: { start: number } }
  | { status: "completed"; input: Record<string, unknown>; output: string; title: string; metadata: Record<string, unknown>; time: { start: number; end: number; compacted?: number }; attachments?: Part[] }
  | { status: "error"; input: Record<string, unknown>; error: string; metadata?: Record<string, unknown>; time: { start: number; end: number } }
```

Prompt part inputs omit `sessionID` and `messageID`, and may omit `id`:

```ts
type PromptPartInput =
  | { id?: PartID; type: "text"; text: string; synthetic?: boolean; ignored?: boolean; time?: { start: number; end?: number }; metadata?: Record<string, unknown> }
  | { id?: PartID; type: "file"; mime: string; filename?: string; url: string; source?: FilePartSource }
  | { id?: PartID; type: "agent"; name: string; source?: { value: string; start: number; end: number } }
  | { id?: PartID; type: "subtask"; prompt: string; description: string; agent: string; model?: { providerID: ProviderID; modelID: ModelID }; command?: string }
```

## HTTP Endpoints

### Global Routes

#### `GET /global/health`

Request: none.

Response `200`:

```ts
{ healthy: true; version: string }
```

#### `GET /global/event`

SSE endpoint. See SSE section.

#### `GET /global/config`

Request: none.

Response `200`: `Config.Info` JSON object. The exact config shape is defined by `Config.Info.zod` in `config/config.ts` and includes the user-visible opencode configuration.

#### `PATCH /global/config`

Request JSON: full `Config.Info` object.

Response `200`: updated `Config.Info` object.

#### `POST /global/dispose`

Request: none.

Response `200`: `true`.

Side effect: disposes all instances and emits global event `global.disposed`.

#### `POST /global/upgrade`

Request JSON:

```ts
{ target?: string }
```

Response `200` on success:

```ts
{ success: true; version: string }
```

Response `400` or `500` on failure:

```ts
{ success: false; error: string }
```

### Instance Utility Routes

#### `POST /instance/dispose`

Request: none.

Response `200`: `true`.

#### `GET /path`

Request: none.

Response `200`:

```ts
{ home: string; state: string; config: string; worktree: string; directory: string }
```

#### `GET /vcs`

Request: none.

Response `200`:

```ts
{ branch: string; default_branch: string }
```

#### `GET /vcs/diff?mode=<mode>`

Request query:

```ts
{ mode: VcsMode }
```

Response `200`: `FileDiff[]`.

#### `GET /command`

Request: none.

Response `200`: `Command.Info[]`.

#### `GET /agent`

Request: none.

Response `200`: `Agent.Info[]`.

#### `GET /skill`

Request: none.

Response `200`: `Skill.Info[]`.

#### `GET /lsp`

Request: none.

Response `200`: `LSP.Status[]`.

#### `GET /formatter`

Request: none.

Response `200`: `Format.Status[]`.

### Session Routes

#### `GET /session/`

Request query:

```ts
{
  directory?: string
  scope?: "project"
  path?: string
  roots?: boolean | "true" | "false"
  start?: number
  search?: string
  limit?: number
}
```

Response `200`: `Session[]`, sorted by most recently updated.

#### `GET /session/status`

Request: none.

Response `200`:

```ts
Record<SessionID, { type: "idle" } | { type: "busy" } | { type: "retry"; attempt: number; message: string; next: number }>
```

#### `POST /session/`

Request JSON:

```ts
{ parentID?: SessionID; title?: string; permission?: PermissionRuleset; workspaceID?: WorkspaceID } | undefined
```

Response `200`: `Session`.

#### `GET /session/:sessionID`

Request path: `sessionID`.

Response `200`: `Session`.

#### `PATCH /session/:sessionID`

Request JSON:

```ts
{ title?: string; permission?: PermissionRuleset; time?: { archived?: number } }
```

Response `200`: updated `Session`.

Notes: `permission` is merged with the existing session ruleset.

#### `DELETE /session/:sessionID`

Request path: `sessionID`.

Response `200`: `true`.

#### `GET /session/:sessionID/children`

Request path: `sessionID`.

Response `200`: `Session[]`.

#### `GET /session/:sessionID/todo`

Request path: `sessionID`.

Response `200`:

```ts
{ content: string; status: string; priority: string }[]
```

#### `POST /session/:sessionID/init`

Request JSON:

```ts
{ modelID: ModelID; providerID: ProviderID; messageID: MessageID }
```

Response `200`: `true`.

Side effect: runs the built-in `INIT` command.

#### `POST /session/:sessionID/fork`

Request JSON:

```ts
{ messageID?: MessageID }
```

Response `200`: child `Session`.

#### `POST /session/:sessionID/abort`

Request path: `sessionID`.

Response `200`: `true`.

Side effect: cancels active processing for the session.

#### `POST /session/:sessionID/share`

Request path: `sessionID`.

Response `200`: updated `Session` with `share` data.

#### `DELETE /session/:sessionID/share`

Request path: `sessionID`.

Response `200`: updated `Session` without share data.

#### `GET /session/:sessionID/diff?messageID=<messageID>`

Request query:

```ts
{ messageID: MessageID }
```

Response `200`: `FileDiff[]`.

#### `POST /session/:sessionID/summarize`

Request JSON:

```ts
{ providerID: ProviderID; modelID: ModelID; auto?: boolean }
```

Response `200`: `true`.

#### `GET /session/:sessionID/message`

Request query:

```ts
{ limit?: number; before?: string }
```

Response `200`: `MessageWithParts[]`.

Pagination behavior:

- If `limit` is absent or `0`, returns all messages.
- If `limit` is present and a next page exists, response exposes `Link: <url>; rel="next"` and `X-Next-Cursor`.
- `before` is an opaque base64url cursor and requires `limit`.

#### `GET /session/:sessionID/message/:messageID`

Request path: `sessionID`, `messageID`.

Response `200`: `MessageWithParts`.

#### `DELETE /session/:sessionID/message/:messageID`

Request path: `sessionID`, `messageID`.

Response `200`: `true`.

Notes: asserts the session is not busy. Does not revert file changes.

#### `DELETE /session/:sessionID/message/:messageID/part/:partID`

Request path: `sessionID`, `messageID`, `partID`.

Response `200`: `true`.

#### `PATCH /session/:sessionID/message/:messageID/part/:partID`

Request JSON: full stored `Part` object whose `id`, `messageID`, and `sessionID` match the path.

Response `200`: updated `Part`.

#### `POST /session/:sessionID/message`

Request JSON:

```ts
{
  messageID?: MessageID
  model?: { providerID: ProviderID; modelID: ModelID }
  agent?: string
  noReply?: boolean
  tools?: Record<string, boolean>
  format?: OutputFormat
  system?: string
  variant?: string
  parts: PromptPartInput[]
}
```

Response `200`: `MessageWithParts` where `info` is an assistant message. The handler uses an HTTP streamed response with `Content-Type: application/json`, but writes one final JSON object, not token chunks. Realtime progress is delivered on `/event`.

#### `POST /session/:sessionID/prompt_async`

Request JSON: same as `POST /session/:sessionID/message`.

Response `204` with no body.

Side effect: starts prompt processing in the background. Failures publish `session.error` on the bus.

#### `POST /session/:sessionID/command`

Request JSON:

```ts
{
  messageID?: MessageID
  agent?: string
  model?: string
  arguments: string
  command: string
  variant?: string
  parts?: { id?: PartID; type: "file"; mime: string; filename?: string; url: string; source?: FilePartSource }[]
}
```

Response `200`: `MessageWithParts` where `info` is an assistant message.

#### `POST /session/:sessionID/shell`

Request JSON:

```ts
{ messageID?: MessageID; agent: string; model?: { providerID: ProviderID; modelID: ModelID }; command: string }
```

Response `200`: `MessageWithParts`.

#### `POST /session/:sessionID/revert`

Request JSON: `SessionRevert.RevertInput` without `sessionID`; practically includes at least the message/part snapshot target used by revert.

Response `200`: updated `Session`.

#### `POST /session/:sessionID/unrevert`

Request path: `sessionID`.

Response `200`: updated `Session`.

#### `POST /session/:sessionID/permissions/:permissionID`

Deprecated alias for permission reply.

Request JSON:

```ts
{ response: "once" | "always" | "reject" }
```

Response `200`: `true`.

### Permission Routes

#### `GET /permission/`

Request: none.

Response `200`:

```ts
type PermissionRequest = {
  id: PermissionID
  sessionID: SessionID
  permission: string
  patterns: string[]
  metadata: Record<string, unknown>
  always: string[]
  tool?: { messageID: MessageID; callID: string }
}

PermissionRequest[]
```

#### `POST /permission/:requestID/reply`

Request JSON:

```ts
{ reply: "once" | "always" | "reject"; message?: string }
```

Response `200`: `true`.

### Question Routes

#### `GET /question/`

Request: none.

Response `200`:

```ts
type QuestionRequest = {
  id: QuestionID
  sessionID: SessionID
  questions: {
    question: string
    header: string
    options: { label: string; description: string }[]
    multiple?: boolean
    custom?: boolean
  }[]
  tool?: { messageID: MessageID; callID: string }
}

QuestionRequest[]
```

#### `POST /question/:requestID/reply`

Request JSON:

```ts
{ answers: string[][] }
```

Each inner array is the selected labels for the corresponding question.

Response `200`: `true`.

#### `POST /question/:requestID/reject`

Request path: `requestID`.

Response `200`: `true`.

### Provider Routes

#### `GET /provider/`

Request: none.

Response `200`:

```ts
type ProviderListResult = {
  all: ProviderInfo[]
  default: Record<string, string>
  connected: string[]
}

type ProviderInfo = {
  id: ProviderID
  name: string
  source: "env" | "config" | "custom" | "api"
  env: string[]
  key?: string
  options: Record<string, unknown>
  models: Record<string, Model>
}

type Model = {
  id: ModelID
  providerID: ProviderID
  api: { id: string; url: string; npm: string }
  name: string
  family?: string
  capabilities: {
    temperature: boolean
    reasoning: boolean
    attachment: boolean
    toolcall: boolean
    input: { text: boolean; audio: boolean; image: boolean; video: boolean; pdf: boolean }
    output: { text: boolean; audio: boolean; image: boolean; video: boolean; pdf: boolean }
    interleaved: boolean | { field: "reasoning_content" | "reasoning_details" }
  }
  cost: { input: number; output: number; cache: { read: number; write: number }; experimentalOver200K?: { input: number; output: number; cache: { read: number; write: number } } }
  limit: { context: number; input?: number; output: number }
  status: "alpha" | "beta" | "deprecated" | "active"
  options: Record<string, unknown>
  headers: Record<string, string>
  release_date: string
  variants?: Record<string, Record<string, unknown>>
}
```

#### `GET /provider/auth`

Request: none.

Response `200`:

```ts
Record<string, {
  type: "oauth" | "api"
  label: string
  prompts?: (
    | { type: "text"; key: string; message: string; placeholder?: string; when?: { key: string; op: "eq" | "neq"; value: string } }
    | { type: "select"; key: string; message: string; options: { label: string; value: string; hint?: string }[]; when?: { key: string; op: "eq" | "neq"; value: string } }
  )[]
}[]>
```

#### `POST /provider/:providerID/oauth/authorize`

Request JSON:

```ts
{ method: number; inputs?: Record<string, string> }
```

Response `200`:

```ts
{ url: string; method: "auto" | "code"; instructions: string } | undefined
```

#### `POST /provider/:providerID/oauth/callback`

Request JSON:

```ts
{ method: number; code?: string }
```

Response `200`: `true`.

### PTY Routes

#### `GET /pty/shells`

Request: none.

Response `200`:

```ts
{ path: string; name: string; acceptable: boolean }[]
```

#### `GET /pty/`

Request: none.

Response `200`: `PtyInfo[]`.

#### `POST /pty/`

Request JSON:

```ts
{ command?: string; args?: string[]; cwd?: string; title?: string; env?: Record<string, string> }
```

Response `200`:

```ts
type PtyInfo = { id: PtyID; title: string; command: string; args: string[]; cwd: string; status: "running" | "exited"; pid: number }
```

#### `GET /pty/:ptyID`

Request path: `ptyID`.

Response `200`: `PtyInfo`.

#### `PUT /pty/:ptyID`

Request JSON:

```ts
{ title?: string; size?: { rows: number; cols: number } }
```

Response `200`: updated `PtyInfo | undefined`.

#### `DELETE /pty/:ptyID`

Request path: `ptyID`.

Response `200`: `true`.

#### `GET /pty/:ptyID/connect?cursor=<cursor>`

WebSocket endpoint. See PTY WebSocket section.

### TUI Routes

These routes publish TUI events onto the instance bus. They are intended for controlling an existing TUI, not for general new-client state synchronization.

#### `POST /tui/append-prompt`

Request JSON: `{ text: string }`.

Response `200`: `true`.

Publishes `tui.prompt.append`.

#### `POST /tui/open-help`

Request: none.

Response `200`: `true`.

Publishes `tui.command.execute` with `command: "help.show"`.

#### `POST /tui/open-sessions`

Request: none.

Response `200`: `true`.

Publishes `tui.command.execute` with `command: "session.list"`.

#### `POST /tui/open-themes`

Request: none.

Response `200`: `true`.

Publishes `tui.command.execute` with `command: "session.list"` in the current source.

#### `POST /tui/open-models`

Request: none.

Response `200`: `true`.

Publishes `tui.command.execute` with `command: "model.list"`.

#### `POST /tui/submit-prompt`

Request: none.

Response `200`: `true`.

Publishes `tui.command.execute` with `command: "prompt.submit"`.

#### `POST /tui/clear-prompt`

Request: none.

Response `200`: `true`.

Publishes `tui.command.execute` with `command: "prompt.clear"`.

#### `POST /tui/execute-command`

Request JSON:

```ts
{ command: string }
```

Response `200`: `true`.

Known aliases are mapped before publish:

```ts
{
  session_new: "session.new",
  session_share: "session.share",
  session_interrupt: "session.interrupt",
  session_compact: "session.compact",
  messages_page_up: "session.page.up",
  messages_page_down: "session.page.down",
  messages_line_up: "session.line.up",
  messages_line_down: "session.line.down",
  messages_half_page_up: "session.half.page.up",
  messages_half_page_down: "session.half.page.down",
  messages_first: "session.first",
  messages_last: "session.last",
  agent_cycle: "agent.cycle"
}
```

#### `POST /tui/show-toast`

Request JSON:

```ts
{ title?: string; message: string; variant: "info" | "success" | "warning" | "error"; duration?: number }
```

Response `200`: `true`.

#### `POST /tui/publish`

Request JSON:

```ts
| { type: "tui.prompt.append"; properties: { text: string } }
| { type: "tui.command.execute"; properties: { command: string } }
| { type: "tui.toast.show"; properties: { title?: string; message: string; variant: "info" | "success" | "warning" | "error"; duration?: number } }
| { type: "tui.session.select"; properties: { sessionID: SessionID } }
```

Response `200`: `true`.

#### `POST /tui/select-session`

Request JSON:

```ts
{ sessionID: SessionID }
```

Response `200`: `true`.

Validates the session exists, then publishes `tui.session.select`.

#### `GET /tui/control/next`

Request: none.

Response `200`:

```ts
{ path: string; body: unknown }
```

This is a queue bridge for a TUI/control process. The request blocks until a queued TUI request exists.

#### `POST /tui/control/response`

Request JSON: `unknown`.

Response `200`: `true`.

Completes the oldest pending TUI bridge call.

## SSE Event Streams

### `GET /event`

Instance-scoped SSE stream.

Headers set by the server:

- `Cache-Control: no-cache, no-transform`
- `X-Accel-Buffering: no`
- `X-Content-Type-Options: nosniff`

Frame format: each SSE message has only a `data` field containing one JSON string:

```ts
type InstanceSseData = { type: string; properties: object }
```

Lifecycle:

- Immediately sends `{ type: "server.connected", properties: {} }`.
- Sends `{ type: "server.heartbeat", properties: {} }` every 10 seconds.
- Subscribes to all instance bus events.
- If `server.instance.disposed` is observed, the stream stops.
- Client abort also unsubscribes and stops heartbeat.

### `GET /global/event`

Global SSE stream.

Frame format: each SSE message has only a `data` field containing one JSON string:

```ts
type GlobalSseData = {
  directory: string
  project?: string
  workspace?: string
  payload: { type: string; properties: object } | { type: "sync"; syncEvent: SyncSerializedEvent }
}
```

Initial connected and heartbeat messages are wrapped only as:

```ts
{ payload: { type: "server.connected" | "server.heartbeat"; properties: {} } }
```

Normal global bus messages include `directory`, and usually `project` and `workspace` when emitted from an instance bus publish.

### Event Types and Payloads

The event schema registry is populated by all `BusEvent.define` calls plus latest `SyncEvent.define` events during projector init. Known event payloads from the researched source are below.

#### Server and Global

```ts
{ type: "server.connected"; properties: {} }
{ type: "server.heartbeat"; properties: {} }
{ type: "server.instance.disposed"; properties: { directory: string } }
{ type: "global.disposed"; properties: {} }
{ type: "installation.updated"; properties: { version: string } }
```

#### Sync Events

On `/event`, latest sync events are published as ordinary bus payloads:

```ts
{ type: "session.created"; properties: { sessionID: SessionID; info: Session } }
{ type: "session.updated"; properties: { sessionID: SessionID; info: Session } }
{ type: "session.deleted"; properties: { sessionID: SessionID; info: Session } }
{ type: "message.updated"; properties: { sessionID: SessionID; info: UserMessage | AssistantMessage } }
{ type: "message.removed"; properties: { sessionID: SessionID; messageID: MessageID } }
{ type: "message.part.updated"; properties: { sessionID: SessionID; part: Part; time: number } }
{ type: "message.part.removed"; properties: { sessionID: SessionID; messageID: MessageID; partID: PartID } }
```

On `/global/event`, persistent sync writes additionally emit:

```ts
{
  type: "sync"
  syncEvent: {
    type: "session.created.1" | "session.updated.1" | "session.deleted.1" | "message.updated.1" | "message.removed.1" | "message.part.updated.1" | "message.part.removed.1" | string
    id: string
    seq: number
    aggregateID: string
    data: unknown
  }
}
```

#### Session and Message Runtime

```ts
{ type: "session.diff"; properties: { sessionID: SessionID; diff: FileDiff[] } }
{ type: "session.error"; properties: { sessionID?: SessionID; error?: unknown } }
{ type: "session.status"; properties: { sessionID: SessionID; status: { type: "idle" } | { type: "busy" } | { type: "retry"; attempt: number; message: string; next: number } } }
{ type: "session.idle"; properties: { sessionID: SessionID } }
{ type: "session.compacted"; properties: { sessionID: SessionID } }
{ type: "message.part.delta"; properties: { sessionID: SessionID; messageID: MessageID; partID: PartID; field: string; delta: string } }
{ type: "todo.updated"; properties: { sessionID: SessionID; todos: { content: string; status: string; priority: string }[] } }
```

#### Permission

```ts
{ type: "permission.asked"; properties: PermissionRequest }
{ type: "permission.replied"; properties: { sessionID: SessionID; requestID: PermissionID; reply: "once" | "always" | "reject" } }
```

#### Question

```ts
{ type: "question.asked"; properties: QuestionRequest }
{ type: "question.replied"; properties: { sessionID: SessionID; requestID: QuestionID; answers: string[][] } }
{ type: "question.rejected"; properties: { sessionID: SessionID; requestID: QuestionID } }
```

#### PTY

```ts
{ type: "pty.created"; properties: { info: PtyInfo } }
{ type: "pty.updated"; properties: { info: PtyInfo } }
{ type: "pty.exited"; properties: { id: PtyID; exitCode: number } }
{ type: "pty.deleted"; properties: { id: PtyID } }
```

#### TUI

```ts
{ type: "tui.prompt.append"; properties: { text: string } }
{ type: "tui.command.execute"; properties: { command: string } }
{ type: "tui.toast.show"; properties: { title?: string; message: string; variant: "info" | "success" | "warning" | "error"; duration?: number } }
{ type: "tui.session.select"; properties: { sessionID: SessionID } }
```

#### File, VCS, Project, Worktree, IDE, MCP, LSP

These are also registered in the same bus registry and can appear on `/event` or `/global/event` if the corresponding subsystem is active:

```ts
{ type: "file.edited"; properties: { file: string } }
{ type: "file.watcher.updated"; properties: { file: string; event: "add" | "change" | "unlink" } }
{ type: "project.updated"; properties: ProjectInfo }
{ type: "vcs.branch.updated"; properties: { branch: string } }
{ type: "lsp.updated"; properties: {} }
{ type: "mcp.tools.changed"; properties: unknown }
{ type: "mcp.browser.open.failed"; properties: unknown }
{ type: "ide.installed"; properties: unknown }
{ type: "worktree.ready"; properties: unknown }
{ type: "worktree.failed"; properties: unknown }
```

Payloads marked `unknown` are defined outside the focus files; clients should tolerate additional event types and unknown properties.

## PTY WebSocket Protocol

Endpoint:

```text
GET /pty/:ptyID/connect?cursor=<cursor>
```

Use `ws://` or `wss://` with the same host and auth strategy. If using Basic auth in browsers where custom headers are unavailable, pass `auth_token=<base64(username:password)>` in the query string.

Query:

- `cursor` is optional.
- Omitted or invalid cursor replays from available buffer start.
- `cursor=-1` means connect at current end and do not replay existing buffered output.
- Any safe integer `>= 0` requests replay from that absolute character cursor.

Server-to-client messages:

- PTY output is sent as WebSocket text frames containing raw terminal data chunks.
- Initial replay is sent in chunks up to 64 KiB from an in-memory buffer capped at 2 MiB.
- After replay, the server sends a binary control frame: first byte `0x00`, followed by UTF-8 JSON `{ "cursor": number }`. This reports the absolute cursor at the current end of the PTY output buffer.
- Live output continues as text frames.

Client-to-server messages:

- Text frames are written directly to the PTY process stdin.
- Binary frames are decoded as UTF-8 and written to stdin by the service, but the Hono route currently forwards only string `event.data`, so clients should send text.

Lifecycle:

1. Create a PTY with `POST /pty/`.
2. Connect WebSocket to `/pty/:ptyID/connect`.
3. Server validates that the PTY exists before completing upgrade; missing PTY errors and closes.
4. Route buffers client messages received before `onOpen` finishes and flushes them after the PTY service attaches.
5. On WebSocket close or error, the subscriber is removed; the PTY process continues running.
6. `DELETE /pty/:ptyID` kills the process, closes subscribers, and emits `pty.deleted`.
7. Process exit emits `pty.exited`, then removes the PTY and emits `pty.deleted`.

## Session Lifecycle

Recommended client lifecycle:

1. Check server with `GET /global/health`.
2. Create a session with `POST /session/`, optionally passing `title`, `permission`, `parentID`, or `workspaceID`.
3. Subscribe to `/event` before or immediately after sending prompts. Use it as the realtime source for `message.updated`, `message.part.updated`, `message.part.delta`, `session.status`, permission events, and question events.
4. Send a prompt with `POST /session/:sessionID/message` for synchronous completion, or `POST /session/:sessionID/prompt_async` for fire-and-forget.
5. Fetch current canonical messages with `GET /session/:sessionID/message`, using cursor pagination if needed.
6. Watch `session.status` for `{ type: "busy" }`, retry status, and `{ type: "idle" }`. Deprecated `session.idle` is also emitted when a session becomes idle.
7. Abort active work with `POST /session/:sessionID/abort`.
8. Archive by `PATCH /session/:sessionID` with `{ time: { archived: <ms> } }`, or delete permanently with `DELETE /session/:sessionID`.

Notes:

- `POST /session/:sessionID/message` returns a single final JSON result through an HTTP stream. It is not the token stream. Use `/event` for live updates.
- `prompt_async` returns `204` immediately and publishes failures via `session.error`.
- `message.part.delta` carries incremental deltas for a specific part field; clients should merge this with later full `message.part.updated` snapshots.

## Tool Approval Flow

Permissions are evaluated by `(permission, pattern)` against the active session ruleset plus persisted approvals.

Flow:

1. A tool call needs permission and rules evaluate to `ask` for one or more patterns.
2. Server creates a `PermissionRequest` and publishes:

```ts
{ type: "permission.asked"; properties: PermissionRequest }
```

3. Client can also poll `GET /permission/` for pending requests.
4. Client replies with `POST /permission/:requestID/reply`:

```ts
{ reply: "once" | "always" | "reject"; message?: string }
```

5. Server publishes `permission.replied`.
6. If `reply` is `once`, only the current pending request continues.
7. If `reply` is `always`, the request's `always` patterns are added to the in-memory approved ruleset as `allow`, and other pending requests in the same session may be auto-approved if they now evaluate to allow.
8. If `reply` is `reject`, the request fails. If `message` is present, the tool receives corrective feedback. All other pending permissions in the same session are rejected.

Deprecated compatibility route:

```text
POST /session/:sessionID/permissions/:permissionID
{ response: "once" | "always" | "reject" }
```

New clients should use `/permission/:requestID/reply` because it supports feedback messages.

## Question and HITL Flow

Questions are human-in-the-loop prompts generated by tools/agents, separate from tool permission approval.

Flow:

1. Server creates a `QuestionRequest` and publishes:

```ts
{ type: "question.asked"; properties: QuestionRequest }
```

2. Client can also poll `GET /question/`.
3. Client answers with `POST /question/:requestID/reply`:

```ts
{ answers: string[][] }
```

4. Server publishes `question.replied` and resolves the waiting operation.
5. Client can dismiss with `POST /question/:requestID/reject`.
6. Server publishes `question.rejected` and the waiting operation fails with `QuestionRejectedError`.

Question shape:

- `questions` is an ordered array.
- Each question has `question`, `header`, `options`, optional `multiple`, and optional `custom`.
- Each answer is an array of selected labels for the corresponding question.

## Existing TUI Registration vs New Clients

The current TUI has two distinct integration modes.

### Local Worker TUI

The TUI worker imports `Server.Default().app.fetch` directly and exposes an internal RPC method `fetch`. It injects Basic auth automatically from `OPENCODE_SERVER_PASSWORD` and `OPENCODE_SERVER_USERNAME` when needed. It also starts/stops a server through RPC and forwards global bus events as `global.event` RPC notifications.

This is not an HTTP client registration API. It is an internal process bridge for the bundled TUI.

### Attached TUI

`opencode attach <url>` validates a session/server and then runs the TUI against an existing HTTP server. If a password is supplied, it sends `Authorization: Basic base64("opencode:<password>")`; note this attach path defaults the username literal to `opencode`.

The TUI control endpoints under `/tui/*` are command/event injection endpoints. `/tui/control/next` and `/tui/control/response` form a queue bridge for pending TUI requests; they are not a general event subscription system.

### Recommended New Client Connection

New clients should not attempt to register as a TUI. Instead:

1. Use Basic auth or `auth_token` query when server auth is enabled.
2. Call `GET /global/health` and `GET /path` to establish server/instance context.
3. Subscribe to `/event` for instance realtime updates, or `/global/event` if monitoring multiple instances/workspaces.
4. Use `/session`, `/permission`, `/question`, `/provider`, and `/pty` APIs directly.
5. Use `/tui/*` only if intentionally controlling a running TUI UI, such as appending prompt text or opening a dialog.
