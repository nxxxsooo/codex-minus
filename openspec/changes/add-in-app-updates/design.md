# Design — add-in-app-updates

## D1. Channel: a static manifest on the GitHub release, no server

The updater needs a URL that always describes the newest version. A dedicated update server is
infrastructure this product has none of and wants none of; GitHub already hosts the artifacts and
serves `releases/latest/download/<asset>` as a stable redirect. The release job publishes
`latest.json` (version, publish date, per-platform artifact URL + detached signature) as one more
release asset, and the app's endpoint is pinned to
`https://github.com/nxxxsooo/codex-minus/releases/latest/download/latest.json`. Inside the
manifest, artifact URLs use the tag-specific `releases/download/vX/` form so a later release never
retargets an older manifest's links.

## D2. Key custody: one minisign keypair, private key never in the repo

`tauri signer generate` produced the keypair on the owner's Mac. The private key lives in
`~/.tauri/codex-minus-updater.key` (mode 600) and in the repository secret
`TAURI_SIGNING_PRIVATE_KEY`; it has no password (custody is the file plus the secret — a password
would be a second secret stored beside the first). The public key is pinned in `tauri.conf.json`
and ships inside every build, so an installed app only accepts updates signed by that key.
Losing the private key does not brick installs — it only means future updates must be installed
manually once, with a new keypair pinned from then on. The owner should back the key file up with
their usual secrets.

## D3. UX: silent startup check, persistent banner, one-click install

One check at startup, no periodic timer, no manual button: the fleet opens the manager to change
provider settings, which is exactly when an update matters, and a failed check (offline, GitHub
unreachable) is silence, not a notice. A newer version shows a slim persistent banner naming it;
its single action downloads with visible progress, verifies, installs, and relaunches via the
process plugin. An install failure turns the banner action into 重试 and reports through the
standard failure notice with 详情 — never a spinner, never a bare code (D3 of the parent
simplification applies unchanged). Presentation rules live in a leaf module beside their test;
App.tsx only wires.

## D4. One Windows format: NSIS only, and the MSI is dropped

Tauri's updater applies NSIS packages, so a machine that wants in-app updates must have installed
from the `-setup.exe` — updating an MSI install with the NSIS package leaves two entries in
"Apps". Keeping the MSI on the release page would therefore keep alive exactly one thing: the
possibility of installing through the channel updates cannot ride. MSI's actual constituency —
Group-Policy / enterprise silent deployment — does not exist in this fleet, so the MSI target,
its WiX language config, and its build/packaging/assertion lines are removed rather than
documented around (user decision 2026-08-14:「只留一个」). A machine that installed an older
`.msi` uninstalls it before the 0.4.8 hand-install — the release notes say so, and user data
under `~/.codex-session-delete/` survives uninstall by design. `installMode: "passive"` shows the
native progress UI without asking questions; the NSIS installer carries SimpChinese and English.

## D5. macOS artifact: the ad-hoc-signed .app, tarred by the build

`signingIdentity: "-"` means the .app that `tauri build` bundles is already ad-hoc signed, so the
`.app.tar.gz` + `.sig` that `createUpdaterArtifacts` emits during the same build are the correct
update payload as-is; the workflow's post-build `codesign --force --deep` re-sign only feeds the
hand-install zip. The updater extracts over the installed bundle without setting the quarantine
attribute, so the replaced app launches without a Gatekeeper prompt.

## D6. CI: builds always sign; the release job composes the manifest

`TAURI_SIGNING_PRIVATE_KEY` is injected from the secret in every build (PRs included — same-repo
branches see secrets, and a missing secret fails the build loudly rather than shipping unsigned
artifacts). Build jobs upload the `.sig` files beside the installers; the release job renames
installer and signature together, then writes `latest.json` with `jq` from the renamed names and
signature contents, and the existing one-artifact-per-slot assertions extend to the signatures so
a build that silently stopped signing fails the release, not the fleet.

## D7. Verifiability

An updater can only be proven by updating: the real test is a locally built older version pointed
at the published manifest. After the first release carrying this feature, the check is: build the
app locally with a lower version number, launch, see the banner, click through, and land in the
released version. That drill is the change's verification task, on macOS and the Windows VM both.
