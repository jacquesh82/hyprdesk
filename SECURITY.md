# Security policy

## Supported versions

Only the **latest release** of hyprdesk receives security fixes. Please make
sure you can reproduce the issue on the latest version before reporting it.

## Reporting a vulnerability

Use GitHub's private vulnerability reporting:

<https://github.com/jacquesh82/hyprdesk/security/advisories/new>

Please do **not** open a public issue for a security problem. Reports sent
through the form above reach the maintainers privately; describe the
affected component, the steps to reproduce, and the impact. We will reply
there and coordinate a fix and its disclosure with you.

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

- A report is acknowledged through the private advisory, and the exchange
  stays there until a fix is released.
- Fixes land in a patch release, credited to the reporter unless they prefer
  otherwise.
