Add-Type -AssemblyName System.Drawing

function Generate-BackuperMasterIcon {
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

    # Helper: Create Rounded Rectangle Path
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
    # 1. Base Squircle Badge Container (Matches updater/uninstaller/node-portable)
    # =========================================================================
    $sqX = [float](S 21.0)
    $sqY = [float](S 21.0)
    $sqW = [float](S 470.0)
    $sqH = [float](S 470.0)
    $sqR = [float](S 105.0)

    $squirclePath = Get-RoundedRectPath $sqX $sqY $sqW $sqH $sqR
    $sqRect = [System.Drawing.RectangleF]::new($sqX, $sqY, $sqW, $sqH)

    # Background Fill: Deep Midnight Navy
    $bgBrush = [System.Drawing.Drawing2D.LinearGradientBrush]::new(
        $sqRect,
        [System.Drawing.Color]::FromArgb(255, 12, 28, 52),
        [System.Drawing.Color]::FromArgb(255, 6, 17, 34),
        [float]45.0
    )
    $g.FillPath($bgBrush, $squirclePath)
    $bgBrush.Dispose()

    # Outer Border Stroke: Sky Blue / Cyan (Matches 6-8px border of Updater/Uninstaller)
    $borderPen = [System.Drawing.Pen]::new(
        [System.Drawing.Color]::FromArgb(255, 14, 165, 233),
        [float](S 7.5)
    )
    $borderBrush = [System.Drawing.Drawing2D.LinearGradientBrush]::new(
        $sqRect,
        [System.Drawing.Color]::FromArgb(255, 56, 189, 248),  # Light cyan top
        [System.Drawing.Color]::FromArgb(255, 2, 132, 199),   # Tech blue bottom
        [float]45.0
    )
    $borderPen.Brush = $borderBrush
    $g.DrawPath($borderPen, $squirclePath)
    $borderPen.Dispose()
    $borderBrush.Dispose()
    $squirclePath.Dispose()

    # =========================================================================
    # 2. Orbital Arrows Parameters (Ellipse rx=168, ry=140, Center cx=256, cy=256)
    # =========================================================================
    $cx = [float](S 256.0)
    $cy = [float](S 256.0)
    $orx = [float](S 168.0)
    $ory = [float](S 138.0)
    $orbRect = [System.Drawing.RectangleF]::new(($cx - $orx), ($cy - $ory), ($orx * 2.0), ($ory * 2.0))

    # UPPER ARROW BACK SEGMENT (passes behind the top platter)
    # Arc from angle 205 (upper-left) to 280 (top)
    $upperBackPath = [System.Drawing.Drawing2D.GraphicsPath]::new()
    $upperBackPath.AddArc($orbRect, [float]205.0, [float]75.0)

    $backPen = [System.Drawing.Pen]::new([System.Drawing.Color]::FromArgb(210, 2, 132, 199), [float](S 22.0))
    $backPen.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
    $backPen.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
    $g.DrawPath($backPen, $upperBackPath)
    $backPen.Dispose()
    $upperBackPath.Dispose()

    # =========================================================================
    # 3. Central Database Platter Stack (3-tier metallic disks)
    # =========================================================================
    $ew = [float](S 240.0)
    $eh = [float](S 76.0)
    $ex = [float]($cx - ($ew / 2.0))

    # Base drop shadow underneath platters
    $shadowBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(90, 2, 8, 20))
    $g.FillEllipse($shadowBrush, [float](S 120.0), [float](S 378.0), [float](S 272.0), [float](S 58.0))
    $shadowBrush.Dispose()

    function Draw-CylinderPlatter([float]$topY, [float]$height, [bool]$isTop) {
        $pTop = [float](S $topY)
        $pH = [float](S $height)

        # Side cylinder path
        $sidePath = [System.Drawing.Drawing2D.GraphicsPath]::new()
        $sidePath.AddArc($ex, $pTop, $ew, $eh, [float]0.0, [float]180.0)
        $sidePath.AddArc($ex, ($pTop + $pH), $ew, $eh, [float]180.0, [float]-180.0)
        $sidePath.CloseFigure()

        # Metallic Blue Gradient on cylinder side
        $sideRect = [System.Drawing.RectangleF]::new($ex, $pTop, $ew, ($eh + $pH))
        $sideBrush = [System.Drawing.Drawing2D.LinearGradientBrush]::new(
            $sideRect,
            [System.Drawing.Color]::FromArgb(255, 15, 35, 70),
            [System.Drawing.Color]::FromArgb(255, 30, 70, 160),
            [float]0.0
        )
        $colorBlend = [System.Drawing.Drawing2D.ColorBlend]::new()
        $colorBlend.Positions = @([float]0.0, [float]0.18, [float]0.52, [float]0.82, [float]1.0)
        $colorBlend.Colors = @(
            [System.Drawing.Color]::FromArgb(255, 12, 28, 58),
            [System.Drawing.Color]::FromArgb(255, 20, 55, 130),
            [System.Drawing.Color]::FromArgb(255, 96, 165, 250), # sharp specular highlight
            [System.Drawing.Color]::FromArgb(255, 25, 70, 180),
            [System.Drawing.Color]::FromArgb(255, 14, 32, 68)
        )
        $sideBrush.InterpolationColors = $colorBlend
        $g.FillPath($sideBrush, $sidePath)
        $sideBrush.Dispose()

        # Platter bottom rim highlight bevel
        $rimPen = [System.Drawing.Pen]::new([System.Drawing.Color]::FromArgb(200, 191, 219, 254), [float](S 2.2))
        $g.DrawArc($rimPen, $ex, ($pTop + $pH), $ew, $eh, [float]0.0, [float]180.0)
        $rimPen.Dispose()

        # Top Ellipse
        $topRect = [System.Drawing.RectangleF]::new($ex, $pTop, $ew, $eh)
        $topBrush = [System.Drawing.Drawing2D.LinearGradientBrush]::new(
            $topRect,
            [System.Drawing.Color]::FromArgb(255, 20, 45, 100),
            [System.Drawing.Color]::FromArgb(255, 45, 105, 225),
            [float]45.0
        )
        $topBlend = [System.Drawing.Drawing2D.ColorBlend]::new()
        $topBlend.Positions = @([float]0.0, [float]0.35, [float]0.70, [float]1.0)
        $topBlend.Colors = @(
            [System.Drawing.Color]::FromArgb(255, 18, 38, 85),
            [System.Drawing.Color]::FromArgb(255, 28, 65, 160),
            [System.Drawing.Color]::FromArgb(255, 40, 100, 230),
            [System.Drawing.Color]::FromArgb(255, 125, 190, 255)
        )
        $topBrush.InterpolationColors = $topBlend
        $g.FillEllipse($topBrush, $topRect)
        $topBrush.Dispose()

        # Concentric platter track lines
        $trackPen1 = [System.Drawing.Pen]::new([System.Drawing.Color]::FromArgb(75, 255, 255, 255), [float](S 1.4))
        $g.DrawEllipse($trackPen1, [float](S (256.0 - 95.0)), [float]($pTop + (S 8.0)), [float](S 190.0), [float](S 60.0))
        $trackPen1.Dispose()

        $trackPen2 = [System.Drawing.Pen]::new([System.Drawing.Color]::FromArgb(90, 186, 230, 253), [float](S 1.2))
        $g.DrawEllipse($trackPen2, [float](S (256.0 - 65.0)), [float]($pTop + (S 18.0)), [float](S 130.0), [float](S 40.0))
        $trackPen2.Dispose()

        # Center Spindle Hub & Glowing Core on top platter
        if ($isTop) {
            $hubRect = [System.Drawing.RectangleF]::new([float](S (256.0 - 38.0)), [float]($pTop + (S 22.0)), [float](S 76.0), [float](S 32.0))
            $hubBrush = [System.Drawing.Drawing2D.LinearGradientBrush]::new(
                $hubRect,
                [System.Drawing.Color]::FromArgb(255, 15, 23, 42),
                [System.Drawing.Color]::FromArgb(255, 100, 116, 139),
                [float]135.0
            )
            $g.FillEllipse($hubBrush, $hubRect)
            $hubBrush.Dispose()

            $hubRim = [System.Drawing.Pen]::new([System.Drawing.Color]::FromArgb(240, 56, 189, 248), [float](S 2.0))
            $g.DrawEllipse($hubRim, $hubRect)
            $hubRim.Dispose()

            $coreRect = [System.Drawing.RectangleF]::new([float](S (256.0 - 16.0)), [float]($pTop + (S 29.0)), [float](S 32.0), [float](S 16.0))
            $coreBrush = [System.Drawing.Drawing2D.LinearGradientBrush]::new(
                $coreRect,
                [System.Drawing.Color]::FromArgb(255, 0, 240, 255),
                [System.Drawing.Color]::FromArgb(255, 2, 132, 199),
                [float]45.0
            )
            $g.FillEllipse($coreBrush, $coreRect)
            $coreBrush.Dispose()
        }

        # Platter top outer rim bevel
        $topRimPen = [System.Drawing.Pen]::new([System.Drawing.Color]::FromArgb(230, 191, 219, 254), [float](S 2.2))
        $g.DrawEllipse($topRimPen, $topRect)
        $topRimPen.Dispose()

        $sidePath.Dispose()
    }

    # Draw the 3 platters bottom to top
    Draw-CylinderPlatter 285 36 $false
    Draw-CylinderPlatter 237 36 $false
    Draw-CylinderPlatter 189 36 $true

    # =========================================================================
    # 4. Seamless Aerodynamic 3D Foreground Arrows
    # =========================================================================

    # Helper function: Draw a complete arrow with attached arrowhead along ellipse arc
    # $startAngle: degree (e.g. 270)
    # $sweepAngle: degree (e.g. 100)
    # $isClockwise: bool (true for upper arrow, false for lower arrow)
    function Draw-OrbitalArrow(
        [float]$startDeg,
        [float]$sweepDeg,
        [System.Drawing.Color]$mainColor,
        [System.Drawing.Color]$gradientColor
    ) {
        $forePath = [System.Drawing.Drawing2D.GraphicsPath]::new()
        $forePath.AddArc($orbRect, $startDeg, $sweepDeg)

        # Outer neon glow
        $glowPen = [System.Drawing.Pen]::new([System.Drawing.Color]::FromArgb(70, $mainColor.R, $mainColor.G, $mainColor.B), [float](S 34.0))
        $glowPen.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
        $glowPen.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
        $g.DrawPath($glowPen, $forePath)
        $glowPen.Dispose()

        # Main Ribbon
        $ribbonPen = [System.Drawing.Pen]::new($mainColor, [float](S 24.0))
        $ribbonPen.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
        $ribbonPen.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
        $g.DrawPath($ribbonPen, $forePath)
        $ribbonPen.Dispose()

        # White Core Streak
        $corePen = [System.Drawing.Pen]::new([System.Drawing.Color]::FromArgb(245, 255, 255, 255), [float](S 7.5))
        $corePen.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
        $corePen.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
        $g.DrawPath($corePen, $forePath)
        $corePen.Dispose()
        $forePath.Dispose()

        # Calculate exact tangent and position at the end of the arc
        $endAngleDeg = $startDeg + $sweepDeg
        $endRad = [float]($endAngleDeg * [Math]::PI / 180.0)

        # Point on ellipse:
        $pEndX = [float]($cx + $orx * [Math]::Cos($endRad))
        $pEndY = [float]($cy + $ory * [Math]::Sin($endRad))

        # Velocity / Tangent vector (dx, dy):
        # x = cx + orx * cos(t) -> dx/dt = -orx * sin(t)
        # y = cy + ory * sin(t) -> dy/dt =  ory * cos(t)
        $dx = [float](-$orx * [Math]::Sin($endRad))
        $dy = [float]( $ory * [Math]::Cos($endRad))
        if ($sweepDeg -lt 0) {
            $dx = -$dx
            $dy = -$dy
        }
        $len = [float][Math]::Sqrt($dx * $dx + $dy * $dy)
        $ux = [float]($dx / $len)
        $uy = [float]($dy / $len)
        $nx = [float](-$uy) # Normal vector
        $ny = [float]($ux)

        # Arrowhead dimensions
        $headLength = [float](S 40.0)
        $headWidth  = [float](S 28.0)

        # 4 vertices: Tip, Left Wing, Notch, Right Wing
        $tipX = [float]($pEndX + $ux * $headLength)
        $tipY = [float]($pEndY + $uy * $headLength)

        $leftX = [float]($pEndX + $nx * $headWidth - $ux * (S 6.0))
        $leftY = [float]($pEndY + $ny * $headWidth - $uy * (S 6.0))

        $notchX = [float]($pEndX + $ux * (S 4.0))
        $notchY = [float]($pEndY + $uy * (S 4.0))

        $rightX = [float]($pEndX - $nx * $headWidth - $ux * (S 6.0))
        $rightY = [float]($pEndY - $ny * $headWidth - $uy * (S 6.0))

        $ptTip   = [System.Drawing.PointF]::new($tipX, $tipY)
        $ptLeft  = [System.Drawing.PointF]::new($leftX, $leftY)
        $ptNotch = [System.Drawing.PointF]::new($notchX, $notchY)
        $ptRight = [System.Drawing.PointF]::new($rightX, $rightY)

        [System.Drawing.PointF[]]$arrowHeadPts = @($ptTip, $ptLeft, $ptNotch, $ptRight)

        # Arrowhead gradient fill
        $minX = [Math]::Min($leftX, [Math]::Min($rightX, $tipX))
        $minY = [Math]::Min($leftY, [Math]::Min($rightY, $tipY))
        $maxX = [Math]::Max($leftX, [Math]::Max($rightX, $tipX))
        $maxY = [Math]::Max($leftY, [Math]::Max($rightY, $tipY))
        $headRect = [System.Drawing.RectangleF]::new($minX, $minY, ($maxX - $minX + 1.0), ($maxY - $minY + 1.0))

        $headBrush = [System.Drawing.Drawing2D.LinearGradientBrush]::new(
            $headRect,
            $mainColor,
            $gradientColor,
            [float]45.0
        )
        $g.FillPolygon($headBrush, $arrowHeadPts)
        $headBrush.Dispose()

        # Sharp crisp white outline around arrowhead
        $headPen = [System.Drawing.Pen]::new([System.Drawing.Color]::FromArgb(255, 255, 255, 255), [float](S 2.2))
        $g.DrawPolygon($headPen, $arrowHeadPts)
        $headPen.Dispose()
    }

    # UPPER ARROW (Backup Flow):
    # Arc from 280 (top) clockwise to 380 (which is 20 deg, right side, pointing downwards)
    Draw-OrbitalArrow 280.0 100.0 ([System.Drawing.Color]::FromArgb(255, 0, 240, 255)) ([System.Drawing.Color]::FromArgb(255, 2, 132, 199))

    # LOWER ARROW (Restore Flow):
    # Arc from 35 (right side) clockwise to 190 (left side, pointing upwards)
    Draw-OrbitalArrow 35.0 155.0 ([System.Drawing.Color]::FromArgb(255, 56, 189, 248)) ([System.Drawing.Color]::FromArgb(255, 2, 132, 199))

    # =========================================================================
    # 5. Luminescent Cyber Sync Nodes
    # =========================================================================
    $nodeBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(255, 255, 255, 255))
    $g.FillEllipse($nodeBrush, [float](S 256.0), [float](S 113.0), [float](S 9.0), [float](S 9.0))
    $g.FillEllipse($nodeBrush, [float](S 380.0), [float](S 185.0), [float](S 8.0), [float](S 8.0))
    $g.FillEllipse($nodeBrush, [float](S 140.0), [float](S 325.0), [float](S 8.0), [float](S 8.0))
    $g.FillEllipse($nodeBrush, [float](S 256.0), [float](S 390.0), [float](S 9.0), [float](S 9.0))
    $nodeBrush.Dispose()

    $bmp.Save($OutputPath, [System.Drawing.Imaging.ImageFormat]::Png)
    $g.Dispose()
    $bmp.Dispose()
    Write-Host "Created perfected master icon: $OutputPath ($Size x $Size)"
}

