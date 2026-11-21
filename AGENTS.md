# isoboard

## Source of truth

- Read [product spec](docs/src/content/docs/developers/spec.md) for product behavior. Open decisions and
  internal plans live in the gitignored `.ai/` folder when present.
- Read [setup](docs/src/content/docs/developers/setup.md) and [testing and CI](docs/src/content/docs/developers/testing-and-ci.md) for setup and commands.
- Keep both accurate when behavior, scope, or setup changes.
- Resolve conflicts explicitly. Do not treat unfinished code as a new product
  decision or describe planned features as implemented.
- The conversion prompt template in `src-tauri/src/inference/conversion.jinja`
  is the source of truth for requested HTML; `params.rs` is its rendering
  adapter. Do not narrow or rewrite the template to fit a renderer. Prompt
  edits are deliberate product-contract changes. Rendering, policy, and
  clipboard code must implement its style, table, wrapper, and image-marker
  requirements.
- Keep all trusted first-pass prompt wording and prompt conditionals, including
  the Additional Rules section, in that bundled Jinja template. Rust supplies
  typed context and owns rendering, caching, readiness, and transport, not
  instruction text. User rules are literal context data, never template source.
  Future stage prompts must also use authoritative bundled Jinja templates.
- Naming: Use **isoboard** for the app and **isOCR** for its custom fine-tuned OCR model.

## Development

### Pre-release configuration schema

The app is unreleased. Keep the TOML schema at version 1 until the first public
release. An empty `config.toml`, a file containing only `version = 1`, and files
with omitted sections or fields load the documented defaults. Explicit invalid
or unknown values, unknown fields, and unsupported versions are errors; preserve
the last valid configuration. Do not add migration, legacy virtual-profile, or
shortcut-only format-preservation branches before release.
The defaults are version 1, the standard default capture binding, the `default`
profile, and empty provider and provisioning records.

The models.dev catalog cache is disposable application data, not a persisted
configuration format. Do not add compatibility or migration branches for old
cache schemas; an unsupported cache is discarded and refreshed from the
network. Keep the conditional ETag/Last-Modified and body-hash refresh path,
and retain a valid current cache when the network is unavailable.

The app uses Tauri 2, Rust, Svelte 5, SvelteKit, TypeScript, Vite, and Bun.
Run commands from this repository root using the README. The Makefile keeps
frontend and desktop commands explicit; `make check`, `make format-check`, and
`make lint` cover both sides of the app. `make check` also runs the committed
Specta binding drift guard.

### Configuration module map

The Rust configuration subsystem is split by responsibility:

- `src-tauri/src/config/types.rs` defines the version-one schema, validation,
  snapshots, and error values.
- `src-tauri/src/config/storage.rs` parses and atomically writes TOML.
- `src-tauri/src/config/controller.rs` owns transactional state, shortcut
  registration, profile changes, and rollback behavior.
- `src-tauri/src/config/watcher.rs` debounces external file edits and emits
  reload events.
- `src-tauri/src/config/tauri.rs` adapts native Tauri startup and shortcuts.
- `src-tauri/src/config/commands.rs` contains the Tauri IPC handlers.
- `src-tauri/src/config/mod.rs` is the small runtime facade used by `lib.rs`.

Keep validation and state transitions in the controller and types modules;
future platform or IPC integrations should stay in their adapter modules.
`inference_profiles.rs` owns the typed persisted `InferenceOptions` and
`ReasoningLevel` contract. IPC DTOs and command signatures are owned by the
Rust definitions registered in `contracts.rs`; `src/lib/generated/bindings.ts`
is committed generated output and must be refreshed with `make bindings` after
contract changes. Frontend modules may define labels, ordering, and
presentation projections, but must derive their serialized types from the
generated bindings rather than restating Rust unions or payload fields.
The generated directory is excluded from Prettier and ESLint; Specta owns its
format, while the binding drift check and TypeScript checker validate it.
`model_catalog.rs` owns advisory upstream metadata, while
`src/lib/model-catalog-state.ts` adapts that open-world metadata for the
frontend. `src/lib/inference-options.ts` owns only frontend labels and
normalization helpers; keep those adapters aligned with the generated Rust
enum through the binding drift check and focused boundary tests.

### Clipboard history

