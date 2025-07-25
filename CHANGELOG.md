# v0.3.x
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

