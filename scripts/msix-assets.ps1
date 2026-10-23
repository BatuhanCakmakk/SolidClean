# MSIX paketi için ikon varlıklarını üretir: src-tauri/msix/Assets.
# İkon değiştiğinde çalıştırın: npm run msix:assets
$root = Split-Path $PSScriptRoot
$assets = Join-Path $root 'src-tauri\msix\Assets'
$work = Join-Path $env:TEMP 'solidclean-msix-assets'

Remove-Item $work -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory $work | Out-Null
Remove-Item $assets -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory $assets | Out-Null

$full = Join-Path $root 'app-icon.svg'
# 16-32 px: parçacıksız, ortalanmış sürüm.
$small = Join-Path $root 'design\icon-solidclean-small.svg'
# Başlat kutucuğu: aynı ikon, kenarlarda daha fazla boşlukla.
$tile = Join-Path $work 'tile.svg'
$text = [IO.File]::ReadAllText($full) -replace 'viewBox="[^"]+"', 'viewBox="-7 -95 1160 1160"'
[IO.File]::WriteAllText($tile, $text, (New-Object Text.UTF8Encoding $false))

function Render([string]$svg, [string]$name, [int[]]$sizes) {
  $out = Join-Path $work $name
  cmd /c "npx tauri icon `"$svg`" --png $($sizes -join ',') -o `"$out`" >nul 2>nul"
  if ($LASTEXITCODE -ne 0) { throw "tauri icon failed for $svg" }
  return $out
}

function Put([string]$dir, [int]$size, [string[]]$names) {
  foreach ($n in $names) { Copy-Item (Join-Path $dir "${size}x${size}.png") (Join-Path $assets $n) }
}

# Görev çubuğu, Başlat ve Explorer bu boyutları kullanır. "unplated" sürümler
# ikonun arkasına vurgu rengi plakası konmasını engeller.
function TargetSize([string]$dir, [int]$size) {
  Put $dir $size @(
    "Square44x44Logo.targetsize-$size.png",
    "Square44x44Logo.targetsize-${size}_altform-unplated.png",
    "Square44x44Logo.targetsize-${size}_altform-lightunplated.png"
  )
}

$s = Render $small 'small' @(16, 20, 24, 30, 32)
foreach ($size in 16, 20, 24, 30, 32) { TargetSize $s $size }

$f = Render $full 'full' @(36, 40, 44, 48, 50, 60, 64, 72, 80, 88, 96, 100, 176, 200, 256)
foreach ($size in 36, 40, 48, 60, 64, 72, 80, 96, 256) { TargetSize $f $size }
Put $f 44 @('Square44x44Logo.scale-100.png')
Put $f 88 @('Square44x44Logo.scale-200.png')
Put $f 176 @('Square44x44Logo.scale-400.png')
Put $f 50 @('StoreLogo.scale-100.png')
Put $f 100 @('StoreLogo.scale-200.png')
Put $f 200 @('StoreLogo.scale-400.png')

$t = Render $tile 'tile' @(150, 300, 600)
Put $t 150 @('Square150x150Logo.scale-100.png')
Put $t 300 @('Square150x150Logo.scale-200.png')
Put $t 600 @('Square150x150Logo.scale-400.png')

Remove-Item $work -Recurse -Force
"{0} assets -> {1}" -f (Get-ChildItem $assets).Count, $assets
