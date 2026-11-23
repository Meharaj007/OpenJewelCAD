$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path -Parent $PSScriptRoot
Push-Location $repoRoot
try {
    $manifest = Get-Content (Join-Path $repoRoot 'crates/cli/Cargo.toml') -Raw
    if ($manifest -notmatch '(?m)^version\s*=\s*"([^"]+)"') {
        throw 'Could not read the desktop app version from crates/cli/Cargo.toml.'
    }
    $version = $Matches[1]
    $projectName = "OpenJewelCAD-$version-windows-x64"

    $vcpkgRoot = $env:VCPKG_ROOT
    if (-not $vcpkgRoot) {
        $vcpkgRoot = Join-Path $HOME 'Documents/vcpkg'
    }
    $toolchain = Join-Path $vcpkgRoot 'scripts/buildsystems/vcpkg.cmake'
    if (-not (Test-Path $toolchain)) {
        throw "vcpkg was not found at '$vcpkgRoot'. Set VCPKG_ROOT to your vcpkg folder first."
    }

    $cmakeCache = Join-Path $repoRoot 'build/CMakeCache.txt'
    $normalizedToolchain = $toolchain -replace '\\', '/'
    if (Test-Path $cmakeCache) {
        $cacheLine = Select-String -Path $cmakeCache -Pattern '^CMAKE_TOOLCHAIN_FILE:[^=]*=' | Select-Object -First 1
        $cachedToolchain = if ($cacheLine) { ($cacheLine.Line -split '=', 2)[1].Trim() } else { '' }
        $normalizedCachedToolchain = $cachedToolchain -replace '\\', '/'
        if (-not $normalizedCachedToolchain -or $normalizedCachedToolchain -ne $normalizedToolchain) {
            throw "The existing build folder uses '$cachedToolchain'. Remove or reconfigure it to use '$normalizedToolchain', then run the alpha packager again."
        }
    }
    else {
        cmake -S . -B build -DCMAKE_TOOLCHAIN_FILE="$normalizedToolchain" -DVCPKG_TARGET_TRIPLET=x64-windows
        if ($LASTEXITCODE -ne 0) { throw 'CMake configuration failed.' }
    }
    cmake --build build --config Release
    if ($LASTEXITCODE -ne 0) { throw 'The native Release build failed.' }
    ctest --test-dir build -C Release --output-on-failure
    if ($LASTEXITCODE -ne 0) { throw 'The native OCCT smoke test failed.' }

    cargo fmt --all -- --check
    if ($LASTEXITCODE -ne 0) { throw 'Rust formatting check failed.' }

    $kernelDir = Join-Path $repoRoot 'build/Release'
    $kernelDll = Join-Path $kernelDir 'openjewelcad_kernel.dll'
    if (-not (Test-Path $kernelDll)) { throw "The Release kernel DLL was not created at '$kernelDll'." }
    $env:OPENJEWELCAD_KERNEL_DLL = $kernelDll
    $env:PATH = "$kernelDir;$env:PATH"

    cargo test --locked --workspace
    if ($LASTEXITCODE -ne 0) { throw 'The Rust workspace tests failed.' }
    cargo build --locked --release -p openjewelcad-cli
    if ($LASTEXITCODE -ne 0) { throw 'The desktop Release build failed.' }

    $outputDir = Join-Path $repoRoot 'dist'
    New-Item -ItemType Directory -Path $outputDir -Force | Out-Null
    $stageParent = Join-Path $env:TEMP ("OpenJewelCAD-alpha-" + [guid]::NewGuid().ToString('N'))
    $packageDir = Join-Path $stageParent $projectName
    $kernelPackageDir = Join-Path $packageDir 'kernel'
    $noticesDir = Join-Path $packageDir 'THIRD_PARTY_NOTICES'
    New-Item -ItemType Directory -Path $kernelPackageDir -Force | Out-Null
    New-Item -ItemType Directory -Path $noticesDir -Force | Out-Null

    Copy-Item (Join-Path $repoRoot 'target/release/openjewelcad.exe') $packageDir
    Copy-Item (Join-Path $kernelDir '*.dll') $kernelPackageDir
    Copy-Item (Join-Path $repoRoot 'LICENSE') $packageDir
    Copy-Item (Join-Path $repoRoot 'ALPHA_RELEASE_NOTES.md') $packageDir
    Copy-Item (Join-Path $repoRoot 'Start-OpenJewelCAD-Portable.bat') $packageDir

    $installedShare = Join-Path $repoRoot 'build/vcpkg_installed/x64-windows/share'
    if (-not (Test-Path $installedShare)) {
        $installedShare = Join-Path $vcpkgRoot 'installed/x64-windows/share'
    }
    if (-not (Test-Path $installedShare)) { throw "vcpkg license notices were not found under '$installedShare'." }
    $vcpkgNoticeDir = Join-Path $noticesDir 'vcpkg'
    New-Item -ItemType Directory -Path $vcpkgNoticeDir -Force | Out-Null
    $noticeCount = 0
    Get-ChildItem $installedShare -Directory | ForEach-Object {
        $copyrightFile = Join-Path $_.FullName 'copyright'
        if (Test-Path $copyrightFile) {
            Copy-Item $copyrightFile (Join-Path $vcpkgNoticeDir "$($_.Name)-copyright.txt")
            $noticeCount++
        }
    }
    if ($noticeCount -eq 0) { throw 'No vcpkg dependency copyright notices were found.' }

    $metadata = cargo metadata --format-version 1 --locked --filter-platform x86_64-pc-windows-msvc | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0) { throw 'Could not inspect the Rust dependency licenses.' }
    $cargoNoticeDir = Join-Path $noticesDir 'rust-crates'
    New-Item -ItemType Directory -Path $cargoNoticeDir -Force | Out-Null
    $packageIndex = 0
    $dependencyRows = [System.Collections.Generic.List[object]]::new()
    foreach ($dependency in $metadata.packages) {
        if ($dependency.source -notlike 'registry+*') { continue }
        $crateDir = Split-Path -Parent $dependency.manifest_path
        $licenseFiles = Get-ChildItem $crateDir -File | Where-Object {
            $_.Name -match '^(LICENSE|LICENCE|COPYING|NOTICE)([-_.].*)?$'
        }
        $packageIndex++
        $crateNotice = Join-Path $cargoNoticeDir ("{0}-{1}" -f $dependency.name, $dependency.version)
        New-Item -ItemType Directory -Path $crateNotice -Force | Out-Null
        foreach ($licenseFile in $licenseFiles) {
            Copy-Item $licenseFile.FullName $crateNotice
        }
        $dependencyRows.Add([PSCustomObject]@{
            Name = $dependency.name
            Version = $dependency.version
            License = $dependency.license
            Authors = ($dependency.authors -join '; ')
            Repository = $dependency.repository
            IncludedLicenseFiles = (($licenseFiles.Name | Sort-Object) -join '; ')
        })
    }
    if ($packageIndex -eq 0) { throw 'No registry Rust dependencies were found in Cargo metadata.' }
    $dependencyRows | Export-Csv -NoTypeInformation -Encoding utf8 (Join-Path $cargoNoticeDir 'DEPENDENCIES.csv')

    $noticesReadme = @"
