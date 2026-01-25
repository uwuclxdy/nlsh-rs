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
→ df -h
[enter to execute, ctrl+c to cancel]
...
$ nlsh-rs "show disk usage"
→ df -h
[enter to execute, ctrl+c to cancel]
...
```

**flags:**
- `--help` - show help
- `api` - configure API provider
- `uninstall` - remove nlsh-rs

## How it works

1. translates natural language to shell commands using AI
2. asks for confirmation
3. command runs in parent shell and appears in history

## TODO

[ ] support for rotation of multiple API keys in case of rate limits
[ ] support for providing more context to the model

## Credits

for inspiration and prompt: https://github.com/junaid-mahmood/nlsh
