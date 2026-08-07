//! WASM `MetadataProviderPlugin` for VNDB (vndb.org) - the Visual Novel Database. Provides
//! description/release date/cover art for visual novels, a genre this app's other metadata
//! providers (IGDB, RAWG, TheGamesDB, all modern/PC-console leaning) don't meaningfully cover.
//!
//! Uses VNDB's own official Kana API v2 (`api.vndb.org/kana`) - a real, documented, public
//! REST-ish JSON API, confirmed working with a live unauthenticated request during research
//! for this plugin (`POST /vn` with a `filters`/`fields` JSON body). No API key required for
//! this level of usage - unlike TheGamesDB/IGDB/RAWG's plugins, there's no `settingsSchema` in
//! `plugin.json` and nothing to configure in Settings.
//!
//! No genres - deliberately, not an oversight. VNDB's tag system is a large, freeform,
//! community-curated vocabulary (plot elements, structure, content warnings, and genre-ish
//! tags all mixed together, easily dozens per title), not a small controlled genre list like
//! the other providers return. Forcing that into `MetadataResult.genres: list<string>` would
//! mean picking an arbitrary subset with no principled cutoff - same "skip a field that doesn't
//! cleanly fit" precedent `sgdb-metadata-wasm-plugin` already sets for its own missing fields.
//!
//! VNDB's `description` field uses its own lightweight BBCode-style markup (`[b]`, `[i]`,
//! `[url=..]`), not Markdown - converted to Markdown here (`bbcode_to_markdown`) since this
//! app's own description field is documented as Markdown-rendered; left as literal bracket
//! syntax otherwise, which would show up as visible junk in the UI.

#[allow(warnings)]
mod bindings;

use bindings::exports::gamelib::plugin::metadata_plugin::{Guest, MetadataCandidate, MetadataResult};
use bindings::gamelib::plugin::host;

struct VndbPlugin;

const API_URL: &str = "https://api.vndb.org/kana/vn";

#[derive(serde::Deserialize)]
struct SearchResponse {
    results: Vec<VnSummary>,
}

#[derive(serde::Deserialize)]
struct VnSummary {
    id: String,
    title: String,
    released: Option<String>,
}

#[derive(serde::Deserialize)]
struct DetailResponse {
    results: Vec<VnDetail>,
}

#[derive(serde::Deserialize)]
struct VnDetail {
    released: Option<String>,
    description: Option<String>,
    image: Option<VnImage>,
}

#[derive(serde::Deserialize)]
struct VnImage {
    url: String,
}

/// `filter_key` is `"search"` (fuzzy title match, for `search_candidates`) or `"id"` (exact
/// lookup, for `fetch_metadata_by_id`) - VNDB's own `filters` shape is `[key, "=", value]`
/// either way, just with a different key/value pair.
fn query(fields: &str, filter_key: &str, filter_value: &str) -> Result<String, String> {
    let body = serde_json::json!({
        "filters": [filter_key, "=", filter_value],
        "fields": fields,
    })
    .to_string();

    host::http_request(
        "POST",
        API_URL,
        &[("Content-Type".to_string(), "application/json".to_string())],
        Some(&body),
    )
}

fn candidate_label(title: &str, released: &Option<String>) -> String {
    match released.as_ref().and_then(|d| d.split('-').next()) {
        Some(year) if !year.is_empty() => format!("{} ({})", title, year),
        _ => format!("{} (release date unknown)", title),
    }
}

/// Converts VNDB's own lightweight BBCode-style markup to Markdown, since this app's
/// description field renders Markdown, not VNDB's syntax. Only handles the tags VNDB's own
/// docs describe (`[b]`/`[i]`/`[url=..]`) - anything else passes through unchanged rather than
/// risk mangling text this doesn't recognize.
fn bbcode_to_markdown(text: &str) -> String {
    let mut out = text.replace("[b]", "**").replace("[/b]", "**");
    out = out.replace("[i]", "*").replace("[/i]", "*");

    // [url=<href>]<label>[/url] -> [<label>](<href>) - no regex crate; small enough to parse
    // by hand with find/split rather than pull in a dependency for one tag shape.
    let mut result = String::with_capacity(out.len());
    let mut rest = out.as_str();
    while let Some(start) = rest.find("[url=") {
        result.push_str(&rest[..start]);
        let after_tag = &rest[start + 5..];
        let Some(close_bracket) = after_tag.find(']') else {
            result.push_str(&rest[start..]);
            rest = "";
            break;
        };
        let href = &after_tag[..close_bracket];
        let after_href = &after_tag[close_bracket + 1..];
        let Some(end_tag) = after_href.find("[/url]") else {
            result.push_str(&rest[start..]);
            rest = "";
            break;
        };
        let label = &after_href[..end_tag];
        result.push_str(&format!("[{}]({})", label, href));
        rest = &after_href[end_tag + 6..];
    }
    result.push_str(rest);
    result
}

impl Guest for VndbPlugin {
    fn search_candidates(title: String) -> Result<Vec<MetadataCandidate>, String> {
        let body = query("title, released", "search", &title)?;
        let search: SearchResponse = serde_json::from_str(&body).map_err(|e| e.to_string())?;

        Ok(search
            .results
            .into_iter()
            .filter(|vn| vn.title.eq_ignore_ascii_case(&title))
            .map(|vn| MetadataCandidate {
                id: vn.id,
                label: candidate_label(&vn.title, &vn.released),
                image_url: None,
            })
            .collect())
    }

    fn fetch_metadata_by_id(id: String) -> Result<Option<MetadataResult>, String> {
        let response = query("released, description, image.url", "id", &id)?;
        let detail: DetailResponse = serde_json::from_str(&response).map_err(|e| e.to_string())?;
        let Some(vn) = detail.results.into_iter().next() else {
            return Ok(None);
        };

        Ok(Some(MetadataResult {
            description: vn.description.map(|d| bbcode_to_markdown(&d)),
            release_date: vn.released,
            genres: Vec::new(),
            cover_art_url: vn.image.map(|img| img.url),
            background_art_url: None,
        }))
    }
}

bindings::export!(VndbPlugin with_types_in bindings);
