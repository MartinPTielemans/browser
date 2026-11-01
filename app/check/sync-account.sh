# Settings › Sync signs the browser in to an ilo account, on an ilo-server of the run's own: SQLite,
# DEV_MODE, which logs its sign-in codes, and a throwaway address, never the hosted server or a real
# account. ILO_SERVER names the server's binary, built from its repo (check-linux.yml builds it);
# without it the run skips. Sign In opens the server's page in the core's window, where the run
# types the address and the code the server logged, and declines a passkey. The account's first
# device makes the recovery words, which Done hides, and sync turns on. Then what the server keeps:
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
db="$out/sync-account-server.db"
step=0
rm -f "$saved" "$launch_frame" "$profile_dir/Account" "$netlog" "$db"
server_log="$out/sync-account-server.log"
# One answer from the server's database: the query $1.
stored() {
  python3 -I -c '
import sqlite3, sys
print(sqlite3.connect(sys.argv[1]).execute(sys.argv[2]).fetchone()[0])' "$db" "$1"
}
server_port=$(python3 -I -c '
import socket
s = socket.socket()
s.bind(("127.0.0.1", 0))
print(s.getsockname()[1])')
# localhost, as passkeys, which the server offers, take no address for their origin.
server_url="http://localhost:$server_port"
# helper, so cleanup stops it after a failure.
DEV_MODE=1 HOST=127.0.0.1 PORT=$server_port PUBLIC_URL=$server_url NO_COLOR=1 \
  DATABASE_URL="sqlite:$db?mode=rwc" "$ILO_SERVER" > "$server_log" 2>&1 3>&- &
helper=$!
for _ in $(seq 100); do
  grep -q 'ilo-server is up' "$server_log" && break
  kill -0 "$helper" 2>/dev/null || fail "ilo-server exited: $(tail -1 "$server_log")"
  sleep 0.1
done
grep -q 'ilo-server is up' "$server_log" || fail "ilo-server didn't start"
email="check-$(date +%s)@example.org"
# What the core calls this device to the server and the account's other devices.
label="ilo on Linux"
[ "$(uname)" = Darwin ] && label="ilo on Mac"
! windows || label="ilo on Windows"

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
# Keys for the sign-in window, a window of Chromium's own, which the driver's keys don't reach on
# Linux, as they go to the browser windows, overlays and Settings alone: there they're the X
# server's, from xdotool, to the sign-in window, which it gives the keyboard first. The Mac's go
# through the driver (unrun there).
sign_in_window=
sign_in_keys() {
  if [ "$(uname)" != Linux ]; then
    send "$@"
    return
  fi
  if [ -z "$sign_in_window" ]; then
    sign_in_window=$(timeout 10 xdotool search --sync --onlyvisible --name '^localhost$' | head -1)
    [ -n "$sign_in_window" ] || fail "the X server has no sign-in window"
  fi
  xdotool windowactivate --sync "$sign_in_window" 2>/dev/null ||
    xdotool windowfocus --sync "$sign_in_window" ||
    fail "the sign-in window couldn't have the keyboard"
  case $1 in
  type) xdotool type --delay 20 "$2" ;;
  key) xdotool key "$(echo "$2" | sed 's/return/Return/; s/tab/Tab/')" ;;
  esac
}
# The page focuses its field once it's there.
sign_in_keys type "$email"
[ -z "$sign_in_window" ] || import -window "$sign_in_window" "$out/sync-sign-in-address.png" || true
sign_in_keys key return
await 'browser: sync sign-in loaded /authorize/email$' || fail "the server didn't take the address"
code=
for _ in $(seq 100); do
  code=$(grep -o '[0-9]\{6\} is your ilo sign-in code' "$server_log" | tail -1 | cut -c1-6)
  [ -n "$code" ] && break
  sleep 0.1
done
[ -n "$code" ] || fail "ilo-server logged no sign-in code"
sign_in_keys type "$code"
sign_in_keys key return
# A new account's first sign-in offers a passkey, with Add a Passkey focused: Not Now is next.
await 'browser: sync sign-in loaded /authorize/code$' || fail "the server didn't take the code"
sign_in_keys key tab
sign_in_keys key return
await '^browser: account FirstDevice None$' ||
  fail "signing in didn't make this the account's first device"
await '^browser: account On None$' || fail "the first device's recovery words didn't turn sync on"
# The sign-in window has closed. On Linux, Settings has the keyboard again as the window manager
# would give it, from xdotool, as Settings' window is titled by its tab; the Mac's AppKit gives it.
if [ "$(uname)" = Linux ]; then
  settings=$(timeout 10 xdotool search --onlyvisible --name '^Sync$' | head -1)
  [ -n "$settings" ] || fail "the X server has no Settings window"
  xdotool windowactivate --sync "$settings" 2>/dev/null || xdotool windowfocus --sync "$settings" ||
    fail "Settings couldn't have the keyboard again"
fi
at=$(drawn "sync done") || fail "Settings › Sync didn't show the recovery words with Done"
../tools/window-screenshot.sh "$pid" "$out/sync-words.png" || fail "no window to capture"
send clickkey $at
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

# Sign Out, as the account's only device, asks first in Settings' dialog, where return signs out.
post clickkey $(drawn "sync sign-out")
expect "^browser: dialog shown Sign out of $email?\$" "Sign Out on the only device didn't ask first"
expect '^browser: dialog settled$' "Settings' dialog didn't settle"
send key return
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
google=$(printf '%s\n' "$urls" | grep -i '^[a-z]*://[^/]*\(google\|gstatic\|gvt1\|9oo91e\|qjz9zk\)' || true)
[ -z "$google" ] ||
  fail "the browser asked Google's servers: $(printf "%s\n" "$google" | tr "\n" " ")"
kill "$helper" 2>/dev/null || true
helper=
echo "ok: signed in to a local ilo-server as $email, synced ($store) and signed out, nothing" \
  "sent to Google's servers. Log: $log, the server's: $server_log"
