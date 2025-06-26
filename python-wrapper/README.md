# PyCrucible
#### "Run Python Apps Instantly, No Setup Required."

A robust, cross-platform builder and launcher for Python apps using UV.

## Overview

This tool runs a Python application with a help of UV binary. It extracts your package (ZIP or directory), loads an optional configuration from `pycrucible.toml`, and uses `uv` to run your app in an ephemeral environment.

## Installation
You can install pycrucible with the usual:
```bash
pip install pycrucible
```


## PyCrucible configuration
A directory with a single `.py` file is all you need to start.
There are however multiple configuration options to suit your specific needs.

##### Note - when using any configuration, only `entrypoint` is required. Other options are optional.

Configuration can be set in two ways:
- `pycrucible.toml`
- `pyproject.toml`

Both of these files have exact same configuration options. You can find example file for `pycrucible.toml` [here](https://raw.githubusercontent.com/razorblade23/PyCrucible/refs/heads/main/pycrucible.toml.example)

### Diffrence between configuration options
##### Note - In both `pycrucible.toml` and `pyproject.toml` directive `entrypoint` can also be replaced by just `entry`.


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

#### Default configuration
```rust
ProjectConfig {
        package: PackageConfig {
            entrypoint: "main.py".into(),
            patterns: FilePatterns {
                include: vec!["**/*.py".to_string()],
                exclude: vec![
                    ".venv/**/*".to_string(),
                    "**/__pycache__/**".to_string(),
                    ".git/**/*".to_string(),
                    "**/*.pyc".to_string(),
                    "**/*.pyo".to_string(),
                    "**/*.pyd".to_string(),
                ],
            },
        },
        source: None,
        uv: None,
        env: None,
        hooks: None,
    }
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


## Prepare your python project
Your project should include at least:
- A directory with your Python application (with an entry point (default: __main__.py))
- Some kind of manifest file declaring dependacies and/or configuration
- (optional) configuration file or section
    - only `entrypoint` is required if using this configuration file, other options are optional
    - if this file is not present, it will be created with default values.

## Usage
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

`pycrucible.exe -e . -o ./launcher.exe`

This will embed your project into another binary (that we called "launcher.exe")

You can run your project from binary by running

`launcher.exe`

Now you can copy that "launcher" on practicly any machine with the same architecture.
Machine only needs internet connection in order to download the dependacies.
This proccess is extremely fast (but reliant on internet connection)