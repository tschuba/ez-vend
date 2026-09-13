## Context

The shared `AppViewHeader` component in `lib.rs` renders `<h1>{page_title}</h1>` for all pages. It already performs a path-match to determine the title text; this change extends that same path-match to conditionally render page-level header actions.

The `ImportButton` component exists and is fully functional (including the new conflict wizard from `cross-device-event-merge`). It is not broken — it was simply orphaned: `components/mod.rs` declares `mod import_button` but omits the `pub use import_button::*;` re-export, so the component is unreachable from other crates.

A `C1` (context-signal) infrastructure for injecting arbitrary header actions from any page is planned as a separate change.

## Goals / Non-Goals

**Goals:**
- Make ImportButton accessible again from the Events page
- Use visual treatment consistent with the design: ghost+border style, `ButtonSize::Small`, `LuUpload` icon, responsive label (`hidden sm:inline` on mobile)
- Keep the change minimal — no new infrastructure, no new locale keys

**Non-Goals:**
- Context-signal infrastructure for generic header actions (C1 change)
- Placing Import on any page other than `/booths`
- Moving the Export button from the footer to the header

## Decisions

### D1: C2 (path-check in `lib.rs`) rather than C1 (context signal)

`AppViewHeader` already switches on the current path to determine the title text. Adding a conditional render of `ImportButton` for `/booths` is a one-liner that follows the existing pattern. C1 would require ~30 lines of new `HeaderActionsContext` infrastructure and a mount/unmount signal-clearing pattern in the booth list page — justified only when a second page needs header actions. The C1 change is tracked separately.

Alternative considered: place ImportButton in the fixed toolbar alongside the search bar. Rejected — the toolbar is scoped to the booth list's operational controls (search, filter). Import is a dataset-level administrative action; the page-header semantic anchor is more appropriate (UX Architect assessment).

Alternative considered: add ImportButton to the smart footer (`StorageIndicator`). Rejected — the footer communicates backup status and urgency; Import does not belong to that frame.

### D2: Ghost + border visual style, not Secondary

`ButtonVariant::Secondary` in this codebase renders as a filled gray button — too heavy next to a `text-2xl font-bold` h1. The correct treatment is `ButtonVariant::Ghost` with an additional `border border-gray-300 hover:border-gray-400 hover:bg-gray-50` via the `class` prop, producing an outlined ghost button that sits quietly alongside the heading (UI Designer assessment).

### D3: `LuUpload` icon

`LuUpload` best represents "choose a file and bring it into the app". `LuFolderInput` suggests folder navigation; `LuArrowDownToLine` implies download-to-disk. The icon is added to `icons.rs` alongside existing Lucide imports.

### D4: Responsive label — icon-only on mobile

At `< sm` (< 640px) the text label collapses via `hidden sm:inline`; a `sr-only` span preserves screen-reader accessibility. The icon alone provides sufficient affordance at small widths without crowding the h1.

## Risks / Trade-offs

**C2 path-check leaks booth logic into shared layout** → Accepted as a known short-term trade-off. The C1 change will clean this up. The path-check follows the existing pattern already in `AppViewHeader` (the title switch), so it is not introducing a new anti-pattern.

**`pub use import_button::*;` was missing** → The bug was silent: no compile error because `import_button` was still declared with `mod import_button`. The fix is a one-line addition to `components/mod.rs`. No other breakage expected.
