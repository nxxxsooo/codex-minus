# Proposal

## Why

Codex Minus needs consistent Chromium rendering and a unified desktop debugging workflow across macOS and Windows. The user has approved an Electron migration with a full visual redesign, while retaining the existing Rust domain core and the current provider, catalog, session and configuration guarantees.

## What Changes

- **BREAKING — desktop runtime and distribution:** replace the Tauri shell, plugins and packaging with Electron, a restricted preload bridge, and a managed Rust backend process.
- Redesign application navigation, provider lists and detail editing, model tables, session management, diagnostic dialogs, preferences and update presentation in one coherent light/dark visual system.
- Preserve existing stored data locations, OAuth ownership, Context protection, transactional commits, catalog ownership, explicit activation and session lifecycle semantics.
- Integrate the already-open session cleanup (#48) and long-context/image controls (#49) into this branch before finishing the remaining Electron surfaces; preserve their reviewed safety contracts alongside the more recent supplemental model-list work.
- Review the provider workflow first, then migrate the remaining surfaces after its implemented review milestone.
- Research UI appearance only, present attributable reference images, and obtain approval of an isolated high-fidelity preview before production implementation.
- Deliver the completed v0.5.0 Electron application from `codex/electron-redesign`. The user later authorized publication and explicitly selected automatic upgrade from the old Tauri release; replacement of the maintainer's installed app remains separately authorized.

## Capabilities

### New Capabilities

- `electron-desktop-runtime`: native lifecycle, restricted IPC, backend supervision, existing-data continuity and verified desktop distribution/update behavior under Electron.
- `desktop-workspace-presentation`: coherent desktop navigation, appearance, responsive work surfaces and accessible state presentation across preserved workflows.

### Modified Capabilities

None. The existing catalog and native-capability requirements are preservation boundaries rather than new business behavior in this change.

## Impact

- `src/App.tsx`, `src/styles.css`, shared UI primitives and extracted screen components.
- New Electron main/preload and frontend desktop adapter; `src/backend-types.ts` remains the domain message contract.
- `src-tauri/src/lib.rs`, command/runtime wrappers and crate layout; retain Rust transaction, catalog, session and pinned upstream core implementations.
- `package.json`, lockfiles, Vite/build configuration, CI, packaging, signed updater integration, release/version checks and platform documentation.
- Current supported artifacts: macOS arm64, Windows x64 and Windows arm64. Native Windows runtime verification needs an appropriate host; cross-compilation alone is insufficient evidence.

## Visual artifact location

The design source of truth is the user's Stitch project `15861189140316747035`, with local artifacts in the governed workspace's `Designs/personal/codex-minus/` directory. Its `index.html` is the suite review entry; `bauhaus/manifest.json` maps 36 review slots to actual MCP screen IDs and export URLs. The selected baseline is user-labeled `SCREEN_8`, Light Mode Bauhaus, MCP screen `7721fbec6a8f42ed8b42dd710037140e`, using design system `assets/13370780385722276554`. Pencil is a retired exploration, not the current design workflow. This OpenSpec change owns the implementation contract and links to that design home. The initial repository-local `review/` was relocated after the user's storage correction.

## Confirmed Product Brief

- **User/context:** existing Codex Minus desktop users managing providers, models and local sessions.
- **Outcome:** consistent rendering/debugging and a clearer, cohesive full-application interface.
- **Confirmed scope:** Electron shell plus existing Rust core; current macOS/Windows support and existing data; full UI redesign; provider workflow as the first review slice.
- **Preserve:** bilingual and light/dark presentation, provider edits and explicit mode transitions, model editing/ownership, session archive/delete/adaptation semantics, official auth ownership, fail-closed configuration transactions and user-confirmed Codex restart.
- **Non-goals:** backend rewrite, new provider protocols, new model policies, feature removal, publishing or live installation during design.
- **Success:** representative provider edit/save/activation is understandable and verifiable in Electron, existing safety invariants remain true, and every remaining supported surface follows the approved visual system.
- **Evidence:** source baseline `60fa8ab` / `v0.4.18`; `src/App.tsx` currently mounts provider and session routes, with Doctor inside provider detail. Old architectural descriptions mentioning a standalone Doctor route are not the current UI inventory.
- **Residual risks:** sidecar death during writes, old/new manager concurrency, updater transition across runtimes, packaged process/resource paths, native lifecycle differences and increased Chromium resource use.
- **Approval:** Product Brief and research/preview choices confirmed in conversation on 2026-09-22. The initial Linear/Raycast/TablePlus direction was superseded by the user's selection of Stitch `SCREEN_8`, Light Mode Bauhaus. After delivery of the 36-screen suite and the requested continuation, the user explicitly selected “确认，开始实现” for the completed Build Contract. Production implementation is authorized under that contract; the first implemented provider workflow remains the review milestone before wider UI migration.
