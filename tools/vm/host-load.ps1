$f = 'C:\VMs\host-load.csv'
'utc,cpu_pct,avail_mb,pages_sec' | Set-Content $f
$end = (Get-Date).AddHours(4)
while ((Get-Date) -lt $end) {
    $c = (Get-Counter '\Processor(_Total)\% Processor Time', '\Memory\Available MBytes', '\Memory\Pages/sec' -ErrorAction SilentlyContinue).CounterSamples
    $line = '{0},{1},{2},{3}' -f (Get-Date).ToUniversalTime().ToString('HH:mm:ss'), [int]$c[0].CookedValue, [int]$c[1].CookedValue, [int]$c[2].CookedValue
    Add-Content $f $line
    Start-Sleep -Seconds 20
}
