# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## About Envyr

Envyr is a Rust CLI tool that automagically packages applications and runs them in sandboxed environments. It detects language types (Python, Node.js, Shell scripts), installs dependencies, and executes projects in Docker containers without requiring local setup.

## Key Commands

### Development
- `cargo build` - Build the project
- `cargo build --release` - Build optimized release version
- `cargo check` - Check code for errors without building
- `cargo run -- <args>` - Run envyr with arguments
- `cargo test` - Run all tests (unit and integration)
- `cargo test --bin envyr` - Run unit tests only
- `cargo test --test integration_tests` - Run integration tests only

### Cross-platform builds (via Makefile)
- `make build-linux` - Build for both x86_64 and aarch64 Linux targets
- `make build-linux-x86` - Build for x86_64-unknown-linux-musl
- `make build-linux-aarch64` - Build for aarch64-unknown-linux-musl
- `make build-darwin-x86` - Build for x86_64-apple-darwin
- `make build-darwin-aarch64` - Build for aarch64-apple-darwin

### Testing Envyr functionality
- `cargo run -- run --autogen <git-repo-url>` - Test running a repository with auto-detection
- `cargo run -- generate <project-path>` - Generate metadata for a project
- `cargo run -- alias list` - List stored aliases
- `cargo run -- run --timeout <seconds> --autogen <project>` - Run with execution timeout

## Architecture Overview

### Core Components

1. **CLI Interface** (`src/main.rs`): Built with clap, handles three main commands:
   - `generate`: Creates `.envyr/meta.json` metadata files
   - `run`: Executes projects in sandboxed environments
   - `alias`: Manages command aliases for frequently used projects

2. **Package Detection** (`src/envyr/package.rs`): 
   - Analyzes project directories to detect language type (Python, Node.js, Shell, Other)
   - Identifies entrypoints, dependencies, and interpreters
   - Supports override options for manual configuration

3. **Fetchers** (`src/envyr/adapters/`):
   - `git.rs`: Handles git repository cloning and caching
   - `fetcher.rs`: Abstracts different source types (local paths, git repos)

4. **Execution Engines**:
   - `docker.rs`: Docker container-based execution (primary implementation)
   - `meta.rs`: Metadata generation and management
   - Future: Nix and native execution (marked as todo)

5. **Template System** (`src/envyr/templates.rs`): Generates Dockerfiles and configurations

### Data Flow
1. Source fetching (git clone or local path resolution)
2. Project analysis and metadata generation
3. Sandbox creation (Docker image build)
4. Execution with proper volume mounts and environment setup

### Configuration
- User data stored in `~/.envyr/`
- Project metadata in `.envyr/meta.json` within each analyzed project
- Alias configurations for frequently used commands

## Language Support

Currently implemented:
- **Python**: Detects `.py` files, `requirements.txt`, uses pipreqs for auto-generation
- **Node.js**: Requires `package.json`, uses npm for dependency management  
- **Shell**: Detects shebang, manual dependency specification needed

## Key Files to Understand

- `src/main.rs:200-350` - Main command handling and configuration parsing
- `src/envyr/package.rs:44-47` - Project analysis entry point
- `src/envyr/docker.rs` - Docker execution implementation
- `src/envyr/meta.rs:22-40` - Metadata file generation

## Testing

The project now includes comprehensive test coverage:

### Unit Tests
- **utils.rs tests**: Core utility function testing (file detection, shebang parsing, etc.)
- **package.rs tests**: Project analysis, Pack building and serialization
- **docker.rs tests**: Docker command generation, Dockerfile templates, image naming
- **meta.rs tests**: Metadata generation, alias management

### Integration Tests
- **CLI command tests**: Help, version, generate, alias commands
- **Project generation tests**: Python, Node.js, Shell project detection and metadata creation
- **Error handling tests**: Invalid paths, missing dependencies

### Test Coverage Areas
- ✅ File type detection (Python, Node.js, Shell)
- ✅ Project analysis and entrypoint detection
- ✅ Docker image naming and command generation
- ✅ Environment variable and volume mapping
- ✅ Alias storage and retrieval
- ✅ CLI argument parsing and validation
- ✅ Template generation (Dockerfile, .dockerignore)
- ✅ Timeout functionality (Docker and Podman compatible)
- ✅ Real-time output preservation during timeout operations

### Running Tests
```bash
# Run all tests
cargo test

# Run only unit tests  
cargo test --bin envyr

# Run only integration tests
cargo test --test integration_tests

# Run specific test module
cargo test envyr::utils::tests

# Run with verbose output
cargo test -- --nocapture
```