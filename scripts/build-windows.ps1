param(
    [string]$ToolsRoot = 'D:\SurvivorsTools',
    [string]$OutputDir = ''
)
$ErrorActionPreference = 'Stop'
$repo = Split-Path $PSScriptRoot -Parent
if (!$OutputDir) { $OutputDir = Join-Path $repo 'out\AzerothSurvivors' }
$ToolsRoot = [IO.Path]::GetFullPath($ToolsRoot)
$OutputDir = [IO.Path]::GetFullPath($OutputDir)
if ($ToolsRoot -notmatch '^D:\\' -or $OutputDir -notmatch '^D:\\') {
    throw 'Keep the toolchain and game output on D:.'
}
$env:RUSTUP_HOME = Join-Path $ToolsRoot 'rustup'
$env:CARGO_HOME = Join-Path $ToolsRoot 'cargo'
$env:CARGO_TARGET_DIR = Join-Path $ToolsRoot 'target'
$env:TEMP = Join-Path $ToolsRoot 'tmp'
$env:TMP = $env:TEMP
$env:PATH = "$ToolsRoot\cargo\bin;$ToolsRoot\mingw64\bin;$env:PATH"
$env:CARGO_PROFILE_RELEASE_DEBUG = '0'
$env:CARGO_BUILD_JOBS = '2'
foreach ($tool in @('cargo', 'gcc', 'objdump')) {
    if (!(Get-Command $tool -ErrorAction SilentlyContinue)) { throw "Missing $tool in $ToolsRoot" }
}
New-Item -ItemType Directory -Force $OutputDir, $env:TEMP | Out-Null
Push-Location $repo
try {
    cargo build --locked --release -p warcraft-survivors --no-default-features
    if ($LASTEXITCODE) { throw 'Windows compilation failed.' }
    $exe = Join-Path $OutputDir 'AzerothSurvivors.exe'
    Copy-Item -LiteralPath "$env:CARGO_TARGET_DIR\release\warcraft-survivors.exe" -Destination $exe
    $pending = [Collections.Generic.Queue[string]]::new()
    $pending.Enqueue($exe)
    $copied = @{}
    while ($pending.Count) {
        $binary = $pending.Dequeue()
        $imports = objdump -p $binary
        if ($LASTEXITCODE) { throw "Cannot inspect DLL dependencies: $binary" }
        foreach ($line in $imports) {
            if ($line -match 'DLL Name:\s*(\S+)') {
                $name = $Matches[1]
                $source = Join-Path "$ToolsRoot\mingw64\bin" $name
                if ((Test-Path -LiteralPath $source) -and !$copied.ContainsKey($name)) {
                    Copy-Item -LiteralPath $source -Destination $OutputDir
                    $copied[$name] = $true
                    $pending.Enqueue((Join-Path $OutputDir $name))
                }
            }
        }
    }
    Copy-Item -LiteralPath "$repo\LICENSE-MIT", "$repo\LICENSE-APACHE", "$repo\CONTROLS.md" -Destination $OutputDir
    @'
AZEROTH SURVIVORS

Double-cliquez sur AzerothSurvivors.exe, choisissez une classe et une zone,
puis appuyez sur Entree pour preparer talents, equipement et sorts avant de commencer.

Deplacement : fleches ou touches configurables dans Raccourcis
Saut : Espace | Pause : Echap | Amelioration : 1 / 2 / 3
Sort de deplacement (niveau 5) : F | Zoom : molette
Camera : clic droit + glisser | Ctrl+C : retour a la vue du dessus
C : personnage et equipement | B : sac a dos
N : arbres de talents Blizzard, conserves entre les parties (points a partir du niveau 10).
Les portraits et vies sont en haut a gauche ; les sorts automatiques en bas.
Clic ou touches 1 a 6 : lancer un sort (si sa recharge est terminee).
Clic droit sur un sort : activer/desactiver son lancement automatique.
F10 / Options : menus Blizzard pour graphismes, raccourcis et son.
E : ouvrir un coffre proche contre de l'or | P : grimoire | J : marchand de potions.
H : potion de vie (50%, recharge 20 s). Achat : 20 or, 3 par partie.
Personnalisez votre apparence et votre pseudo au menu.
Nouvelle partie ou Reprendre la partie : reprise en pause.
4 boss par monde, toutes les 3 minutes. Vaincre les 4 debloque le monde suivant.
Personnage, or, equipement et partie sauvegardes automatiquement.
Conservez benilla-config/survivors-save.toml et sa copie .bak.

Le jeu fonctionne hors ligne. Conservez le dossier Data-frFR (prioritaire)
ou Data et les eventuelles
bibliotheques DLL avec l'executable. Les reglages et rapports de plantage
sont dans benilla-config. CONTROLS.md contient toutes les commandes.

Jeu original gratuit cree par Aethus : https://wow.aethus.pl/
Version locale : https://github.com/OutThisLife/azeroth-survivors-re
Depot de cette version : https://github.com/Nyaru01/Survivors
Moteur benilla / wenilla, sous licences MIT ou Apache-2.0.
World of Warcraft appartient a Blizzard Entertainment.
'@ | Set-Content -LiteralPath (Join-Path $OutputDir 'LIRE-MOI.txt') -Encoding utf8
    Write-Host "Executable ready: $exe"
    $gameData = if (Test-Path -LiteralPath "$OutputDir\Data-frFR\patch-2.MPQ") { Join-Path $OutputDir 'Data-frFR' } else { Join-Path $OutputDir 'Data' }
    if (!(Test-Path -LiteralPath "$gameData\terrain.MPQ") -or
        !(Test-Path -LiteralPath "$gameData\patch.MPQ")) {
        Write-Warning 'WoW 1.12.1 MPQ data is required in the Data folder beside the executable.'
    }
} finally { Pop-Location }
