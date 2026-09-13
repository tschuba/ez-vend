## ADDED Requirements

### Requirement: Pages can inject header actions via context

The system SHALL provide a `HeaderActionsContext` (a `RwSignal<Option<View>>`) at the App root so that any page component can inject a `View` into the shared page header's right slot.

#### Scenario: Page provides header actions on mount

- **WHEN** a page component mounts and calls `use_context::<RwSignal<Option<View>>>().set(Some(view))`
- **THEN** the shared page header SHALL render that view right-aligned, opposite the h1

#### Scenario: Page clears header actions on unmount

- **WHEN** a page component unmounts (navigation away)
- **THEN** the page SHALL have cleared the context signal via `on_cleanup(|| signal.set(None))`
- **AND** the page header SHALL render no right-slot content

#### Scenario: Pages with no header actions show no right slot

- **WHEN** a page does not write to `HeaderActionsContext`
- **THEN** the page header SHALL render only the h1, with no right-slot content visible

### Requirement: Header right slot renders arbitrary view

The shared `AppViewHeader` SHALL read `HeaderActionsContext` and render the signal value in a right-aligned flex item when `Some`, and render nothing when `None`.

#### Scenario: Slot renders provided view

- **WHEN** `HeaderActionsContext` signal is `Some(view)`
- **THEN** `AppViewHeader` SHALL render `view` right-aligned in the header flex row

#### Scenario: Slot is empty by default

- **WHEN** no page has written to `HeaderActionsContext`
- **THEN** `AppViewHeader` SHALL render only the h1 with no additional content