Clipboard history is a separate SQLite-backed subsystem. Keep the history
interface independent from TOML configuration and put SQLite details in its
adapter. Insertions and count/byte-limit pruning must be one transaction. Store
the exact raw inference HTML, canonical clean HTML, sanitized body, derived
plain text, generated source thumbnail, timestamp, source kind, and processing
profile ID/name snapshot. Successful and failed entries also store a compact
capture-time snapshot of selected provider/models, conversion flags, effective
application mode, and rule/pass usage, without credentials, endpoints, or rule
text. Keep original captures only while `[history] keep_originals` is on, as
owner-only files beside the database with their own count and byte budget, and
let the database decide which files exist. Enforce the entry and original
limits from `config/application.jsonc` at the storage boundary.

Successful history insertion begins only after native clipboard publication
succeeds. A failed capture persists only after processing or clipboard
publication fails. Pending captures stay in memory. A history write failure is
reported separately and must not change a result already published to the
clipboard. Restoring a successful row uses the saved clipboard data and does
not invoke inference. Keep single-entry deletion and clear-all behind the same
interface.

Clean rich previews must render sanitized HTML in an isolated surface. Raw
inference HTML is source text only and is never rendered. Debug source
actions copy the full raw or clean HTML as plain text to the clipboard. The
menu-bar history uses compact text previews. Settings uses vertical history
tabs with a thumbnail, plain-text start, and clipboard action. The selected
entry opens a rendered clean preview and its metadata beside the list. The
history view supersedes the old Preview and HTML Result sections.

### Configuration references and fixed providers

Profiles own their model settings. Capture bindings reference a Profile by ID;
model settings select a fixed provider/backend. Reuse `ModelSettingsEditor` for
both stages. Do not introduce standalone inference-profile identities or CRUD.
Provider credentials and endpoints remain in shared backend-owned records and
must never be copied into profiles. The rules-model mode and single remembered
independent configuration are persisted in TOML. Inactive settings must not affect
execution readiness or credential access. Provider-created profiles are ordinary
editable objects; the TOML provisioning ledger prevents their recreation after
deletion. Shortcut profile selects remain selection-only.

- After frontend changes, run `make frontend-check` and `make frontend-lint`.
- After Rust changes, run `make desktop-format`, then `make desktop-check` and
  `make desktop-lint`.
- Use `make format` for the complete formatter and `make format-check` for its
  read-only verification.
- Verify native behavior in the desktop app. A browser preview cannot validate
  capture, global shortcuts, permissions, or the system clipboard.
- Keep platform-specific desktop APIs behind narrow interfaces. Shared
  orchestration, validation, and event sequencing must depend on the interface
  contract rather than operating-system APIs; add platform behavior in a
  platform-specific implementation of that contract. Window presentation and
  tray activation follow the same boundary as capture, shortcuts, and clipboard
  access.
- Add meaningful tests for product behavior as it is implemented. Do not add
  trivial tests or tests for temporary scripts.
- Report which checks ran and any remaining validation gaps.

### UI components

- Every displayed UI element must be implemented as a component in
  `src/lib/components`; pages compose these components rather than defining
  displayed elements directly.
- Use shadcn-svelte selectively for behavior-heavy primitives. Generated or
  headless primitives live under `src/lib/components/ui`; feature panes use
  app-facing wrappers under `src/lib/components` and do not import Bits UI or
  generated primitives directly, except for standard DropdownMenu items,
  groups, and submenus supplied as snippets to a shared menu control. The
  shared control owns the trigger and popup structure. The current adopted components are Tabs for
  the app mode selector and sidebar navigation, plus Button,
  Select, Switch, and DropdownMenu.
- shadcn-svelte components are copied open code. Review and customize each
  generated component as part of the app, and preserve isoboard's semantic
  tokens, compact copy, and normal foreground text. Map theme variables to
  isoboard tokens instead of accepting registry default styling.
- Use a shadcn-svelte primitive when it reduces accessibility, focus, or
  keyboard maintenance. Keep simple product-specific components app-owned.
- CLI `init`, `add`, and update operations must not overwrite `src/app.css` or
  other user customizations. Review generated diffs before accepting updates.
- Before implementing a control, read the component inventory in
  [UI guide](docs/src/content/docs/developers/ui.md) and search `src/lib/components` and its `ui` directory
  for existing behavior and presentation. Use or extend the existing component
  before creating another implementation.
- Extract generic UI at its first use. If a control's layout or interaction can
  serve an unrelated feature (for example a split button, action menu, or status
  badge), implement it as a shared component with content, callbacks, and state
  passed through props or snippets. Do not wait for a second caller, and do not
  put feature names or feature-specific types in its API.
- Feature components own domain data, labels, actions, and state. They compose
  shared controls; they must not copy generic control markup, keyboard behavior,
  or styling. Add a feature wrapper only when it owns meaningful domain behavior
  or composition; avoid wrappers that merely rename or forward a shared control.
