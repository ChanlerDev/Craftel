# macOS releases and updates

CRAFTEL uses Tauri 2's official updater/process plugins and `tauri-action`: this is the smallest maintained path from an updater-signed GitHub Release to an in-app check, user-approved download, installation, and restart. Builds are separate Apple Silicon and Intel artifacts, reducing downloads versus a universal binary. The updater has **no binary delta support**. It checks at startup at most once per 24 hours, never polls repeatedly, and never downloads without the user pressing **Download**. Only if package size grows substantially should we evaluate Sparkle or a dynamic update service.

There are two unrelated signatures. The Tauri updater's Minisign key proves that an update came from CRAFTEL and is mandatory for automatic updates; it is free and does not require Apple. Apple Developer ID signing and notarization establish Gatekeeper trust. CRAFTEL currently has no paid Apple Developer account, so release builds use Apple's **ad-hoc signature** (`APPLE_SIGNING_IDENTITY=-`). This is important for Apple Silicon binary integrity, but it is not identity signing or notarization and does not eliminate Gatekeeper warnings.

## One-time setup

The repository must remain public for the fixed unauthenticated `releases/latest/download/latest.json` endpoint. Generate a signing key with `pnpm --filter @craftel/desktop tauri signer generate -w ~/.tauri/craftel.key`; never add it to Git. Configure only these repository secrets:

- `TAURI_UPDATER_PUBLIC_KEY`: contents of `~/.tauri/craftel.key.pub`.
- `TAURI_SIGNING_PRIVATE_KEY`: contents of `~/.tauri/craftel.key`; back it up securely because losing it prevents updates to existing installations.
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: the key password, or an empty secret if the key has none.

No Apple secrets are currently required. GitHub's generated token uploads draft assets. If a paid Apple Developer account is added later, replace the workflow's ad-hoc identity with Developer ID certificate and notarization credentials; do not mix that migration into a routine release.

## Release flow

CRAFTEL currently permits **patch releases only**. From `0.1.0`, the only accepted next version is `0.1.1`; minor, major, skipped patch, repeated, downgraded, and prerelease versions are rejected before any ref is pushed.

1. Land normal changes on `main` and wait for the `CI` workflow to pass. A push to `main` runs the complete Linux check plus an Apple Silicon native compile/bundle check; it does not create a Release.
2. Open **Actions → Draft macOS release → Run workflow**, select `main`, and normally leave **version** blank so the workflow calculates the immediate next patch. To make the intended number explicit, enter exactly that same next patch (for example `0.1.1`); any other value fails.
3. The single global release lane verifies that the selected commit is still `origin/main` HEAD, that its push-triggered `CI` run succeeded, and that updater keys exist. It synchronizes package, Cargo, Tauri, and lockfile versions, runs `pnpm check`, and permits only those four version files to change.
4. The workflow creates an auditable `chore: release vX.Y.Z` commit and annotated matching tag. It pushes `main` and the tag together with `git push --atomic` and a lease on the original main SHA. If main advances, a competing release creates the tag, branch policy blocks the commit, or either ref cannot be written, neither ref is updated.
5. The same workflow run then ad-hoc signs each `.app`, serially packages ARM64 and x86_64 DMGs, and uploads them with updater archives, signatures, and a merged `latest.json` to a **Draft GitHub Release**. The DMG container itself is unsigned, and nothing is notarized without an Apple Developer account.
6. Inspect both architectures, updater entries, signatures, and release notes. Manually publish the draft only when they are correct. Draft assets are not served by `releases/latest`; publishing makes that release's `latest.json` visible at the configured updater endpoint.
7. Installed apps check that `latest.json` at startup at most once per 24 hours. If its patch version is newer, the app prompts; it downloads only after the user presses **Download**, verifies the Tauri signature, installs, and offers restart.

The workflow uses its scoped `GITHUB_TOKEN`, not a PAT. GitHub intentionally does not start another workflow from a branch or tag pushed with that token, so the dispatch run builds directly after pushing instead of waiting for a tag-triggered run. The retained `v*` trigger is a recovery/compatibility path for a tag pushed outside Actions; its preflight applies the same main ancestry, synchronized-version, and next-patch checks. This prevents the dispatch-created tag from causing a duplicate build.

