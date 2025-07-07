# PyCrucible
#### "Run Python Apps Instantly, No Setup Required."

A robust, cross-platform embedder for Python applications using `uv` for bootstraping.

## Overview

This tool runs a Python application with a help of UV binary. It extracts your application, loads an optional configuration from `pycrucible.toml` or `pyproject.toml`, and uses `uv` to run your app in an ephemeral environment.

### What this means?
What this means is that you get a single binary, which can then be transfered to other machines running the same platform.

Only internet connection required. No python installation needed. You run the executable and it takes care of everything.

## How to get `PyCrucible`
There are a couple of ways to get PyCrucible.

### Using `PyPI`
PyCrucible is published to `PyPI` for every release. All you need to do is:
```bash
pip install pycrucible
```

### Using `Github Releases`
You can download pre-made binaries for your system from [Github Releases](https://github.com/razorblade23/PyCrucible/releases/latest) page

### Downloading and building the source code
1. Ensure you have [Rust](https://www.rust-lang.org/) installed.

2. Clone the repository
```git clone https://github.com/razorblade23/PyCrucible```

3. Change directory to be inside of a project
```cd PyCrucible```

4. Build the binary
```cargo build --release```

> [!NOTE]
> The resulting binary will be in `target/release/pycrucible`.

## How to use `PyCrucible`
All you need for starting is a single `main.py` file with some code.

Run `pip install pycrucible`. This will download and install PyCrucible.

Change directory into your project and run
#### Linux and MacOS
```bash
pycrucible -e . -o -/launcher
```

#### Windows
```bash
pycrucible -e . -o -/launcher.exe
```

This will embed your project and produce a new binary which we called `launcher` (or `launcher.exe` on Windows).

This is now all you need to distribute your python project to other people.

No python required on their end. Just this single binary.

Running `pycrucible --help` reveals more options:
```bash
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


## How to configure `PyCrucible`
Configuration can be set in two ways:
- `pycrucible.toml`
- `pyproject.toml`

> [!NOTE]
> When both `pycrucible.toml` and `pyproject.toml` are discovered, configuration from `pycrucible.toml` will take effect.

> [!IMPORTANT]
> When using any configuration, only `entrypoint` is required. Other options are optional.

> [!TIP]
> In both `pycrucible.toml` and `pyproject.toml` directive `entrypoint` can also be replaced by just `entry`.

Both of these files have exact same configuration options. You can find example file for `pycrucible.toml` [here](https://raw.githubusercontent.com/razorblade23/PyCrucible/refs/heads/main/pycrucible.toml.example)

In `pycrucible.toml` you would define configuration like this:
```toml
entrypoint = "src/main.py"
# or
entry = "src/main.py"

[package.patterns]
include = [
    "**/*.py",
]
exclude = [
    "**/__pycache__/**",
]

[hooks]
pre_run = "some_script.py"
post_run = "some_other_script.py"
```

In `pyproject.toml` you would define configuration like this:
```toml
[tool.pycrucible]
entrypoint = "src/main.py"
# or
entry = "src/main.py"

[tool.pycrucible.patterns]
include = [
    "**/*.py",
]
exclude = [
    "**/__pycache__/**",
]

[tool.pycrucible.hooks]
pre_run = "some_script.py"
post_run = "some_other_script.py"
```

### Update your project from GitHub
In configuration file its possible to set your GitHub repository, so the resulting binary will always check for update before running the application.

In `pycrucible.toml` it would look like this:
```toml
[source]
repository = "https://github.com/username/repo"
branch = "main"
update_strategy = "pull"
```


In `pyproject.toml` it would look like this-
```toml
[tool.pycrucible.source]
repository = "https://github.com/username/repo"
branch = "main"
update_strategy = "pull"
```

#### Default configuration
```python
entrypoint = "main.py"

patterns.include = [
    "**/*.py",
]
patterns.exclude = [
    ".venv/**/*",
    "**/__pycache__/**",
    ".git/**/*",
    "**/*.pyc",
    "**/*.pyo",
    "**/*.pyd"
]

source = None
uv = None
env = None
hooks = None
```
If any of these configuration options is not used, it will be replaced with default value.
#### NOTE - `entrypoint` directive is required when using any configuration options.

## Features
- **Cross-Platform**: 
    - [x] Windows support
    - [x] macOS support (testing)
    - [x] Linux support
- **Configurable**: 
    - [ ] Use `pycrucible.toml` or `pyproject.toml` to customize embedding details
        - [x] entrypoint
        - [x] include/exlude files
        - [ ] arguments to `uv`
        - [ ] env variables
        - [x] pre and post run hooks (python scripts)
        - [ ] offline mode
        - [ ] extract to temporary directory
        - [ ] remove extracted files after running
    - [x] Support for multiple ways of defining requirements
        - [x] `uv` initialized `pyproject.toml`
        - [x] `requirements.txt`
        - [x] `pylock.toml`
        - [x] `setup.py`
        - [x] `setup.cfg`
    - [x] Load the project as a directory
    - [ ] Load the project as .zip archive
- **Tests**:
    - [x] Unit tests cover configuration, extraction, and hook execution
- **Source Update**:
    - [x] Initiate an update of source code pulling from GitHub


## Thanks to
The idea is inspired by [Packaged](https://packaged.live/).

Thanks to all the briliant developers at `Astral` - they did awesome job with [uv](https://astral.sh/blog/uv)
