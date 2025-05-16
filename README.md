# PyCrucible
#### "Run Python Apps Instantly, No Setup Required."

A robust, cross-platform builder and launcher for Python apps using UV.

## Overview

This tool runs a Python application with a help of UV binary. It extracts your package (ZIP or directory), loads an optional configuration from `pycrucible.toml`, and uses `uv` to run your app in an ephemeral environment.

## Features

- **Cross-Platform**: 
    - [x] Windows support
    - [ ] macOS support
    - [x] Linux support (tested on ubuntu)
- **Configurable**: 
    - [x] Use `pycrucible.toml` to customize project details
    - [ ] Use standard `requirements.txt` manifest
    - [x] Use UV initialized `pyproject.toml` manifest
    - [x] Load the project as a directory
    - [ ] Load the project as .zip archive
- **Hooks**:
    - [ ] Run pre‑ and post‑execution scripts
- **Cleanup**: 
    - [ ] Optionally remove temporary files after execution
- **Tests**:
    - [ ] Unit tests cover configuration, extraction, and hook execution
- **Source Update**:
    - [ ] Initiate an update of source code pulling from GitHub


## Building

Ensure you have [Rust](https://www.rust-lang.org/) installed.

```bash
cargo build --release
```

The resulting binary will be in `target/release/pycrucible`.

## Usage

Package your Python app as a ZIP file or a directory. Your package should include at least:
- A directory with your Python application (with an entry point (default: __main__.py))
- A `pyproject.toml` file and project initialized with `UV`
- (optional) `pycrucible.toml` file with (in your project directory) for custom include/exclude, uv commands, enviroment variables and pre/post hooks
    - EXAMPLE: Example can be found in root directory under the `pycrucible.toml.example` name
    - WARNING: Only include/exclude implemented for now !

### Run the builder:
#### Usage
```
$ pycrucible --help
Tool to generate python executable by melding UV and python source code in crusable of one binary

Usage: pycrucible [OPTIONS] <SOURCE_DIR>

Arguments:
  <SOURCE_DIR>  

Options:
  -B, --uv-path <UV_PATH>          [default: ./uv]
  -o, --output-path <OUTPUT_PATH>  [default: ./pycrucible-launcher]
  -t, --target                     [default: None]
  -h, --help                       Print help
  -V, --version                    Print version
```

This will produce a binary to your specified location and name.

You just need to run the launcher which will take care of downloading and installing `python` and all the dependacies listed.

### Cross-compilation
It is possible to cross-compile both builder and launcher for a number of operating systems.
This is achived using [cross](https://github.com/cross-rs/cross) crate.

`Cross` uses containerization engine (docker or podman) to generate executable for non-native systems.

#### Install `cross` crate
```bash
cargo install cross
```
This will install Cross to `$HOME/.cargo/bin`

#### Install docker or podman
- [Docker](https://docs.docker.com/engine/install/)
- [Podman](https://podman.io/docs/installation)

With these steps completed you are ready for cross-compilation

#### Cross-compile builder
```bash
cross build --release -target <TARGET_PLATFORM>
```

#### Cross-compile launcher
```bash
pycrucible pycrucible -t <TARGET_PLATFORM>  <SOURCE_DIR>
```

#### Supported platforms for cross compilation
- `x86_64-unknown-linux-gnu`  (Linux x64)
- `x86_64-pc-windows-gnu`    (Windows x64)

##### Aditional platforms can be added but are not supported by base `cross` crate. Requires additional configuration


### This project uses `uv`
- This project uses `uv` as python dependancy manager and is required as part of compilation step.
- The application will download `uv` (for specified target) if the binary is not found and not specified as a flag.


## Thanks to
The idea is inspired by [Packaged](https://packaged.live/)

Thanks to all the briliant developers at [UV](https://astral.sh/blog/uv)
