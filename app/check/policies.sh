# A policy written where an administrator puts ilo's, under its bundle id (docs/policies.md),
# reaches Chromium: a download folder set by policy is where a page's download goes. Written with
# defaults, the policy is recommended, as one in the user's own preferences is, so the run has a new
# profile of its own, where no folder the user picked outranks it. It isn't the default search
# engine, which on a Mac without MDM Chromium ignores, as it does every policy it calls sensitive.
# The Mac's alone: Linux's and Windows' policies go where only an administrator can write.

policies_check() {
  if [ "$(uname)" != Darwin ]; then
    echo "skip: policies. Linux's and Windows' go where only an administrator can write."
    return 0
  fi
  serve
  downloads="$PWD/$out/policies/downloads"
  profile_dir="$PWD/$out/policies/profile"
  rm -rf "${out:?}/policies"
  mkdir -p "$downloads" "$profile_dir/Default"
  # A new profile asks where to save each download, which every run that downloads turns off, as
  # check.sh does for the dev profile. This one leaves the folder to the policy.
  printf '{"download": {"prompt_for_download": false}}' > "$profile_dir/Default/Preferences"
  # The run's policy, which lib.sh's cleanup takes out of the app's preferences after a failure too.
  policies=DownloadDirectory
  defaults write "$app_id" DownloadDirectory -string "$downloads"
  # "p" typed in the page downloads a file with a name of the run's own, which a download that went
  # to the Downloads folder instead leaves there until the run takes it out. The title names each
  # key the page gets, so a key that never reached it tells apart from a download that never came.
  # The link has text: a page whose elements draw nothing doesn't get the keys typed in its first
  # second or two, as one with an empty link or div showed, while one with text gets them at once.
  file=ilo-policy-check.txt
  printf '%s' "<title>loading</title><a id=p download=$file href=notes.txt>Notes</a>
<script>
requestAnimationFrame(() => setTimeout(() => document.title = \"ready\"));
onkeydown = e => {
  document.title = \"key \" + e.key;
  if (e.key == \"p\") p.click();
};
</script>" > "$out/site/policy.html"
  log="$out/policies.log"
  step=0
  BROWSER_DRIVE=1 BROWSER_URL="${site}policy.html" \
    BROWSER_PROFILE_DIR="$profile_dir" "$browser" < "$out/drive" > "$log" 2>&1 &
  pid=$!
  exec 3> "$out/drive"
  await_load
  await '^browser: title [0-9]* ready$' || fail "the policies run's page didn't draw"
  send click $page_point
  if [ "$active" != 1 ]; then
    exec 3>&-
    quit
    forget_policies
    echo "skip: policies. Browser wasn't the active app, so the page ignored the click."
    return 0
  fi
  send type p
  expect '^browser: title [0-9]* key p$' "the page's script didn't get the p typed"
  if ! await "^browser: download [0-9]* done $file\$"; then
    # A Save panel or an alert holds a download back without a line of its own: front names it.
    send front
    ../tools/window-screenshot.sh "$pid" "$out/policies-fail.png" all
    key=$(grep '^browser: front ' "$log" | tail -1 | sed 's/.*, key \(.*\), ax focused .*/\1/')
    fail "the page's download didn't finish, with the key window $key. Windows: $out/policies-fail.png"
  fi
  if [ ! -f "$downloads/$file" ]; then
    rm -f "$HOME/Downloads/$file"
    fail "the download didn't go in the policy's folder"
  fi
  ../tools/window-screenshot.sh "$pid" "$out/policies.png" || fail "no window to capture"
  exec 3>&-
  quit
  forget_policies
  echo "ok: a download folder set by policy under the app's bundle id is where a page's download" \
    "goes. Window: $out/policies.png"
}
policies_check
