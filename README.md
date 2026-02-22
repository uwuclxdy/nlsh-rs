# nlsh-rs - Natural Language Shell written in Rust

[![asciicast](https://asciinema.org/a/772400.svg)](https://asciinema.org/a/772400)

> Inspired by [nlsh](https://github.com/junaid-mahmood/nlsh)

## Requirements

1. [Rust](https://www.rust-lang.org/tools/install)

## Installation

from crates.io **(recommended)**:
```bash
cargo install nlsh-rs
```

latest commit:
```bash
curl -sSL https://raw.githubusercontent.com/uwuclxdy/nlsh-rs/main/install.sh | sh
```

## Setup

### Configure AI provider

```bash
nlsh-rs api
```

Select provider and enter credentials. Config is stored in `~/.config/nlsh-rs/config.toml`.

## Supported Providers

- **Gemini** - free tier available at [aistudio.google.com/apikey](https://aistudio.google.com/apikey)
- **Ollama** - local models
- **OpenAI-Compatible APIs** - chatgpt or compatible APIs (LMStudio, Groq, etc.)

> You can get free OpenAI compatible API access to some models at https://www.askcodi.com/ and https://openrouter.ai/models?q=free

## Usage

```bash
$ nlsh-rs
nlsh-rs> show disk usage
$ df -h
Run this? (Y/e/n)
[Y/Enter] to execute, [E] to explain, [Arrow Up] to edit, [N] to cancel
...
$ nlsh-rs show disk usage
$ df -h
Run this? (Y/e/n)
[Y/Enter] to execute, [E] to explain, [Arrow Up] to edit, [N] to cancel
...
```
Explain command:
```bash
$ df -h
✅ Displays free disk space of mounted filesystems in a human readable format.
Run this? (Y/n)
[Y/Enter] to execute, [Arrow Up] to edit, [N] to cancel
```

Edit command:
```bash
$ df -h --total▉
[Enter] to confirm, [Ctrl+C] to quit
```

**subcommands:**
- `--help` - show help
- `api` - configure API provider
- `uninstall` - remove nlsh-rs
- `prompt` - show/edit the prompt templates
- `explain` - explain a command

## How it works

1. translates natural language to shell commands using AI
2. asks for confirmation
3. command runs in parent shell and appears in history

## TODO

- [ ] support for rotation of multiple API keys in case of rate limits
- [ ] give more context to the model about the machine
- [ ] access to nlsh-rs's commands inside interactive mode with `/`.

## Credits

for inspiration and prompt: https://github.com/junaid-mahmood/nlsh
