## Purpose

Give Manager-owned network operations one explicit, observable proxy policy without changing operating-system settings, live Codex routing, or credential ownership.

## ADDED Requirements

### Requirement: Manager-scoped network modes
The system SHALL expose exactly three Manager network modes: `auto`, `direct`, and `custom`. The selected mode SHALL apply only to network requests initiated by Codex-- Manager and isolated child processes explicitly integrated with this capability.

#### Scenario: No saved policy exists
- **WHEN** the Manager network policy has never been saved
- **THEN** the system uses `auto` without writing or changing any operating-system, Codex, provider, or authentication configuration

#### Scenario: Direct mode is selected
- **WHEN** the user selects `direct`
- **THEN** integrated operations receive no proxy variables or proxy client configuration while retaining only their existing safe non-proxy environment

#### Scenario: Custom mode is selected
- **WHEN** the user selects `custom` with a valid non-credential-bearing HTTP, HTTPS, SOCKS5, or SOCKS5H proxy URL and optional bypass entries
- **THEN** integrated operations use that endpoint and bypass set independently of process and operating-system proxy settings

#### Scenario: Policy scope is observed
- **WHEN** the user saves or applies any Manager network mode
- **THEN** the system does not modify live Codex conversation routing, provider configuration, `config.toml`, `auth.json`, operating-system proxy settings, or another application's environment

### Requirement: Deterministic automatic resolution
In `auto` mode, the system MUST resolve one coherent source in this order: valid Manager-process proxy variables, supported static operating-system proxy settings, then direct access. It MUST NOT silently combine proxy endpoints from different sources.

#### Scenario: Process proxy variables are valid
- **WHEN** the Manager process contains a coherent non-empty proxy environment
- **THEN** `auto` uses that process environment and its matching bypass entries without consulting operating-system static proxy endpoints

#### Scenario: Only static operating-system proxy exists
- **WHEN** no usable process proxy exists and the active operating-system network configuration exposes supported static proxy endpoints
- **THEN** `auto` maps those endpoints and operating-system bypass entries into one resolved policy for the integrated operation

#### Scenario: No proxy source exists
- **WHEN** neither a usable process proxy nor a supported static operating-system proxy exists
- **THEN** `auto` resolves to direct access and reports `direct-fallback` as the source

#### Scenario: Process variables conflict
- **WHEN** case variants or duplicate process proxy variables specify contradictory non-empty values
- **THEN** `auto` reports an action-required conflict and does not start a credential-bearing integrated operation

#### Scenario: Automatic configuration is unsupported
- **WHEN** the selected operating-system source depends on PAC, WPAD, or another automatic configuration that cannot be faithfully projected to the integrated client
- **THEN** `auto` reports that source as unsupported and requires the user to choose `direct` or a valid `custom` endpoint instead of silently using direct access

#### Scenario: Bypass rules are applied
- **WHEN** the resolved source contains host bypass entries
- **THEN** the system preserves normalized bypass semantics for that source and does not display, persist, or apply unrelated bypass entries from another source

### Requirement: Non-secret policy persistence
The system MUST persist only the selected mode, a validated non-credential-bearing custom endpoint, and normalized custom bypass entries in owner-readable Manager state. It MUST reject values that would turn the policy store into a credential store.

#### Scenario: Custom endpoint contains credentials
- **WHEN** a custom proxy URL contains a username, password, token, or other user-information component
- **THEN** the system rejects the save without persisting or returning the credential-bearing value

#### Scenario: Custom endpoint is invalid
- **WHEN** a custom proxy URL uses an unsupported scheme, lacks a valid host or port where required, or contains control characters
- **THEN** the system rejects the save and retains the previous valid policy

#### Scenario: Policy save succeeds
- **WHEN** a valid mode and its mode-specific fields are saved
- **THEN** the system atomically replaces the Manager network policy with owner-only access and returns a redacted status projection

#### Scenario: Policy state is logged or returned
- **WHEN** status, diagnostics, errors, or audit events include network-policy information
- **THEN** they contain only mode, source, redacted host and port, bypass count, duration, and outcome without proxy credentials, OAuth data, API keys, query strings, or response bodies

### Requirement: Observable network diagnosis
The system SHALL show the selected mode, resolved source, redacted endpoint, bypass summary, support state, and latest in-memory connection-test result in a compact Network section under Config Doctor or advanced settings.

#### Scenario: Network status is opened
- **WHEN** the user opens the Network section
- **THEN** the system resolves and displays policy status without sending a network request

#### Scenario: Connection test succeeds
- **WHEN** the user explicitly runs a connection test and the resolved route completes DNS, proxy negotiation when applicable, TLS, and receives any valid HTTP response from the fixed official test origin
- **THEN** the system reports transport success with duration and redacted route information without requiring or sending ChatGPT OAuth credentials

#### Scenario: Connection test fails
- **WHEN** DNS, proxy connection, TLS, timeout, unsupported configuration, or another transport stage fails
- **THEN** the system reports a stable failure category, resolved source, and actionable next step without exposing raw secret-bearing errors

#### Scenario: Proxy authentication is required
- **WHEN** the resolved route returns a proxy-authentication challenge
- **THEN** the system reports authenticated proxies as unsupported in v1 and does not ask for or persist proxy credentials

