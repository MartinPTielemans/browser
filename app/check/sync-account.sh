# Settings › Sync signs the browser in to an ilo account, on an ilo-server of the run's own: SQLite,
# DEV_MODE, which logs its sign-in codes, and a throwaway address, never the hosted server or a real
# account. ILO_SERVER names the server's binary, built from its repo (check-linux.yml builds it);
# without it the run skips. Sign In opens the server's page in the core's window, where the run
# types the address and the code the server logged, and declines a passkey. The account's first
# device makes the recovery words, which Done hides, and sync turns on. On the Mac, Copy puts the
# words on the run's own pasteboard, the driver's, which holds them no longer 90 s later (#641);
# Linux's copy is sync-devices', and Windows' is unrun. Then what the server keeps:
# each item it holds of a type Chromium encrypts is ciphertext alone, nameless or named "encrypted",
# the Spaces among them, and the device is named by its label, such as "ilo on Linux", not its
# owner's name. No request in Chromium's net log of the whole run goes to Google's servers. Sign
# Out, as the account's only device, asks first, and the server forgets the device.

if [ -z "${ILO_SERVER:-}" ]; then
  echo "skip: sync-account. It needs ILO_SERVER, an ilo-server built from its repo, to sign in to."
  return 0
fi
log="$out/sync-account.log"
netlog="$out/sync-account-net.json"
step=0
rm -f "$saved" "$launch_frame" "$profile_dir/Account" "$netlog"
. check/ilo-server.sh

BROWSER_DRIVE=1 BROWSER_URL='data:text/html,<title>account check</title>' "$browser" \
  --sync-url="$server_url" --log-net-log="$netlog" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
await 'browser: sync service disabled, not signed in$' ||
  fail "signed out, sync isn't disabled for want of an account"
send menu Settings…
expect '^browser: settings shown ' "Settings… didn't show Settings"
send clickkey $(drawn "toolbar Sync")
expect '^browser: settings tab Sync$' "a click on Sync didn't show its tab"
at=$(drawn "sync sign-in") || fail "Settings › Sync didn't draw Sign In"
../tools/window-screenshot.sh "$pid" "$out/sync-signed-out.png" || fail "no window to capture"
send clickkey $at
expect '^browser: account SigningIn None$' "Sign In didn't start signing in"
await 'browser: sync sign-in loaded /authorize$' ||
  fail "the sign-in window didn't load the server's page"
sign_in first "$out/sync-sign-in-address.png"
await '^browser: account FirstDevice None$' ||
  fail "signing in didn't make this the account's first device"
await '^browser: account On None$' || fail "the first device's recovery words didn't turn sync on"
settings_keyboard
at=$(drawn "sync done") || fail "Settings › Sync didn't show the recovery words with Done"
../tools/window-screenshot.sh "$pid" "$out/sync-words.png" || fail "no window to capture"
if [ "$(uname)" = Darwin ]; then
  send clickkey $(drawn "sync copy")
  copied_at=$(date +%s)
  words='^browser: pasteboard text \([^ ][^ ]* \)\{23\}[^ ][^ ]*$'
  for _ in $(seq 30); do
    send pasteboard
    logged "$words" && break
    sleep 0.1
  done
  expect "$words" "Copy didn't put the 24 recovery words on the run's pasteboard"
fi
send clickkey $(drawn "sync done")
await 'browser: sync service active$' ||
  fail "sync's service didn't become active: $(grep 'browser: sync service ' "$log" | tail -1)"
grep -q 'browser: sync engine ' "$log" || fail "sync's engine made no request"

