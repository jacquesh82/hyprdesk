## What this changes

One concern per pull request: describe it here, with screenshots when the
visible interface moves.

## Why

## Checklist

- [ ] `bash scripts/verify.sh` passes (fmt, Clippy with `-D warnings`,
      tests, build)
- [ ] Tests added or updated to cover the change
- [ ] Documentation updated (README, `docs/`) if the visible behaviour
      changed
- [ ] `locales/en.toml` completed if new user-facing strings were added —
      `HYPRDESK_I18N_TODO=/tmp/todo.toml cargo test --locked catalogue_complet`
      lists the missing entries
