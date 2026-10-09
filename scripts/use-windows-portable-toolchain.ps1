# Dot-source this file to activate an already-downloaded portable toolchain.
# It changes only this PowerShell process; no registry or machine PATH changes.
param([string]$ToolchainRoot = (Join-Path $PSScriptRoot '../.local/setup'))
$taskRoot = (Resolve-Path -LiteralPath $ToolchainRoot -ErrorAction Stop).Path
$taskLlvm = Join-Path $taskRoot 'llvm/bin'
$taskCrt = Get-ChildItem -LiteralPath (Join-Path $taskRoot 'winsysroot/VC/Tools/MSVC') -Directory | Sort-Object { [version]$_.Name } -Descending | Select-Object -First 1
$taskSdk = Join-Path $taskRoot 'winsysroot/Windows Kits/10'
$taskSdkVersion = Get-ChildItem -LiteralPath (Join-Path $taskSdk 'Include') -Directory | Sort-Object { [version]$_.Name } -Descending | Select-Object -First 1
$taskCompiler = Join-Path $taskLlvm 'clang-cl.exe'
$taskLinker = Join-Path $taskLlvm 'lld-link.exe'
$taskIncludes = @("$($taskCrt.FullName)/include", "$($taskSdkVersion.FullName)/ucrt", "$($taskSdkVersion.FullName)/um", "$($taskSdkVersion.FullName)/shared")
$taskLibraries = @("$($taskCrt.FullName)/lib/x64", "$taskSdk/Lib/$($taskSdkVersion.Name)/ucrt/x64", "$taskSdk/Lib/$($taskSdkVersion.Name)/um/x64")
foreach ($taskPath in @($taskCompiler, $taskLinker) + $taskIncludes + $taskLibraries) {
    if (!(Test-Path -LiteralPath $taskPath)) { throw "Portable toolchain incomplete: $taskPath" }
}
$env:PATH = "$env:USERPROFILE\.cargo\bin;$taskLlvm;$env:PATH"
$env:CC = $taskCompiler
$env:CXX = $taskCompiler
$env:RC = Join-Path $taskLlvm 'llvm-rc.exe'
$env:CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER = $taskLinker
$env:INCLUDE = $taskIncludes -join ';'
$env:LIB = $taskLibraries -join ';'
Write-Host "Portable Windows x64 toolchain active for this PowerShell session."
