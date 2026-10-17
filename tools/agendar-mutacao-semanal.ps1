# Registers (or updates) the weekly full mutation run as a Windows scheduled task for the
# current user: Mondays 09:00, or as soon as possible if the computer was off (T-00.06).
# Run with: pnpm mutation:agendar
# Remove with: Unregister-ScheduledTask -TaskName "Limpador - mutacao semanal"
# (ASCII only: Windows PowerShell 5.1 reads scripts without BOM as ANSI.)
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$node = (Get-Command node).Source

$action = New-ScheduledTaskAction -Execute $node -Argument "tools\weekly-mutation.mjs" -WorkingDirectory $root
$trigger = New-ScheduledTaskTrigger -Weekly -DaysOfWeek Monday -At 9:00
$settings = New-ScheduledTaskSettingsSet -StartWhenAvailable -ExecutionTimeLimit (New-TimeSpan -Hours 3)

Register-ScheduledTask -TaskName "Limpador - mutacao semanal" -Action $action -Trigger $trigger `
    -Settings $settings -Force `
    -Description "Roda pnpm mutation:full no Limpador e avisa por notificacao se alguma meta de mutacao cair." |
    Select-Object TaskName, State
