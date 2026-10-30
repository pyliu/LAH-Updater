# -*- coding: utf-8 -*-
# 智慧控管與監控系統 - 服務監控儀表板 (dashboard)
# 方案 A【深邃脈衝儀表板】專屬圖示產出腳本 (generate_icons.ps1)
# 視覺意象：科技深青色漸層底 + 同心微光弧形指針儀表 + 翠綠至青藍即時服務心跳脈衝折線 + 正常狀態指示光暈

Add-Type -AssemblyName System.Drawing

function Generate-DashboardMasterIcon {
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

    # 1. 基底外框：科技深青漸層圓角徽章 (Squircle)
    $sqX = [float](24.0 * $s)
    $sqY = [float](24.0 * $s)
    $sqW = [float](464.0 * $s)
    $sqH = [float](464.0 * $s)
    $sqR = [float](108.0 * $s)

    $squirclePath = Get-RoundedRectPath $sqX $sqY $sqW $sqH $sqR
    $sqRect = [System.Drawing.RectangleF]::new($sqX, $sqY, $sqW, $sqH)

    # 漸層背景：深邃夜青 (#0a1c2e) 至 科技深藍海青 (#0f3d59)
    $bgBrush = [System.Drawing.Drawing2D.LinearGradientBrush]::new(
        $sqRect,
        [System.Drawing.Color]::FromArgb(255, 10, 28, 46),
        [System.Drawing.Color]::FromArgb(255, 15, 61, 89),
        [float]135.0
    )
    $g.FillPath($bgBrush, $squirclePath)
    $bgBrush.Dispose()

    # 外邊框雙層光暈：青藍光邊
    $outerBorderPen = [System.Drawing.Pen]::new(
        [System.Drawing.Color]::FromArgb(180, 0, 210, 211),
        [float](6.0 * $s)
    )
    $g.DrawPath($outerBorderPen, $squirclePath)
    $outerBorderPen.Dispose()

    $innerSqPath = Get-RoundedRectPath ($sqX + 8.0 * $s) ($sqY + 8.0 * $s) ($sqW - 16.0 * $s) ($sqH - 16.0 * $s) ($sqR - 6.0 * $s)
    $innerBorderPen = [System.Drawing.Pen]::new(
        [System.Drawing.Color]::FromArgb(80, 72, 219, 251),
        [float](2.0 * $s)
    )
    $g.DrawPath($innerBorderPen, $innerSqPath)
    $innerBorderPen.Dispose()
    $innerSqPath.Dispose()

    # 2. 儀表板同心圓弧刻度 (Concentric Gauge Arcs)
    $centerX = [float](256.0 * $s)
    $centerY = [float](230.0 * $s)

    # 外圈刻度底弧 (從 140 度至 400 度，共 260 度)
    $gaugeRadius = [float](140.0 * $s)
    $gaugeRect = [System.Drawing.RectangleF]::new($centerX - $gaugeRadius, $centerY - $gaugeRadius, $gaugeRadius * 2.0, $gaugeRadius * 2.0)

    $gaugeBasePen = [System.Drawing.Pen]::new(
        [System.Drawing.Color]::FromArgb(60, 0, 210, 211),
        [float](14.0 * $s)
    )
    $g.DrawArc($gaugeBasePen, $gaugeRect, [float]140.0, [float]260.0)
    $gaugeBasePen.Dispose()

    # 儀表發光有效進度弧 (140 度旋轉 190 度，顯示約 73% 正常運轉率)
    $gaugeActivePen = [System.Drawing.Pen]::new(
        [System.Drawing.Color]::FromArgb(235, 0, 210, 211),
        [float](14.0 * $s)
    )
    $gaugeActivePen.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
    $gaugeActivePen.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
    $g.DrawArc($gaugeActivePen, $gaugeRect, [float]140.0, [float]190.0)
    $gaugeActivePen.Dispose()

    # 內圈微光輔助弧線
    $innerGaugeRadius = [float](115.0 * $s)
    $innerGaugeRect = [System.Drawing.RectangleF]::new($centerX - $innerGaugeRadius, $centerY - $innerGaugeRadius, $innerGaugeRadius * 2.0, $innerGaugeRadius * 2.0)
    $innerGaugePen = [System.Drawing.Pen]::new(
        [System.Drawing.Color]::FromArgb(50, 255, 255, 255),
        [float](3.0 * $s)
    )
    $innerGaugePen.DashStyle = [System.Drawing.Drawing2D.DashStyle]::Dash
    $g.DrawArc($innerGaugePen, $innerGaugeRect, [float]145.0, [float]250.0)
    $innerGaugePen.Dispose()

    # 3. 服務心跳脈衝折線 (ECG / Pulse Wave)
    $p0 = [System.Drawing.PointF]::new([float](70.0 * $s),  [float](290.0 * $s))
    $p1 = [System.Drawing.PointF]::new([float](145.0 * $s), [float](290.0 * $s))
    $p2 = [System.Drawing.PointF]::new([float](170.0 * $s), [float](275.0 * $s))
    $p3 = [System.Drawing.PointF]::new([float](190.0 * $s), [float](320.0 * $s))
    $p4 = [System.Drawing.PointF]::new([float](225.0 * $s), [float](155.0 * $s)) # 主尖峰
    $p5 = [System.Drawing.PointF]::new([float](255.0 * $s), [float](365.0 * $s)) # 回彈點
    $p6 = [System.Drawing.PointF]::new([float](280.0 * $s), [float](270.0 * $s))
    $p7 = [System.Drawing.PointF]::new([float](305.0 * $s), [float](305.0 * $s))
    $p8 = [System.Drawing.PointF]::new([float](325.0 * $s), [float](290.0 * $s))
    $p9 = [System.Drawing.PointF]::new([float](442.0 * $s), [float](290.0 * $s))

    $pts = ($p0, $p1, $p2, $p3, $p4, $p5, $p6, $p7, $p8, $p9)

    $pulsePath = [System.Drawing.Drawing2D.GraphicsPath]::new()
    $pulsePath.AddLines([System.Drawing.PointF[]]$pts)

    # 脈衝外層綠光暈
    $pulseGlowPen = [System.Drawing.Pen]::new(
        [System.Drawing.Color]::FromArgb(80, 46, 204, 113),
        [float](16.0 * $s)
    )
    $pulseGlowPen.LineJoin = [System.Drawing.Drawing2D.LineJoin]::Round
    $pulseGlowPen.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
    $pulseGlowPen.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
    $g.DrawPath($pulseGlowPen, $pulsePath)
    $pulseGlowPen.Dispose()

    # 脈衝主核心亮線 (翠綠 #2ecc71)
    $pulsePen = [System.Drawing.Pen]::new(
        [System.Drawing.Color]::FromArgb(255, 46, 204, 113),
        [float](7.5 * $s)
    )
    $pulsePen.LineJoin = [System.Drawing.Drawing2D.LineJoin]::Round
    $pulsePen.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
    $pulsePen.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
    $g.DrawPath($pulsePen, $pulsePath)
    $pulsePen.Dispose()

    # 脈衝頂點微光白芯高光
    $pulsePeakPen = [System.Drawing.Pen]::new(
        [System.Drawing.Color]::FromArgb(240, 230, 255, 240),
        [float](3.0 * $s)
    )
    $pulsePeakPen.LineJoin = [System.Drawing.Drawing2D.LineJoin]::Round
    $pulsePeakPen.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
    $pulsePeakPen.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
    $g.DrawPath($pulsePeakPen, $pulsePath)
    $pulsePeakPen.Dispose()
    $pulsePath.Dispose()

    # 4. 服務狀態在線指示燈 (Online Beacon)
    $beaconX = [float](375.0 * $s)
    $beaconY = [float](375.0 * $s)
    $beaconR = [float](32.0 * $s)

    # 外圈擴散光暈
    $beaconGlowBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(80, 46, 204, 113))
    $g.FillEllipse($beaconGlowBrush, $beaconX - $beaconR * 1.5, $beaconY - $beaconR * 1.5, $beaconR * 3.0, $beaconR * 3.0)
    $beaconGlowBrush.Dispose()

    # 內圈實心亮燈
    $beaconBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(255, 46, 204, 113))
    $g.FillEllipse($beaconBrush, $beaconX - $beaconR, $beaconY - $beaconR, $beaconR * 2.0, $beaconR * 2.0)
    $beaconBrush.Dispose()

    # 燈心白光高光點
    $beaconCoreBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(255, 230, 255, 230))
    $g.FillEllipse($beaconCoreBrush, $beaconX - $beaconR * 0.45, $beaconY - $beaconR * 0.45, $beaconR * 0.9, $beaconR * 0.9)
    $beaconCoreBrush.Dispose()

    # 5. 輸出儲存
    $outDir = [System.IO.Path]::GetDirectoryName($OutputPath)
    if (![string]::IsNullOrEmpty($outDir) -and -not (Test-Path $outDir)) {
        New-Item -ItemType Directory -Force -Path $outDir | Out-Null
    }

    $bmp.Save($OutputPath, [System.Drawing.Imaging.ImageFormat]::Png)
    $g.Dispose()
    $bmp.Dispose()
    Write-Host "Master Icon Generated: $OutputPath"
}

