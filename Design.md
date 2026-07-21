# CRAFTEL visual contract

This document is the authoritative visual contract for CRAFTEL. Implementation and future design changes must follow it. Multica is the primary reference for application density and board surfaces; Vercel Geist is the primary reference for color roles, typography, and controls. CRAFTEL adopts their shared restraint, not their product behavior or information architecture.

## Principles and CRAFTEL-specific decisions

1. **Work first.** The task board, documents, and run activity are the visual focus. Branding and explanation stay quiet.
2. **One application, not a collection of cards.** Contiguous panes establish structure. Tonal separation comes before borders; borders come before shadows. Cards may use only the documented hairline surface shadow.
3. **Dense, never cramped.** A 4px base rhythm, 14px body type, and compact controls maximize useful content while preserving 32px minimum pointer targets (44px on coarse pointers).
4. **Achromatic first.** Black is the primary action. Blue means focus, link, or active execution only. Green, amber, and red mean success, warning, and error only. Navigation and selection remain neutral.
5. **Workflow remains legible.** Inbox → Defining → Implementation → Reviewing → Done is always present above the board in a low-height strip. The Reviewing → Implementation changes-requested path and the distinction between dragging and starting an agent are explicit.
6. **Local-first context is useful metadata.** Paths and IDs use mono; titles and prose use the system sans stack. There are no display or serif faces.

## Product hierarchy and interaction contract

CRAFTEL is a Markdown task-workspace orchestrator, not a Kanban app with agent controls added to cards. The durable task documents are the product asset; sessions and runs are the means used to refine or execute them.

- **Board is overview.** It supports scanning, ordering, stage movement, and one clear entry into a task workspace. A card is never a miniature workspace: it does not show document excerpts, editors, or run controls.
- **Workspace is the task control center.** Its first viewport must answer three questions in order: current workflow stage, current contract or evidence artifact, and the one legal next action.
- **Workflow stage and run state are independent.** Moving or dragging changes only the stage. Starting, continuing, or stopping an agent is always explicit.
- **Active runs pin the stage.** A queued or running task cannot be dragged, moved, or advanced. Stop or finish the run first so a phase command can never apply to a different workflow stage.
- **Documents express task meaning inside a real task workspace.** The left navigation is rooted at the task's actual `craftel/tasks/TNNNN-slug/` directory and shows its Markdown files and semantic subdirectories (`decisions/`, `discussions/`, `plans/`, `reviews/`, `notes/`, and `subtasks/`). `TASK.md` appears first as a managed read-only projection; `SPEC.md` is the defining and implementation contract. Never replace this directory navigation with a duplicate task-content card or an abstract file list that hides where artifacts live.
- **Current phase comes before history.** The phase panel shows the current session/run, observable assistant and tool events, result, and the current legal intervention. Prior sessions and runs are secondary history, not competing primary controls. Hidden reasoning is never displayed or implied.

### State and action matrix

Only the listed primary action is prominent. A prompt composer appears only when that action needs user input. Actions from other rows are not rendered disabled “for discoverability”; they are absent.

| Valid workspace state | Primary artifact | Primary action | Composer / supporting behavior |
|---|---|---|---|
| Inbox | Task brief | Move to Defining | No agent controls; moving does not run |
| Defining, no resumable session | `SPEC.md` | Start Defining | Starts a defining session |
| Defining, active run | `SPEC.md` | Stop run | Live evidence; no follow-up or Next |
| Defining, resumable terminal run | `SPEC.md` | Move to Implementation | Message composer can Continue Defining in the same phase session |
| Implementation, idle | Approved `SPEC.md` + plan | Start Implementation | Entering the stage did not run |
| Implementation, resumable or changes requested | Review findings + plan | Continue Implementation | Message composer; same implementation session |
| Implementation, active run | Implementation evidence | Stop run | Live evidence only |
| Reviewing, no verdict | New review packet | Start fresh Review | Every formal review creates a clean session |
| Reviewing, active review | Review packet | Stop run | Live review evidence only |
| Reviewing, approved | Latest review packet | Mark Done | Explicit human handoff; pass alone never completes |
| Done | Documents + review evidence | None | Terminal, read-only delivery context |

Review changes-requested transitions back to Implementation and exposes the review findings there. A repaired task returning to Reviewing starts a new formal review session. Review approval stays in Reviewing until the human `Mark Done` action.

### Core journey contract

The defining journey establishes the product's causal structure: capture a brief in Inbox → move to Defining → start or continue the defining conversation → inspect the resulting `SPEC.md` changes → explicitly move to Implementation → explicitly start Implementation. Conversation is useful because it changes the durable contract, not because chat history is itself the contract.

`craftel pass` from Defining records a successful defining attempt but stays in Defining; the human `Move to Implementation` action owns contract approval. Implementation success may advance to Reviewing. Review pass records approval but stays in Reviewing until human delivery.

