# Microsoft Store için MSIX paketi üretir: src-tauri/target/msix/SolidClean_<sürüm>_x64.msix
#   npm run msix                 (uygulamayı derler ve paketler)
#   npm run msix -- -SkipBuild   (mevcut release exe'yi paketler)
# Gereksinim: Windows SDK (makeappx.exe, makepri.exe).
param([switch]$SkipBuild)

$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot
$tauri = Join-Path $root 'src-tauri'
$msixSrc = Join-Path $tauri 'msix'

$conf = [IO.File]::ReadAllText((Join-Path $tauri 'tauri.conf.json')) | ConvertFrom-Json
$identity = [IO.File]::ReadAllText((Join-Path $msixSrc 'identity.json')) | ConvertFrom-Json
# Store, sürümün dördüncü hanesinin 0 olmasını ister.
$version = "$($conf.version).0"

$kits = 'C:\Program Files (x86)\Windows Kits\10\bin'
$sdk = Get-ChildItem $kits -Directory -ErrorAction SilentlyContinue |
  Where-Object { Test-Path (Join-Path $_.FullName 'x64\makeappx.exe') } |
  Sort-Object { [version]$_.Name } | Select-Object -Last 1
if (-not $sdk) { throw 'Windows SDK not found (makeappx.exe). Install it with Visual Studio Installer.' }
$bin = Join-Path $sdk.FullName 'x64'

if (-not $SkipBuild) {
  Push-Location $root
  try {
    cmd /c "npx tauri build --no-bundle"
    if ($LASTEXITCODE -ne 0) { throw 'tauri build failed' }
  } finally { Pop-Location }
}
$exe = Join-Path $tauri 'target\release\solidclean.exe'
if (-not (Test-Path $exe)) { throw "Missing $exe (run without -SkipBuild)" }

$out = Join-Path $tauri 'target\msix'
$layout = Join-Path $out 'layout'
Remove-Item $layout -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory $layout -Force | Out-Null
Copy-Item $exe $layout
Copy-Item (Join-Path $msixSrc 'Assets') (Join-Path $layout 'Assets') -Recurse
Copy-Item (Join-Path $msixSrc 'Strings') (Join-Path $layout 'Strings') -Recurse

$manifest = [IO.File]::ReadAllText((Join-Path $msixSrc 'AppxManifest.xml')).
  Replace('{{Name}}', $identity.name).
  Replace('{{Publisher}}', [Security.SecurityElement]::Escape($identity.publisher)).
  Replace('{{PublisherDisplayName}}', [Security.SecurityElement]::Escape($identity.publisherDisplayName)).
  Replace('{{Version}}', $version)
$manifestPath = Join-Path $layout 'AppxManifest.xml'
[IO.File]::WriteAllText($manifestPath, $manifest, (New-Object Text.UTF8Encoding $false))

# resources.pri: Windows'un ölçek/targetsize ikon varyantlarını ve dile göre açıklamayı seçebilmesi için.
$priConfig = Join-Path $out 'priconfig.xml'
& (Join-Path $bin 'makepri.exe') createconfig /cf $priConfig /dq en-US /pv 10.0.0 /o | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'makepri createconfig failed' }
# Varsayılan ayar dil ve ölçek kaynaklarını bundle için ayrı .pri dosyalarına böler; tek bir
# .msix bunları okumaz. Hepsi tek resources.pri içinde kalsın diye bölme kuralını kaldır.
$config = New-Object Xml.XmlDocument
$config.Load($priConfig)
foreach ($node in @($config.SelectNodes('//packaging'))) { [void]$node.ParentNode.RemoveChild($node) }
$config.Save($priConfig)
& (Join-Path $bin 'makepri.exe') new /pr $layout /cf $priConfig /mn $manifestPath /of (Join-Path $layout 'resources.pri') /o | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'makepri new failed' }
# Metinler artık resources.pri içinde; .resw dosyalarının pakette olmasına gerek yok.
Remove-Item (Join-Path $layout 'Strings') -Recurse -Force

$msix = Join-Path $out "SolidClean_${version}_x64.msix"
& (Join-Path $bin 'makeappx.exe') pack /d $layout /p $msix /o | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'makeappx pack failed' }

"Package: $msix ($([math]::Round((Get-Item $msix).Length / 1MB, 1)) MB)"
"Identity: $($identity.name) / $($identity.publisher) / $version"
