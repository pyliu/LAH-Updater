$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false)
$request = $env:LAH_NODE_UNINSTALL_REQUEST | ConvertFrom-Json

function Service-Exe([string]$command) {
    if ($command -match '^\s*"([^"]+)"') { return $matches[1] }
    if ($command -match '^\s*(.+?\.exe)(?:\s|$)') { return $matches[1] }
    return $command.Trim()
}

function Owned-Service {
    $service = Get-CimInstance Win32_Service -Filter "Name='PM2_Portable_Service'"
    if ($service -and (Service-Exe $service.PathName) -ine $request.service_exe) {
        throw 'Service executable changed; removal stopped.'
    }
    return $service
}

switch ($request.action) {
    'inventory' {
        $services = @(Get-CimInstance Win32_Service | ForEach-Object {
            $exe = Service-Exe $_.PathName
            $xml = [IO.Path]::ChangeExtension($exe, '.xml')
            if ($_.PathName -match '--config(?:=|\s+)(?:"([^"]+)"|(\S+))') {
                $xml = if ($matches[1]) { $matches[1] } else { $matches[2] }
                if (-not [IO.Path]::IsPathRooted($xml)) { $xml = Join-Path ([IO.Path]::GetDirectoryName($exe)) $xml }
            }
            $configuration = ''
            if ([IO.File]::Exists($xml)) { $configuration = [IO.File]::ReadAllText($xml) }
            $configNode = ''; $configHome = ''; $configRunner = ''
            if ($_.Name -eq 'PM2_Portable_Service' -and $configuration) {
                $document = [xml]$configuration
                $configNode = [string]$document.service.executable
                $configRunner = [string]$document.service.arguments
                $configHome = [string]($document.service.env | Where-Object { $_.name -eq 'PM2_HOME' }).value
            }
            [pscustomobject]@{
                name = $_.Name; executable = $exe; configuration = $configuration
                running = ($_.State -ne 'Stopped'); pid = [uint32]$_.ProcessId
                config_node = $configNode; config_home = $configHome; config_runner = $configRunner
                command_line = [string]$_.PathName
            }
        })
        $processes = @(Get-CimInstance Win32_Process | ForEach-Object {
            [pscustomobject]@{
                pid = [uint32]$_.ProcessId; parent_pid = [uint32]$_.ParentProcessId
                name = $_.Name; executable = [string]$_.ExecutablePath
                created = $(if ($_.CreationDate) { [string]$_.CreationDate.ToUniversalTime().Ticks } else { '' })
            }
        })
        @{ services = $services; processes = $processes } | ConvertTo-Json -Depth 5 -Compress
    }
    'stop-service' {
        if (Owned-Service) {
            $service = Get-Service -Name PM2_Portable_Service
            if ($service.Status -ne 'Stopped') {
                $service.Stop()
                $service.WaitForStatus('Stopped', [TimeSpan]::FromSeconds(25))
            }
        }
    }
    'stop-processes' {
        $known = @{}
        foreach ($expected in $request.processes) {
            $expected | Add-Member -NotePropertyName exited -NotePropertyValue $null
            $known[[uint32]$expected.pid] = $expected
        }
        function Capture-Descendants {
            $all = @(Get-CimInstance Win32_Process)
            do {
                $added = $false
                foreach ($child in $all) {
                    $childId = [uint32]$child.ProcessId
                    $parentId = [uint32]$child.ParentProcessId
                    if ($known.ContainsKey($childId) -or -not $known.ContainsKey($parentId)) { continue }
                    $parent = $known[$parentId]
                    $born = $child.CreationDate.ToUniversalTime().Ticks
                    if ($born -lt [long]$parent.created) { continue }
                    $liveParent = $all | Where-Object { $_.ProcessId -eq $parentId } | Select-Object -First 1
                    $parentMatches = $liveParent -and ([string]$liveParent.CreationDate.ToUniversalTime().Ticks -eq $parent.created)
                    # Exit time bounds parent ownership even after its PID becomes available again.
                    if ($parent.exited -and $born -gt $parent.exited) { continue }
                    if (-not $parentMatches -and -not $parent.exited) {
                        throw "Cannot verify a new orphan process: $childId; removal stopped."
                    }
                    if (-not $child.ExecutablePath) { throw "Cannot verify child executable: $childId" }
                    $known[$childId] = [pscustomobject]@{
                        pid = $childId; parent_pid = $parentId; name = $child.Name
                        executable = [string]$child.ExecutablePath; created = [string]$born; exited = $null
                    }
                    $added = $true
                }
            } while ($added)
        }
        for ($round = 0; $round -lt 32; $round++) {
          Capture-Descendants
          $active = @($known.Values | Where-Object { -not $_.exited } | Sort-Object created)
          if (-not $active.Count) { return }
          foreach ($expected in $active) {
            $process = Get-CimInstance Win32_Process -Filter "ProcessId=$([uint32]$expected.pid)"
            if (-not $process) { continue }
            $created = [string]$process.CreationDate.ToUniversalTime().Ticks
            if ($created -ne $expected.created -or $process.ExecutablePath -ine $expected.executable) {
                throw "Process identity changed: $($expected.pid); removal stopped."
            }
            $handle = Get-Process -Id $expected.pid -ErrorAction SilentlyContinue
            if ($handle) {
                # Keep the verified handle so PID reuse cannot select a different process.
                $null = $handle.Handle
                # CIM dates have microsecond precision; process handles retain 100 ns ticks.
                if ([math]::Abs($handle.StartTime.ToUniversalTime().Ticks - [long]$expected.created) -gt 9) {
                    throw "Process start time changed: $($expected.pid)"
                }
                $handle.Kill()
                if (-not $handle.WaitForExit(10000)) { throw "Process did not exit: $($expected.pid)" }
                $expected.exited = $handle.ExitTime.ToUniversalTime().Ticks
                Capture-Descendants
            }
          }
          Capture-Descendants
          $remaining = @(Get-CimInstance Win32_Process | Where-Object { $known.ContainsKey([uint32]$_.ProcessId) -and -not $known[[uint32]$_.ProcessId].exited })
          if (-not $remaining.Count) { return }
        }
        throw 'New child processes keep appearing; removal stopped.'
    }
    'delete-service' {
        if (Owned-Service) {
            if ((Get-Service PM2_Portable_Service).Status -ne 'Stopped') { throw 'Service is still running.' }
            $output = & "$env:SystemRoot\System32\sc.exe" delete PM2_Portable_Service
            if ($LASTEXITCODE -ne 0) { throw ($output -join "`n") }
            for ($attempt = 0; $attempt -lt 40; $attempt++) {
                if (-not (Get-CimInstance Win32_Service -Filter "Name='PM2_Portable_Service'")) { return }
                Start-Sleep -Milliseconds 250
            }
            throw 'Service is marked for deletion; close Services and retry before deleting files.'
        }
    }
    default { throw 'Unsupported removal action.' }
}
