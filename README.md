# ballast

A live, detailed view of your storage: disks, partitions, I/O, ZFS, RAID, and SMART.

`ballast` shows disks, partitions, loopback devices, filesystem types, mount points, and I/O stats (read, write, IOPS, latency, queue depth, etc...).
It's built as a btop-style TUI with vim keybinds using [Ratatui](https://github.com/ratatui/ratatui).

Written in Rust. Single binary. No daemon, no database, no webserver.

## Status

**Working:**
- **Disk usage**: per-device usage, device name, used %, and used/total, via `statvfs`
- **I/O throughput**: per-device read, write, IOPS, latency (ms), queue depth, and utilization %
- **Partitions and mounts**: partition layout, filesystem type, and mount points per disk

**Planned, not yet implemented:**
- Unified SMART health table (temperature, wear level, power-on hours, reallocated sectors)
- ZFS pool topology tree (vdevs, member disks, health state)
- ZFS ARC cache stats (hit ratio, size)
- mdadm RAID array support

## Architecture

`ballast` is split into a workspace of crates:

- **`ballast-core`**: platform-agnostic logic; data models, polling orchestration, and the traits that platform crates implement
- **`ballast-platform-linux`**: Linux-specific data collection (statvfs, /sys, /proc, and eventually zpool/smartctl/mdadm integration)
- **`ballast-tui`**: the binary crate and entry point; renders the terminal UI and wires `ballast-core` together with the platform crate for the current target

The goal of this split is to keep platform-specific code isolated behind a common interface in `ballast-core`, so that FreeBSD and macOS support can be added as new
platform crates (e.g. `ballast-platform-bsd`, `ballast-platform-macos`) without touching core logic or the TUI. This is still a work in progress.

## Installation

### Cargo

```sh
```

### Nix


```sh
```

## From Source

```sh
cargo install --path .
```

This will build and install `ballast` in your `~/.cargo/bin`. Make sure that `~/.cargo/bin` is in your `$PATH` variable.

## Platform support

| Platform | Status |
|---|---|
| Linux | Tested, primary target |
| FreeBSD | Not yet supported, planned |
| macOS | Not yet supported, planned |

## Contributing

Ideas, bug reports, and pull requests are welcome.

**Before opening an issue:** search existing issues and discussions first; your question or bug may already be covered.

**Before opening a PR:** for anything beyond a small fix (typos, obvious bugs), open an issue first to discuss the approach.
This is especially true for items in the [Planned](#status) list and for new platform crates. ZFS topology, SMART parsing,
and platform abstraction all involve real design decisions, and it's better to align before writing the code.

Run `cargo fmt` and `cargo clippy` before submitting.

This project uses [Conventional Commits](https://www.conventionalcommits.org/).

**AI usage policy:** you're welcome to use AI tools while writing a contribution, but you must understand your own code.
If you can't explain what your change does and how it interacts with the rest of `ballast` without leaning on an AI tool, it isn't ready to submit.
Low-effort, AI-generated PRs that the submitter can't explain will be closed.

## License

`ballast` is licensed under the MIT License.

SPDX-License-Identifier: `MIT`. See [LICENSE](LICENSE) for the full text. By submitting a contribution, you agree it is licensed under the same terms.
