# Security policy

## Supported versions

| Version | Security fixes |
|---|---|
| Latest minor release (e.g. 0.1.x) | Yes |
| Previous minor release | On request, for deployments that cannot move yet |
| Older releases | No |

Please make sure you can reproduce the issue on the latest release before
reporting it. Organisations that need a longer support window for a given
version can ask for it through the channels below; it is agreed case by case
and written down in the advisory or the release notes.

## Reporting a vulnerability

Use GitHub's private vulnerability reporting:

<https://github.com/jacquesh82/hyprdesk/security/advisories/new>

Please do **not** open a public issue for a security problem. Reports sent
through the form above reach the maintainers privately; describe the
affected component, the steps to reproduce, and the impact. We will reply
there and coordinate a fix and its disclosure with you.

Fallback channel, if the form is unavailable to you: contact the maintainer
through their GitHub profile (<https://github.com/jacquesh82>) and ask for a
private channel — do not include vulnerability details in that first
message.

## Scope

In scope:

- the **hyprdesk daemon** (session supervision, Lua configuration, the
  client socket, D-Bus services: notifications, tray, polkit, MPRIS,
  logind);
- the **lock screen** and its PAM binding ([src/lock/pam.rs](src/lock/pam.rs)),
  including the fingerprint path through fprintd;
- the **greeter** (login screen): the greetd IPC protocol, the screen shown
  before login, and its system setup steps run through `pkexec`
  (`hyprdesk greeter setup`);
- the **installation and update scripts** ([scripts/](scripts/)), including
  what they write to `hyprland.lua`, systemd units, `/etc/greetd` and
  `/etc/pam.d`, and the packages built by [scripts/package.sh](scripts/package.sh).

Out of scope — report these upstream:

- Hyprland itself, and the Wayland libraries;
- greetd, cage, fprintd, PAM distributions, systemd;
- third-party components hyprdesk integrates with (LYRA service, the myMFA
  Linux client, gnome-keyring, cliphist, hypridle…), unless hyprdesk is
  what exposes the flaw.

## Handling

Indicative timelines, counted from the report:

| Step | Target |
|---|---|
| Acknowledgement | within 5 business days |
| First assessment (severity, affected versions) | within 10 business days |
| Fix released — critical | within 30 days |
| Fix released — high | within 60 days |
| Fix released — medium / low | next planned release |

- The exchange stays in the private advisory until a fix is released.
- Every fixed vulnerability is published as a **GitHub Security Advisory**
  (GHSA, with a CVE requested when it applies), linked from the release
  notes, and credited to the reporter unless they prefer otherwise.
- Fixes land in a patch release of the supported versions listed above.

## Verifying a release

Each release published from a `v*` tag carries:

- `SHA256SUMS` — checksums of the `.deb`, `.rpm`, `.pkg.tar.zst` packages
  and of the SBOM;
- `SHA256SUMS.sigstore.json` — a keyless [cosign](https://docs.sigstore.dev/)
  signature of `SHA256SUMS`, bound to this repository's `ci.yml` workflow
  and to the tag;
- `hyprdesk.cdx.json` — a CycloneDX SBOM of the Rust dependencies;
- a build provenance attestation (`gh attestation verify <package> --repo jacquesh82/hyprdesk`).

```sh
cosign verify-blob --bundle SHA256SUMS.sigstore.json \
  --certificate-identity "https://github.com/jacquesh82/hyprdesk/.github/workflows/ci.yml@refs/tags/vX.Y.Z" \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com SHA256SUMS
sha256sum -c --ignore-missing SHA256SUMS
```

`scripts/install.sh` does both before installing anything as root (the
signature when `cosign` is installed; set `HYPRDESK_REQUIRE_SIGNATURE=1` to
make it mandatory).

## Trust model

What hyprdesk assumes, and what it does not protect against:

- **Code that runs as you is trusted.** `hyprdesk.lua`, the plugins in
  `hyprdesk/widgets/` and `settings.lua` run with your rights. hyprdesk
  never runs code you did not accept:
  - an archive (`hyprdesk import`, the **Sharing** package, a theme) is
    checked without running its Lua, read in a sandbox (no files, no
    commands), and its `hyprdesk.lua` and plugins are only installed after an
    explicit consent (`--trust-code`); a theme alone carries no code; AI
    services declared by an archive are ignored;
  - a third-party plugin goes into quarantine first and is installed only
    after its review card (source, commit, SHA-256, start of the code) has
    been accepted; git sources must be `https://` or ssh.
- **The myMFA vault, once open, answers any process of the session.** The
  myMFA Linux client exposes `today.mindlog.MyMfa1` on the session bus;
  hyprdesk calls it (passwords, codes, content) without a token, like any
  other client would. Checking the caller (executable, confirmation for a
  reveal requested by another process, access log) belongs to the myMFA
  daemon. Until it does, treat malware running as you as able to read the
  open vault, and lock the vault when you leave the machine.
- **Root only runs root-owned binaries.** Scripts and `pkexec` steps run
  `/usr/bin/hyprdesk` (package) or a root-owned `/usr/local/bin/hyprdesk`,
  never a binary from `~/.local/bin`.
- **Data sent to AI services** is sent on a gesture: a failed update opens
  a window with **Ask LYRA** instead of reaching LYRA on its own; SmartUI's
  automatic icons are off until enabled; plain `http://` endpoints are only
  accepted towards `localhost`.

### Machine policy

An administrator can pin some settings in `/etc/hyprdesk/policy.toml`
(owned by root, not writable by group or others — otherwise it is ignored);
neither the Lua configuration, the Settings nor an imported archive can
override it:

```toml
fingerprint = false            # no fingerprint unlock
idle_lock_max = 600            # idle lock enforced, at most 600 s
third_party_plugins = false    # no third-party plugin installation
index_removable = false        # removable media are not indexed
clipboard_history = false      # no clipboard history
```
