## Context

Leptos provides a context system (`provide_context` / `use_context`) that is scoped to the component tree. This is the idiomatic mechanism for passing data or signals between a parent component (the App root) and a deeply nested descendant (a page component) without prop-drilling.

The `AppViewHeader` component currently lives in `lib.rs` and is rendered at the App root level, above the page router outlet. Pages are rendered inside the router outlet, which is a sibling/descendant of `AppViewHeader` in the component tree.

The C2 path-check (`restore-import-button-c2`) is the predecessor. This change supersedes it.

## Goals / Non-Goals

**Goals:**
- Remove booth-specific logic from `lib.rs`
- Allow any page to inject a `View` into the header's right slot
- Keep the signal lifecycle (mount/unmount) owned by the page, not the layout

**Non-Goals:**
- Multiple named slots in the header (left, center, right) — one right slot is sufficient
- Slot support for the navbar or footer
- Animation or transition when the slot content changes

## Decisions

### D1: `WriteSignal<Option<View>>` as the context type

The context carries a `RwSignal<Option<View>>` (or equivalently a `WriteSignal`/`ReadSignal` pair). `Option<View>` means the slot is empty by default — no special "no action" state needed, and `AppViewHeader` can simply `Show when=|| slot.get().is_some()`.

Alternative considered: `RwSignal<Vec<View>>` for multiple actions. Rejected — overkill for the current use case; a single `View` is composable (a page can put any layout including multiple buttons inside one `View`).

Alternative considered: passing a `Callback` or `StoredValue<Box<dyn Fn() -> View>>`. Rejected — a signal is reactive and will update the header when the page changes its actions; a stored value would not.

### D2: Pages own mount/unmount lifecycle

Each page that injects header actions MUST clear the signal when it unmounts. This is done with `on_cleanup(|| set_header_actions.set(None))` in the page component. The shared header never clears the signal itself.

This keeps the layout layer passive — it only renders what it is given.

### D3: Context provided at App root, consumed independently by header and pages

`provide_context(RwSignal::new(None::<View>))` is called once in the `App` component. Both `AppViewHeader` and page components reach it via `use_context::<RwSignal<Option<View>>>()`. No prop-threading required.

### D4: `View` as the payload type

Pages compose their own button layout (icon, label, variant) and pass the result as a `View`. This gives pages full control over the visual treatment without the context needing to know about `ButtonVariant`, `ButtonSize`, or `ImportButton` internals.

## Risks / Trade-offs

**Signal not cleared on fast navigation** → Mitigated by `on_cleanup` in each page that sets actions. If a page forgets to clear, the header shows stale actions from the previous page — the same kind of bug as a memory leak, visible and easy to catch in review.

**`View` is not `PartialEq`, so `RwSignal<Option<View>>` will re-render on every set** → Acceptable. Header actions change at most once per navigation event, which is far below any perceptible render cost.

**Supersedes C2** → When this change ships, the path-check in `lib.rs` (from `restore-import-button-c2`) is deleted and the booth list page gains the `on_cleanup` pattern. The net change to user-visible behavior is zero.
