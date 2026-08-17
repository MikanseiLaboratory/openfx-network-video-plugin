$ErrorActionPreference = "Stop"

$NDI_SDK_URL = if ($env:NDI_SDK_URL) { $env:NDI_SDK_URL } else { "https://downloads.ndi.tv/SDK/NDI_SDK/NDI%206%20SDK.exe" }
$LLVM_VERSION = if ($env:LLVM_VERSION) { $env:LLVM_VERSION } else { "20.1.8" }
$accepted = @(
    "4D5DD36A1C7C7634F408BF459B068787CCE6F5310A3EFE832D76B1DDEB54E499",
    "97E8993C6B94213A950E6ACBE9AA4D35831D137324D762F7C74851D3FEEF80D9"
)

$sdkDir = if ($env:NDI_SDK_DIR) { $env:NDI_SDK_DIR } else { Join-Path $env:TEMP "ndi-sdk" }
$installer = Join-Path $env:TEMP "NDI_SDK_Installer.exe"

if (-not (Test-Path "C:\Program Files\LLVM\bin\clang.exe")) {
    choco install llvm --version=$LLVM_VERSION -y
}

if (-not (Test-Path (Join-Path $sdkDir "include\Processing.NDI.Lib.h"))) {
    Write-Host "Downloading NDI SDK..."
    curl.exe -L -o $installer $NDI_SDK_URL
    $actual = (Get-FileHash $installer -Algorithm SHA256).Hash.ToUpperInvariant()
    if ($accepted -notcontains $actual) {
        throw "NDI SDK hash mismatch. Got $actual. Accepted: $($accepted -join ', ')"
    }
    Write-Host "NDI SDK hash verified: $actual"
    $proc = Start-Process -FilePath $installer -ArgumentList "/VERYSILENT","/SP-","/SUPPRESSMSGBOXES","/NORESTART","/NOCANCEL","/DIR=$sdkDir","/LOG=$env:TEMP\ndi_install.log" -PassThru
    if (-not $proc.WaitForExit(300000)) {
        $proc | Stop-Process -Force
    }
    if (-not (Test-Path (Join-Path $sdkDir "include\Processing.NDI.Lib.h"))) {
        throw "NDI SDK installation failed - header file not found"
    }
}

$bin = Join-Path $sdkDir "Bin\x64"
$env:NDI_SDK_DIR = $sdkDir
$env:Path = "$bin;C:\Program Files\LLVM\bin;$env:Path"
if ($env:GITHUB_ENV) {
    "NDI_SDK_DIR=$sdkDir" | Out-File $env:GITHUB_ENV -Append -Encoding utf8
    $bin | Out-File $env:GITHUB_PATH -Append -Encoding utf8
    "C:\Program Files\LLVM\bin" | Out-File $env:GITHUB_PATH -Append -Encoding utf8
}
Write-Host "NDI_SDK_DIR=$sdkDir"
