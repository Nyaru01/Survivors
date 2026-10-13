# Windows executable

The native `warcraft-survivors` launcher runs the survival game offline, without a WoW server
or account. It requires the English WoW 1.12.1 (build 5875) MPQ archives; newer WoW clients are
not compatible. Game data stays local and is never committed to Git.

## Build on D:

Install the Rust version pinned in `rust-toolchain.toml` for `x86_64-pc-windows-gnu`, with
`RUSTUP_HOME=D:\SurvivorsTools\rustup` and `CARGO_HOME=D:\SurvivorsTools\cargo`. A minimal
Rust installation is enough; rustup must not change the system PATH. Extract a 64-bit MinGW
UCRT toolchain into `D:\SurvivorsTools\mingw64` (so `bin\gcc.exe` exists).

From the repository root in PowerShell:

```powershell
.\scripts\build-windows.ps1
```

The script keeps Cargo caches, compilation output and temporary files in `D:\SurvivorsTools`,
limits compilation to two concurrent jobs, and builds without development instruments. It
copies the executable and any required MinGW DLLs into `out\AzerothSurvivors`.

## Play

Place the client MPQ files directly in `out\AzerothSurvivors\Data`, beside
`AzerothSurvivors.exe`. The whole game folder can then be moved together. Double-click the
executable, choose a class and zone, and press Enter. Use the arrow keys or WASD to move,
Space to jump, Escape to pause and 1/2/3 to choose an upgrade. See `CONTROLS.md` for all inputs.

Settings and crash reports are stored in `benilla-config` beside the executable. The game
reads the MPQ files without modifying them. An inherited `WOW_DATA` environment variable
overrides the adjacent Data folder; remove that override when testing the packaged game.

The browser build is a separate target: the Bash setup scripts currently support Linux and
macOS and are not used to produce this Windows executable.
