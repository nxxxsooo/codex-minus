# Windows Known Folder APIs cannot be redirected by HOME. Run only on a disposable hosted runner,
# preserve any image/test residue, and give each acceptance command empty app-owned directories.
param(
  [Parameter(Mandatory = $true)][string]$Program,
  [string[]]$CommandArguments = @()
)
$ErrorActionPreference = "Stop"
if ($env:GITHUB_ACTIONS -ne "true" -or $env:RUNNER_OS -ne "Windows") {
  throw "Requires a disposable Windows Actions runner and a command"
}
$profile = [Environment]::GetFolderPath("UserProfile")
$backup = Join-Path $profile (".codex-minus-ci-" + [Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $backup | Out-Null
$names = @(".codex", ".codex-session-delete")
$prepared = @()
$exitCode = 1
try {
  foreach ($name in $names) {
    $path = Join-Path $profile $name
    if (Test-Path -LiteralPath $path) { Move-Item -LiteralPath $path -Destination (Join-Path $backup $name) }
    $prepared += $name
  }
  & $Program @CommandArguments
  $exitCode = $LASTEXITCODE
} finally {
  foreach ($name in $prepared) {
    $path = Join-Path $profile $name
    if (Test-Path -LiteralPath $path) { Remove-Item -LiteralPath $path -Recurse -Force }
    $saved = Join-Path $backup $name
    if (Test-Path -LiteralPath $saved) { Move-Item -LiteralPath $saved -Destination $path }
  }
  # Nonrecursive removal fails without prompting if anything could not be restored.
  [IO.Directory]::Delete($backup, $false)
}
exit $exitCode
