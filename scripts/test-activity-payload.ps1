$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot
# This harness loads only isolated helpers; it never loads or connects to ShadowBot.
Add-Type -Path @(
    (Join-Path $root 'payload/injectee/tests/StudioActionsStub.cs'),
    (Join-Path $root 'payload/injectee/src/ActivityInstallGuard.cs'),
    (Join-Path $root 'payload/injectee/src/ActivityMetadata.cs'),
    (Join-Path $root 'payload/injectee/tests/ActivityLifecycleTests.cs')
)
[XbotInjectee.ActivityLifecycleTests]::Run()
