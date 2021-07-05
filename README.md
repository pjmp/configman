# configman [![Crates.io](https://img.shields.io/crates/v/configman)](https://crates.io/crates/configman) ![License](https://img.shields.io/crates/l/configman)

> Heavily inspired by [stow](https://www.gnu.org/software/stow/).


# Installation

If you have rust toolchain installed, `configman` is available on [crates.io](https://crates.io/crates/configman), if you don't have rust toolchain installed, please install rust by going to the [official website](https://www.rust-lang.org/tools/install).

Run

```bash
cargo install configman
```

# Usage
```
USAGE:
    configman [FLAGS] [OPTIONS]

FLAGS:
        --dry-run        Do not do anything; just show what would happen.
    -h, --help           Prints help information
    -i, --interactive    Prompts user every time it tries to modify filesystem.
        --remove         Unlink the symlinks in destination path linked from the source directory.
    -v, --verbose
    -V, --version        Prints version information

OPTIONS:
    -d, --dest <destination>    Destination directory (default is $HOME dir)
    -s, --src <source>          Source directory (default is current dir)

```