## Semantic surface model

From back to front: `shell` is the cool-gray app chrome, `canvas` is the near-white working background, `column` is the faintest board grouping tint, `surface` is interactive content, `surface-subtle` groups controls, and `surface-raised` is transient. `overlay` dims content behind dialogs. Adjacent workspace panes share hairline edges. Only board cards use the low `surface` shadow; menus, dialogs, and dragging use stronger elevation.

## Tokens

Token values below are exact. CSS variable names are the API.

### Color

| Token | Light value | Role |
|---|---:|---|
| `--color-shell` | `oklch(0.964435 0.001327 286.375)` | application chrome |
| `--color-canvas` | `oklch(0.988087 0 0)` | primary work background |
| `--color-column` | `oklch(0.967 0.001 286.375 / 55%)` | board-column grouping only |
| `--color-surface` | `oklch(1 0 0)` | cards, editor, fields |
| `--color-surface-subtle` | `oklch(0.967 0.001 286.375)` | grouped controls |
| `--color-surface-hover` | `oklch(0.967 0.001 286.375)` | neutral hover |
| `--color-surface-selected` | `oklch(0.95 0.002 286.375)` | neutral selection |
| `--color-surface-raised` | `oklch(1 0 0)` | menus and dialogs |
| `--color-overlay` | `oklch(0.141 0.005 285.823 / 48%)` | modal scrim |
| `--color-text` | `oklch(0.141 0.005 285.823)` | primary text |
| `--color-text-muted` | `oklch(0.552 0.016 285.938)` | secondary text |
| `--color-text-subtle` | `oklch(0.705 0.015 286.067)` | tertiary text |
| `--color-text-inverse` | `oklch(0.985 0 0)` | text on strong fills |
| `--color-border` | `oklch(0.92 0.004 286.32)` | controls and surface boundary |
| `--color-border-subtle` | `oklch(0.945 0.003 286.32)` | internal boundary |
| `--color-primary` | `oklch(0.21 0.006 285.885)` | the single primary action |
| `--color-primary-hover` | `oklch(0.30 0.006 285.885)` | primary-action hover |
| `--color-brand` | `oklch(0.55 0.16 255)` | active execution and links only |
| `--color-brand-soft` | `oklch(0.55 0.16 255 / 8%)` | active/drop background only |
| `--color-focus` | `oklch(0.55 0.16 255)` | keyboard focus outer ring |
| `--color-success` / `--color-success-soft` | `oklch(0.55 0.16 145)` / `oklch(0.55 0.16 145 / 8%)` | success state only |
| `--color-warning` / `--color-warning-soft` | `oklch(0.65 0.14 75)` / `oklch(0.75 0.16 85 / 10%)` | warning state only |
| `--color-error` / `--color-error-soft` | `oklch(0.577 0.245 27.325)` / `oklch(0.577 0.245 27.325 / 8%)` | error state only |

These values intentionally track Multica's neutral OKLCH hierarchy. Their role assignment follows Geist: background levels are not interchangeable with gray interaction levels, and chromatic values do not decorate static surfaces.

### Typography

| Token | Value |
|---|---|
| `--font-sans` | `"Geist Variable", Geist, Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif` |
| `--font-mono` | `"Geist Mono Variable", ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace` |
| `--text-xs` / `--text-sm` / `--text-md` | `11px` / `12px` / `14px` |
| `--text-lg` / `--text-xl` / `--text-page` | `16px` / `18px` / `24px` |
| `--leading-tight` / `--leading-normal` / `--leading-relaxed` | `1.25` / `1.45` / `1.6` |
| `--weight-medium` / `--weight-semibold` / `--weight-bold` | `500` / `600` / `600` |

### Spacing, dimensions, radii, borders

| Group | Tokens and exact values |
|---|---|
| spacing | `--space-1: 4px`, `--space-2: 8px`, `--space-3: 12px`, `--space-4: 16px`, `--space-6: 24px` |
| dimensions | `--sidebar-width: 232px`, `--topbar-height: 60px`, `--workflow-height: 76px`, `--column-width: 276px`, `--control-height: 32px`, `--touch-target: 44px` |
| radii | `--radius-sm: 4px`, `--radius-md: 6px`, `--radius-lg: 8px` |
| borders | `--border-hairline: .5px solid var(--color-border-subtle)`, `--border-default: 1px solid var(--color-border)` |

### Shadows, motion, z-index

