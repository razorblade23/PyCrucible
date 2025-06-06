# PyCrucible
#### "Run Python Apps Instantly, No Setup Required."

A robust, cross-platform builder and launcher for Python apps using UV.

## Overview

This tool runs a Python application with a help of UV binary. It extracts your package (ZIP or directory), loads an optional configuration from `pycrucible.toml`, and uses `uv` to run your app in an ephemeral environment.

## Features

- **Cross-Platform**: 
    - [x] Windows support
    - [ ] macOS support
    - [x] Linux support
- **Configurable**: 
    - [x] Use `pycrucible.toml` to customize project details
    - [ ] Use standard `requirements.txt` manifest
    - [x] Use UV initialized `pyproject.toml` manifest
    - [x] Load the project as a directory
    - [ ] Load the project as .zip archive
- **Hooks**:
    - [x] Run pre‑ and post‑execution scripts
- **Cleanup**: 
    - [ ] Optionally remove files after execution (reccomended for temporary directories)
- **Tests**:
    - [ ] Unit tests cover configuration, extraction, and hook execution
- **Source Update**:
    - [ ] Initiate an update of source code pulling from GitHub


## Building from source

1. Ensure you have [Rust](https://www.rust-lang.org/) installed.

2. Clone the repository
 - `git clone https://github.com/razorblade23/PyCrucible`

3. Change directory to be inside of a project
 - `cd PyCrucible`

4. Build the binary
 - `cargo build --release`

#### The resulting binary will be in `target/release/pycrucible`.


## Downloading pre-made binary
You can download pre-made binaries for your system from [Releases](https://github.com/razorblade23/PyCrucible/releases/latest) page


## Usage
Your package should include at least:
- A directory with your Python application (with an entry point (default: __main__.py))
- A `uv` initialized project with `pyproject.toml` file
- (optional) `pycrucible.toml` file with (in your project directory) for custom include/exclude, uv commands, enviroment variables and pre/post hooks
    - EXAMPLE: Example can be found in root directory under the `pycrucible.toml.example` name
    - WARNING: Only include/exclude implemented for now !


```
$ pycrucible --help
Tool to generate python executable by melding UV and python source code in crucible of one binary

Usage: pycrucible [OPTIONS]

Options:
  -e, --embed <EMBED>
          Directory containing Python project to embed. When specified, creates a new binary with the embedded project
  -o, --output <OUTPUT>
          Output path for the new binary when using --embed
      --uv-path <UV_PATH>
          Path to `uv` executable. If not found, it will be downloaded automatically [default: `.`]
      --extract-to-temp
          Extract Python project to a temporary directory when running
      --debug
          Enable debug output
      --delete-after-run <DELETE_AFTER_RUN>
          Delete extracted files after running. Note: requires re-downloading dependencies on each run [default: false]
  -h, --help
          Print help
  -V, --version
          Print version
```

### Usage examples (Linux)
You can copy built/downloaded binary to your project folder and just run:

`./pycrucible -e . -o ./launcher`

This will embed your project into another binary (that we called "launcher")

You can run your project from binary by running

`./launcher`

### Usage examples (Windows)
You can copy built/downloaded binary to your project folder and just run:

`pycrucible.exe -e . -o ./launcher`

This will embed your project into another binary (that we called "launcher")

You can run your project from binary by running

`launcher.exe`

Now you can copy that "launcher" on practicly any machine with the same architecture.
Machine only needs internet connection in order to download the dependacies.
This proccess is extremely fast (but reliant on internet connection)


## Thanks to
The idea is inspired by [Packaged](https://packaged.live/)
Thanks to all the briliant developers at [UV](https://astral.sh/blog/uv)
