$ErrorActionPreference = "Stop"
if ($env:GITHUB_ACTIONS -ne "true") { throw "Run only on a disposable GitHub Actions Windows runner" }
$root = Join-Path $env:RUNNER_TEMP "codex-minus-legacy-upgrade"
New-Item -ItemType Directory -Force -Path $root | Out-Null
$installer = @(Get-ChildItem dist-desktop/CodexMinus_*-setup.exe)
if ($installer.Count -ne 1) { throw "Expected one Electron installer" }
$oldName = "CodexMinus_0.4.18_x64-setup.exe"
gh release download v0.4.18 --repo $env:GITHUB_REPOSITORY --pattern $oldName --pattern "$oldName.sig" --dir $root
if ($LASTEXITCODE -ne 0) { throw "Legacy artifact download failed" }
$old = Join-Path $root $oldName
$hash = (Get-FileHash $old -Algorithm SHA256).Hash.ToLowerInvariant()
& src-tauri/target/x86_64-pc-windows-msvc/release/codex-minus-core.exe --verify-update $old "$old.sig" $hash (Get-Item $old).Length
if ($LASTEXITCODE -ne 0) { throw "Legacy signature failed" }
$destination = Join-Path $root "installed"
$legacy = Start-Process $old -ArgumentList "/S", "/D=$destination" -Wait -PassThru
if ($legacy.ExitCode -ne 0) { throw "Legacy install failed" }
$key = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Codex Minus"
$registered = Get-ItemProperty $key
if ($registered.MainBinaryName -ne "codex-minus.exe" -or $registered.DisplayVersion -ne "0.4.18") { throw "Wrong legacy registration" }
if ($registered.InstallLocation.Trim('"') -ne $destination) { throw "Legacy install path mismatch" }
$env:USERPROFILE = Join-Path $root "home"
$env:HOME = $env:USERPROFILE
$env:CODEX_HOME = Join-Path $env:USERPROFILE ".codex"
$data = Join-Path $env:USERPROFILE ".codex-session-delete"
New-Item -ItemType Directory -Force -Path $env:CODEX_HOME, $data | Out-Null
$sentinels = @{
  (Join-Path $env:CODEX_HOME "auth.json") = "official-auth-sentinel"
  (Join-Path $env:CODEX_HOME "config.toml") = "protected-context-sentinel"
  (Join-Path $data "settings.json") = "saved-profiles-sentinel"
}
foreach ($item in $sentinels.GetEnumerator()) { [IO.File]::WriteAllText($item.Key, $item.Value) }
# Preserve the old updater's exact flags while making the requested restart headless on CI.
$env:ELECTRON_RUN_AS_NODE = "1"
$update = Start-Process $installer[0].FullName -ArgumentList "/P", "/R", "/UPDATE", "/ARGS" -Wait -PassThru
Remove-Item Env:ELECTRON_RUN_AS_NODE
if ($update.ExitCode -ne 0) { throw "Legacy automatic-upgrade handoff failed" }
if (Test-Path $key) { throw "Duplicate legacy uninstaller registration remains" }
foreach ($item in $sentinels.GetEnumerator()) {
  if ([IO.File]::ReadAllText($item.Key) -ne $item.Value) { throw "Upgrade changed user data" }
}
node scripts/verify-package.mjs "$destination" win32 x64
if ($LASTEXITCODE -ne 0) { throw "Upgraded Electron package failed" }
Write-Output '{"from":"0.4.18","legacyPathReused":true,"oldRegistryRetired":true,"dataUnchanged":true,"headlessRestart":true}'