| Token | Value / use |
|---|---|
| `--shadow-surface` | `0 1px 2px rgb(15 23 42 / .04), 0 1px 1px rgb(15 23 42 / .03)` |
| `--shadow-menu` | `0 8px 24px rgb(15 23 42 / .08), 0 2px 6px rgb(15 23 42 / .05)` |
| `--shadow-dialog` | `0 16px 40px rgb(15 23 42 / .14), 0 3px 10px rgb(15 23 42 / .08)` |
| `--shadow-drag` | `0 12px 28px rgb(15 23 42 / .16)` |
| `--motion-fast` / `--motion-normal` | `100ms` / `160ms` |
| `--ease-standard` | `cubic-bezier(.2,0,0,1)` |
| `--z-menu` / `--z-overlay` / `--z-toast` | `20` / `40` / `60` |

Motion is limited to color, border, and opacity. No hover lift or scale. Under `prefers-reduced-motion: reduce`, durations are zero.

## Theme contract

Light mode is implemented now. Geist Sans and Geist Mono are self-hosted application dependencies; rendering never depends on a network font request. Dark mode compatibility is mandatory: components consume only semantic tokens and must not infer meaning from a literal light color. A future `[data-theme="dark"]` may remap the color tokens while preserving contrast, surface order, state meaning, border relationships, and all non-color tokens. Images or icons must not rely on a white background.

## Component rules

- **Sidebar:** 232px (allowed range 216–240px), shell surface, right hairline, 32–36px rows. Selected project uses a neutral fill and stronger text; brand color is forbidden for selection. The single “Open Project” action is black.
- **Board:** compact top bar; workflow strip does not exceed 76px on wide screens. Columns are fixed 276px and horizontally scroll as one board. Columns use the faint `column` tint without a visible default outline; stage colors are forbidden.
- **Task card:** show only ID/run state, title, and one visible “Open workspace” action. Never show document or task-content excerpts: a truncated preview is not actionable. Metadata editing is a secondary “Edit task details” menu action, not a competing visible entry point. Cards use a .5px boundary and `shadow-surface`; hover remains neutral. Dragging and starting execution remain separate controls.
- **Workspace:** a compact stage/context header precedes the split view and contains one state-valid primary action. The semantic document navigation, editor, and current-phase panel share edges with no independent shadows. The document area is visually primary; agent activity explains how the current artifact is being refined or consumed. Run history and metadata remain compact and secondary. At narrow widths, semantic tabs switch between documents/editor and current-phase activity, with the default chosen from current work rather than presenting unrelated modules.
- **Markdown modes:** agent-authored documents expose mutually exclusive Edit, Preview, and Split controls. Split keeps the source editor and sanitized rendered Markdown visible together and updates the preview from the in-memory draft without writing on each keystroke. `TASK.md` is selectable and rendered as a special SQLite-backed managed projection; it never exposes document editing or revision restoration controls.
- **Dialogs and revisions:** the only persistent overlay treatment. Use scrim plus dialog shadow; fields and action placement remain consistent.
- **Activity:** current-phase evidence stays legible; prior sessions and runs are secondary history. IDs/log output use mono and messages use sans. State color is reserved for actual status. Start, Stop, Continue, Fresh Review, and Mark Done are mutually exclusive state-derived actions, never a generic toolbar.

## Responsive behavior

- **Wide target (≥1100px):** 232px sidebar, 60px top bar, five 276px columns in a horizontal scroller, workflow in one compact row, three-pane workspace.
- **Intermediate target (701–1099px):** sidebar may compress to 216px; workflow and board scroll horizontally without hiding stages; workspace switches to tabs below 900px.
- **Narrow target (≤700px):** sidebar becomes compact top project chrome; board remains a horizontal five-column scroller; workflow remains a single horizontally scrollable strip (never a tall vertical banner); workspace uses Documents/Activity tabs. Coarse pointers receive 44px targets.

## Accessibility

All controls are keyboard reachable and use the Geist-style two-layer focus ring: a 2px surface gap plus a 2px blue outer ring. Do not encode state by color alone. Maintain WCAG 2.2 AA text contrast, semantic regions/headings, accessible drag labels, live status and alert roles, modal focus containment, and 44px targets for coarse pointer/touch environments. Horizontal overflow must be discoverable and keyboard/trackpad operable. Reduced-motion preferences are honored.

## Forbidden patterns

No serif/display fonts; warm paper, orange branding, gradients, decorative textures, stage rainbow colors, glass effects, shadows on static panes, hover lift/scale, oversized hero banners, pill-shaped general controls, hidden workflow stages, icon-only unlabeled critical actions, or color-only status. Do not make a stage move start an agent.

## Token governance

Components consume semantic custom properties from the root token block. Raw colors, spacing, radii, shadows, and ad-hoc sizes are forbidden in component rules except documented one-offs required by native behavior (for example `0`, `100%`, grid fractions, and content-driven minimums). Add a token only when a semantic role repeats; do not add tokens merely to avoid choosing an existing role. Any token change updates this document and the CSS token block in the same commit.
