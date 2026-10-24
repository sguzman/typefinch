# GitHub SVG profile-card preview acceptance

The [README example](../assets/typefinch-card-demo.svg) is a **synthetic native
CI fixture**, not the profile or results of a real user. The file comes from
the actual Typefinch compact-profile SVG exporter, using the test profile
"Finch" and synthetic typing-history data, after native software-rendered QA
run 40048592389 produced `typefinch-profile.svg`.

The sample contains no JavaScript, remote URLs, scripts, links, fonts, raw
typing, paths or user credentials. It is checked into the standalone repo
only as a public demonstration. Changing colors, fonts, templates or export
format should update the synthetic sample deliberately, never by copying
any private profile.

Acceptance layers:

1. Rust unit tests reject unescaped personal markup, verify dimensions,
   color contrast, and ensure both Classic and Compact templates render with
   real visible text through resvg.
2. Native Linux graphical QA exports actual SVG and paired Markdown from
   the real UI; the XML parser validates markup, and local PNG rendering
   checks text visibility.
3. Public GitHub README must reference the **checked-in SVG**, not a
   private path, external image service or dynamically generated user data.
4. A separate browser check must confirm the SVG loads and renders at its
   expected dimensions **from GitHub's public URL**. GitHub HTML markup or
   successful HTTP alone does not certify an image rendered. Only after this
   check passes can T701's "Rendered validation on GitHub" be accepted.

Manual publication remains supported without an OAuth flow; the optional
T704 automatic profile-update acceptance gates remain unimplemented.
