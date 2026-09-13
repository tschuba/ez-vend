## 1. Fix Missing Re-Export

- [x] 1.1 Add `pub use import_button::*;` to `crates/ez-vend-ui/src/components/mod.rs` after the `export_button` line

## 2. Add LuUpload Icon

- [x] 2.1 Add `LuUpload` to the Lucide icon imports and re-exports in `crates/ez-vend-ui/src/components/icons.rs`

## 3. Update ImportButton Component

- [x] 3.1 Add `LuUpload` icon rendering to `ImportButton` in `import_button.rs`: icon left of label, `h-4 w-4 shrink-0`
- [x] 3.2 Make the label responsive: wrap label text in `<span class="hidden sm:inline">` + add `<span class="sr-only sm:hidden">` for mobile screen-reader accessibility

## 4. Page Header Placement

- [x] 4.1 Add `justify-between` to the `AppViewHeader` flex container in `lib.rs` so h1 and actions are at opposite ends
- [x] 4.2 Conditionally render `<ImportButton variant=ButtonVariant::Ghost size=ButtonSize::Small class="border border-gray-300 hover:border-gray-400 hover:bg-gray-50 gap-1.5".to_string() />` when `path == "/booths"` in `AppViewHeader`

## 5. Visual Verification

- [x] 5.1 Run `trunk serve` and verify the Import button appears in the Events page header on desktop (label + icon visible)
- [x] 5.2 Verify the Import button is absent on Vendors, Checkout, and Settings pages
- [x] 5.3 Verify clicking Import opens the file picker and import modal correctly
- [x] 5.4 Verify on a narrow viewport (< 640px) the label collapses to icon-only
