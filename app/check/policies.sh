# A policy written where an administrator puts ilo's, under its bundle id (docs/policies.md),
# reaches Chromium: a default search engine set by policy gives the command bar's search row its
# URL. Written with defaults, the policy is recommended, as one in the user's own preferences is,
# so the run has a new profile of its own, where no engine the user picked outranks it. The Mac's
# alone: Linux's and Windows' policies go where only an administrator can write.

policies_check() {
  if [ "$(uname)" != Darwin ]; then
    echo "skip: policies. Linux's and Windows' go where only an administrator can write."
    return 0
  fi
  # The run's policy, which lib.sh's cleanup takes out of the app's preferences after a failure too.
  policies="DefaultSearchProviderEnabled DefaultSearchProviderName DefaultSearchProviderKeyword"
  policies="$policies DefaultSearchProviderSearchURL"
  defaults write "$app_id" DefaultSearchProviderEnabled -bool true
  defaults write "$app_id" DefaultSearchProviderName -string "Check Search"
  defaults write "$app_id" DefaultSearchProviderKeyword -string check
  defaults write "$app_id" DefaultSearchProviderSearchURL \
    -string 'https://search.example/?q={searchTerms}'
  profile_dir="$PWD/$out/policies/profile"
  rm -rf "$profile_dir"
  mkdir -p "$profile_dir"
  log="$out/policies.log"
  step=0
  BROWSER_DRIVE=1 BROWSER_URL='data:text/html,<h1>Policies</h1>' \
    BROWSER_PROFILE_DIR="$profile_dir" "$browser" < "$out/drive" > "$log" 2>&1 &
  pid=$!
  exec 3> "$out/drive"
  await_load
  send click $page_point
  if [ "$active" != 1 ]; then
    exec 3>&-
    quit
    forget_policies
    echo "skip: policies. Browser wasn't the active app, so the command bar didn't get its keys."
    return 0
  fi
  send key t cmd
  expect '^browser: command bar shown$' "⌘T didn't open the command bar"
  send type policy
  searched='https://search\.example/?q=policy'
  expect "^browser: command bar suggestions \"policy\" in [0-9.]* ms: .*$searched" \
    "the command bar's search row didn't use the policy's engine"
  ../tools/window-screenshot.sh "$pid" "$out/policies.png" || fail "no window to capture"
  send key escape
  exec 3>&-
  quit
  forget_policies
  echo "ok: a default search engine set by policy under the app's bundle id gives the command" \
    "bar's search row its URL"
}
policies_check
