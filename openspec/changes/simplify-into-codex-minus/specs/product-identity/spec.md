## ADDED Requirements

### Requirement: One product name across shipped surfaces

The product SHALL present the name "Codex Minus" in the window title, in-app branding, user-facing messages, bundle `productName`, and installer/release artifact names. The binary name SHALL be pinned to `codex-minus` independently of the display name, and the Windows single-instance check SHALL match that pinned name. The bundle identifier, settings directory, on-disk catalog prefixes, temp-file markers, and historical changelog entries MUST remain unchanged so no existing state is orphaned.

#### Scenario: The rename ships without losing state

- **WHEN** a user installs the renamed application beside the old one and launches it
- **THEN** it opens the existing settings and profiles unchanged, and the old bundle remains until removed by hand

#### Scenario: Installer artifacts carry the new name

- **WHEN** release artifacts are produced
- **THEN** the app bundle, MSI, and NSIS filenames and release names carry "Codex Minus" and the CI signing/zip steps reference the renamed bundle path
