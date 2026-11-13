# -*- coding: utf-8 -*-
# 智慧控管與監控系統 - 地籍異動即時通簡訊控管 (moi-sms)
# 方案 A【碧藍雙訊盾徽】專屬圖示產出腳本 (generate_icons.ps1)
# 視覺意象：深海藍漸層圓角底 + 純白地政產權房屋輪廓 + 亮青即時訊號波紋 + 右下雙通道信封徽章

Add-Type -AssemblyName System.Drawing

function Generate-MoiSmsMasterIcon {
    param (
        [int]$Size = 512,
        [string]$OutputPath = "icon.png"
    )

    $bmp = [System.Drawing.Bitmap]::new($Size, $Size, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
    $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $g.Clear([System.Drawing.Color]::Transparent)

    $s = [float]$Size / 512.0

    function Get-RoundedRectPath([float]$x, [float]$y, [float]$width, [float]$height, [float]$radius) {
        $path = [System.Drawing.Drawing2D.GraphicsPath]::new()
        $diameter = $radius * 2.0
        $arc = [System.Drawing.RectangleF]::new($x, $y, $diameter, $diameter)
        $path.AddArc($arc, [float]180.0, [float]90.0)
        $arc.X = $x + $width - $diameter
        $path.AddArc($arc, [float]270.0, [float]90.0)
        $arc.Y = $y + $height - $diameter
        $path.AddArc($arc, [float]0.0, [float]90.0)
        $arc.X = $x
        $path.AddArc($arc, [float]90.0, [float]90.0)
        $path.CloseFigure()
        return $path
    }

    # 1. 基底外框：深海藍漸層圓角徽章
    $sqX = [float](24.0 * $s)
    $sqY = [float](24.0 * $s)
    $sqW = [float](464.0 * $s)
    $sqH = [float](464.0 * $s)
    $sqR = [float](108.0 * $s)

    $squirclePath = Get-RoundedRectPath $sqX $sqY $sqW $sqH $sqR
    $sqRect = [System.Drawing.RectangleF]::new($sqX, $sqY, $sqW, $sqH)

    $bgBrush = [System.Drawing.Drawing2D.LinearGradientBrush]::new(
        $sqRect,
        [System.Drawing.Color]::FromArgb(255, 20, 50, 82),
        [System.Drawing.Color]::FromArgb(255, 32, 78, 129),
        [float]135.0
    )
    $g.FillPath($bgBrush, $squirclePath)
    $bgBrush.Dispose()

    # 雙層亮青邊框
    $outerPen = [System.Drawing.Pen]::new([System.Drawing.Color]::FromArgb(190, 56, 189, 248), [float](6.0 * $s))
    $g.DrawPath($outerPen, $squirclePath)
    $outerPen.Dispose()

    $innerSqPath = Get-RoundedRectPath ($sqX + 9.0 * $s) ($sqY + 9.0 * $s) ($sqW - 18.0 * $s) ($sqH - 18.0 * $s) ($sqR - 7.0 * $s)
    $innerPen = [System.Drawing.Pen]::new([System.Drawing.Color]::FromArgb(75, 125, 211, 252), [float](2.0 * $s))
    $g.DrawPath($innerPen, $innerSqPath)
    $innerPen.Dispose()
    $innerSqPath.Dispose()
    $squirclePath.Dispose()

    # 2. 中央純白幾何地政產權房屋輪廓
    $housePen = [System.Drawing.Pen]::new([System.Drawing.Color]::FromArgb(245, 255, 255, 255), [float](20.0 * $s))
    $housePen.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
    $housePen.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
    $housePen.LineJoin = [System.Drawing.Drawing2D.LineJoin]::Round

    # 屋頂折線
    [System.Drawing.PointF[]]$roofPoints = @(
        [System.Drawing.PointF]::new([float](102.0 * $s), [float](240.0 * $s)),
        [System.Drawing.PointF]::new([float](234.0 * $s), [float](124.0 * $s)),
        [System.Drawing.PointF]::new([float](366.0 * $s), [float](240.0 * $s))
    )
    $g.DrawLines($housePen, $roofPoints)

    # 屋身左側、底線與右側（右側留白給右下信封徽章）
    [System.Drawing.PointF[]]$bodyPoints = @(
        [System.Drawing.PointF]::new([float](138.0 * $s), [float](216.0 * $s)),
        [System.Drawing.PointF]::new([float](138.0 * $s), [float](368.0 * $s)),
        [System.Drawing.PointF]::new([float](260.0 * $s), [float](368.0 * $s))
    )
    $g.DrawLines($housePen, $bodyPoints)
    $g.DrawLine($housePen, [float](330.0 * $s), [float](216.0 * $s), [float](330.0 * $s), [float](262.0 * $s))
    $housePen.Dispose()

    # 屋內產權核心窗格
    $winBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(230, 186, 230, 253))
    $winPath = Get-RoundedRectPath ([float](206.0 * $s)) ([float](242.0 * $s)) ([float](56.0 * $s)) ([float](56.0 * $s)) ([float](10.0 * $s))
    $g.FillPath($winBrush, $winPath)
    $winPath.Dispose()
    $winBrush.Dispose()

    # 3. 右上方即時訊號波紋 (2 道亮青弧線)
    $wavePen1 = [System.Drawing.Pen]::new([System.Drawing.Color]::FromArgb(235, 56, 189, 248), [float](14.0 * $s))
    $wavePen1.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
    $wavePen1.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
    $g.DrawArc($wavePen1, [float](278.0 * $s), [float](108.0 * $s), [float](90.0 * $s), [float](90.0 * $s), [float](-82.0), [float](76.0))
    $wavePen1.Dispose()

    $wavePen2 = [System.Drawing.Pen]::new([System.Drawing.Color]::FromArgb(180, 125, 211, 252), [float](12.0 * $s))
    $wavePen2.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
    $wavePen2.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
    $g.DrawArc($wavePen2, [float](264.0 * $s), [float](74.0 * $s), [float](142.0 * $s), [float](142.0 * $s), [float](-82.0), [float](76.0))
    $wavePen2.Dispose()

    # 4. 右下角雙通道訊息信封圓角徽章
    $envX = [float](262.0 * $s)
    $envY = [float](272.0 * $s)
    $envW = [float](168.0 * $s)
    $envH = [float](124.0 * $s)
    $envPath = Get-RoundedRectPath $envX $envY $envW $envH ([float](24.0 * $s))
    $envRect = [System.Drawing.RectangleF]::new($envX, $envY, $envW, $envH)

    $envBgBrush = [System.Drawing.Drawing2D.LinearGradientBrush]::new(
        $envRect,
        [System.Drawing.Color]::FromArgb(255, 14, 165, 233),
        [System.Drawing.Color]::FromArgb(255, 2, 132, 199),
        [float]135.0
    )
    $g.FillPath($envBgBrush, $envPath)
    $envBgBrush.Dispose()

    $envBorderPen = [System.Drawing.Pen]::new([System.Drawing.Color]::FromArgb(245, 255, 255, 255), [float](8.0 * $s))
    $g.DrawPath($envBorderPen, $envPath)
    $envBorderPen.Dispose()
    $envPath.Dispose()

    # 信封 V 形折線
    $flapPen = [System.Drawing.Pen]::new([System.Drawing.Color]::FromArgb(250, 255, 255, 255), [float](10.0 * $s))
    $flapPen.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
    $flapPen.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
    $flapPen.LineJoin = [System.Drawing.Drawing2D.LineJoin]::Round
    [System.Drawing.PointF[]]$flapPoints = @(
        [System.Drawing.PointF]::new([float](282.0 * $s), [float](298.0 * $s)),
        [System.Drawing.PointF]::new([float](346.0 * $s), [float](346.0 * $s)),
        [System.Drawing.PointF]::new([float](410.0 * $s), [float](298.0 * $s))
    )
    $g.DrawLines($flapPen, $flapPoints)
    $flapPen.Dispose()

    $g.Dispose()
    $bmp.Save($OutputPath, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
    Write-Host "Generated master icon: $OutputPath ($Size x $Size)"
}

function Save-IconBundle {
    param (
        [string]$TargetDir = (Join-Path $PSScriptRoot "src-tauri\icons")
    )

    if (-not (Test-Path $TargetDir)) {
        New-Item -ItemType Directory -Force -Path $TargetDir | Out-Null
    }

    $master512Path = Join-Path $TargetDir "icon.png"
    Generate-MoiSmsMasterIcon -Size 512 -OutputPath $master512Path

    $masterBmp = [System.Drawing.Bitmap]::FromFile((Resolve-Path $master512Path))

    function Resize-Image([int]$size, [string]$outPath) {
        $destBmp = [System.Drawing.Bitmap]::new($size, $size, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
        $g = [System.Drawing.Graphics]::FromImage($destBmp)
        $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
        $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
        $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
        $g.Clear([System.Drawing.Color]::Transparent)
        $g.DrawImage($masterBmp, 0, 0, $size, $size)
        $g.Dispose()
        $destBmp.Save($outPath, [System.Drawing.Imaging.ImageFormat]::Png)
        $destBmp.Dispose()
        Write-Host "Created: $outPath ($size x $size)"
    }

    Resize-Image 256 (Join-Path $TargetDir "128x128@2x.png")
    Resize-Image 128 (Join-Path $TargetDir "128x128.png")
    Resize-Image 32  (Join-Path $TargetDir "32x32.png")

    $uiFaviconPath = Join-Path $PSScriptRoot "ui\favicon.png"
    $uiDir = Split-Path $uiFaviconPath
    if (-not (Test-Path $uiDir)) { New-Item -ItemType Directory -Force -Path $uiDir | Out-Null }
    Resize-Image 32 $uiFaviconPath

    $icoSizes = @(256, 128, 64, 48, 32, 16)
    $icoPngBytes = @()

    foreach ($s in $icoSizes) {
        $tempBmp = [System.Drawing.Bitmap]::new($s, $s, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
        $g = [System.Drawing.Graphics]::FromImage($tempBmp)
        $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
        $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
        $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
        $g.Clear([System.Drawing.Color]::Transparent)
        $g.DrawImage($masterBmp, 0, 0, $s, $s)
        $g.Dispose()

        $ms = [System.IO.MemoryStream]::new()
        $tempBmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
        $bytes = $ms.ToArray()
        $ms.Dispose()
        $tempBmp.Dispose()

        $icoPngBytes += ,@($s, $bytes)
    }

    $masterBmp.Dispose()

    $icoPath = Join-Path $TargetDir "icon.ico"
    $fs = [System.IO.File]::Create((Resolve-Path -Path $TargetDir | Join-Path -ChildPath "icon.ico"))
    $bw = [System.IO.BinaryWriter]::new($fs)

    $bw.Write([UInt16]0)
    $bw.Write([UInt16]1)
    $bw.Write([UInt16]$icoPngBytes.Count)

    $offset = 6 + ($icoPngBytes.Count * 16)
    foreach ($entry in $icoPngBytes) {
        $s = $entry[0]
        $bytes = $entry[1]
        $wByte = if ($s -eq 256) { [byte]0 } else { [byte]$s }
        $hByte = if ($s -eq 256) { [byte]0 } else { [byte]$s }

        $bw.Write([byte]$wByte)
        $bw.Write([byte]$hByte)
        $bw.Write([byte]0)
        $bw.Write([byte]0)
        $bw.Write([UInt16]1)
        $bw.Write([UInt16]32)
        $bw.Write([UInt32]$bytes.Length)
        $bw.Write([UInt32]$offset)

        $offset += $bytes.Length
    }

    foreach ($entry in $icoPngBytes) {
        $bw.Write($entry[1])
    }

    $bw.Flush()
    $bw.Dispose()
    $fs.Dispose()

    Write-Host "Created multi-resolution ICO: $icoPath with $($icoPngBytes.Count) frames (256, 128, 64, 48, 32, 16)"
}

Save-IconBundle
