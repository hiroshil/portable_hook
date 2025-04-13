# Portable Visual Novel Galge Hook

This project is designed to hook into a visual novel galge game running in a download edition, allowing it to read and write data to the current directory instead of the default `C:` hard disk. The project leverages the `retour` and `windows-rs` crates to achieve this functionality. Currently, the project hooks into the `SHGetFolderPath` and `SHGetPathFromIDList` functions to redirect file operations. Future implementations may include reading registry keys and other system functions.

## Features

- **File Redirection**: Redirects file operations from the default `C:` drive to the current directory.
- **Hooked Functions**:
  - `SHGetFolderPath`
  - `SHGetPathFromIDList`
  - `SHGetSpecialFolderPath`
- **Future Plans**:
  - Implement registry key reading functionality.

## Prerequisites

Before building the project, ensure you have the following installed:

- Rust (nightly toolchain)
- `rustup` (Rust toolchain installer)

## Setup

1. **Install Rust Nightly Toolchain**:
   ```sh
   rustup toolchain install nightly
   rustup default nightly
   ```

2. **Add Required Targets**:
   ```sh
   rustup target add i686-pc-windows-msvc
   rustup component add rust-src --toolchain nightly-x86_64-pc-windows-msvc
   ```

## Building the Project

### Build for x64 Architecture

To build the project for x64 architecture, run the following command:

```sh
cargo +nightly build
```

### Build for x86 Architecture on x64 Machine

To build the project for x86 architecture on an x64 machine, use the following command:

```sh
cargo +nightly build --target=i686-pc-windows-msvc -Zbuild-std
```

## Usage

After building the project, you can use the generated DLL binary to hook into the visual novel galge game. Use any dependency injection or DLL injection method to link the provided DLL into the target executable. This will modify the executable's behavior according to the DLL's functionality. The DLL will redirect file operations to the current directory, allowing the game to read and write data from the customized location.

## Acknowledgments

This project was made possible thanks to the following libraries and their developers:

- **[retour](https://crates.io/crates/retour)**: A Rust library for function hooking.
- **[windows-rs](https://crates.io/crates/windows)**: A Rust library for interacting with Windows APIs.

Special thanks to the developers of these libraries for their hard work and contributions to the Rust ecosystem.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for more details.

## Contributing

Contributions are welcome! If you have any suggestions, bug reports, or feature requests, please open an issue or submit a pull request.

---

Happy hacking! 🚀
