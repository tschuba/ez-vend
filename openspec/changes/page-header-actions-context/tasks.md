## 1. Context Infrastructure

- [ ] 1.1 Define `HeaderActionsContext` as a type alias `RwSignal<Option<View>>` in `crates/ez-vend-ui/src/lib.rs` (or a dedicated `header_actions.rs` module re-exported from `lib.rs`)
- [ ] 1.2 Call `provide_context(create_rw_signal(None::<View>))` in the `App` component in `lib.rs` before the router outlet renders

## 2. Update AppViewHeader

- [ ] 2.1 Remove the path-check for `ImportButton` from `AppViewHeader` in `lib.rs` (C2 code from `restore-import-button-c2`)
- [ ] 2.2 Read `HeaderActionsContext` in `AppViewHeader` via `use_context::<RwSignal<Option<View>>>()`
- [ ] 2.3 Render the context value right-aligned in the header flex row: `<Show when=|| slot.get().is_some()>{ slot.get() }</Show>`

## 3. Booth List Page — Inject Header Actions

- [ ] 3.1 In `booth_list.rs`, read `HeaderActionsContext` via `use_context`
- [ ] 3.2 On component mount (inside `create_effect` or directly in the component body), set the signal to `Some(view! { <ImportButton variant=ButtonVariant::Ghost size=ButtonSize::Small class="border border-gray-300 hover:border-gray-400 hover:bg-gray-50 gap-1.5".to_string() /> })`
- [ ] 3.3 Register `on_cleanup(|| signal.set(None))` so the header clears when navigating away from `/booths`

## 4. Verification

- [ ] 4.1 Navigate to `/booths` — Import button appears in header
- [ ] 4.2 Navigate to `/vendors`, `/checkout`, `/settings` — header shows no Import button
- [ ] 4.3 Navigate back to `/booths` — Import button reappears (signal re-set on mount)
- [ ] 4.4 Click Import — file picker and modal open correctly
