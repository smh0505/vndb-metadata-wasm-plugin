# vndb-metadata-wasm-plugin

A `MetadataProviderPlugin` for [Concourse](https://github.com/smh0505/Concourse) implemented as
a WASM component. Fetches description/release date/cover art from
[VNDB](https://vndb.org/) (the Visual Novel Database) by title search - covers visual novels, a
genre this app's other metadata providers (IGDB, RAWG, TheGamesDB, all modern/PC-console
leaning) don't meaningfully reach.

This is a real, separate repo on purpose - same reasoning as `steam-source-wasm-plugin`: a
plugin whose source lives inside the host app's own repo doesn't genuinely exercise the
"install arbitrary third-party code" model the WASM plugin system is for.

Uses VNDB's own official [Kana API v2](https://api.vndb.org/kana) - a real, documented, public
REST-ish JSON API (`POST /vn` with a `filters`/`fields` JSON body). No API key required for
this level of usage, unlike TheGamesDB/IGDB/RAWG's plugins - nothing to configure in Settings.

**No genres** - deliberately, not an oversight. VNDB's tag system is a large, freeform,
community-curated vocabulary (plot elements, structure, content warnings, and genre-ish tags all
mixed together, easily dozens per title), not a small controlled genre list like the other
providers return. Same "skip a field that doesn't cleanly fit" precedent
`sgdb-metadata-wasm-plugin` already sets for its own missing fields.

VNDB's own `search` filter is fuzzy, not exact - same convention `rawg-metadata-wasm-plugin`/
`thegamesdb-metadata-wasm-plugin` already established: only listings whose title is an exact
case-insensitive match to the query are ever surfaced as candidates. VNDB's `description` field
uses its own lightweight BBCode-style markup (`[b]`, `[i]`, `[url=..]`), converted to Markdown
here since this app's own description field is documented as Markdown-rendered.

## Permissions

Declares `httpScopes: ["api.vndb.org"]` (Milestone 13 URL allowlisting).

## Building

```sh
rustup target add wasm32-wasip1   # once
cargo install cargo-component     # once
cargo component build
```

Output: `target/wasm32-wasip1/debug/vndb_metadata_wasm_plugin.wasm`.

## Installing into a running Concourse

Either build locally (above) or grab the prebuilt `.wasm` + `plugin.json` from this repo's
[Releases](https://github.com/smh0505/vndb-metadata-wasm-plugin/releases) - CI
(`.github/workflows/publish.yml`) publishes a new release automatically whenever
`plugin.json`'s `version` is bumped on `main`. Concourse's Settings -> Metadata Provider tab ->
Add Plugin also accepts a Release's `plugin.json` URL directly (metadata-kind plugins install by
URL, same as source plugins) - the latest one always lives at:

```
https://github.com/smh0505/vndb-metadata-wasm-plugin/releases/latest/download/plugin.json
```

Copy the compiled `.wasm` and `plugin.json` into
`<app data dir>/wasm-plugins/metadata/vndb-wasm/` (Windows:
`%APPDATA%\com.bloppy.concourse\wasm-plugins\metadata\vndb-wasm\`). It'll show up in Settings'
Plugins panel under the Metadata Provider tab next time the app starts, as **VNDB**.

## Signing

Every release's `.wasm` is signed with a [Sigstore](https://www.sigstore.dev/) build-provenance
attestation (`actions/attest-build-provenance` in CI) binding it to the exact commit and
workflow run that built it. Verify manually with the GitHub CLI:

```sh
gh attestation verify <file> --repo smh0505/vndb-metadata-wasm-plugin
```

Concourse checks this automatically on install (`plugin_verification.rs`) and shows the result
in the install-confirmation dialog - advisory only for now, not a hard gate (see that module's
own doc comment for why: it proves the artifact really came from this repo's CI, not that this
repo's author is trustworthy - that's a separate, harder problem).

## Versioning

Plain SemVer (`Cargo.toml` + `plugin.json`'s `version`), independent of Concourse's own
milestone-tracked version - patch for fixes, minor for backward-compatible new capabilities,
major for breaking manifest/WIT interface changes. Full convention:
[`.claude/CLAUDE.md`](https://github.com/smh0505/Concourse/blob/main/.claude/CLAUDE.md) (Plugin Versioning) in the main [Concourse](https://github.com/smh0505/Concourse) repo.
