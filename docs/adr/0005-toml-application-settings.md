# Store application settings in TOML

Application settings live in `config.toml`; profiles, local connections, tags, and remembered window size remain in `config.json`. TOML makes hand-edited preferences and comments useful, while `toml_edit` patches only changed fields so application writes do not discard unrelated edits.

Legacy JSON settings migrate once when no TOML file exists: write the complete effective TOML settings first, then remove the JSON `settings` object and retain its window size as JSON `windowSize`. Missing TOML fields use `AppSettings` defaults; malformed values, unknown fields, and unsupported versions enter Recovery rather than being overwritten. Existing JSON export/import remains the user-facing backup format and adapts both storage files internally.
