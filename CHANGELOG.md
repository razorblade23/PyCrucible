# v0.4.x
## v0.4.0
- Downloading only `uv` binary, disregarding the rest of the archive.
    - `--uv-version` - Select the version of uv to download. [default: `0.9.21`]
- Added support for .whl embedding
    - new CLI options are provided for this mode only
        - `--extract-to-temp` - ["wheel" mode only] - sets configuration option with the same name
        - `--delete-after-run` - ["wheel" mode only] - sets configuration option with the same name
- Added `no-uv-embed` mode. This considerably reduces artifact size to ~2MB + payload size. Adds a couple of seconds to first run.
    - `--no-uv-embed` - disables embedding of `uv` during embedding, forcing for download on first run. 

# v0.3.x
## v0.3.5 - v0.3.9
### Fix
- Fixing issue #40 with `uv` and multiple `-quiet` arguments. It seems to be contained to some versions of uv only.

## v0.3.4
### Fix
- Fixing maturin `manylinux` build

## v0.3.3
### Fix
- `manylinux` compatible build with support for glibc 2.28 and multiple Python versions in PyPi builds

## v0.3.2
### Improvements
- Multiple `manylinux` compatible builds with support for glibc 2.28, 2.35 and 2.39 in PyPi builds

## v0.3.1
### Improvements
- Added possibility to pass runtime arguments for running embedded binary
- Replaced manual build proccess with `cargo-dist` for automated building and signing of binaries
- Deleted images from the root of project (duplicates) - thanks @EranYonai

## v0.3.0
### Improvements
- Split one single binary to two binaries (embedder and runner). This reduced the size of the overhead to ~2 MB (from previous 9+ MB). Embedder has runner already embedded so you only really need the embedder to produce final binaries.
- Made `-o` (`--output`) optional. Default value is `./launcher` (or `./launcher.exe` on windows).
- Introduced changelog (past entries will be missing)
- Implemented setting env variables from configuration
- `--extract-to-temp` and `--delete-after-run` is now set in configuration file rather then in CLI.
- Changes to footer structure. Now we have 1 byte that represents `extraction flag` so we can read it at runtime. 1 is temporary directory and 0 is non-temp directory
- If running from temporary directory, it automaticly deletes extracted files on end of the execution. This is to ensure no lingering files remain, filliing up disk space
- Images for the project are now contained in `assets` directory

# v0.2.x
## v0.2.9
### Improvements
- Automated PyPI packaging using `maturin`

## v0.2.7
### Improvements
- Smarter handling of `uv` binary. Now it executes `which` command (in multi-platform way) to find one on the PATH. If that does not exists, it looks for it next to binary. If that does not exist either, it will download the latest release from github (choosing the right platform by itself).

## v0.2.6
### Improvements
- `pyproject.toml` is now supported as a configuration option.

