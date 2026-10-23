# 🔐 Typefinch privacy contract

- **Local-first:** No login server, telemetry, analytics upload, advertisements, tracking endpoints, or required network connection.
- **Typing data:** Ordinary scored sessions persist only calculated aggregates, not raw text. The local process necessarily observes characters typed into its own test surface during a session, but does not log keys outside that focused surface.
- **Keyboard discovery:** Hyprland device metadata is queried read-only at startup and on explicit refresh. The application currently does not persist physical keyboard identifiers.
- **Profiles:** Locally generated UUID identity, nickname and preferences. Names are stored in settings JSON; SQLite records use UUIDs rather than names.
- **Exports:** An explicit SVG export includes the nickname, selected layout and summary scores; publishing it on GitHub makes those details public. No automatic publishing has been implemented.
- **QA:** `TYPEFINCH_QA_REPORT` exists solely for an explicitly opted-in, isolated synthetic GUI test. Its report contains numeric metrics and no raw typed text, keyboard addresses, credentials or actual user profile names. Ordinary runs do not set this variable or write that report.
- **Future:** Optional device-specific capture must be permission-gated, disclosed and off by default; online sync and GitHub updates likewise require explicit opt-in and separate security review.

The persistent directory is `$XDG_CONFIG_HOME/typefinch`, falling back to `~/.config/typefinch`. Keep it out of the public source repository.

- **Custom test text:** User-provided passages remain in process memory while the window is open. Their text is not written to profile settings, SQLite history, exports, or CI reports. Scored custom-mode sessions store only aggregate metrics and the mode identifier. Closing Typefinch discards the custom passage.

- **Raw speed/accuracy:** The additional stored raw metrics count attempts and mistakes, not the characters themselves. Corrected mistakes stay reflected in raw speed and raw accuracy without saving the original mistyped strings.
