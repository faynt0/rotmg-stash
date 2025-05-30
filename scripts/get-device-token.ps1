# This script queries WMI classes and concatenates the SerialNumber values, then computes a SHA1 hash of the result.

# List of WMI classes (in the required order)
$wmiClasses = @("Win32_BaseBoard", "Win32_BIOS", "Win32_OperatingSystem")
$hardwareInfo = ""

foreach ($wmiClass in $wmiClasses) {
    try {
        # Use Get-CimInstance for modern systems; alternatively you can use Get-WmiObject
        $instances = Get-CimInstance -ClassName $wmiClass -ErrorAction SilentlyContinue
        foreach ($instance in $instances) {
            $serial = $instance.SerialNumber
            if ($serial) {
                $hardwareInfo += $serial
            }
        }
    }
    catch {
        Write-Output "Failed to query WMI class $wmiClass"        
    }
}

if ([string]::IsNullOrEmpty($hardwareInfo)) {
    Write-Output "No hardware info found."
    exit 1
}

$sha1 = [System.Security.Cryptography.SHA1]::Create()
$bytes = [System.Text.Encoding]::UTF8.GetBytes($hardwareInfo)
$hashBytes = $sha1.ComputeHash($bytes)
$deviceToken = -join ($hashBytes | ForEach-Object { $_.ToString("x2") })

# Output the device token without the \r\n
Write-Host -NoNewline $deviceToken