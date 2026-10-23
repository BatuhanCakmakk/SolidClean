# Microsoft Store ekran görüntülerini üretir: store/screenshots/out/<dil>/NN-<slayt>.png (3840×2160)
#   npm run dev                          (başka bir terminalde, sahte veriyle)
#   ./scripts/store-screenshots.ps1      (tüm slaytlar, TR ve EN)
#   ./scripts/store-screenshots.ps1 -Slides hero -Langs en
# Slaytlar store/screenshots/index.html içinde tanımlı; Edge'in başsız modu kullanılır.
param(
  [string[]]$Slides = @('hero', 'map', 'explain', 'safety', 'apps', 'theme'),
  [string[]]$Langs = @('tr', 'en'),
  [string]$Server = 'http://localhost:1420'
)

$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot
$out = Join-Path $root 'store\screenshots\out'

$edge = @(
  "${env:ProgramFiles(x86)}\Microsoft\Edge\Application\msedge.exe",
  "$env:ProgramFiles\Microsoft\Edge\Application\msedge.exe"
) | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $edge) { throw 'Microsoft Edge not found.' }

try { Invoke-WebRequest "$Server/store/screenshots/" -UseBasicParsing -TimeoutSec 5 | Out-Null }
catch { throw "Dev server is not running at $Server. Start it with: npm run dev" }

# Kullanıcının Edge profiline dokunmamak için geçici bir profil.
$tempProfile = Join-Path $env:TEMP "solidclean-shots-$PID"

foreach ($lang in $Langs) {
  $dir = Join-Path $out $lang
  New-Item -ItemType Directory -Force $dir | Out-Null
  $n = 0
  foreach ($slide in $Slides) {
    $n++
    $file = Join-Path $dir ('{0:D2}-{1}.png' -f $n, $slide)
    if (Test-Path $file) { Remove-Item $file }
    # Sanal zaman, sayfadaki bekleme döngülerini (uygulamanın tarama ve analiz adımları) gerçek beklemeden ilerletir.
    $edgeArgs = @(
      '--headless', '--disable-gpu', '--hide-scrollbars', '--no-first-run', '--mute-audio',
      "--user-data-dir=$tempProfile",
      '--window-size=1920,1080', '--force-device-scale-factor=2',
      '--virtual-time-budget=30000',
      "--screenshot=$file",
      "$Server/store/screenshots/?slide=$slide&lang=$lang"
    )
    # Edge stderr'e zararsız günlükler yazar; Windows PowerShell bunları hata sayar.
    $ErrorActionPreference = 'Continue'
    & $edge @edgeArgs 2>&1 | Out-Null
    $ErrorActionPreference = 'Stop'
    if (-not (Test-Path $file)) { throw "Screenshot failed: $lang/$slide" }
    Write-Host "  $lang/$(Split-Path $file -Leaf)"
  }
}

Remove-Item -Recurse -Force $tempProfile -ErrorAction SilentlyContinue
Write-Host "Done: $out"
