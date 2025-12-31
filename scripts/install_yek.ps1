# install_yek.ps1
# Install Yek on Windows via PowerShell
param(
    [string]$InstallDir = "$HOME\.local\bin"
)

# Exit on error
$ErrorActionPreference = "Stop"

Write-Host "Yek Windows Installer"

if (!(Test-Path -Path $InstallDir)) {
    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
}

Write-Host "Selected install directory: $InstallDir"

# Detect architecture
$arch = $ENV:PROCESSOR_ARCHITECTURE
switch ($arch) {
    "AMD64" { $target = "x86_64-pc-windows-msvc" }
    "ARM64" { $target = "aarch64-pc-windows-msvc" }
    default {
        Write-Host "Unsupported or unknown architecture: $arch"
        Write-Host "Please build from source or check for a compatible artifact."
        exit 1
    }
}

$repoOwner = "mohsen1"
$repoName  = "yek"
$assetName = "yek-$target.zip"

Write-Host "OS/ARCH => Windows / $arch"
Write-Host "Asset name => $assetName"

Write-Host "Fetching latest release info from GitHub..."
$releasesUrl  = "https://api.github.com/repos/$repoOwner/$repoName/releases/latest"
try {
    $releaseData = Invoke-RestMethod -Uri $releasesUrl
} catch {
    Write-Host "Failed to fetch release info from GitHub."
    Write-Host "Please build from source or check back later."
    exit 1
}

# Find the asset download URL
$asset = $releaseData.assets | Where-Object { $_.name -eq $assetName }
if (!$asset) {
    Write-Host "Failed to find an asset named $assetName in the latest release."
    Write-Host "Check that your OS/ARCH is built or consider building from source."
    exit 1
}

$downloadUrl = $asset.browser_download_url
Write-Host "Downloading from: $downloadUrl"

$zipPath = Join-Path $env:TEMP $assetName
Invoke-WebRequest -Uri $downloadUrl -OutFile $zipPath -UseBasicParsing

Write-Host "Extracting archive..."
$extractDir = Join-Path $env:TEMP "yek-$($arch)"
if (Test-Path $extractDir) {
    Remove-Item -Recurse -Force $extractDir
}
Expand-Archive -Path $zipPath -DestinationPath $extractDir

Write-Host "Moving binary to $InstallDir..."
$targetDir = Join-Path $extractDir "yek-$target"
$binaryPath = Join-Path $targetDir "yek.exe"
if (!(Test-Path $binaryPath)) {
    Write-Host "yek.exe not found in the extracted folder."
    exit 1
}
$destinationPath = Join-Path $InstallDir "yek.exe"
Move-Item -Path $binaryPath -Destination $destinationPath -Force

Write-Host "Cleanup temporary files..."
Remove-Item -Force $zipPath
Remove-Item -Recurse -Force $extractDir

Write-Host "Installation complete!"

# Check if $InstallDir is in PATH
$pathDirs = $ENV:PATH -split ";"
$resolvedInstallDir = Resolve-Path $InstallDir -ErrorAction SilentlyContinue
if ($resolvedInstallDir -and ($pathDirs -notcontains $resolvedInstallDir.Path)) {
    Write-Host "NOTE: $InstallDir is not in your PATH. Add it by running something like:"
    Write-Host "`$env:Path += `";$($resolvedInstallDir.Path)`""
    Write-Host "Or update your system's environment variables to persist this."
}

Write-Host "Now you can run: yek --help"