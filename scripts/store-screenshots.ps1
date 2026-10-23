# Microsoft Store görsellerini üretir (Edge'in başsız modu, 2× çözünürlük):
#   ekran görüntüleri  store/screenshots/out/<dil>/NN-<slayt>.png (3840×2160)
#   logolar (-Art)     store/art/out/poster-art.png (1440×2160), box-art.png (2160×2160)
#
#   npm run dev                          (başka bir terminalde, sahte veriyle)
#   ./scripts/store-screenshots.ps1      (tüm slaytlar, TR ve EN)
#   ./scripts/store-screenshots.ps1 -Slides hero -Langs en
#   ./scripts/store-screenshots.ps1 -Art
# Slaytlar store/screenshots/index.html, logolar store/art/index.html içinde tanımlı.
param(
  [string[]]$Slides = @('hero', 'map', 'explain', 'safety', 'apps', 'theme'),
  [string[]]$Langs = @('tr', 'en'),
  [switch]$Art,
  [string]$Server = 'http://localhost:1420'
)

$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot

$edge = @(
  "${env:ProgramFiles(x86)}\Microsoft\Edge\Application\msedge.exe",
  "$env:ProgramFiles\Microsoft\Edge\Application\msedge.exe"
) | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $edge) { throw 'Microsoft Edge not found.' }

try { Invoke-WebRequest "$Server/store/screenshots/" -UseBasicParsing -TimeoutSec 5 | Out-Null }
catch { throw "Dev server is not running at $Server. Start it with: npm run dev" }

# Kullanıcının Edge profiline dokunmamak için geçici bir profil.
$tempProfile = Join-Path $env:TEMP "solidclean-shots-$PID"

function Capture([string]$url, [string]$file, [int]$width, [int]$height) {
  if (Test-Path $file) { Remove-Item $file }
  # Sanal zaman, sayfadaki bekleme döngülerini (uygulamanın tarama ve analiz adımları) gerçek beklemeden ilerletir.
  $edgeArgs = @(
    '--headless', '--disable-gpu', '--hide-scrollbars', '--no-first-run', '--mute-audio',
    "--user-data-dir=$tempProfile",
    "--window-size=$width,$height", '--force-device-scale-factor=2',
    '--virtual-time-budget=30000',
    "--screenshot=$file",
    $url
  )
  # Edge stderr'e zararsız günlükler yazar; Windows PowerShell bunları hata sayar.
  $ErrorActionPreference = 'Continue'
  & $edge @edgeArgs 2>&1 | Out-Null
  $ErrorActionPreference = 'Stop'
  if (-not (Test-Path $file)) { throw "Capture failed: $url" }
}

if ($Art) {
  $out = Join-Path $root 'store\art\out'
  New-Item -ItemType Directory -Force $out | Out-Null
  Capture "$Server/store/art/?format=poster" (Join-Path $out 'poster-art.png') 720 1080
  Capture "$Server/store/art/?format=box" (Join-Path $out 'box-art.png') 1080 1080
  Write-Host "  poster-art.png, box-art.png"
} else {
  $out = Join-Path $root 'store\screenshots\out'
  foreach ($lang in $Langs) {
    $dir = Join-Path $out $lang
    New-Item -ItemType Directory -Force $dir | Out-Null
    $n = 0
    foreach ($slide in $Slides) {
      $n++
      $file = Join-Path $dir ('{0:D2}-{1}.png' -f $n, $slide)
      Capture "$Server/store/screenshots/?slide=$slide&lang=$lang" $file 1920 1080
      Write-Host "  $lang/$(Split-Path $file -Leaf)"
    }
  }
}

Remove-Item -Recurse -Force $tempProfile -ErrorAction SilentlyContinue
Write-Host "Done: $out"