- Keep Bits UI imports, including type imports, inside
  `src/lib/components/ui`. App components derive types from their local
  primitives with `ComponentProps`. ESLint enforces the import boundary.
- When different controls need different semantics (such as value selection
  versus an action menu), use the appropriate behavior primitive and share their
  surface and item presentation. Do not create an independently styled dropdown.
- Document every new shared control in [UI guide](docs/src/content/docs/developers/ui.md), including its
  intended use and composition with existing controls. Before completion,
  review new components for generic markup left inside feature code and run
  frontend lint and checks.
- Use Tailwind utilities for component layout and states. Keep `src/app.css`
  limited to `@theme` identity tokens, document defaults, and small visual
  effects that do not belong to a component.
- Keep component color classes semantic: do not use raw color literals (`#`,
  `rgb`, or `rgba`) or direct palette colors such as `white`, `black`, or
  `red`. Reuse `@theme` tokens with Tailwind opacity modifiers when a tint is
  needed. If a shared visual value is missing, add one semantic token at the
  theme layer and reuse it across components.
- Select, Combobox, and action menus share the dropdown surface defined in
  `ui/menu-styles.ts`, with the glass presentation specified in docs/src/content/docs/developers/ui.md.
  Dialogs and error overlays use their established surfaces. The decorated
  settings window keeps its translucent shell over the native backdrop on macOS.
  Preserve the intentional translucent treatment of grouped surfaces and
  resting controls.
- Prefer direct actions and autosave when the result is clear without a separate
  confirmation control.
- Keep layouts stable while an action is in progress. Remove redundant helper
  copy and confirmation controls, while retaining useful errors and accessible
  labels.
### Profile tabs

- Profiles and History use the shared `SidebarTabs` family of wrappers.
- Profile tabs select the active capture profile and show one editor beside the
  list. Actions stay outside tab triggers; each pane scrolls independently.
- Each editor owns a complete profile draft and composes the shared model editor
  for primary and independent rules settings. Keep autosave transactional and
  preserve local edits when external snapshots refresh unchanged fields.

### Versions and changelog

`build.rs` parses `CHANGELOG.md` and fails every build on a heading that is
not a version, so there is no `## Unreleased` section. The release script also
requires a section for the exact version being released. See
[releases](docs/src/content/docs/developers/releases-and-notices.md) for the
format.

Add a changelog line with every user-visible change:

- If the current version is already released (its `v<version>` tag exists),
  bump the version and add a new section at the top of `CHANGELOG.md`. A fix
  bumps the patch version; a new feature bumps the minor version.
- If the current version is unreleased, add the line to its section. When a
  feature lands in a pending patch version, change it to the next minor
  version, for example 1.0.1 to 1.1.0, and rename its section.
- Keep the version the same in `package.json`, `src-tauri/Cargo.toml`, and
  `src-tauri/tauri.conf.json`, and rebuild to refresh `Cargo.lock`.

Changes users cannot see, such as documentation and internal refactors, need
no changelog line or version bump.

### Testing

Keep tests lean and high-value. Write the minimum set of tests needed to cover meaningful behavior and important edge cases, with minimal overlap.

Do not test trivial or obvious behavior, implementation details, or functionality already guaranteed by well-established libraries/frameworks unless our integration with them adds meaningful risk. Avoid tests whose maintenance cost outweighs their value.

Prefer testing observable behavior over internals. Avoid excessive mocking—especially mocks that replace so much of the system that the test no longer exercises real application behavior. Mock only at clear external boundaries when necessary.

Remove obsolete production code with no runtime callers instead of keeping it
alive only to satisfy tests, and delete tests whose sole coverage is that code.
Keep tests that exercise current behavior, along with intentional test
fixtures and helpers that model an external boundary.

## Writing

Use short, plain sentences, following the style of the spec. The objective is to be clear and understood, not verbose.
Keep implementation history out of code comments and user flows.
Leave unresolved product choices visible instead of silently deciding them.

### UI copy

- Keep displayed copy minimal. Put labels and actions first.
- Add help text only when it is needed to complete an action or recover from an error.
- Remove redundant status messages and implementation details from the normal settings surface.
- Keep actionable errors and useful configuration access available when users need them.

### Text legibility

- Keep normal UI text at the standard foreground color; do not use muted, subtle, or reduced-opacity text for hierarchy, metadata, help, configuration, or placeholders. Reserve dimmed text for a meaningful interaction state such as disabled, hover, pressed, or invalid, and preserve accessible contrast.
- The Additional Rules example placeholder is a narrow exception: use muted,
  italic text to distinguish its example from saved instructions.