function Save-IconBundle {
    param (
        [string]$TargetDir = (Join-Path $PSScriptRoot "src-tauri\icons")
    )

    if (-not (Test-Path $TargetDir)) {
        New-Item -ItemType Directory -Force -Path $TargetDir | Out-Null
    }

    # 1. 產出 512x512 Master PNG
    $master512Path = Join-Path $TargetDir "icon.png"
    Generate-DashboardMasterIcon -Size 512 -OutputPath $master512Path

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

    # 2. Tauri 2 所需標準 PNG 尺寸
    Resize-Image 256 (Join-Path $TargetDir "128x128@2x.png")
    Resize-Image 128 (Join-Path $TargetDir "128x128.png")
    Resize-Image 32  (Join-Path $TargetDir "32x32.png")

    # 同步產出 ui/favicon.png
    $uiFaviconPath = Join-Path $PSScriptRoot "ui\favicon.png"
    $uiDir = Split-Path $uiFaviconPath
    if (-not (Test-Path $uiDir)) { New-Item -ItemType Directory -Force -Path $uiDir | Out-Null }
    Resize-Image 32 $uiFaviconPath

    # 3. 建立支援 Windows Vista/7/10/11 的多解析度 ICO 檔案 (256, 128, 64, 48, 32, 16)
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

    # 寫入 ICO 二進位結構
    $icoPath = Join-Path $TargetDir "icon.ico"
    $fs = [System.IO.File]::Create((Resolve-Path -Path $TargetDir | Join-Path -ChildPath "icon.ico"))
    $bw = [System.IO.BinaryWriter]::new($fs)

    # 1. ICONDIR (6 bytes)
    $bw.Write([UInt16]0)                  # Reserved
    $bw.Write([UInt16]1)                  # 1 = Icon
    $bw.Write([UInt16]$icoPngBytes.Count) # 總影格數 (6)

    # 2. ICONDIRENTRY (16 bytes each)
    $offset = 6 + ($icoPngBytes.Count * 16)
    foreach ($entry in $icoPngBytes) {
        $s = $entry[0]
        $bytes = $entry[1]

        $wByte = if ($s -eq 256) { [byte]0 } else { [byte]$s }
        $hByte = if ($s -eq 256) { [byte]0 } else { [byte]$s }

        $bw.Write([byte]$wByte)          # Width
        $bw.Write([byte]$hByte)          # Height
        $bw.Write([byte]0)               # Color count
        $bw.Write([byte]0)               # Reserved
        $bw.Write([UInt16]1)             # Planes
        $bw.Write([UInt16]32)            # Bit count
        $bw.Write([UInt32]$bytes.Length) # 資源長度
        $bw.Write([UInt32]$offset)       # 資源位移偏移量

        $offset += $bytes.Length
    }

    # 3. 各影格之 PNG 二進位數據
    foreach ($entry in $icoPngBytes) {
        $bw.Write($entry[1])
    }

    $bw.Flush()
    $bw.Dispose()
    $fs.Dispose()

    Write-Host "Created multi-resolution ICO: $icoPath with $($icoPngBytes.Count) frames (256, 128, 64, 48, 32, 16)"
}

Save-IconBundle
