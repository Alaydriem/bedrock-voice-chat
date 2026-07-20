# Instruments the running BVC server while a swarm run streams: waits for
# streaming to start, then samples bvc-server process CPU + NIC egress + the
# /metrics inbound-frame rate over a 30s window, and prints per-delivery
# constants. `route_audio_frame` duration alone misses QUIC encrypt+sendmsg
# (~7/8 of server CPU), so total process CPU is what makes the capacity map real.
#
# Usage: run in a second shell WHILE `swarm controller` is streaming.
#   pwsh -File tools/swarm/measure-server.ps1
# Adjust $Server / $Fanout to match the run (Fanout = group_size - 1).

param(
  [string]$Server = 'https://local.bedrockvc.stream:443',
  [int]$Fanout = 49,
  [int]$SampleSeconds = 30
)

$ErrorActionPreference = 'Continue'

function Get-Frames {
  try {
    $r = Invoke-WebRequest -Uri "$Server/metrics/" -SkipCertificateCheck -TimeoutSec 8 -UseBasicParsing
    if ($r.Content -match 'bvc_audio_frames_routed_total\s+([0-9.]+)') { return [double]$matches[1] }
  } catch {}
  return -1.0
}

$prev = Get-Frames
Write-Host "waiting for streaming (start frames=$prev)..."
$active = $false
for ($i = 0; $i -lt 90; $i++) {
  Start-Sleep -Seconds 3
  $cur = Get-Frames
  $rate = if ($cur -gt 0 -and $prev -gt 0) { ($cur - $prev) / 3 } else { 0 }
  if ($cur -gt 0 -and $rate -gt 500) { Write-Host ("streaming active (inbound ~{0:N0}/s)" -f $rate); $active = $true; break }
  $prev = $cur
}
if (-not $active) { Write-Host "never detected streaming"; exit 1 }

$c0 = (Get-Process bvc-server).CPU
$f0 = Get-Frames
$net0 = (Get-NetAdapterStatistics | Measure-Object -Property SentBytes -Sum).Sum
$t0 = Get-Date
Start-Sleep -Seconds $SampleSeconds
$c1 = (Get-Process bvc-server).CPU
$f1 = Get-Frames
$net1 = (Get-NetAdapterStatistics | Measure-Object -Property SentBytes -Sum).Sum
$t1 = Get-Date

$wall = ($t1 - $t0).TotalSeconds
$cores = ($c1 - $c0) / $wall
$inbound = ($f1 - $f0) / $wall
$deliv = $inbound * $Fanout
$egress = ($net1 - $net0) / $wall

Write-Host "============== SERVER MEASUREMENT =============="
Write-Host ("wall window:        {0:N1} s" -f $wall)
Write-Host ("server CPU:         {0:N2} cores" -f $cores)
Write-Host ("inbound frames/s:   {0:N0}" -f $inbound)
Write-Host ("deliveries/s (x{0}): {1:N0}" -f $Fanout, $deliv)
Write-Host ("egress:             {0:N1} Mbps" -f ($egress * 8 / 1e6))
Write-Host ("--- derived constants ---")
Write-Host ("CPU per delivery:   {0:N3} us" -f ($cores * 1e6 / $deliv))
Write-Host ("wire bytes/delivery:{0:N0} B" -f ($egress / $deliv))
Write-Host "==============================================="