Set **Settings → Actions → General → Workflow permissions** so this workflow may request write access to repository contents. That setting scopes `GITHUB_TOKEN`; it does not bypass branch protection. If `main` has a ruleset, it must permit direct updates by the repository's GitHub Actions app/token through an explicitly reviewed actor-level bypass or compatible rule configuration, otherwise the atomic push safely fails. The release actor also needs repository write access to run the workflow. Do not add a broad PAT or disable required review rules merely for releases. The generated release commit does not launch `CI` because of GitHub's token recursion guard; the workflow compensates by requiring green CI on the source main SHA and running the full suite again after changing versions but before the atomic push.

If preparation fails, fix the reported prerequisite and start a new run from current `main`. If packaging fails after the atomic push, use **Re-run failed jobs** (or rerun the failed matrix job) so the successful preparation job and its exact commit output are retained; do not use **Re-run all jobs**, because the original dispatch SHA intentionally becomes stale after the release commit. The `v*` recovery trigger applies when a valid tag is newly pushed outside Actions, not to an existing dispatch-created tag. Do not start another patch release merely to repair a failed package. Before the first release, configure the three updater secrets described above and keep the updater private key and password backed up; they are not regenerated per patch.

For local diagnosis only, `pnpm release:version <next-patch>` and `pnpm release:check` remain available, but the normal release path is the manual GitHub Actions workflow. These scripts do not commit, tag, push, or publish by themselves.

For a local build, export the updater public key and private signing key, then run `pnpm release:mac aarch64-apple-darwin`; on an Intel-capable macOS runner use `pnpm release:mac x86_64-apple-darwin`. The script defaults `APPLE_SIGNING_IDENTITY` to `-` for ad-hoc signing. Tauri merges `--config` after the main config; the script creates a mode-0600, gitignored overlay and removes it after the build. Ordinary development does not need updater keys.

The corrupt 1×1 placeholder PNG that previously broke macOS bundling has been replaced with a valid 512×512 RGBA PNG. Keep `icons/icon.png` at a normal-density ICNS size supported by Tauri (512×512 or smaller); Tauri treats a 1024×1024 PNG as valid only when its filename ends in `@2x.png`. A future brand review may replace the artwork, but the checked-in asset is now valid for both Tauri's window-icon loader and macOS ICNS generation.

## Installing an ad-hoc signed build

After dragging `CRAFTEL.app` from the DMG into `/Applications`, macOS will normally block this non-notarized download. First try Control-clicking the app, choosing **Open**, and confirming; recent macOS versions may instead require **System Settings → Privacy & Security → Open Anyway**. If Gatekeeper still blocks it, remove only CRAFTEL's quarantine attribute:

```bash
xattr -dr com.apple.quarantine /Applications/CRAFTEL.app
open /Applications/CRAFTEL.app
```

The command is `xattr`, not `xattc`. Do not run `xattr -cr` against all of `/Applications` or disable Gatekeeper globally: both are unnecessarily broad. Run the command again only if a later update is quarantined and blocked. This bypass is suitable for known CRAFTEL artifacts while no Apple account exists, but it is not equivalent to notarization; users must verify that the DMG came from the project's GitHub Release.

Validate ARM natively and Intel under Rosetta (and preferably Intel hardware): launch each app and check update/no-update/error/download/restart behavior. `codesign --verify --deep --strict --verbose=2 CRAFTEL.app` should validate the ad-hoc structure. `codesign -dv --verbose=4 CRAFTEL.app` should show an ad-hoc signature. `spctl --assess --type execute --verbose=2 CRAFTEL.app` is expected to reject it because there is no trusted Developer ID, and `xcrun stapler validate` is expected to find no notarization ticket. These expected failures must not be reported as signed/notarized success. This Linux orb cannot perform native macOS packaging, Gatekeeper, Rosetta, or launch verification.
