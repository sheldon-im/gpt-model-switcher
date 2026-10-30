param([Parameter(Mandatory=$true)][string]$OutputDirectory)
$ErrorActionPreference = 'Stop'
$repo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
Set-Location -LiteralPath $repo
if ((git status --porcelain | Out-String).Trim()) { throw 'Commit completed source and validation before packaging.' }
$outputPath = [IO.Path]::GetFullPath($OutputDirectory)
New-Item -ItemType Directory -Path $outputPath -Force | Out-Null
$packageWork = Join-Path $repo ('work/package-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $packageWork -Force | Out-Null
$sourceCommit = (git rev-parse HEAD).Trim()
$version = (Get-Content -LiteralPath (Join-Path $repo 'package.json') -Raw | ConvertFrom-Json).version
if ($version -notmatch '^\d+\.\d+\.\d+$') { throw 'Unexpected version' }

& cargo test --workspace --locked 2>&1 | Tee-Object -FilePath (Join-Path $packageWork 'cargo-tests.txt')
if ($LASTEXITCODE -ne 0) { throw 'Rust tests failed' }
& pnpm test 2>&1 | Tee-Object -FilePath (Join-Path $packageWork 'frontend-tests.txt')
if ($LASTEXITCODE -ne 0) { throw 'Frontend tests failed' }
& pnpm tauri build --bundles nsis
if ($LASTEXITCODE -ne 0) { throw 'Windows bundle failed' }

$rustText = Get-Content -LiteralPath (Join-Path $packageWork 'cargo-tests.txt') -Raw
$rustCount = 0
foreach ($match in [regex]::Matches($rustText, 'test result: ok\. (\d+) passed')) { $rustCount += [int]$match.Groups[1].Value }
$frontendText = Get-Content -LiteralPath (Join-Path $packageWork 'frontend-tests.txt') -Raw
$frontendText = [regex]::Replace($frontendText, '\x1b\[[0-9;]*m', '')
$frontendCount = [regex]::Match($frontendText,'Tests\s+(\d+) passed').Groups[1].Value
if ($rustCount -le 0 -or !$frontendCount) { throw 'Could not verify test result counts' }
$exe = Join-Path $repo 'target/release/gpt-model-switcher.exe'
$installer = Join-Path $repo "target/release/bundle/nsis/GPT Model Switcher_${version}_x64-setup.exe"
if (!(Test-Path -LiteralPath $exe) -or !(Test-Path -LiteralPath $installer)) { throw 'Missing Windows artifacts' }
$nvdaResources = Join-Path $repo 'target/release/nvda'
if (!(Test-Path -LiteralPath (Join-Path $nvdaResources 'nvdaControllerClient.dll'))) { throw 'Missing NVDA controller resource; do not ship a silent portable package.' }
$record = [ordered]@{
  recordedAtUtc = [DateTime]::UtcNow.ToString('o'); sourceCommit = $sourceCommit
  platform = 'Windows x64'; osVersion = [Environment]::OSVersion.Version.ToString()
  rustTestsPassed = $rustCount; frontendTestsPassed = [int]$frontendCount
  windowsReleaseBuild = 'passed'; nsisBuild = 'passed'
  windowsSigning = (Get-AuthenticodeSignature -LiteralPath $exe).Status.ToString()
  desktopSettingsRead = 'passed on desktop 26.1027.3384.0; no model mutation'
  generationGuard = 'passed on actual app; no model mutation'
  newRouteActualSwitch = 'user reports quiet switching; exact pairs not independently revalidated'
  NVDA = '0.1.0 user reported missing speech; 0.1.1 controller requests accepted in native test with foreground preserved; listening confirmation pending'
  nvdaControllerVersion = '2026.3, API 3.0, x64, unmodified signed upstream DLL'
  macOSBuild = 'not tested on Windows'; VoiceOver = 'not tested'
  currentLimit = 'Private desktop settings protocol; local work conversations only; reject ambiguous identity and unsupported views'
}
$recordPath = Join-Path $outputPath '검증-실행결과.json'
$record | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $recordPath -Encoding utf8
Copy-Item -LiteralPath (Join-Path $repo 'README.md') -Destination (Join-Path $outputPath '사용-검증-안내.md') -Force
Copy-Item -LiteralPath $installer -Destination (Join-Path $outputPath "gpt-model-switcher-${version}-windows-x64-test-setup.exe") -Force

$portable = Join-Path $packageWork 'gpt-model-switcher'
New-Item -ItemType Directory -Path $portable -Force | Out-Null
Copy-Item -LiteralPath $exe -Destination $portable
Copy-Item -LiteralPath $nvdaResources -Destination $portable -Recurse
Copy-Item -LiteralPath (Join-Path $repo 'README.md'),(Join-Path $repo 'docs/VERIFICATION.md'),$recordPath -Destination $portable
Compress-Archive -LiteralPath $portable -DestinationPath (Join-Path $outputPath "gpt-model-switcher-${version}-windows-x64-test.zip") -Force

$sourceZip = Join-Path $outputPath "gpt-model-switcher-${version}-macos-source.zip"
& git archive --format=zip --prefix=gpt-model-switcher/ "--output=$sourceZip" HEAD
if ($LASTEXITCODE -ne 0) { throw 'Source archive failed' }
Add-Type -AssemblyName System.IO.Compression.FileSystem
$archive = [IO.Compression.ZipFile]::Open($sourceZip, [IO.Compression.ZipArchiveMode]::Update)
try {
  $entry = $archive.CreateEntry('gpt-model-switcher/SOURCE_SNAPSHOT.json')
  $writer = [IO.StreamWriter]::new($entry.Open(), [Text.UTF8Encoding]::new($false))
  try { $writer.Write(($record | ConvertTo-Json -Depth 5)) } finally { $writer.Dispose() }
} finally { $archive.Dispose() }

$deliverables = Get-ChildItem -LiteralPath $outputPath -File | Where-Object { $_.Name -like "gpt-model-switcher-${version}-*" -or $_.Name -in @('사용-검증-안내.md','검증-실행결과.json') }
$hashes = foreach ($file in $deliverables) { '{0}  {1}' -f (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLowerInvariant(),$file.Name }
$hashes | Set-Content -LiteralPath (Join-Path $outputPath 'SHA256SUMS.txt') -Encoding utf8
Write-Output "Local test artifacts created in $outputPath"