# What the server keeps, once the Spaces are there: Chromium leaves Nigori, its device records and
# its priority preferences unencrypted (ilo-server's README, What the server can see); every other
# item the browser sent holds only ciphertext. The server's own folders, with a server tag, aren't.
check_store() {
  python3 -I - "$db" << 'PY'
import sqlite3, sys

def varint(b, i):
    value = shift = 0
    while True:
        c = b[i]
        i += 1
        value |= (c & 0x7F) << shift
        shift += 7
        if c < 0x80:
            return value, i

def fields(b):
    i, out = 0, []
    while i < len(b):
        key, i = varint(b, i)
        number, kind = key >> 3, key & 7
        if kind == 0:
            value, i = varint(b, i)
        elif kind == 1:
            value, i = b[i:i + 8], i + 8
        elif kind == 2:
            n, i = varint(b, i)
            value, i = b[i:i + n], i + n
        elif kind == 5:
            value, i = b[i:i + 4], i + 4
        else:
            raise ValueError(f"wire type {kind}")
        out.append((number, kind, value))
    return out

NIGORI, PASSWORD, DEVICE_INFO, PRIORITY_PREFERENCE, ILO_SPACE = (
    47745, 45873, 154522, 163425, 500000001)

# EncryptedData: its key's name and the blob, nothing readable.
def sealed(b):
    try:
        return all(n in (1, 2) and k == 2 for n, k, v in fields(b))
    except (IndexError, ValueError):
        return False

# Passwords encrypt into their own specifics (PasswordSpecifics.encrypted), beside an encrypted
# backup of their notes (field 5), as the core's commit guard has them (sync_guard.cc).
def password_sealed(b):
    parts = fields(b)
    return any(n == 1 for n, k, v in parts) and all(
        n in (1, 5) and k == 2 and sealed(v) for n, k, v in parts)
db = sqlite3.connect(sys.argv[1])
rows = db.execute("SELECT data_type, deleted, server_tag, entity FROM sync_entities").fetchall()
spaces, labels, plain = 0, [], []
for data_type, deleted, server_tag, entity in rows:
    if deleted or server_tag:
        continue
    item = fields(entity)
    specifics = fields(b"".join(v for n, k, v in item if n == 21))
    if data_type == DEVICE_INFO:
        for n, k, v in specifics:
            if n == DEVICE_INFO:
                labels += [w.decode() for m, j, w in fields(v) if m == 2]
        continue
    if data_type in (NIGORI, PRIORITY_PREFERENCE):
        continue
    names = [v for n, k, v in item if n == 7]
    if data_type == PASSWORD:
        ciphertext = any(n == PASSWORD for n, k, v in specifics) and all(
            n == PASSWORD and password_sealed(v) for n, k, v in specifics)
    else:
        ciphertext = any(n == 1 and sealed(v) for n, k, v in specifics) and all(
            (n == 1 and sealed(v)) or (k == 2 and not v) for n, k, v in specifics)
    if not ciphertext or any(name not in (b"", b"encrypted") for name in names):
        plain.append(f"type {data_type} named {names[0].decode(errors='replace') if names else ''}")
    if data_type == ILO_SPACE:
        spaces += 1
print(f"spaces {spaces}, devices {'|'.join(labels) or 'none'}, plain {'|'.join(plain) or 'none'}")
PY
}
for _ in $(seq 60); do
  store=$(check_store) || fail "the server's database couldn't be read"
  case $store in "spaces 0,"*) ;; *) break ;; esac
  sleep 0.5
done
case $store in
"spaces 0,"*) fail "no Space reached the server: $store" ;;
*", plain none") ;;
*) fail "the server holds what it can read: $store" ;;
esac
case $store in
*", devices $label"*) ;;
*) fail "the device isn't named by its label: $store" ;;
esac
devices=$(stored "SELECT group_concat(name, '|') FROM devices")
case $devices in
"$label"*) ;;
*) fail "the server knows the device as \"$devices\", not by its label" ;;
esac

# The words leave the pasteboard 90 s after the copy, which the pasteboard's own change shows.
if [ "$(uname)" = Darwin ]; then
  while [ $(($(date +%s) - copied_at)) -le 100 ]; do
    send pasteboard
    logged '^browser: pasteboard text $' && break
    sleep 1
  done
  expect '^browser: pasteboard text $' "the words were on the pasteboard still 100 s after the copy"
  kept=$(($(date +%s) - copied_at))
  [ "$kept" -ge 89 ] || fail "the words left the pasteboard $kept s after the copy, not 90 s"
fi

# Sign Out, as the account's only device, asks first, where return signs out: in Settings' dialog
# on Linux, and elsewhere in the system's alert, with Sign Out its default button. The alert has
# the keyboard once the driver logs it as a modal, AppKit's and the task dialog's alike.
post clickkey $(drawn "sync sign-out")
asked="Sign Out on the only device didn't ask first"
if [ "$(uname)" = Linux ]; then
  expect "^browser: dialog shown Sign out of $email?\$" "$asked"
  expect '^browser: dialog settled$' "Settings' dialog didn't settle"
else
  [ "$(uname)" != Darwin ] || expect "^browser: alert shown Sign out of $email?\$" "$asked"
  expect "^browser: modal shown Sign out of $email?, This is the account's only device\\. " "$asked"
fi
# Not send: the alert runs its own loop until it closes, so the driver may only finish after.
post key return
await '^browser: account SignedOut None$' || fail "Sign Out didn't sign out"
../tools/window-screenshot.sh "$pid" "$out/sync-signed-out-again.png" || fail "no window to capture"
for _ in $(seq 50); do
  left=$(stored "SELECT count(*) FROM devices")
  [ "$left" = 0 ] && break
  sleep 0.2
done
[ "$left" = 0 ] || fail "signed out, the server still knows the device"
exec 3>&-
quit
[ -s "$netlog" ] || fail "Chromium wrote no net log"
urls=$(grep -o '"url":"[a-z]*://[^"]*"' "$netlog" | cut -d'"' -f4 | sort -u)
# Google's servers, but the security components' updates, which a run as long as the Mac's, with
# the words' 90 s on the clipboard, reaches.
google=$(printf '%s\n' "$urls" | google_asked)
[ -z "$google" ] ||
  fail "the browser asked Google's servers: $(printf "%s\n" "$google" | tr "\n" " ")"
kill "$helper" 2>/dev/null || true
helper=
echo "ok: signed in to a local ilo-server as $email, synced ($store) and signed out, nothing" \
  "sent to Google's servers. Log: $log, the server's: $server_log"
