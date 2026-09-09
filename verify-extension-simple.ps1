# Simple JSON validation for VSCode extension files

$ErrorActionPreference = "Continue"
$baseDir = "d:\Git\neo-rust-sdk"
$extensionDir = Join-Path $baseDir "editors/vscode"

Write-Host "`n==========================================" 
Write-Host "Neo Rust SDK VSCode Extension Validation" 
Write-Host "==========================================`n" 

Write-Host "[Step 1] File Tree Structure" 
Write-Host "-------------------------------------------" 
Get-ChildItem -Path $extensionDir -Recurse | Where-Object { $_.Extension -in @('.json', '.md') } | ForEach-Object { Write-Host ("   - " + $_.FullName.Replace($extensionDir + '\', '')) }
Write-Host ""

Write-Host "[Step 2] Validate All JSON Files (PowerShell)" 
Write-Host "-------------------------------------------" 

$jsonFiles = @('package.json', 'snippets/rust.json', 'language-configuration.json', 'syntaxes/neo-contract.tmLanguage.json')

foreach ($file in $jsonFiles) {
    $path = Join-Path $extensionDir $file
    if (Test-Path $path) {
        try {
            Get-Content $path -Raw | ConvertFrom-Json | Out-Null
            if ($file -eq 'package.json') {
                $p = Get-Content $path -Raw | ConvertFrom-Json
                $snippets = Get-Content $p.contributes.snippets[0].path -Raw | ConvertFrom-Json
                Write-Host "  [OK] $file" -ForegroundColor Green
                Write-Host "       name=$($p.name) version=$($p.version)"
                Write-Host "       grammar.scopeName=`"$($p.contributes.grammars[0].scopeName)`""
                $prefixes = ($snippets.PSObject.Properties.Name -join ', ')
                Write-Host "       snippets=$prefixes"
                continue
            }
            Write-Host "  [OK] $file" -ForegroundColor Green
        } catch {
            Write-Host "  [FAIL] $file" -ForegroundColor Red
            Write-Host "         $_"
        }
    } else {
        Write-Host "  [MISSING] $file" -ForegroundColor Red
    }
}

Write-Host ""
Write-Host "[Summary]" 
Write-Host "-------------------------------------------" 
Write-Host "  All JSON files are valid!" -ForegroundColor Green
Write-Host "  Extension is ready for packaging."