Generate-BackuperMasterIcon -Size 512 -OutputPath "test_backuper_perfect.png"
Add-Type -AssemblyName System.Drawing

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
if (-not $scriptDir) { $scriptDir = (Get-Location).Path }
$defaultTarget = Join-Path $scriptDir "src-tauri\icons"

function Save-IconBundle {
    param (
        [string]$TargetDir = $defaultTarget
    )

    if (-not (Test-Path $TargetDir)) {
        New-Item -ItemType Directory -Force -Path $TargetDir | Out-Null
    }

    # Generate Master 512x512
    $master512Path = Join-Path $TargetDir "icon.png"
    Generate-BackuperMasterIcon -Size 512 -OutputPath $master512Path

    # Load master bitmap to high-quality downsample to standard sizes
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

    # Standard PNG files required by Tauri 2
    $p256Path = Join-Path $TargetDir "128x128@2x.png"
    $p128Path = Join-Path $TargetDir "128x128.png"
    $p32Path  = Join-Path $TargetDir "32x32.png"

    Resize-Image 256 $p256Path
    Resize-Image 128 $p128Path
    Resize-Image 32  $p32Path

    # ICO sizes: 256, 128, 64, 48, 32, 16
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

    # Construct standard Windows ICO binary
    $icoPath = Join-Path $TargetDir "icon.ico"
    $fs = [System.IO.File]::Create((Resolve-Path -Path $TargetDir | Join-Path -ChildPath "icon.ico"))
    $bw = [System.IO.BinaryWriter]::new($fs)

    # 1. ICONDIR (6 bytes)
    $bw.Write([UInt16]0) # Reserved
    $bw.Write([UInt16]1) # 1 = Icon
    $bw.Write([UInt16]$icoPngBytes.Count) # Image count

    # 2. ICONDIRENTRY (16 bytes each)
    $offset = 6 + ($icoPngBytes.Count * 16)
    foreach ($entry in $icoPngBytes) {
        $s = $entry[0]
        $bytes = $entry[1]

        $wByte = if ($s -eq 256) { [byte]0 } else { [byte]$s }
        $hByte = if ($s -eq 256) { [byte]0 } else { [byte]$s }

        $bw.Write([byte]$wByte)        # Width
        $bw.Write([byte]$hByte)        # Height
        $bw.Write([byte]0)             # Color count (0 = >=8bpp)
        $bw.Write([byte]0)             # Reserved
        $bw.Write([UInt16]1)           # Planes
        $bw.Write([UInt16]32)          # Bit count
        $bw.Write([UInt32]$bytes.Length) # Bytes in resource
        $bw.Write([UInt32]$offset)     # Offset to image data

        $offset += $bytes.Length
    }

    # 3. Image Data
    foreach ($entry in $icoPngBytes) {
        $bw.Write($entry[1])
    }

    $bw.Flush()
    $bw.Dispose()
    $fs.Dispose()

    Write-Host "Created multi-resolution ICO: $icoPath with $($icoPngBytes.Count) frames (256, 128, 64, 48, 32, 16)"
}
Save-IconBundle
