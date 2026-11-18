$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot
Add-Type -Path @(
    (Join-Path $root 'payload/injectee/src/FlowParameterTransfer.cs'),
    (Join-Path $root 'payload/injectee/tests/FlowParameterTransferTests.cs')
)
[XbotInjectee.FlowParameterTransferTests]::Run()
