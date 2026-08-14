## ADDED Requirements

### Requirement: Every release publishes a signed update manifest

Each tagged release SHALL publish, beside its installers, a `latest.json` manifest naming the release version and, per supported platform, the artifact URL and its detached minisign signature. Artifacts SHALL be signed with the project's updater private key, which SHALL exist only on the owner's machine and in the repository's CI secret; the corresponding public key SHALL be pinned in the app configuration. The release pipeline MUST fail if any expected installer or signature is missing, rather than publish an incomplete manifest.

#### Scenario: A tag produces a complete manifest

- **WHEN** a `v*` tag builds successfully
- **THEN** the release carries `latest.json` referencing the macOS app archive and both Windows NSIS installers by tag-pinned URL, each with a signature the pinned public key verifies

#### Scenario: A build that stopped signing cannot release

- **WHEN** a platform build uploads an installer without its signature
- **THEN** the release job fails before publishing anything

### Requirement: The app offers and applies updates in-app

The app SHALL check the update manifest once at startup. A failed check (offline, endpoint unreachable, malformed manifest) SHALL be silent and leave the app fully usable. When the manifest names a newer version, the app SHALL show a persistent banner naming that version with a single install action; the action SHALL download with visible progress, verify the signature against the pinned key, install, and relaunch. An installation failure SHALL surface through the standard failure notice with its detail affordance and return the banner to a retriable state; no update path may block provider management.

#### Scenario: A fleet machine picks up a fix release

- **WHEN** the user launches a version older than the latest release and clicks the banner's install action
- **THEN** the app downloads with visible progress, installs, relaunches as the new version, and the banner is gone

#### Scenario: Offline launch stays quiet

- **WHEN** the app starts without network access
- **THEN** no update notice appears and provider management works unchanged

#### Scenario: A tampered artifact is refused

- **WHEN** the downloaded artifact does not verify against the pinned public key
- **THEN** the installed app is untouched and the failure is reported with detail
