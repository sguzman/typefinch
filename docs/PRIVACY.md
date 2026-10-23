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

- **JSON history export:** F7 (or the explicit export button) writes a versioned, aggregate-only file for the active profile to `exports/typefinch-history.json`. Unlike the dashboard's recent-session cache, it includes every saved session for that profile. The output includes its pseudonymous profile ID and never includes raw passages, typed strings, credentials, or hardware addresses. No automatic publishing takes place.

- **Dropped files:** Only a user-initiated local file drop in Custom mode is read. Typefinch rejects files above 64 KiB, invalid UTF-8, unsupported control characters and empty passages. Only the first 4,096 characters are retained in memory for practice; file paths and file contents are not stored in session history or remote services.

- **Adaptive missed-key drills:** Expected characters associated with mistakes are collected only in the running session's memory. Generated practice text is not persisted, published, or included in analytics history; the durable database still stores only aggregate counts.

- **Explicit full backup (F8):** The two private backup files include readable settings/profile nicknames and a consistent full SQLite snapshot. The files are unique, written into the local exports directory and never automatically uploaded. On Linux, new backup files are permission-restricted to their owner (0600). The application does not yet offer automated restore; retain both companion files for recovery.

- **Achievements:** The badge panel computes earned status entirely from locally stored aggregate session records, with no online accounts, tracking, additional per-key storage, or publication. Badges are recalculated from history rather than written into a new identity database.

- **Daily target:** Uses the device's local date and existing stored aggregate session timestamps. It does not create a separate activity log, communicate with a calendar service, or require any account. Today's totals are recomputed from the active profile's history.

- **Optional streak display:** Disabled by default; when enabled, derives qualifying consecutive calendar days from the same local aggregate session timestamps. No additional log is created and it can be hidden instantly.

- **PNG profile card:** Generated only on explicit F9/button action. It contains the same user-selected profile name and aggregate performance values as the SVG card; no raw typed passages or network requests are included.

- **Key latency:** Mean per-character latency is computed from ordinary foreground committed input only, held in session memory, and cleared when a new test starts. Corrections reset the interval clock; batch text input is excluded. The database retains only aggregate speed/accuracy, not per-key timestamps or the training map.

- **Keyboard path import:** The optional local file path is held only in the current UI process. It is never saved with preferences, uploaded or included in typing history. Paths to directories, devices and symbolic links are rejected. File reading is bounded to 65,537 bytes for strict 64-KiB rejection.

- **Profile-goal transfer:** A manual versioned JSON export contains only goal values, not profile names, identifiers, or typed text. Importing from an explicit local regular file applies only to the currently selected profile's goals. Transfer paths are transient UI state, not persisted. Export file permissions are owner-only on Linux.
