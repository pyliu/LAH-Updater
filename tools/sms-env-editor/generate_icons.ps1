# -*- coding: utf-8 -*-
# 智慧控管與監控系統 - 地籍異動簡訊參數設定工具 (sms-env-editor)
# 專業級原生圖示產出腳本 (generate_icons.ps1)
# 視覺主題：天青石深藍圓角徽章 + 純白簡訊訊息泡泡 + 琥珀金設定齒輪

Add-Type -AssemblyName System.Drawing

function Generate-SmsEditorMasterIcon {
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

    $scale = [float]$Size / 512.0
    function S([float]$val) { return [float]($val * $scale) }

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

    # =========================================================================
    # 1. 基底圓角徽章 (Squircle Container)
    # =========================================================================
    $sqX = [float](S 24.0)
    $sqY = [float](S 24.0)
    $sqW = [float](S 464.0)
    $sqH = [float](S 464.0)
    $sqR = [float](S 105.0)

    $squirclePath = Get-RoundedRectPath $sqX $sqY $sqW $sqH $sqR
    $sqRect = [System.Drawing.RectangleF]::new($sqX, $sqY, $sqW, $sqH)

    # 漸層背景：天青石深藍 (#0e2a54) 至 海洋湛藍 (#0284c7)
    $bgBrush = [System.Drawing.Drawing2D.LinearGradientBrush]::new(
        $sqRect,
        [System.Drawing.Color]::FromArgb(255, 14, 42, 84),
        [System.Drawing.Color]::FromArgb(255, 2, 132, 199),
        [float]45.0
    )
    $g.FillPath($bgBrush, $squirclePath)
    $bgBrush.Dispose()

    # 外邊框亮光光芒
    $borderPen = [System.Drawing.Pen]::new(
        [System.Drawing.Color]::FromArgb(255, 56, 189, 248),
        [float](S 7.5)
    )
    $g.DrawPath($borderPen, $squirclePath)
    $borderPen.Dispose()

    # =========================================================================
    # 2. 簡訊訊息對話泡泡 (Speech Bubble)
    # =========================================================================
    $bX = [float](S 58.0)
    $bY = [float](S 68.0)
    $bW = [float](S 334.0)
    $bH = [float](S 240.0)
    $bR = [float](S 42.0)

    function Get-BubblePath([float]$x, [float]$y, [float]$w, [float]$h, [float]$r) {
        $p = [System.Drawing.Drawing2D.GraphicsPath]::new()
        $d = $r * 2.0
        # 頂部與右側圓角
        $p.AddArc($x, $y, $d, $d, [float]180.0, [float]90.0)
        $p.AddArc($x + $w - $d, $y, $d, $d, [float]270.0, [float]90.0)
        $p.AddArc($x + $w - $d, $y + $h - $d, $d, $d, [float]0.0, [float]90.0)
        # 底部對話尾巴 (Tail) 延伸至左下方
        $tailStart = $x + (S 120.0)
        $tailTipX  = $x + (S 36.0)
        $tailTipY  = $y + $h + (S 48.0)
        $tailEnd   = $x + (S 65.0)

        $p.AddLine($x + $w - $r, $y + $h, $tailStart, $y + $h)
        $p.AddLine($tailStart, $y + $h, $tailTipX, $tailTipY)
        $p.AddLine($tailTipX, $tailTipY, $tailEnd, $y + $h)
        $p.AddArc($x, $y + $h - $d, $d, $d, [float]90.0, [float]90.0)
        $p.CloseFigure()
        return $p
    }

    # 泡泡柔和陰影
    $shadowPath = Get-BubblePath ($bX + (S 6.0)) ($bY + (S 9.0)) $bW $bH $bR
    $shadowBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(75, 4, 15, 35))
    $g.FillPath($shadowBrush, $shadowPath)
    $shadowBrush.Dispose()
    $shadowPath.Dispose()

    # 泡泡主體填充：微漸層白亮色
    $bubblePath = Get-BubblePath $bX $bY $bW $bH $bR
    $bubbleRect = [System.Drawing.RectangleF]::new($bX, $bY, $bW, $bH + (S 48.0))
    $bubbleBrush = [System.Drawing.Drawing2D.LinearGradientBrush]::new(
        $bubbleRect,
        [System.Drawing.Color]::FromArgb(255, 255, 255, 255),
        [System.Drawing.Color]::FromArgb(255, 238, 245, 253),
        [float]90.0
    )
    $g.FillPath($bubbleBrush, $bubblePath)
    $bubbleBrush.Dispose()

    # 泡泡純白精緻邊框
    $bubblePen = [System.Drawing.Pen]::new([System.Drawing.Color]::FromArgb(255, 255, 255, 255), [float](S 3.0))
    $g.DrawPath($bubblePen, $bubblePath)
    $bubblePen.Dispose()
    $bubblePath.Dispose()

    # =========================================================================
    # 3. 泡泡內部：訊息波紋與訊號傳遞條 (SMS Message Bars)
    # =========================================================================
    function Fill-Pill([float]$px, [float]$py, [float]$pw, [float]$ph, [System.Drawing.Color]$col) {
        $pillPath = Get-RoundedRectPath $px $py $pw $ph ($ph / 2.0)
        $pillBrush = [System.Drawing.SolidBrush]::new($col)
        $g.FillPath($pillBrush, $pillPath)
        $pillBrush.Dispose()
        $pillPath.Dispose()
    }

    # 條目 1：深海藍主訊息列
    Fill-Pill (S 102.0) (S 118.0) (S 180.0) (S 22.0) ([System.Drawing.Color]::FromArgb(255, 2, 132, 199))
    # 條目 2：天空藍內容列
    Fill-Pill (S 102.0) (S 158.0) (S 236.0) (S 22.0) ([System.Drawing.Color]::FromArgb(255, 56, 189, 248))
    # 條目 3：短波訊號列 + 2 顆即時傳送圓點
    Fill-Pill (S 102.0) (S 198.0) (S 120.0) (S 22.0) ([System.Drawing.Color]::FromArgb(255, 148, 163, 184))
    Fill-Pill (S 236.0) (S 198.0) (S 22.0)  (S 22.0) ([System.Drawing.Color]::FromArgb(255, 56, 189, 248))
    Fill-Pill (S 270.0) (S 198.0) (S 22.0)  (S 22.0) ([System.Drawing.Color]::FromArgb(255, 2, 132, 199))

    # =========================================================================
    # 4. 琥珀金設定齒輪 (Amber Gold Settings Gear - 8 Teeth)
    # =========================================================================
    $cX = [float](S 358.0)
    $cY = [float](S 358.0)
    $rOut  = [float](S 96.0)  # 齒頂半徑
    $rRoot = [float](S 76.0)  # 齒根半徑
    $rHole = [float](S 32.0)  # 軸心孔半徑

    function Get-GearPath([float]$centerX, [float]$centerY, [float]$rO, [float]$rR, [float]$rH) {
        $path = [System.Drawing.Drawing2D.GraphicsPath]::new()
        $path.FillMode = [System.Drawing.Drawing2D.FillMode]::Alternate

        $numTeeth = 8
        $step = [Math]::PI * 2.0 / $numTeeth
        $halfToothRoot = 0.17 # 弧度
        $halfToothTip  = 0.09 # 弧度

        $pts = [System.Collections.Generic.List[System.Drawing.PointF]]::new()

        for ($i = 0; $i -lt $numTeeth; $i++) {
            $midAngle = $i * $step

            # 1. 齒頂左
            $a1 = $midAngle - $halfToothTip
            $pts.Add([System.Drawing.PointF]::new($centerX + $rO * [Math]::Cos($a1), $centerY + $rO * [Math]::Sin($a1)))

            # 2. 齒頂右
            $a2 = $midAngle + $halfToothTip
            $pts.Add([System.Drawing.PointF]::new($centerX + $rO * [Math]::Cos($a2), $centerY + $rO * [Math]::Sin($a2)))

            # 3. 齒根右
            $a3 = $midAngle + $halfToothRoot
            $pts.Add([System.Drawing.PointF]::new($centerX + $rR * [Math]::Cos($a3), $centerY + $rR * [Math]::Sin($a3)))

            # 4. 下一齒齒根左
            $nextAngle = ($i + 1) * $step
            $a4 = $nextAngle - $halfToothRoot
            $pts.Add([System.Drawing.PointF]::new($centerX + $rR * [Math]::Cos($a4), $centerY + $rR * [Math]::Sin($a4)))
        }

        $path.AddPolygon($pts.ToArray())

        # 軸心孔
        $path.AddEllipse(($centerX - $rH), ($centerY - $rH), ($rH * 2.0), ($rH * 2.0))
        return $path
    }

    # 齒輪陰影
    $gearShadowPath = Get-GearPath ($cX + (S 6.0)) ($cY + (S 8.0)) $rOut $rRoot $rHole
    $gearShadowBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(90, 4, 15, 35))
    $g.FillPath($gearShadowBrush, $gearShadowPath)
    $gearShadowBrush.Dispose()
    $gearShadowPath.Dispose()

    # 齒輪純白外框 (讓齒輪與背景、泡泡產生立體高對比脫鉤)
    $gearPath = Get-GearPath $cX $cY $rOut $rRoot $rHole
    $gearRimPen = [System.Drawing.Pen]::new([System.Drawing.Color]::FromArgb(255, 255, 255, 255), [float](S 7.0))
    $gearRimPen.LineJoin = [System.Drawing.Drawing2D.LineJoin]::Round
    $g.DrawPath($gearRimPen, $gearPath)
    $gearRimPen.Dispose()

    # 齒輪漸層填充：亮琥珀金 (#fbbf24) 至 暖金橙 (#d97706)
    $gearRect = [System.Drawing.RectangleF]::new(($cX - $rOut), ($cY - $rOut), ($rOut * 2.0), ($rOut * 2.0))
    $gearBrush = [System.Drawing.Drawing2D.LinearGradientBrush]::new(
        $gearRect,
        [System.Drawing.Color]::FromArgb(255, 251, 191, 36),
        [System.Drawing.Color]::FromArgb(255, 217, 119, 6),
        [float]45.0
    )
    $g.FillPath($gearBrush, $gearPath)
    $gearBrush.Dispose()

    # 齒輪內緣深色輪廓筆觸
    $gearInnerPen = [System.Drawing.Pen]::new([System.Drawing.Color]::FromArgb(255, 180, 83, 9), [float](S 2.0))
    $g.DrawPath($gearInnerPen, $gearPath)
    $gearInnerPen.Dispose()
    $gearPath.Dispose()

    # 軸心內部深色金屬軸承孔
    $holeBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(255, 14, 42, 84))
    $g.FillEllipse($holeBrush, ($cX - $rHole), ($cY - $rHole), ($rHole * 2.0), ($rHole * 2.0))
    $holeBrush.Dispose()

    $holePen = [System.Drawing.Pen]::new([System.Drawing.Color]::FromArgb(255, 254, 240, 138), [float](S 3.0))
    $g.DrawEllipse($holePen, ($cX - $rHole), ($cY - $rHole), ($rHole * 2.0), ($rHole * 2.0))
    $holePen.Dispose()

    # =========================================================================
    # 5. 輸出儲存
    # =========================================================================
    $g.Dispose()
    $bmp.Save($OutputPath, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
    Write-Host "Master Icon generated: $OutputPath ($Size x $Size)"
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
    Generate-SmsEditorMasterIcon -Size 512 -OutputPath $master512Path

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
