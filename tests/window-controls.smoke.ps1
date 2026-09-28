param(
    [string]$Executable = "$PSScriptRoot\..\src-tauri\target\release\fast-tools.exe"
)

$ErrorActionPreference = "Stop"

Add-Type -AssemblyName UIAutomationClient
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;

public static class WindowTestNative {
    [DllImport("user32.dll")]
    public static extern bool IsIconic(IntPtr window);

    [DllImport("user32.dll")]
    public static extern bool IsZoomed(IntPtr window);

    [DllImport("user32.dll")]
    public static extern bool IsWindowVisible(IntPtr window);

    [DllImport("user32.dll")]
    public static extern bool ShowWindow(IntPtr window, int command);

    [DllImport("user32.dll")]
    public static extern bool SetForegroundWindow(IntPtr window);
}
"@

function Wait-Until {
    param(
        [scriptblock]$Condition,
        [string]$Failure,
        [int]$TimeoutMilliseconds = 5000
    )

    $timer = [System.Diagnostics.Stopwatch]::StartNew()
    while ($timer.ElapsedMilliseconds -lt $TimeoutMilliseconds) {
        if (& $Condition) {
            return
        }
        Start-Sleep -Milliseconds 100
    }

    throw $Failure
}

function Get-WindowButton {
    param(
        [IntPtr]$Window,
        [string]$Name
    )

    $root = [System.Windows.Automation.AutomationElement]::FromHandle($Window)
    $condition = [System.Windows.Automation.PropertyCondition]::new(
        [System.Windows.Automation.AutomationElement]::NameProperty,
        $Name
    )
    $button = $root.FindFirst(
        [System.Windows.Automation.TreeScope]::Descendants,
        $condition
    )

    if ($null -eq $button) {
        throw "No se encontró el botón '$Name'."
    }

    return $button
}

function Invoke-WindowButton {
    param(
        [IntPtr]$Window,
        [string]$Name
    )

    $button = Get-WindowButton -Window $Window -Name $Name
    $pattern = $button.GetCurrentPattern(
        [System.Windows.Automation.InvokePattern]::Pattern
    )
    $pattern.Invoke()
}

$resolvedExecutable = (Resolve-Path -LiteralPath $Executable).Path
$process = $null

try {
    $process = Start-Process -FilePath $resolvedExecutable -PassThru

    Wait-Until -Failure "La ventana principal no apareció." -Condition {
        $process.Refresh()
        $process.MainWindowHandle -ne [IntPtr]::Zero
    }

    $window = $process.MainWindowHandle
    Wait-Until -Failure "Los controles de ventana no aparecieron." -Condition {
        try {
            $null -ne (Get-WindowButton -Window $window -Name "Minimizar")
        } catch {
            $false
        }
    }

    Invoke-WindowButton -Window $window -Name "Minimizar"
    Wait-Until -Failure "Minimizar no cambió el estado de ventana." -Condition {
        [WindowTestNative]::IsIconic($window)
    }

    [void][WindowTestNative]::ShowWindow($window, 9)
    [void][WindowTestNative]::SetForegroundWindow($window)
    Wait-Until -Failure "La ventana no se restauró tras minimizar." -Condition {
        -not [WindowTestNative]::IsIconic($window)
    }

    Invoke-WindowButton -Window $window -Name "Maximizar o restaurar"
    Wait-Until -Failure "Maximizar no cambió el estado de ventana." -Condition {
        [WindowTestNative]::IsZoomed($window)
    }

    Invoke-WindowButton -Window $window -Name "Maximizar o restaurar"
    Wait-Until -Failure "Restaurar no cambió el estado de ventana." -Condition {
        -not [WindowTestNative]::IsZoomed($window)
    }

    Invoke-WindowButton -Window $window -Name "Cerrar"
    Wait-Until -Failure "Cerrar no ocultó la ventana." -Condition {
        -not [WindowTestNative]::IsWindowVisible($window)
    }

    if ($process.HasExited) {
        throw "Cerrar terminó el proceso en vez de mantenerlo en bandeja."
    }

    Write-Output "OK: minimizar, maximizar, restaurar y cerrar a bandeja funcionan."
} finally {
    if ($null -ne $process -and -not $process.HasExited) {
        Stop-Process -Id $process.Id -Force
        Wait-Process -Id $process.Id -ErrorAction SilentlyContinue
    }
}
