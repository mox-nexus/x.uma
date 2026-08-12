# CLI Applications

Building command-line tools. Best-in-class choices.

> **x.uma override.** This file is general Rust guidance. Where it recommends an
> error-handling crate (`thiserror`, `anyhow`, `color-eyre`, `miette`, `snafu`),
> x.uma does not use one. Errors are hand-written enums with manual `Display` and
> `Error` impls (`rumi/core/src/lib.rs:216-331`). Adding a utility crate
> dependency is a listed anti-pattern in CLAUDE.md. See SKILL.md → x.uma
> Overrides before acting on anything below.

## Argument Parsing

```
Need subcommands?
├── NO → lexopt (52KB, 2s compile)
└── YES → clap derive (600KB, 15s compile)
```

**The ripgrep revelation**: BurntSushi migrated from clap to lexopt in 2024. "Thoroughly solves the problem minimalistically."

| Tool | Uses | Why |
|------|------|-----|
| ripgrep | lexopt | Minimal deps, fast compile |
| fd, bat, delta | clap derive | Need subcommands |

## Stack

| Need | Crate |
|------|-------|
| Args (simple) | **lexopt** |
| Args (complex) | **clap** with derive |
| Colors | **owo-colors** (zero alloc) |
| Progress bars | **indicatif** |
| Interactive prompts | **dialoguer** |
| Errors | **color-eyre** |
| Config files | **confy** (simple) or directories + manual merge |

## Error Handling

**Use color-eyre** for CLIs - colored output, backtraces, suggestions.

```rust
use color_eyre::eyre::Result;

fn main() -> Result<()> {
    color_eyre::install()?;
    // Your code - ? just works with nice output
}
```

## Config Priority

CLI args > env vars > project config > user config > defaults

```rust
// Pattern: merge layers
let config = Config::default()
    .merge_file(dirs::config_dir())?
    .merge_env()?
    .merge_args(args)?;
```

## Exit Codes

Use BSD sysexits (exitcode crate):
- 0: Success
- 1: Generic failure
- 2: Usage error
- 64+: Specific errors (NOINPUT, IOERR)

## Shell Completions

With clap:
```rust
// build.rs
use clap_complete::{generate_to, shells::Bash};
generate_to(Bash, &mut cmd, "myapp", "completions/")?;
```

Ship in packages, install to `/usr/share/bash-completion/`.

## Critical Gotchas

| Issue | Solution |
|-------|----------|
| Slow compile from clap | Use lexopt if no subcommands |
| Colors break pipes | Check `stdout().is_terminal()` |
| Progress bar flicker | Use `indicatif::ProgressBar::with_draw_target` |
| Config file location | Use `directories` crate for XDG paths |
