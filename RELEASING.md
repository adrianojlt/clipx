# Releasing ClipX

The update notification feature checks GitHub Releases to tell users a new
version is available. Three failure modes cannot be caught by code, tests, or
CI, so they are the responsibility of whoever cuts the release. Read the three
warnings below before following the steps.

## Release checklist

Perform these steps in order:

1. **Bump the version in `src-tauri/tauri.conf.json`.** Set `version` to the new
   `X.Y.Z`.
2. **Commit** the version bump.
3. **Tag** the commit: `git tag vX.Y.Z` (the `v` prefix matches the workflow's
   `v*` trigger; the tag version must match the `tauri.conf.json` version).
4. **Push** the commit and the tag: `git push && git push origin vX.Y.Z`. Push
   the tag by name, never with `--tags` (see the warning below). Pushing the
   tag starts the release workflow, which builds every platform and creates a
   **draft** release.
5. **Publish the draft release.** Open the release on GitHub and click Publish.
   The release stays a draft until you do this.

## Warning: an unpublished draft release notifies nobody

The client checks for updates with `GET /releases/latest`, and that endpoint
**excludes drafts and prereleases**. A release left as a draft is invisible to
every client:

- No user is ever notified of the new version.
- No error appears anywhere, not in the app, not in the logs, not in the API
  response. The check simply reports the previous published release (or none).

The release workflow always creates the release as a draft (`releaseDraft: true`
in `.github/workflows/release.yml`). Step 5 is therefore mandatory: the release
does not exist for clients until it is published.

## Warning: `tauri.conf.json` is the only version that matters

`src-tauri/tauri.conf.json` is the single source of truth that both the
frontend `getVersion()` and the backend `app.package_info().version` read from,
and the update check compares that value against the latest published release.

- If you tag `vX.Y.Z` but forget to bump `tauri.conf.json`, the installed app
  keeps reporting its old version. It will **notify forever** (thinking it is
  out of date) or **never** (if the stale version already looks current),
  depending on the values.
- The versions in `package.json` and `src-tauri/Cargo.toml` do **not** affect
  the update check. Keeping them in sync is tidy but not required for
  notifications to work; `tauri.conf.json` is the one that must match the tag.

Always bump `tauri.conf.json` (step 1) in the same release as the git tag
(step 3), and make sure the two versions are identical.

## Warning: `git push --tags` can silently skip the workflow

GitHub drops tag push events when several tags arrive in one push:

> Events will not be created for tags when more than three tags are pushed at
> once.

`git push --tags` pushes every local tag the remote is missing. If that is four
or more, the tags still land on GitHub but **no workflow run is created**, and
nothing reports an error:

- The tag looks perfectly correct on GitHub and points at the right commit.
- The Actions tab simply has no run for it, so no build and no draft release.
- This happened on `v0.1.31`.

Push the tag by name (`git push origin vX.Y.Z`) so the push carries exactly one
ref. If a tag was already pushed this way and no run appeared, delete and
re-push it:

```
git push --delete origin vX.Y.Z
git push origin vX.Y.Z
```

There is no `workflow_dispatch` trigger on the release workflow, so re-pushing
the tag is the only way to start the build.
