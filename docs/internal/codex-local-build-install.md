# Build and install Codex Local on Windows

Run these commands from PowerShell.

## Build and install the latest source

```powershell
Set-Location 'C:\Repos\AI\codex'
& .\scripts\install\install-local-dev.ps1 -RepositoryRoot 'C:\Repos\AI\codex'
```

The script builds release binaries and installs `codex-local.exe` and
`codex-code-mode-host.exe` under
`$env:LOCALAPPDATA\Programs\codex-local\bin`.

Exit every running `codex-local` session before the installation copies the
new binaries. Open a new PowerShell window afterward so a newly added user
`PATH` entry is visible.

## Reinstall an existing release build

Use this only when the release binaries in `codex-rs\target\release` are
already current:

```powershell
Set-Location 'C:\Repos\AI\codex'
& .\scripts\install\install-local-dev.ps1 -RepositoryRoot 'C:\Repos\AI\codex' -SkipBuild
```

## Start Codex Local

```powershell
codex-local
```

For the local actor model, enter context length `32768` when prompted.
