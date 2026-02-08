# Envyr

Envyr automagically packages applications and runs them in sandboxed (or native) environments. It detects the language, installs dependencies, and executes projects — from local paths or git repos — without requiring any local setup.

> This project is in active development and may break.

```bash
# Run a Python script straight from a git repo — no clone, no pip install, nothing
envyr run --autogen git@github.com:tchaudhry91/python-sample-script.git -- https://blog.tux-sudo.com > my_blog.html
```

On first run, envyr fetches the repo, detects the language, builds a sandbox, installs dependencies, and runs the script. Subsequent runs reuse the cache and are near-instant.

## Installation

### From crates.io

```bash
cargo install envyr
```

### From source

```bash
git clone https://github.com/tchaudhry91/envyr.git
cd envyr
cargo install --path .
```

### Requirements

- **Docker executor** (default): Docker or Podman (auto-detected)
- **Native executor**: Python 3 (for Python projects), Node.js/npm (for Node projects)

## Quick Start

```bash
# Run a git repo with auto-detection (Docker, the default)
envyr run --autogen git@github.com:sivel/speedtest-cli.git

# Run a local project
envyr run --autogen ./my-project

# Run natively (no container) — great for piping
echo '{"name": "world"}' | envyr run --executor native --autogen ./my-script

# Pass arguments to the script
envyr run --autogen ./my-project -- arg1 arg2

# Set a timeout (seconds)
envyr run --timeout 30 --autogen ./my-project
```

## Executors

### Docker (default)

Runs the project inside a Docker (or Podman) container. This is the safest option — code runs fully sandboxed.

```bash
envyr run --autogen ./my-project

# With port mapping and volume mounts
envyr run --autogen ./my-project --port-map 8080:80 --fs-map /data:/app/data

# With network access and interactive mode
envyr run --autogen --interactive --network host ./my-project
```

### Native

Runs the project directly on the host. No containerization overhead — ideal for piping JSON through scripts or quick local runs.

```bash
# Pipe JSON through a Python script
echo '{"input": "data"}' | envyr run --executor native --autogen ./my-script

# Pass environment variables
envyr run --executor native --autogen ./my-project --env-map API_KEY=secret MY_VAR
```

For Python projects, envyr creates an isolated venv at `.envyr/venv` and installs dependencies from `requirements.txt`. For Node projects, it runs `npm install` if `node_modules` doesn't exist. Shell scripts run directly.

When running git-fetched code natively, envyr prints a warning to stderr since the code is not sandboxed.

## Language Support

### Python

- Detects `.py` files automatically
- Installs dependencies from `requirements.txt` (or generates one via [pipreqs](https://pypi.org/project/pipreqs))
- Finds the entrypoint via `if __name__ == "__main__"` or shebang; ties broken by priority
- Override with `-x <entrypoint>`

### Node.js

- Requires `package.json` (used for dependency installation and entrypoint detection via `main` field)
- Dependencies installed via `npm install`

### Shell

- Detected via shebang (`#!/bin/bash`, etc.)
- OS-level dependencies can be specified manually during generation

## Aliases

Save frequently used commands as aliases for quick re-use:

```bash
# Create an alias on successful run
envyr run --alias sample --autogen git@github.com:user/repo.git -- default-arg

# Run it later
envyr run sample

# Override args
envyr run sample -- different-arg

# List and manage aliases
envyr alias list
envyr alias delete sample
```

## Generating Metadata

The `generate` command creates `.envyr/meta.json` (and a Dockerfile) for a project. This is useful for project authors who want to commit the `.envyr` folder so others can run the project without `--autogen`:

```bash
envyr generate ./my-project

# With overrides
envyr generate ./my-project --entrypoint app.py --type python
```

When using `--autogen` with `run`, this step happens automatically.

## Override Options

If auto-detection doesn't work, override manually:

| Flag | Description |
|------|-------------|
| `-n, --name` | Project name |
| `-i, --interpreter` | Interpreter path (e.g. `/usr/bin/env python`) |
| `-x, --entrypoint` | Script entrypoint (e.g. `main.py`) |
| `-t, --type` | Language type: `python`, `node`, `shell`, `other` |

## Environment Variables

Pass environment variables to the executed script:

```bash
# Explicit key=value
envyr run --autogen ./my-project --env-map API_KEY=secret

# Passthrough from current shell
envyr run --autogen ./my-project --env-map HOME USER

# Works with both executors
envyr run --executor native --autogen ./my-project --env-map DB_URL=postgres://localhost/mydb
```

## Custom Root Directory

By default, envyr stores cached repos and aliases in `~/.envyr`. Override with `--root`:

```bash
envyr --root /tmp/my-envyr run --autogen ./my-project
```

## Planned Features

- More language support
- Bash script dependency detection

See the [issue tracker](https://github.com/tchaudhry91/envyr/issues) for more.

## License

[Apache-2.0](LICENSE)
