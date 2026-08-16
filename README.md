# Portable Visual Novel Galge Hook

This DLL hooks selected Win32 APIs so legacy games can keep their per-user data beside the executable instead of writing to the real user profile under `C:\Users\<name>`.

## Redirect layout

Run the target with the desired portable directory as its current working directory. When the target asks for supported user folders, the hook returns or rewrites them into that current directory:

```text
%CD%\AppData\Roaming   <- APPDATA / CSIDL_APPDATA / FOLDERID_RoamingAppData
%CD%\AppData\Local     <- LOCALAPPDATA / CSIDL_LOCAL_APPDATA / FOLDERID_LocalAppData
%CD%\AppData\LocalLow  <- FOLDERID_LocalAppDataLow and paths containing \AppData\LocalLow
%CD%\Documents         <- CSIDL_PERSONAL / CSIDL_MYDOCUMENTS / FOLDERID_Documents
%CD%\Saved Games       <- FOLDERID_SavedGames and %USERPROFILE%\Saved Games paths
```

This covers programs that use Shell known-folder APIs, environment-variable APIs, or direct file paths such as:

```text
%APPDATA%\Vendor\Game\save.dat
%LOCALAPPDATA%\Vendor\Game\cache.dat
%USERPROFILE%\AppData\LocalLow\Vendor\Game\save.dat
%USERPROFILE%\Documents\Vendor\Game\config.ini
%USERPROFILE%\My Documents\Vendor\Game\config.ini
%USERPROFILE%\Saved Games\Vendor\Game\save.dat
C:\Users\Alice\Documents\Vendor\Game\config.ini
C:\Users\Alice\Saved Games\Vendor\Game\save.dat
```

## Hooked API families

### Shell folder APIs

- `SHGetFolderPathA/W`
- `SHGetKnownFolderPath`
- `SHGetSpecialFolderPathA/W`
- `SHGetPathFromIDListA/W`

Supported folder IDs include AppData, Local AppData, LocalLow AppData, Documents, and Saved Games where the Windows API exposes them. `SHGetKnownFolderPath` is Unicode-only by design.

### Environment APIs

- `GetEnvironmentVariableA/W` for `APPDATA` and `LOCALAPPDATA`
- `ExpandEnvironmentStringsA/W`, with pre-expansion and post-expansion path rewrite for supported `%APPDATA%`, `%LOCALAPPDATA%`, and `%USERPROFILE%`-based paths

`USERPROFILE` itself is not globally replaced. Instead, file paths under `%USERPROFILE%\Documents`, `%USERPROFILE%\My Documents`, and `%USERPROFILE%\Saved Games` are rewritten when the full path is observed.

### File and directory APIs

- `CreateFileA/W`
- `GetFileAttributesA/W`
- `SetFileAttributesA/W`
- `CreateDirectoryA/W`
- `RemoveDirectoryA/W`
- `DeleteFileA/W`
- `CopyFileA/W`
- `MoveFileExA/W`
- `FindFirstFileA/W`
- `FindFirstFileExA/W`
- `PathFileExistsA/W`

Both ANSI (`A`) and Unicode (`W`) variants are handled where Windows exposes both variants.

## Prerequisites

- Rust nightly toolchain
- Windows MSVC target/toolchain

```sh
rustup toolchain install nightly
rustup default nightly
rustup target add i686-pc-windows-msvc
rustup component add rust-src --toolchain nightly-x86_64-pc-windows-msvc
```

## Build

### x64

```sh
cargo +nightly build --release
```

### x86 from an x64 machine

```sh
cargo +nightly build --release --target=i686-pc-windows-msvc -Zbuild-std
```

## Usage

Inject or load the generated DLL into the game or application you own or are authorized to modify. Run the target with the desired portable directory as its current working directory. Redirected data will appear under the portable `AppData`, `Documents`, and `Saved Games` folders in that directory.

## Notes

- This is a user-mode hook. It only affects APIs called inside the hooked process.
- It does not globally move Windows known folders.
- Apps using NT native APIs directly, custom syscalls, or already-open file handles may bypass these hooks.
- The hook creates parent directories for redirected file writes where practical.
- `Saved Games` has no legacy CSIDL equivalent, so modern programs usually reach it through `SHGetKnownFolderPath(FOLDERID_SavedGames)` or direct `%USERPROFILE%\Saved Games` paths.

## License

MIT. See `LICENSE`.
