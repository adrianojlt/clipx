use crate::error::AppError;
use semver::Version;
use std::time::Duration;
use tauri::AppHandle;

const RELEASES_URL: &str = "https://api.github.com/repos/adrianojlt/clipx/releases/latest";

// GitHub rejects unauthenticated API requests without a User-Agent with 403.
const USER_AGENT: &str = concat!("clipx/", env!("CARGO_PKG_VERSION"));

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(serde::Serialize)]
pub struct UpdateInfo {
    pub version: String,
    pub notes: String,
    pub url: String,
}

// Only the three fields the feature uses; serde ignores the rest of the payload.
#[derive(serde::Deserialize)]
struct GithubRelease {
    tag_name: String,
    html_url: String,
    body: Option<String>,
}

#[tauri::command]
pub async fn check_for_update(app: AppHandle) -> Result<Option<UpdateInfo>, AppError> {

    let current = app.package_info().version.to_string();

    let release: GithubRelease = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .user_agent(USER_AGENT)
        .build()?
        .get(RELEASES_URL)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    if !is_newer(&current, &release.tag_name) {
        return Ok(None);
    }

    Ok(Some(UpdateInfo {
        version: release.tag_name.strip_prefix('v').unwrap_or(&release.tag_name).to_string(),
        notes: release.body.unwrap_or_default(),
        url: release.html_url,
    }))
}

/// Returns true when `latest_tag` names a release newer than `current`.
///
/// A leading `v` on the tag is tolerated. An unparseable version on either
/// side is reported as "no update" rather than an error, so a malformed
/// release tag can never block the app.
fn is_newer(current: &str, latest_tag: &str) -> bool {

    let latest = latest_tag.strip_prefix('v').unwrap_or(latest_tag);

    let current = match Version::parse(current) {
        Ok(v) => v,
        Err(e) => {
            log::warn!("could not parse current version {current}: {e}");
            return false;
        }
    };

    let latest = match Version::parse(latest) {
        Ok(v) => v,
        Err(e) => {
            log::warn!("could not parse release tag {latest_tag}: {e}");
            return false;
        }
    };

    latest > current
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newer_patch_with_v_prefix_is_an_update() {
        assert!(is_newer("0.1.28", "v0.1.29"));
    }

    #[test]
    fn numeric_compare_beats_lexical_compare() {
        assert!(is_newer("0.1.9", "v0.1.10"));
    }

    #[test]
    fn same_version_is_not_an_update() {
        assert!(!is_newer("0.1.28", "0.1.28"));
    }

    #[test]
    fn older_release_is_never_an_update() {
        assert!(!is_newer("0.1.29", "v0.1.28"));
    }

    #[test]
    fn unparseable_tag_is_not_an_update() {
        assert!(!is_newer("0.1.28", "not-a-version"));
    }

    #[test]
    fn prerelease_tag_does_not_panic() {
        let _ = is_newer("0.1.28", "v0.2.0-beta.1");
    }

    // Trimmed capture of GET /repos/{owner}/{repo}/releases/latest, keeping a
    // representative sample of the fields the struct does not declare.
    const RELEASE_FIXTURE: &str = r###"{
      "url": "https://api.github.com/repos/adrianojlt/clipx/releases/1234567",
      "assets_url": "https://api.github.com/repos/adrianojlt/clipx/releases/1234567/assets",
      "html_url": "https://github.com/adrianojlt/clipx/releases/tag/v0.1.29",
      "id": 1234567,
      "author": { "login": "adrianojlt", "id": 42, "type": "User" },
      "tag_name": "v0.1.29",
      "target_commitish": "main",
      "name": "ClipX v0.1.29",
      "draft": false,
      "prerelease": false,
      "created_at": "2026-07-20T10:11:12Z",
      "published_at": "2026-07-20T10:30:00Z",
      "assets": [
        { "name": "ClipX_0.1.29_aarch64.dmg", "size": 4210000, "download_count": 7 }
      ],
      "tarball_url": "https://api.github.com/repos/adrianojlt/clipx/tarball/v0.1.29",
      "body": "## Changes\n- add themes: light and dark\n- fix animation"
    }"###;

    #[test]
    fn release_fixture_deserializes_and_ignores_unknown_fields() {
        let release: GithubRelease = serde_json::from_str(RELEASE_FIXTURE).unwrap();

        assert_eq!(release.tag_name, "v0.1.29");
        assert_eq!(
            release.html_url,
            "https://github.com/adrianojlt/clipx/releases/tag/v0.1.29"
        );
        assert_eq!(
            release.body.as_deref(),
            Some("## Changes\n- add themes: light and dark\n- fix animation")
        );
    }

    #[test]
    fn release_without_body_deserializes() {
        let release: GithubRelease =
            serde_json::from_str(r#"{"tag_name":"v0.1.29","html_url":"https://x","body":null}"#)
                .unwrap();

        assert_eq!(release.body, None);
    }
}
