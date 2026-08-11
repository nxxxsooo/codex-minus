## ADDED Requirements

### Requirement: Network-policy-aware official refresh
The system MUST apply one immutable resolved Manager network-policy snapshot to each isolated official catalog refresh while preserving the existing safe child-environment allowlist, non-refreshable OAuth projection, target trust validation, and live-auth independence.

#### Scenario: Resolved proxy refresh succeeds
- **WHEN** `auto` or `custom` resolves a supported proxy route and the verified target CLI completes a remote official refresh through it
- **THEN** the system validates and commits the target-emitted remote catalog under the existing official-baseline rules and records only the redacted network-policy source in diagnostics

#### Scenario: Direct refresh succeeds
- **WHEN** `auto` resolves to direct access or the user selects `direct` and the verified target CLI completes a remote official refresh
- **THEN** the system validates and commits the result without inheriting process or operating-system proxy endpoints

#### Scenario: Network policy is invalid or unsupported
- **WHEN** the selected Manager network policy cannot produce a supported resolved snapshot
- **THEN** the system does not project OAuth credentials or start the target CLI, retains the last validated baseline, and reports the network-policy action required

#### Scenario: Target falls back to bundled models
- **WHEN** the target CLI exits successfully but does not create the isolated remote cache required to prove an official refresh
- **THEN** the system treats the output as a network refresh failure rather than a successful remote catalog, retains the last validated baseline, and reports that the target fell back to bundled models with the redacted policy source

#### Scenario: Resolved route is unreachable
- **WHEN** the isolated target CLI times out, cannot resolve or connect, or cannot complete proxy or TLS negotiation through the resolved route
- **THEN** the system cleans private temporary state, leaves live auth and live configuration unchanged, retains the last validated baseline, and returns a sanitized network-specific failure category

#### Scenario: Child environment is constructed
- **WHEN** a resolved policy is applied to the isolated target CLI
- **THEN** only normalized proxy and bypass variables from that snapshot join the existing non-secret child allowlist, while API keys, access-token variables, provider endpoints, auth endpoints, and unrelated process variables remain excluded

