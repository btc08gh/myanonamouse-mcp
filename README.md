# myanonamouse-mcp

An MCP (Model Context Protocol) server for [MyAnonamouse](https://www.myanonamouse.net) (MAM), a private tracker specialising in audiobooks and ebooks. Exposes MAM search and user tools to any MCP-compatible client such as Claude Desktop.

## Prerequisites

- A MyAnonamouse account
- Your `mam_id` session cookie (see [Authentication](#authentication) below)
- Rust toolchain (for building from source) — or download a pre-built binary from [Releases](../../releases)

## Building from source

```bash
git clone https://github.com/sandymac/myanonamouse-mcp.git
cd myanonamouse-mcp
cargo build --release
# Binary is at target/release/myanonamouse-mcp
```

## Authentication

MyAnonamouse authenticates via a session cookie named `mam_id`. There is no API key or OAuth flow — you copy the value directly from the site.

**How to obtain your `mam_id`:**

1. Log into MyAnonamouse in your browser
2. Go to **Preferences → Security**
3. Copy the value shown for `mam_id`

> The `mam_id` value is long and may contain an `=` sign at the end. Copy the full value as-is.

Supply the value via the `--mam-session` flag or the `MAM_SESSION` environment variable. The cookie expires periodically; if you get authentication errors, refresh it from Preferences → Security.

## Quick start

Verify your session cookie works:

```bash
myanonamouse-mcp --mam-session <your_mam_id> --test-connection
```

Run the server:

```bash
myanonamouse-mcp --mam-session <your_mam_id>
```

Or set the environment variable to avoid passing the cookie on every invocation:

```bash
export MAM_SESSION=<your_mam_id>
myanonamouse-mcp
```

## Claude Desktop setup

Add the server to your Claude Desktop configuration file.

**macOS:** `~/Library/Application Support/Claude/claude_desktop_config.json`
**Windows:** `%APPDATA%\Claude\claude_desktop_config.json`

```json
{
  "mcpServers": {
    "myanonamouse": {
      "command": "/path/to/myanonamouse-mcp",
      "args": ["--mam-session", "your_mam_id_here"]
    }
  }
}
```

Using an environment variable instead (recommended — keeps the cookie out of the process listing):

```json
{
  "mcpServers": {
    "myanonamouse": {
      "command": "/path/to/myanonamouse-mcp",
      "env": {
        "MAM_SESSION": "your_mam_id_here"
      }
    }
  }
}
```

Restart Claude Desktop after editing the config.

## Available tools

Run `myanonamouse-mcp --list-tools` to see all tools with their current default status.

| Tool | Default | Description |
|---|---|---|
| `search_audiobooks` | ✓ | Search audiobooks by title/author/narrator/series, genre, language |
| `search_ebooks` | ✓ | Search ebooks by title/author/series, genre, language |
| `search_music` | ✓ | Search musicology content (sheet music, tabs, instructional) by genre |
| `search_radio` | ✓ | Search radio recordings by genre |
| `get_torrent_details` | ✓ | Full details for one torrent by ID or hash |
| `get_ip_info` | ✓ | Check current IP and ASN as seen by MAM |
| `search_torrents` | — | Cross-category power search with numeric category/language IDs |
| `list_categories` | — | Full category and subcategory ID table for `search_torrents` |
| `get_user_data` | — | User profile — stats, ratio, seed bonus, notifications |
| `get_user_bonus_history` | — | Bonus point transaction history |
| `update_seedbox_ip` | — | Register current IP as dynamic seedbox IP |

### Tool groups

Tools are organised into groups to keep the default token footprint small. Enable only what you need.

| Group | Flag | Tools | When to use |
|---|---|---|---|
| default | _(on by default)_ | `search_audiobooks`, `search_ebooks`, `search_music`, `search_radio`, `get_torrent_details`, `get_ip_info` | Always available — read-only browsing and search |
| power | `--enable-power-tools` | `search_torrents`, `list_categories` | Cross-category search using raw numeric IDs; useful when the per-category tools aren't flexible enough |
| user | `--enable-user-tools` | `get_user_data`, `get_user_bonus_history` | Access to your account stats, ratio, and bonus point history |
| seedbox | `--enable-seedbox` | `update_seedbox_ip` | Registers your current IP as a dynamic seedbox IP on MAM |

```bash
# Enable the cross-category power search tools
myanonamouse-mcp --mam-session <id> --enable-power-tools

# Enable user profile tools
myanonamouse-mcp --mam-session <id> --enable-user-tools

# Enable the seedbox IP registration tool
myanonamouse-mcp --mam-session <id> --enable-seedbox

# Enable a specific tool by name
myanonamouse-mcp --mam-session <id> --enable-tool=get_user_data

# Disable a default tool (e.g. restrict to ebooks only)
myanonamouse-mcp --mam-session <id> --disable-tool=search_audiobooks --disable-tool=search_music --disable-tool=search_radio
```

`--disable-tool` always wins over group and individual enable flags, regardless of the order flags appear on the command line.

## HTTP transport

The server can also run as an HTTP server (useful for remote access or multi-user deployments):

```bash
myanonamouse-mcp --mam-session <id> --transport http --http-bind 0.0.0.0:8080 --api-token <secret>
```

The MCP endpoint is at `http://host:8080/mcp`. Clients must send `Authorization: Bearer <secret>` with every request. If `--api-token` is omitted, the endpoint is unauthenticated — only do this on a trusted network.

## Tips for best results

- Use `search_type: "active"` to exclude dead torrents (no seeders)
- Use `search_type: "fl"` to find freeleech torrents — these don't count against your download ratio (though you must still seed to site requirements)
- Use `sort: "most seeders"` to surface the best-seeded results first
- Use `sort: "newest"` to find recently added content
- Use `get_torrent_details` after a search to get the full description, ISBN, media info, and all metadata for a specific torrent
- Use `offset` and `limit` to page through large result sets
