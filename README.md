# PyLauncher
#### "Run Python Apps Instantly, No Setup Required."

A robust, cross-platform launcher for Python apps using UV.

## Overview

This tool runs a Python application with a help of UV binary. It extracts your package (ZIP or directory), loads an optional configuration from `installer.toml`, and uses `uv` to run your app in an ephemeral environment.

## Features

- **Cross-Platform**: Supports Windows, macOS, and Linux. (only linux for now)
- **Configurable**: Use `installer.toml` to set the app entry point, UV arguments, environment variables, and hooks. (not implemented)
- **Hooks**: Run pre‑ and post‑execution scripts. (not implemented)
- **Cleanup**: Optionally remove temporary files after execution. (not implemented)
- **Tests**: Unit tests cover configuration, extraction, and hook execution. (not implemented)

## Project Structure

```
pylauncher/
├── Cargo.toml
└── src/
    ├── main.rs
    └── launcher/
        ├── generator.rs
        ├── mod.rs
        └── template.rs
 
```


## Building

Ensure you have [Rust](https://www.rust-lang.org/) installed.

```bash
cargo build --release
```

The resulting binary will be in `target/release/pylauncher`.

## Usage

Package your Python app as a ZIP file or a directory. Your package should include at least:
- An app/ directory with your Python application (with an entry point like __main__.py)
- A `pyproject.toml` or `requirements.txt`(not implemented) file
- (Optional) An installer.toml for configuration
- (Optional) Hook scripts (e.g., in a scripts/ directory)

### Run the builder:
#### Usage
`./pylauncher <source_directory> <uv_binary> <output_launcher>`
`./pylauncher path/to/your_app path/to/uv_binary path/to/output_launcher`

This will produce a binary to your specified location and name.

You just need to run the launcher which will take care of downloading and installing `python` and all the dependacies listed
