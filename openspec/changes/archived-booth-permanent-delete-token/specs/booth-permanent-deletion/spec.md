## ADDED Requirements

### Requirement: Archived booths can be permanently deleted after token confirmation
The system SHALL allow a permanently archived booth to be deleted only when the caller supplies a valid confirmation token for that booth deletion request.

#### Scenario: Archived booth deletion succeeds with a valid token
- **WHEN** the user requests permanent deletion for an archived booth and provides a valid confirmation token
- **THEN** the booth is permanently removed from storage

#### Scenario: Archived booth deletion is rejected without a token
- **WHEN** the user requests permanent deletion for an archived booth without providing a confirmation token
- **THEN** the deletion is rejected and the booth remains archived

#### Scenario: Archived booth deletion is rejected with an invalid token
- **WHEN** the user requests permanent deletion for an archived booth and provides an invalid or expired confirmation token
- **THEN** the deletion is rejected and the booth remains archived

### Requirement: Active booths cannot be permanently deleted
The system SHALL reject any attempt to permanently delete an active booth, regardless of whether a confirmation token is provided.

#### Scenario: Active booth deletion is rejected with a token
- **WHEN** the user requests permanent deletion for an active booth and provides a confirmation token
- **THEN** the deletion is rejected and the booth remains active

#### Scenario: Active booth deletion is rejected without a token
- **WHEN** the user requests permanent deletion for an active booth without providing a confirmation token
- **THEN** the deletion is rejected and the booth remains active
