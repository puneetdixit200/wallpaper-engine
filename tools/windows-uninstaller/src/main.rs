#[cfg(any(test, target_os = "windows"))]
const POWERSHELL_UNINSTALLER: &str = r#"
$ErrorActionPreference = 'Stop'

function Write-Step([string] $Message) {
  Write-Host "[Wallpaper Engine Uninstaller] $Message"
}

function Remove-RunValues([string] $KeyPath) {
  if (-not (Test-Path $KeyPath)) {
    return
  }

  $item = Get-ItemProperty -Path $KeyPath -ErrorAction SilentlyContinue
  if ($null -eq $item) {
    return
  }

  $item.PSObject.Properties |
    Where-Object {
      $_.Name -match 'wallpaper|com\.puneetdixit\.wallpaperengine' -or
      [string]$_.Value -match 'wallpaper-engine|Wallpaper Engine|com\.puneetdixit\.wallpaperengine'
    } |
    ForEach-Object {
      Write-Step "Removing startup entry $($_.Name)"
      Remove-ItemProperty -Path $KeyPath -Name $_.Name -ErrorAction SilentlyContinue
    }
}

function Split-UninstallCommand([string] $CommandLine) {
  if ($CommandLine -match '^\s*"([^"]+)"\s*(.*)$') {
    return @($Matches[1], $Matches[2])
  }

  $parts = $CommandLine.Trim() -split '\s+', 2
  if ($parts.Count -eq 1) {
    return @($parts[0], '')
  }

  return @($parts[0], $parts[1])
}

function Invoke-RegisteredUninstaller([object] $Entry) {
  $commandLine = $Entry.QuietUninstallString
  if ([string]::IsNullOrWhiteSpace($commandLine)) {
    $commandLine = $Entry.UninstallString
  }

  if ([string]::IsNullOrWhiteSpace($commandLine)) {
    return $false
  }

  $split = Split-UninstallCommand $commandLine
  $exe = $split[0]
  $arguments = $split[1]

  if ($exe -match '(?i)msiexec(\.exe)?$') {
    $arguments = $arguments -replace '(?i)/I\s*', '/X '
    if ($arguments -notmatch '(?i)(/quiet|/passive|/qn)') {
      $arguments = "$arguments /passive"
    }
  } elseif ($arguments -notmatch '(?i)(/S|/silent|/quiet)') {
    $arguments = "$arguments /S"
  }

  Write-Step "Running registered uninstaller"
  $process = Start-Process -FilePath $exe -ArgumentList $arguments -Wait -PassThru
  if ($process.ExitCode -ne 0) {
    Write-Step "Registered uninstaller exited with code $($process.ExitCode)"
  }

  return $true
}

function Remove-IfExists([string] $Path) {
  if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path)) {
    return
  }

  Write-Step "Removing $Path"
  Remove-Item -LiteralPath $Path -Recurse -Force -ErrorAction SilentlyContinue
}

Write-Step "Stopping running app processes"
Get-Process -Name 'wallpaper-engine' -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue

Write-Step "Removing startup registrations"
Remove-RunValues 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
Remove-RunValues 'HKLM:\Software\Microsoft\Windows\CurrentVersion\Run'

$startupFolders = @(
  [Environment]::GetFolderPath('Startup'),
  [Environment]::GetFolderPath('CommonStartup')
)
foreach ($folder in $startupFolders) {
  if (Test-Path $folder) {
    Get-ChildItem -LiteralPath $folder -Filter '*Wallpaper Engine*.lnk' -ErrorAction SilentlyContinue |
      ForEach-Object { Remove-IfExists $_.FullName }
  }
}

$uninstallRoots = @(
  'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*',
  'HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*',
  'HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*'
)

$entry = Get-ItemProperty -Path $uninstallRoots -ErrorAction SilentlyContinue |
  Where-Object {
    $_.DisplayName -eq 'Wallpaper Engine' -or
    $_.DisplayName -like 'Wallpaper Engine*' -or
    $_.PSChildName -eq 'com.puneetdixit.wallpaperengine' -or
    $_.UninstallString -match 'wallpaper-engine|Wallpaper Engine'
  } |
  Select-Object -First 1

if ($null -ne $entry) {
  [void](Invoke-RegisteredUninstaller $entry)
} else {
  Write-Step "No registered uninstaller found; cleaning known install paths"
}

$knownPaths = [System.Collections.Generic.List[string]]::new()
@(
  "$env:LOCALAPPDATA\Programs\Wallpaper Engine",
  "$env:LOCALAPPDATA\Wallpaper Engine",
  "$env:APPDATA\Wallpaper Engine",
  "$env:APPDATA\com.puneetdixit.wallpaperengine",
  "$env:LOCALAPPDATA\com.puneetdixit.wallpaperengine",
  "$env:LOCALAPPDATA\puneetdixit.wallpaperengine"
) | ForEach-Object { $knownPaths.Add($_) }

@($env:ProgramFiles, ${env:ProgramFiles(x86)}) |
  Where-Object { -not [string]::IsNullOrWhiteSpace($_) } |
  ForEach-Object { $knownPaths.Add((Join-Path $_ 'Wallpaper Engine')) }

foreach ($path in $knownPaths) {
  Remove-IfExists $path
}

Write-Step "Done"
"#;

#[cfg(target_os = "windows")]
fn main() {
    let status = std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            POWERSHELL_UNINSTALLER,
        ])
        .status()
        .unwrap_or_else(|error| {
            eprintln!("Could not start PowerShell uninstaller: {error}");
            std::process::exit(1);
        });

    std::process::exit(status.code().unwrap_or(1));
}

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("Wallpaper Engine uninstaller is only available on Windows.");
}

#[cfg(test)]
mod tests {
    use super::POWERSHELL_UNINSTALLER;

    #[test]
    fn windows_uninstaller_targets_app_specific_entries() {
        assert!(POWERSHELL_UNINSTALLER.contains("com.puneetdixit.wallpaperengine"));
        assert!(POWERSHELL_UNINSTALLER.contains("Wallpaper Engine"));
        assert!(POWERSHELL_UNINSTALLER.contains("UninstallString"));
    }
}
