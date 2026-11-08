# Generates switche/switche.ico: three rows of [short bar][long bar] on a
# transparent background, where the short bar stands in for the exe column and the long
# bar for the title. Every standard shell size is rendered at its exact pixel dimensions
# (no downscaling), which is what keeps the tray/taskbar renditions crisp at any DPI.
#
# Run: powershell -NoProfile -ExecutionPolicy Bypass -File tools\make-icon.ps1

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

$root    = Split-Path $PSScriptRoot -Parent
$icoPath = Join-Path $root 'switche\switche.ico'

# shell/tray sizes windows picks from, depending on DPI and context
$sizes = 16, 20, 24, 32, 40, 48, 64, 96, 128, 256

$colBar    = [System.Drawing.ColorTranslator]::FromHtml('#FFD700')

function New-Frame([int]$S) {
    $bmp = New-Object System.Drawing.Bitmap($S, $S, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g   = [System.Drawing.Graphics]::FromImage($bmp)
    $g.Clear([System.Drawing.Color]::Transparent)
    $g.SmoothingMode   = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::Half

    # rows of [short bar][gap][long bar] on a transparent background; the bars span most
    # .. of the width, the rows fill the height, and the remaining space goes into the
    # .. gaps .. a small outer margin keeps strokes off the canvas edge so shell
    # .. cropping/rescaling can't clip the rounded ends into hard thresholds
    $rows  = 3
    $m     = [Math]::Max(1, [Math]::Round($S * 0.045))
    $gap   = [Math]::Max(2, [Math]::Round($S * 0.08))
    $short = [Math]::Max(3, [Math]::Round($S * 0.27))
    $long  = ($S - 2 * $m) - $short - $gap
    $barH  = [Math]::Max(2, [Math]::Round($S * 0.19))
    $rem   = ($S - 2 * $m) - $rows * $barH
    $gap1  = [Math]::Floor($rem / 2)
    $gap2  = $rem - $gap1
    # note: the comma operator binds tighter than arithmetic in powershell, so each
    # .. expression here has to be parenthesized
    $ys    = @($m, ($m + $barH + $gap1), ($m + 2 * $barH + $gap1 + $gap2))

    # the bars get fully rounded stadium ends from 32px up, but stay square at the
    # .. smallest sizes where a 2-3px bar has no straight section and the AA caps look soft
    $round = $S -ge 32

    $brush = New-Object System.Drawing.SolidBrush($colBar)
    foreach ($y in $ys) {
        foreach ($bar in @(@($m, [int]$short), @([int]($m + $short + $gap), [int]$long))) {
            $x = $bar[0]; $w = $bar[1]
            if ($round) {
                $bp = New-Object System.Drawing.Drawing2D.GraphicsPath
                $bp.AddArc($x, $y, $barH, $barH, 90, 180)
                $bp.AddArc($x + $w - $barH, $y, $barH, $barH, 270, 180)
                $bp.CloseFigure()
                $g.FillPath($brush, $bp)
                $bp.Dispose()
            } else {
                $g.FillRectangle($brush, [int]$x, [int]$y, [int]$w, [int]$barH)
            }
        }
    }

    $g.Dispose()
    return $bmp
}

function Get-PngBytes([System.Drawing.Bitmap]$bmp) {
    $ms = New-Object System.IO.MemoryStream
    $bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
    $bytes = $ms.ToArray()
    $ms.Dispose()
    return , $bytes
}

# 32bpp bottom-up BMP (DIB) frame with a legacy AND mask derived from the alpha channel
function Get-DibBytes([System.Drawing.Bitmap]$bmp) {
    $w = $bmp.Width
    $h = $bmp.Height
    $rect = New-Object System.Drawing.Rectangle(0, 0, $w, $h)
    $data = $bmp.LockBits($rect, [System.Drawing.Imaging.ImageLockMode]::ReadOnly, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $stride = $data.Stride
    $raw = New-Object byte[] ($stride * $h)
    [System.Runtime.InteropServices.Marshal]::Copy($data.Scan0, $raw, 0, $raw.Length)
    $bmp.UnlockBits($data)

    $maskRow  = [int]([Math]::Ceiling($w / 32.0) * 4)
    $maskSize = $maskRow * $h
    $xorSize  = $stride * $h

    $ms = New-Object System.IO.MemoryStream
    $bw = New-Object System.IO.BinaryWriter($ms)
    $bw.Write([uint32]40)
    $bw.Write([int32]$w)
    $bw.Write([int32]($h * 2))
    $bw.Write([uint16]1)
    $bw.Write([uint16]32)
    $bw.Write([uint32]0)
    $bw.Write([uint32]($xorSize + $maskSize))
    $bw.Write([int32]0); $bw.Write([int32]0)
    $bw.Write([uint32]0); $bw.Write([uint32]0)

    # XOR bitmap, bottom-up
    for ($y = $h - 1; $y -ge 0; $y--) {
        $bw.Write($raw, $y * $stride, $stride)
    }

    # AND mask, bottom-up: 1 bits mark fully transparent pixels
    $mask = New-Object byte[] $maskSize
    for ($j = 0; $j -lt $h; $j++) {
        $y = $h - 1 - $j
        for ($x = 0; $x -lt $w; $x++) {
            $a = $raw[$y * $stride + $x * 4 + 3]
            if ($a -lt 128) {
                $idx = $j * $maskRow + ($x -shr 3)
                $mask[$idx] = $mask[$idx] -bor (0x80 -shr ($x % 8))
            }
        }
    }
    $bw.Write($mask, 0, $maskSize)
    $bw.Flush()

    $bytes = $ms.ToArray()
    $bw.Dispose()
    $ms.Dispose()
    return , $bytes
}

$frames = @()
foreach ($s in $sizes) {
    $bmp = New-Frame $s
    # PNG compresses flat shapes well at larger sizes and is supported by Vista+;
    # .. keep the small shell sizes as BMP for maximum compatibility
    $bytes = if ($s -ge 64) { Get-PngBytes $bmp } else { Get-DibBytes $bmp }
    $frames += , @{ size = $s; bytes = $bytes }
    $bmp.Dispose()
}

$ms = New-Object System.IO.MemoryStream
$bw = New-Object System.IO.BinaryWriter($ms)
$bw.Write([uint16]0)                 # reserved
$bw.Write([uint16]1)                 # type: icon
$bw.Write([uint16]$frames.Count)

$offset = 6 + 16 * $frames.Count
foreach ($f in $frames) {
    $s = $f.size
    $dim = 0
    if ($s -lt 256) { $dim = $s }
    $bw.Write([byte]$dim)
    $bw.Write([byte]$dim)
    $bw.Write([byte]0)               # palette count
    $bw.Write([byte]0)               # reserved
    $bw.Write([uint16]1)             # planes
    $bw.Write([uint16]32)            # bits per pixel
    $bw.Write([uint32]$f.bytes.Length)
    $bw.Write([uint32]$offset)
    $offset += $f.bytes.Length
}
foreach ($f in $frames) {
    $bw.Write($f.bytes, 0, $f.bytes.Length)
}
$bw.Flush()
[System.IO.File]::WriteAllBytes($icoPath, $ms.ToArray())
$bw.Dispose()
$ms.Dispose()

Write-Host ("Wrote {0} ({1} frames, {2:n1} KB)" -f $icoPath, $frames.Count, ((Get-Item $icoPath).Length / 1KB))
