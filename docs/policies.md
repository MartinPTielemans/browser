# Managed policies

ilo reads Chromium's enterprise policies, so an administrator configures it as they would Chromium: [Chrome Enterprise's policy list](https://chromeenterprise.google/policies/) gives the names and values. ilo doesn't add policies of its own. `chrome://policy` in a tab lists the ones ilo applied, and where each came from.

## Where they go

| OS | Mandatory | Recommended (the user's choice wins) |
|---|---|---|
| macOS | A configuration profile, from MDM, for the preference domain `app.ilo.browser`. macOS keeps it in `/Library/Managed Preferences` | `defaults write app.ilo.browser <policy> <value>` |
| Linux | JSON files in `/etc/chromium/policies/managed/` | JSON files in `/etc/chromium/policies/recommended/` |
| Windows | `HKLM\SOFTWARE\Policies\Chromium`, or `HKCU\SOFTWARE\Policies\Chromium` for one user | The `Recommended` key under either |

On the Mac, the domain is the app's bundle id (`app/branding/id`): Chromium reads policies through CFPreferences under it, so a fork with its own id reads its own. The dev build's is `dev.browser.app`, which `app/check/policies.sh` writes a download folder to and checks. Off the Mac, ilo's engine keeps Chromium's names for the folders and keys.

## Worth knowing

- On a Mac that isn't managed (no MDM enrolment), Chromium ignores the policies it marks sensitive, so software can't set them with `defaults`: the default search engine (`DefaultSearchProviderEnabled`, without which the other `DefaultSearchProvider` policies do nothing), site search, the home page and startup pages, among others. It also ignores extensions forced in from outside the Chrome Web Store in `ExtensionInstallForcelist` and `ExtensionSettings`. ilo keeps that rule, and `chrome://policy` lists such a policy as blocked.
- ilo gives uBlock Origin its settings as managed storage, under the platform's policies, so an administrator's policy for its id (`oghkpboejbohceocapipmnjifoklonij`) wins over ilo's.
