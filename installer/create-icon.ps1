# Generates a simple placeholder .ico and installer banner .bmp
# Run this locally if you want to replace with a real logo.
# The GitHub Actions workflow runs this automatically before Inno Setup.

Add-Type -AssemblyName System.Drawing

# ── 256x256 icon ───────────────────────────────────────────────────────────
$sizes = @(256, 128, 64, 32, 16)
$iconPath = "$PSScriptRoot\hsip.ico"

# For a real icon, replace hsip.ico with your actual file.
# This script creates a simple purple shield placeholder.
function Make-Bitmap($size) {
    $bmp = New-Object System.Drawing.Bitmap($size, $size)
    $g   = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = 'AntiAlias'
    $bg = [System.Drawing.Color]::FromArgb(99, 102, 241)   # indigo
    $g.Clear($bg)

    $pen  = New-Object System.Drawing.Pen([System.Drawing.Color]::White, [Math]::Max(1, $size/32))
    $font = New-Object System.Drawing.Font("Segoe UI", [Math]::Max(8, $size * 0.35), [System.Drawing.FontStyle]::Bold)
    $brush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::White)
    $sf   = New-Object System.Drawing.StringFormat
    $sf.Alignment = 'Center'
    $sf.LineAlignment = 'Center'
    $rect = New-Object System.Drawing.RectangleF(0, 0, $size, $size)
    $g.DrawString("H", $font, $brush, $rect, $sf)
    $g.Dispose()
    return $bmp
}

# Build .ico from multiple sizes using raw ICO format
$ms = New-Object System.IO.MemoryStream
$bw = New-Object System.IO.BinaryWriter($ms)

$bitmaps = $sizes | ForEach-Object { Make-Bitmap $_ }
$bitmapData = $bitmaps | ForEach-Object {
    $s = New-Object System.IO.MemoryStream
    $_.Save($s, [System.Drawing.Imaging.ImageFormat]::Png)
    $s.ToArray()
}

# ICO header
$bw.Write([UInt16]0)       # reserved
$bw.Write([UInt16]1)       # ICO type
$bw.Write([UInt16]$sizes.Count)

$offset = 6 + $sizes.Count * 16
for ($i = 0; $i -lt $sizes.Count; $i++) {
    $s = if ($sizes[$i] -ge 256) { 0 } else { $sizes[$i] }
    $bw.Write([byte]$s)    # width
    $bw.Write([byte]$s)    # height
    $bw.Write([byte]0)     # color count
    $bw.Write([byte]0)     # reserved
    $bw.Write([UInt16]1)   # color planes
    $bw.Write([UInt16]32)  # bits per pixel
    $bw.Write([UInt32]$bitmapData[$i].Length)
    $bw.Write([UInt32]$offset)
    $offset += $bitmapData[$i].Length
}
foreach ($d in $bitmapData) { $bw.Write($d) }
[System.IO.File]::WriteAllBytes($iconPath, $ms.ToArray())

# ── 164x314 installer banner .bmp ──────────────────────────────────────────
$banner = New-Object System.Drawing.Bitmap(164, 314)
$g = [System.Drawing.Graphics]::FromImage($banner)
$g.Clear([System.Drawing.Color]::FromArgb(15, 17, 23))
$font2 = New-Object System.Drawing.Font("Segoe UI", 18, [System.Drawing.FontStyle]::Bold)
$brush2 = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(99, 102, 241))
$g.DrawString("HSIP", $font2, $brush2, 20, 20)
$font3 = New-Object System.Drawing.Font("Segoe UI", 8)
$brush3 = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(160, 174, 192))
$g.DrawString("Your personal`ndata security hub", $font3, $brush3, 20, 55)
$g.Dispose()
$banner.Save("$PSScriptRoot\hsip-installer-banner.bmp", [System.Drawing.Imaging.ImageFormat]::Bmp)

Write-Host "Icon and banner created."
