# Optional GitHub profile-card publishing

Typefinch works completely offline and does not require GitHub. Exporting
SVG and Markdown remains a normal, local-only action. The separate
**Optional public GitHub publishing** panel is an explicit, user-initiated
convenience feature.

## Consent and content boundary

The publisher uses the exact card preview selected in Typefinch. Publishing
makes the profile's chosen display name, best benchmark WPM, best accuracy,
total saved session count, layout label and Typefinch artwork publicly
readable. It **never** includes individual keystrokes, raw typed text,
custom imported passages, local filesystem paths, private backup files,
keyboard hardware identifiers or GitHub credentials.

The UI requires (1) a valid `owner/repository` entry on GitHub.com,
(2) a fresh public-data consent checkbox, and (3) clicking **Publish public
card using gh**. Editing the destination or changing the card resets the
consent. There are no automatic updates, cron jobs, background monitors,
default destinations, account discovery or analytics uploads. One request
at a time runs on a worker thread so network latency cannot block typing.

The only GitHub file this feature can create or update is the fixed
`typefinch-profile.svg` in the named repository root. It never changes
`README.md`, profile settings or unrelated files. A user must place the
Markdown embed in a README separately if desired. The repository URL
on confirmation is informational, not a claim that the account's profile
README has been updated.

## Credentials and target integrity

Typefinch does not accept, save, cache, generate, print or directly read
GitHub tokens. It invokes the existing, locally authenticated
[GitHub CLI](https://cli.github.com/manual/gh_auth_login) `gh api`
process through a fixed non-shell argv, with
`GH_PROMPT_DISABLED=1`. CLI authentication and credential storage are
managed **outside** Typefinch; users must choose to install and authenticate
the CLI themselves.

A bounded subprocess deadline prevents a hung network/authentication
helper from hanging the app indefinitely. Before any file lookup or write, the
publisher requires the selected repository's GitHub API metadata to confirm
both `permissions.push = true` and a **public** repository
(`private = false`, `visibility = public`). Private, internal and
indeterminate-visibility repositories are rejected; a successful upload to
a private repository would not fulfill the public-publishing promise.
If the destination already contains a file, the publisher requires
the GitHub file SHA and inspectable base64-encoded text identifying a
Typefinch SVG; foreign content, a directory, corrupt data or an
unavailable lookup always causes refusal. The remote SHA accompanies a
replacement so GitHub can reject concurrent changes. The app never forces
the ref, broadens the target, or deletes files. Offline/authentication
errors fail closed and do not silently queue a future upload.

The UI requires **both** a successful GitHub Contents API PUT receipt
for the fixed path and a fresh read-back of that same path. The read-back
must contain an inspectable Typefinch SVG whose decoded bytes match the exact
generated card. A filename-only receipt, an unreadable remote response, or
a different public card is **not** reported as successful publication.

If GitHub accepted the write but the subsequent verification fails because
of caching, network loss, a changed file or any other reason, the result is
**uncertain**, not a definite failed upload. The message asks the user to
inspect the repository before trying again. The extra read-back uses only
the already authorized fixed repository path and never updates another file.

## Verification and limitations

Unit tests use a fake GitHub API with no credentials and no network, covering
missing consent, invalid repository inputs, permission refusal, new and
existing cards, existing unrelated files, missing or corrupt remote data,
upload failures, mismatch response paths and SHA-bearing replacement.
They also test successful byte-exact read-back, unavailable verification,
a stale/different card and invalid post-upload metadata without any
live GitHub account, credentials or network requests.

Native CI validates app compilation and guarded UI lifecycle. The
public **synthetic** README example is separately tested via headless
Chromium at the exact GitHub commit URL; that is not proof that any
real user has published a private profile. A real, user-authorized
live GitHub account round trip remains separately unverified until a
controlled explicit acceptance test has occurred.

**Threat model:** intentional explicit publication is a disclosure, not a
backup. Users retain responsibility for permissions, organization policies
and whether their chosen profile name is appropriate for public display.
