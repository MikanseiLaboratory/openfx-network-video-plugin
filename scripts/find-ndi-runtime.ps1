$ErrorActionPreference = "Stop"

$candidates = @()
if ($env:NDI_SDK_DIR) {
    $candidates += (Join-Path $env:NDI_SDK_DIR "Bin\x64")
}
if ($env:NDI_RUNTIME_DIR_V6) {
    $candidates += $env:NDI_RUNTIME_DIR_V6
}
$candidates += "C:\Program Files\NDI\NDI 6 SDK\Bin\x64"
$candidates += "C:\Program Files\NDI\NDI 6 Runtime\v6"

$dllName = "Processing.NDI.Lib.x64.dll"
$dir = $candidates | Where-Object { Test-Path (Join-Path $_ $dllName) } | Select-Object -First 1
if (-not $dir) {
    throw "NDI runtime DLL not found. Install the NDI 6 SDK or set NDI_SDK_DIR."
}

$dll = Join-Path $dir $dllName
$licenses = @(
    (Join-Path $dir "Processing.NDI.Lib.Licenses.txt"),
    (Join-Path (Split-Path $dir -Parent) "Processing.NDI.Lib.Licenses.txt"),
    (Join-Path $env:NDI_SDK_DIR "Processing.NDI.Lib.Licenses.txt"),
    "C:\Program Files\NDI\NDI 6 SDK\Processing.NDI.Lib.Licenses.txt",
    "C:\Program Files\NDI\NDI 6 Runtime\v6\Processing.NDI.Lib.Licenses.txt"
) | Where-Object { $_ -and (Test-Path $_) } | Select-Object -First 1

if (-not $licenses) {
    throw "Processing.NDI.Lib.Licenses.txt was not found next to the NDI runtime."
}

[pscustomobject]@{
    Dir      = $dir
    Dll      = $dll
    Licenses = $licenses
}
