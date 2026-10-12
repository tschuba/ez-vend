# UI Style Guide

Conventions for `crates/ui`. Grows as new conventions are established;
sections below are not exhaustive of the whole UI codebase.

## Touch Targets

Interactive controls (buttons, checkboxes, icon-only actions) use a minimum
44×44px hit area, per WCAG 2.5.5, and `touch-manipulation` to avoid the
mobile browser double-tap delay:

```html
class="flex h-11 w-11 items-center justify-center rounded-md ... touch-manipulation"
```

or, for non-square controls where only height is constrained:

```html
class="... min-h-[44px] ... touch-manipulation"
```

See `crates/ui/src/components/export_button.rs` and
`crates/ui/src/pages/vendor_list.rs` (`VendorActionButtons`, vendor
selection checkbox) for examples.
