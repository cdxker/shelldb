# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

GrepDB is a file indexing and searching service that stores text files in a hierarchical directory structure and provides grep functionality via HTTP API. Files are stored per dataset (UUID) and can be tagged for special directory placement (___private, ___user_auth).

## Commands

### Build & Run
```bash
# Build the project
cargo build

# Run the server (binds to 127.0.0.1:8080)
cargo run

# Run with custom config file
cargo run -- --config /path/to/config.yaml
```

### Testing
```bash
# Run basic test (index + search)
./basic_test.sh

# Run comprehensive test suite
./test_api.sh

# Check code for compilation errors
cargo check
```

## Architecture

### Core Design
- **Single-file implementation** (`src/main.rs`) - All functionality is intentionally kept in one file. Do NOT create additional files or modules.
- **Async grep execution** - Uses `web::block` to run blocking grep commands without blocking the async runtime
- **Directory-based storage** - Files stored at `${data_volume_dir}/${dataset_uuid}/${filename}`
- **Tag-based routing** - Files with "private" or "user_auth" tags get stored in `___private/` or `___user_auth/` subdirectories

### API Endpoints

#### POST /api/index
Stores a file in the database. Payload:
```json
{
  "dataset": "uuid-here",
  "filename": "file.txt",
  "file_payload": "content",
  "nested": ["private", "user_auth"]  // optional tags
}
```

#### POST /api/search
Searches files using grep. Payload:
```json
{
  "dataset": "uuid-here",
  "flags": "-r pattern",  // everything except 'grep' itself
  "folder_filter": ["___private", "___user_auth"]  // optional
}
```

### Configuration
Config loaded from YAML file (defaults to `./config.yaml`):
```yaml
data_volume_dir: data_volume_dir  # Base directory for all stored files
```

### Error Handling
The application uses panic-based error handling throughout. All file operations and command executions will panic on failure rather than propagating errors.

### Important Implementation Details
- Grep output strips the `data_volume_dir` prefix from paths containing "___" for cleaner display
- Multiple search paths are passed directly to grep as arguments when folder_filter is specified
- Files can be stored in multiple locations if multiple tags are provided