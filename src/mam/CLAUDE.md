# src/mam

Pure MyAnonamouse HTTP client layer. No MCP types, no rmcp macros, no schemars. Everything here is about talking to the MAM API and working with its data.

## Modules

| File | Purpose |
|---|---|
| `mod.rs` | HTTP client construction, `get_ip_info`, `enrich_error`, module re-exports |
| `lookup.rs` | Static genre/language tables and pure mapping functions: `lookup_genres`, `map_languages`, `parse_sort`, `normalize_lookup` |
| `api.rs` | Async free functions that make HTTP requests to MAM and return `Result<String, String>` with raw JSON |

## Dependency order

```
mod.rs  ←  api.rs
        ←  lookup.rs   (no internal deps)
```

`lookup.rs` is pure (no I/O). `api.rs` is the only module that calls the network.

## Response format

All API functions return raw JSON strings from MAM — no reformatting or deserialization into typed structs. Error-field inspection (e.g. `{"error":"..."}`, `{"Success":false}`) uses `serde_json::Value` minimally before passing the body through.

## Adding a new MAM API endpoint

1. Add a free function to `api.rs` — signature: `pub(crate) async fn foo(client: &reqwest::Client, ...) -> Result<String, String>`.
2. Check HTTP status with `enrich_error`. For endpoints with body-level error fields, parse with `serde_json::Value` to check and return `Err` before returning `Ok(text)`.
3. Wire it up as a thin wrapper tool in `src/tools/mod.rs`.

## Visibility

All items are `pub(crate)` — nothing here is part of the external binary API. The only public items are in `mod.rs`: `BASE_URL`, `build_client`, `get_ip_info`, `enrich_error` — used by `main.rs` for startup and `--test-connection`.
