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

1. `pnpm release:version 1.2.3` synchronizes desktop package, Cargo, Tauri config, and Cargo.lock. Review and commit those changes yourself.
2. Run `pnpm check`, then `pnpm release:check` from a clean tree. Create and push exactly tag `v1.2.3` only after review. Scripts never commit, tag, push, publish, or release.
3. The tag workflow ad-hoc signs each `.app`, packages ARM64 and x86_64 DMGs, and uploads them with updater signatures and `latest.json` to a **draft** GitHub Release. The DMG container itself is unsigned, and nothing is notarized without an Apple Developer account. Rerun the tag-triggered workflow if necessary; manual dispatch is intentionally disabled so a release cannot bypass the tag gate.
4. Inspect every asset and release note, then manually publish the draft. Publishing is what makes the `latest` updater endpoint visible.

For a local build, export the updater public key and private signing key, then run `pnpm release:mac aarch64-apple-darwin`; on an Intel-capable macOS runner use `pnpm release:mac x86_64-apple-darwin`. The script defaults `APPLE_SIGNING_IDENTITY` to `-` for ad-hoc signing. Tauri merges `--config` after the main config; the script creates a mode-0600, gitignored overlay and removes it after the build. Ordinary development does not need updater keys.

## Installing an ad-hoc signed build

After dragging `CRAFTEL.app` from the DMG into `/Applications`, macOS will normally block this non-notarized download. First try Control-clicking the app, choosing **Open**, and confirming; recent macOS versions may instead require **System Settings → Privacy & Security → Open Anyway**. If Gatekeeper still blocks it, remove only CRAFTEL's quarantine attribute:

```bash
xattr -dr com.apple.quarantine /Applications/CRAFTEL.app
open /Applications/CRAFTEL.app
```

The command is `xattr`, not `xattc`. Do not run `xattr -cr` against all of `/Applications` or disable Gatekeeper globally: both are unnecessarily broad. Run the command again only if a later update is quarantined and blocked. This bypass is suitable for known CRAFTEL artifacts while no Apple account exists, but it is not equivalent to notarization; users must verify that the DMG came from the project's GitHub Release.

Validate ARM natively and Intel under Rosetta (and preferably Intel hardware): launch each app and check update/no-update/error/download/restart behavior. `codesign --verify --deep --strict --verbose=2 CRAFTEL.app` should validate the ad-hoc structure. `codesign -dv --verbose=4 CRAFTEL.app` should show an ad-hoc signature. `spctl --assess --type execute --verbose=2 CRAFTEL.app` is expected to reject it because there is no trusted Developer ID, and `xcrun stapler validate` is expected to find no notarization ticket. These expected failures must not be reported as signed/notarized success. This Linux orb cannot perform native macOS packaging, Gatekeeper, Rosetta, or launch verification.