Third-party notices for Open Jewel CAD $version

The vcpkg directory contains copyright/license notices for the installed native dependency set, including Open CASCADE Technology. The rust-crates directory lists Windows-target registry dependencies with their declared SPDX license expressions and includes license texts provided in their crate archives. Each component remains under its own license; some crate archives declare licenses without bundling a separate license text.
"@
    Set-Content -Path (Join-Path $noticesDir 'README.txt') -Value $noticesReadme -Encoding utf8

    $zipPath = Join-Path $outputDir "$projectName.zip"
    if (Test-Path $zipPath) { [System.IO.File]::Delete($zipPath) }
    Add-Type -AssemblyName System.IO.Compression
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $archive = [System.IO.Compression.ZipFile]::Open($zipPath, [System.IO.Compression.ZipArchiveMode]::Create)
    try {
        Get-ChildItem $packageDir -Recurse -File | ForEach-Object {
            $relativePath = $_.FullName.Substring($packageDir.Length).TrimStart('\', '/') -replace '\\', '/'
            $entry = $archive.CreateEntry($relativePath, [System.IO.Compression.CompressionLevel]::Optimal)
            $entry.LastWriteTime = [DateTimeOffset]::new(2026, 11, 23, 0, 0, 0, [TimeSpan]::Zero)
            $sourceStream = [System.IO.File]::OpenRead($_.FullName)
            $targetStream = $entry.Open()
            try {
                $sourceStream.CopyTo($targetStream)
            }
            finally {
                $targetStream.Dispose()
                $sourceStream.Dispose()
            }
        }
    }
    finally {
        $archive.Dispose()
    }
    if (-not (Test-Path $zipPath)) { throw 'The alpha ZIP was not created.' }
    $hash = (Get-FileHash $zipPath -Algorithm SHA256).Hash.ToLowerInvariant()
    Set-Content -Path "$zipPath.sha256" -Value "$hash  $([IO.Path]::GetFileName($zipPath))" -Encoding ascii

    Write-Host "Prepared $zipPath"
    Write-Host "SHA-256: $hash"
    Write-Host 'This script prepares a local alpha package. It does not publish a GitHub release.'
}
finally {
    Pop-Location
}
