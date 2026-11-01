# Releasing

How a release of ilo is made, and what it needs.

## Notes

Each version's release notes are its section of [CHANGELOG.md](../CHANGELOG.md), `## <version>`, written for users in plain language. `tools/changelog.sh <version>` prints them. They become the release's body, which [ilobrowser.com/releases](https://ilobrowser.com/releases) renders when the site rebuilds for the release, and the Mac's update prompt shows them, embedded in the appcast. WinSparkle's prompt, and the Mac's first launch of an update, open the site's page at the version's anchor, `/releases#<version>` (`tools/appcast.sh`). A section headed `## <version> (draft)` is notes not yet reviewed: a try builds with them, and a release stops on them. Notes are never copied into the site by hand.

## The release workflow

`.github/workflows/release.yml` makes a release from an annotated tag with a one-line message, `git tag -a v<version> -m "ilo <version>"`, on a commit whose `app/Cargo.toml` has that version and whose `CHANGELOG.md` has its section, no longer marked draft. Each OS builds on its prebuilt engine for that commit, and the run stops if there isn't one. The Windows engine builds nightly, so before pushing the tag, run Engine on Windows (cross-compiled) on its commit, `gh workflow run engine-windows-cross.yml --ref main` with main at that commit, and wait for its release `engine-windows-<commit>`. It makes a draft release whose files keep their names, so `releases/latest/download/<name>` always links the newest: `ilo-mac-arm64.dmg`, `ilo-windows-x64.msix`, `ilo-windows-x64-setup.exe`, `ilo-linux-x86_64.flatpak`, `ilo.flatpakref`, `SHA256SUMS` and `appcast.xml`. Once the draft is published, the `publish` job puts its `appcast.xml` in release `updates`, the feed the Mac and Windows builds read (`app/branding/update-feed`). The run's `winget` artifact holds winget's manifests for the release's installer, for a pull request to [microsoft/winget-pkgs](https://github.com/microsoft/winget-pkgs).

The draft also holds `ilo-demo.tar.gz`, the site's live demo built from the same commit (`.github/workflows/demo.yml`), when the commit has one. Once the draft is published, [ilo-site](https://github.com/MartinPTielemans/ilo-site)'s release workflow puts that demo on [ilobrowser.com](https://ilobrowser.com), links its source to the tag and deploys the site, whose download buttons link the newest release. It checks every half hour, or at once with the secret `SITE_DISPATCH_TOKEN`, a fine-grained token with Contents read and write on `MartinPTielemans/ilo-site` alone. Every other release here, the appcast's `updates`, is made with `--latest=false`, or the site's buttons would link it. The prebuilt engines aren't released here, so this repository's Releases hold only ilo's: they're in [ilo-engine](https://github.com/MartinPTielemans/ilo-engine), the repository `engine/prebuilt.ini` names, as `engine-mac-`, `engine-linux-` and `engine-windows-<commit>`, also with `--latest=false`. The engine workflows publish there with the secret `ENGINE_RELEASE_TOKEN`, a fine-grained token with Contents read and write on `MartinPTielemans/ilo-engine` alone.

The Mac's DMG is built only with these repository secrets:

| Secret | What |
|---|---|
| `MAC_CERTIFICATE_P12` | The Developer ID Application certificate and its key, as a base64 `.p12` |
| `MAC_CERTIFICATE_PASSWORD` | The `.p12`'s password |
| `MAC_NOTARY_KEY` | An App Store Connect API key for notarization, as a base64 `.p8` |
| `MAC_NOTARY_KEY_ID`, `MAC_NOTARY_ISSUER_ID` | That key's ID and its issuer's |
| `UPDATE_PRIVATE_KEY` | The updates' signing key: the contents of the file `generate_keys --account ilo -x <file>` writes (`tools/appcast.sh`) |

The Mac and Windows builds carry its public half from `app/branding/update-key`, or from an `UPDATE_PUBLIC_KEY` secret, which overrides the file when it's set.

Crash reports' symbols go to the collector `app/branding/crash-reports` names only with the secret `SENTRY_AUTH_TOKEN`, an organization auth token that may upload debug files, and the variables `SENTRY_URL`, `SENTRY_ORG` and `SENTRY_PROJECT`, the collector's address, organization and project: for ilo's, `https://de.sentry.io` (Sentry's EU region), `ilo-rg` and `ilo`. Each OS's job uploads its own (`tools/symbols.sh`), and `app/build.sh prod` uploads the Mac's, the owner's daily copy too, with the same four in its environment. Without the token the packages are still built, and say no symbols were uploaded.

The Windows packages are unsigned: the Store signs the MSIX, and winget takes the installer.

Linux installs from GitHub update from ilo's Flatpak repository, a static OSTree repository in a Cloudflare R2 bucket (`tools/flatpak-repo.sh`). The bundle and `ilo.flatpakref` name it and carry its public key. The run adds the build to the bucket, unseen; publishing the draft makes it live. Publish each draft before building the next, since a build doesn't follow a draft that isn't live yet. The Linux files are built only with these:

| Secret | What |
|---|---|
| `FLATPAK_GPG_KEY` | The repository's signing key, ASCII-armoured, without a passphrase |
| `R2_BUCKET` | The bucket's name |
| `R2_ENDPOINT` | `https://<account-id>.r2.cloudflarestorage.com` |
| `R2_ACCESS_KEY_ID`, `R2_SECRET_ACCESS_KEY` | An R2 API token with Object Read & Write on that bucket alone |

and the repository variable `FLATPAK_REPO_URL`, the bucket's public address, such as `https://flatpak.<domain>/`.

Once, before the first release:

1. Make the key: `gpg --quick-gen-key "ilo Flatpak repository" rsa4096 sign never`, with an empty passphrase. Put `gpg --armor --export-secret-keys <fingerprint>` in `FLATPAK_GPG_KEY` and keep a copy offline forever. Every installed copy checks it, and a new key strands them.
2. In Cloudflare, create the R2 bucket, connect a custom domain to it for public reads, and create the API token.
3. Add the secrets and the variable. The address is permanent too, as installs keep it.
