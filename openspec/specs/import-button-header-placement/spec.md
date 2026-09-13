### Requirement: Import button visible in Events page header

The Events page (`/booths`) SHALL display an Import button in the page header, right-aligned opposite the "Events" h1, so users can import backups at any time regardless of backup status.

#### Scenario: Import button present on Events page

- **WHEN** the user navigates to the Events page (`/booths`)
- **THEN** the page header SHALL show an Import button to the right of the "Events" heading

#### Scenario: Import button absent on other pages

- **WHEN** the user navigates to any page other than `/booths` (Vendors, Checkout, Settings)
- **THEN** the page header SHALL NOT show an Import button

#### Scenario: Import button opens import modal

- **WHEN** the user clicks the Import button in the page header
- **THEN** the import file picker SHALL open, followed by the import modal (simple modal or conflict wizard as determined by `analyze_import`)

### Requirement: Import button visual treatment

The Import button in the page header SHALL use an outlined ghost style subordinate to the Create Event FAB, with a responsive label that collapses to icon-only on small screens.

#### Scenario: Desktop label visible

- **WHEN** the viewport width is ≥ 640px (sm breakpoint)
- **THEN** the Import button SHALL display both the `LuUpload` icon and the "Import" label text side by side

#### Scenario: Mobile icon-only

- **WHEN** the viewport width is < 640px
- **THEN** the Import button SHALL display only the `LuUpload` icon; the label text SHALL be hidden visually but present for screen readers (`sr-only`)

#### Scenario: Visual contrast with Create Event FAB

- **WHEN** both the Import button and the Create Event FAB are visible
- **THEN** the Import button SHALL use a ghost+border style (transparent background, `border-gray-300`) and the FAB SHALL retain its filled teal-600 style, making the FAB the visually dominant action
