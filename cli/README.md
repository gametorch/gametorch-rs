# gametorch CLI

The command-line interface for [GameTorch](https://gametorch.app), wrapping the
official [Rust SDK](https://crates.io/crates/gametorch). It exposes **every**
public API operation: catalogs, projects, sprites, sounds, animations, exports,
saved animations, labels, art styles, usage, API keys, account and health.

## Install

```sh
cargo install gametorch-cli
```

This installs a binary named `gametorch`.

## Getting started

```sh
export GAMETORCH_API_KEY=gt2_...          # create one at https://gametorch.app
# optional: export GAMETORCH_BASE_URL=http://localhost:8300/api

gametorch health
gametorch catalog sprite
gametorch project create --name "My Game"
gametorch sprite generate --project my-game \
  --prompt "a red fox, side view" --mode single \
  --image-model openai/gpt-image-2.5-flare --wait
```

Every command supports `--json` for machine-readable output, and content
downloads accept `--output <file>` (`-` for stdout). Run `gametorch --help` for
the full command tree.

## Links

- Repository: <https://github.com/gametorch/gametorch-rs>
- API reference: <https://gametorch.app/api/docs>
- Agent guide: <https://gametorch.app/llms.txt>
- Privacy: <https://gametorch.app/privacy>
- Terms: <https://gametorch.app/terms>

## License

MIT.
