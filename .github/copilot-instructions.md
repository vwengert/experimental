# Copilot Instructions
## Project Overview
This is a Rust workspace with three crates:
- `domain`: core models, DTO mappings, schema/config handling, persistence helpers
- `gui`: Slint-based desktop UI
- `cli`: minimal command-line entrypoint
Workspace root is `Cargo.toml` with members `cli`, `domain`, `gui`.
## Devcontainer Conventions
- Treat `.devcontainer/devcontainer.json` and `.devcontainer/Dockerfile` as the source of truth for containerized development.
- Keep `.devcontainer/README.md` aligned with the actual devcontainer setup (installed tools, extensions, and open/rebuild flow).
- When adding developer tooling, prefer reproducible setup in the devcontainer over one-off local-only steps.
## Workspace Path Conventions
- When a Dev Container is active, treat `/workspaces/experimental` as the canonical repository root.
- Prefer container paths for all reads, edits, and commands when both host and container paths are visible.
- Do not assume host IDE metadata paths (for example under `~/.config/JetBrains/...`) are the repo root.
- If a file is not found, verify path context first (container root vs host path) before proceeding.
## Architecture and Boundaries
- Keep domain logic in `domain`.
- Keep UI state and event wiring in `gui`.
- Keep CLI thin and delegate to `domain`.
- Do not move business logic into Slint files.
## Domain Conventions
### Modules
- Core models: `domain/src/models/model.rs`
- Schema/config models: `domain/src/models/schema.rs`
- Persistence helpers: `domain/src/utility/persistence.rs`
- DTOs: `domain/src/dto/`
### Config file
- Built-in config is embedded with `include_str!` from:
  - `domain/assets/lists.config.json`
- `lists.config.json` format:
  - top-level: `title`, `description`, `properties`
  - `properties.units`: unit groups
  - `properties.elements`: element definitions with ordered `fields`
### Lists JSON contract
The persisted lists format is wrapped DTO format only (legacy bare format is intentionally not supported):
- `title`
- `description`
- `properties.lists`
Each line object uses:
- `element` (line type/name)
- `data` (array of key/value/unit objects)
Use DTO mappings in `domain/src/dto/lists.rs` to convert between file format and domain models.
## GUI / Slint Conventions
- Keep action dispatch flow centralized in `gui/src/app_state.rs`.
- Keep `ActionType` handling exhaustive and consistent with Slint action definitions.
- UI models should preserve schema field order from `ElementSchema.fields()`.
- Do not reintroduce legacy keys (`title` for line DTO key, `sets` for line data array).
## Code Change Guidelines
- Prefer small, targeted changes that match existing naming and structure.
- Keep serde field names and mapping behavior backward-compatible only where explicitly required.
- Add/update tests when changing DTO/schema/persistence contracts.
- Update documentation when behavior or setup changes:
  - keep `README.md` in sync for build/test/run or workflow changes
  - keep `.devcontainer/README.md` in sync for devcontainer/tooling changes
- Run at least:
  - `cargo test -p domain`
  - `cargo check`
## Naming Preferences
- Domain line model: `ItemLine { title, data }`
- DTO line model: `ItemLineDto { element, data }`
- Schema field spec type: `FieldSpec`
