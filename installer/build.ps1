$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$cargoToml = Join-Path $root "Cargo.toml"
$versionMatch = Select-String -LiteralPath $cargoToml -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1
if (-not $versionMatch) {
  throw "Could not read workspace version from $cargoToml"
}
$version = $versionMatch.Matches[0].Groups[1].Value

$icon = Join-Path $PSScriptRoot "wireline.ico"
$exeSource = Join-Path $root "target\release\skypeline-client.exe"
$dist = Join-Path $root "dist"
$portable = Join-Path $dist "portable\Wireline"
$zip = Join-Path $dist "Wireline-$version-Portable.zip"
$setup = Join-Path $dist "Wireline-$version-Setup.exe"
$setupLatest = Join-Path $dist "Wireline-Setup.exe"
$webSetup = Join-Path $dist "Wireline-WebSetup.exe"

function New-WirelineIcon {
  param([string]$Path)

  Add-Type -AssemblyName System.Drawing
  Add-Type -Namespace Wireline -Name NativeMethods -MemberDefinition @'
[DllImport("user32.dll")]
public static extern bool DestroyIcon(IntPtr hIcon);
'@

  $bitmap = New-Object System.Drawing.Bitmap 256, 256
  $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
  $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
  $graphics.Clear([System.Drawing.Color]::Transparent)

  $radius = 56
  $graphicsPath = New-Object System.Drawing.Drawing2D.GraphicsPath
  $graphicsPath.AddArc(0, 0, $radius, $radius, 180, 90)
  $graphicsPath.AddArc((256 - $radius), 0, $radius, $radius, 270, 90)
  $graphicsPath.AddArc((256 - $radius), (256 - $radius), $radius, $radius, 0, 90)
  $graphicsPath.AddArc(0, (256 - $radius), $radius, $radius, 90, 90)
  $graphicsPath.CloseFigure()

  $rect = New-Object System.Drawing.Rectangle 0, 0, 256, 256
  $brush = New-Object System.Drawing.Drawing2D.LinearGradientBrush(
    $rect,
    [System.Drawing.Color]::FromArgb(255, 10, 20, 45),
    [System.Drawing.Color]::FromArgb(255, 38, 135, 220),
    45
  )
  $graphics.FillPath($brush, $graphicsPath)

  $pen = New-Object System.Drawing.Pen([System.Drawing.Color]::FromArgb(190, 255, 255, 255), 7)
  $graphics.DrawArc($pen, 28, 30, 200, 196, -35, 250)

  $font = New-Object System.Drawing.Font("Segoe UI Semibold", 118, [System.Drawing.FontStyle]::Bold, [System.Drawing.GraphicsUnit]::Pixel)
  $format = New-Object System.Drawing.StringFormat
  $format.Alignment = [System.Drawing.StringAlignment]::Center
  $format.LineAlignment = [System.Drawing.StringAlignment]::Center
  $textRect = New-Object System.Drawing.RectangleF 0, 8, 256, 240
  $graphics.DrawString("W", $font, [System.Drawing.Brushes]::White, $textRect, $format)

  $hIcon = $bitmap.GetHicon()
  $iconObject = [System.Drawing.Icon]::FromHandle($hIcon)
  $stream = [System.IO.File]::Create($Path)
  try {
    $iconObject.Save($stream)
    Write-Host "Generated icon: $Path"
  }
  finally {
    $stream.Dispose()
    $iconObject.Dispose()
    [Wireline.NativeMethods]::DestroyIcon($hIcon) | Out-Null
    $graphics.Dispose()
    $brush.Dispose()
    $pen.Dispose()
    $font.Dispose()
    $format.Dispose()
    $graphicsPath.Dispose()
    $bitmap.Dispose()
  }
}

if (-not (Test-Path -LiteralPath $icon)) {
  New-WirelineIcon -Path $icon
}

Write-Host "Building client release binary..."
Push-Location $root
try {
  cargo build --release -p skypeline-client
  if ($LASTEXITCODE -ne 0) {
    throw "cargo build failed with exit code $LASTEXITCODE"
  }
}
finally {
  Pop-Location
}

if (-not (Test-Path -LiteralPath $exeSource)) {
  throw "Expected client binary at $exeSource"
}

Write-Host "Assembling portable app..."
if (Test-Path -LiteralPath $portable) {
  Remove-Item -LiteralPath $portable -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $portable | Out-Null
Copy-Item -LiteralPath $exeSource -Destination (Join-Path $portable "Wireline.exe") -Force
Copy-Item -LiteralPath $icon -Destination (Join-Path $portable "wireline.ico") -Force

$readme = @"
Wireline $version

Run Wireline.exe to launch the desktop client.
"@
Set-Content -LiteralPath (Join-Path $portable "README.txt") -Value $readme -Encoding UTF8

if (Test-Path -LiteralPath $zip) {
  Remove-Item -LiteralPath $zip -Force
}
Compress-Archive -Path (Join-Path $portable "*") -DestinationPath $zip -CompressionLevel Optimal

Write-Host "Building installers..."
$makensis = (Get-Command makensis -ErrorAction SilentlyContinue).Source
if (-not $makensis) {
  foreach ($candidate in @(
    "C:\Program Files (x86)\NSIS\makensis.exe",
    "C:\Program Files\NSIS\makensis.exe"
  )) {
    if (Test-Path -LiteralPath $candidate) {
      $makensis = $candidate
      break
    }
  }
}
if (-not $makensis) {
  throw "makensis was not found. Install NSIS 3 or add it to PATH."
}

Push-Location $PSScriptRoot
try {
  & $makensis "/DAPP_VERSION=$version" "Wireline.nsi"
  if ($LASTEXITCODE -ne 0) {
    throw "makensis Wireline.nsi failed with exit code $LASTEXITCODE"
  }
  Copy-Item -LiteralPath $setup -Destination $setupLatest -Force

  & $makensis "/DAPP_VERSION=$version" "WirelineWebSetup.nsi"
  if ($LASTEXITCODE -ne 0) {
    throw "makensis WirelineWebSetup.nsi failed with exit code $LASTEXITCODE"
  }
}
finally {
  Pop-Location
}

Write-Host ""
Write-Host "Artifacts:"
foreach ($artifact in @((Join-Path $portable "Wireline.exe"), $zip, $setup, $setupLatest, $webSetup)) {
  $hash = (Get-FileHash -LiteralPath $artifact -Algorithm SHA256).Hash
  Write-Host "$artifact"
  Write-Host "  SHA256: $hash"
}
