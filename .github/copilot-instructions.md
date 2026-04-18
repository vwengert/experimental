# Copilot Instructions

## Project Overview

This is a Rust workspace with three crates:

- `domain`: core models, DTO mappings, schema/config handling, persistence helpers
- `gui`: Slint-based desktop UI
- `cli`: minimal command-line entrypoint

Workspace root is `Cargo.toml` with members `cli`, `domain`, `gui`.

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
- Run at least:
  - `cargo test -p domain`
  - `cargo check`

## Naming Preferences

- Domain line model: `ItemLine { title, data }`
- DTO line model: `ItemLineDto { element, data }`
- Schema field spec type: `FieldSpec`

