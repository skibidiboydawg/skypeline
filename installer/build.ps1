$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$cargoToml = Join-Path $root "Cargo.toml"
$versionMatch = Select-String -LiteralPath $cargoToml -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1
if (-not $versionMatch) {
  throw "Could not read workspace version from $cargoToml"
}
$version = $versionMatch.Matches[0].Groups[1].Value

$icon = Join-Path $PSScriptRoot "skypeline.ico"
$nsi = Join-Path $PSScriptRoot "Skypeline.nsi"
$exeSource = Join-Path $root "target\release\skypeline-client.exe"
$dist = Join-Path $root "dist"
$portable = Join-Path $dist "portable\Skypeline"
$zip = Join-Path $dist "Skypeline-$version-Portable.zip"
$setup = Join-Path $dist "Skypeline-$version-Setup.exe"

function New-SkypelineIcon {
  param([string]$Path)

  Add-Type -AssemblyName System.Drawing
  Add-Type -Namespace Skypeline -Name NativeMethods -MemberDefinition @'
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

  $font = New-Object System.Drawing.Font("Segoe UI Semibold", 132, [System.Drawing.FontStyle]::Bold, [System.Drawing.GraphicsUnit]::Pixel)
  $format = New-Object System.Drawing.StringFormat
  $format.Alignment = [System.Drawing.StringAlignment]::Center
  $format.LineAlignment = [System.Drawing.StringAlignment]::Center
  $textRect = New-Object System.Drawing.RectangleF 0, 8, 256, 240
  $graphics.DrawString("S", $font, [System.Drawing.Brushes]::White, $textRect, $format)

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
    [Skypeline.NativeMethods]::DestroyIcon($hIcon) | Out-Null
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
  New-SkypelineIcon -Path $icon
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
Copy-Item -LiteralPath $exeSource -Destination (Join-Path $portable "Skypeline.exe") -Force
Copy-Item -LiteralPath $icon -Destination (Join-Path $portable "skypeline.ico") -Force

$readme = @"
Skypeline $version

Run Skypeline.exe to launch the desktop client.

Hosted service: https://skypeline-co.onrender.com

Sign up with email and password, then enter the confirmation code Neon Auth
emails you. Verification is required before signing in.

Preferences are stored in config.json next to this executable.
No password or token is stored in config.json.
"@
Set-Content -LiteralPath (Join-Path $portable "README.txt") -Value $readme -Encoding UTF8

if (Test-Path -LiteralPath $zip) {
  Remove-Item -LiteralPath $zip -Force
}
Compress-Archive -Path (Join-Path $portable "*") -DestinationPath $zip -CompressionLevel Optimal

Write-Host "Building installer..."
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
  & $makensis "/DAPP_VERSION=$version" "Skypeline.nsi"
  if ($LASTEXITCODE -ne 0) {
    throw "makensis failed with exit code $LASTEXITCODE"
  }
}
finally {
  Pop-Location
}

Write-Host ""
Write-Host "Artifacts:"
foreach ($artifact in @((Join-Path $portable "Skypeline.exe"), $zip, $setup)) {
  $hash = (Get-FileHash -LiteralPath $artifact -Algorithm SHA256).Hash
  Write-Host "$artifact"
  Write-Host "  SHA256: $hash"
}
