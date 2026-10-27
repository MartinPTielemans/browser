# The check's web server, which serve in lib.sh starts for the runs that load its pages. Its
# page's text field, along its bottom, puts what's in it in the title, and an unseen link in its
# top-left 40 points goes to a text file, and so does another, 200 by 40 points, 100 in from the page's
# left and top, which the link target's plate shows the address of. "d" typed in the page downloads a file, "q" the same
# file as another, "s" one that takes longer than the run to come, and "k" opens the cookie page,
# once a check, as later runs type in it too. "x" and "z" each open check.crx, an extension the
# extensions run puts there, which Chrome installs from. "a" asks to manage windows on all the
# displays, a permission with no question from macOS, and puts the answer in the title. Keys with
# ⌘, ⌃ or ⌥ are the browser's. /wait is a page that never comes. login.html is a login form, its
# username field across the page's top half and its password field across the bottom, that posts
# to /login, which sends the browser on to welcome.html, as a site that took the login would.
mkdir "$out/site"
printf '%s' '<title>loading</title><link rel="icon" href="icon.png">
<textarea id=text style="position:fixed;left:0;bottom:0;width:100%;height:60px"></textarea>
<a id=d download href="notes.txt"></a><a id=s download href="slow.bin"></a>
<a id=q download="trash.txt" href="notes.txt"></a>
<a id=k href="clear.html"></a><a id=x href="check.crx"></a><a id=z href="check.crx?again"></a>
<a href="notes.txt" style="position:fixed;left:0;top:0;width:40px;height:40px"></a>
<a href="notes.txt" style="position:fixed;left:100px;top:100px;width:200px;height:40px"></a>
<script>
requestAnimationFrame(() => setTimeout(() => document.title = "ready"));
text.oninput = () => document.title = "value " + text.value;
onkeydown = e => {
  if (e.metaKey || e.ctrlKey || e.altKey) {
    return;
  }
  if (e.key == "a" && e.target == document.body) {
    document.title = "asking";
    getScreenDetails().then(
      () => document.title = "granted", error => document.title = "refused " + error.name);
  }
  const link = e.target == document.body && document.getElementById(e.key);
  if (link && !localStorage[e.key]) {
    localStorage[e.key] = 1;
    link.click();
  }
};
</script>' > "$out/site/index.html"
printf 'notes\n' > "$out/site/notes.txt"
# Its title says "login" once it's drawn, and "filled" once Chrome has filled in a saved password.
printf '%s' '<title>loading</title>
<form method="post" action="login">
<input id=user name=username autocomplete=username
  style="position:fixed;left:0;top:0;width:100%;height:50%;font:24px system-ui">
<input id=pass name=password type=password autocomplete=current-password
  style="position:fixed;left:0;top:50%;width:100%;height:50%;font:24px system-ui">
<button style="position:fixed;right:0;bottom:0;width:1px;height:1px"></button>
</form>
<script>
requestAnimationFrame(() => setTimeout(() => document.title = "login"));
const filled = setInterval(() => {
  if (pass.matches(":autofill")) {
    clearInterval(filled);
    document.title = "filled";
  }
}, 100);
</script>' > "$out/site/login.html"
printf '%s' '<title>welcome</title><p style="font:24px system-ui">Welcome</p>' \
  > "$out/site/welcome.html"
# The cookie page's title shows the site's cookie and how many times the server has sent its
# script, which the cache keeps for an hour. "c" in it sets a cookie.
printf '%s' '<title>loading</title><script src="count.js"></script>
<script>
document.title = "cookie " + (document.cookie || "none") + " cache " + count;
onkeydown = e => {
  if (e.key == "c") {
    document.cookie = "c=1";
    document.title = "cookie set";
  }
};
</script>' > "$out/site/clear.html"
# Pages that ask for a key system as soon as they load, Widevine, which ilo can't play, and ClearKey,
# which it can. Each puts "asked" in its title once the request is answered, either way. Widevine's
# is asked from a promise, so the title comes in an engine with no request, as Safari's may have none
# for this page. Widevine's first asks the server for /seen with the item the session page below
# stored, as its script found it.
printf '%s' '<title>loading</title><script>
fetch("seen?item=" + localStorage.item);
Promise.resolve().then(() => navigator.requestMediaKeySystemAccess("com.widevine.alpha", [{
  initDataTypes: ["cenc"],
  videoCapabilities: [{contentType: `video/mp4; codecs="avc1.42E01E"`}],
}])).catch(() => 0).then(() => document.title = "drm asked");
</script>' > "$out/site/drm.html"
printf '%s' '<title>loading</title><script>
navigator.requestMediaKeySystemAccess("org.w3.clearkey", [{
  initDataTypes: ["webm"],
  videoCapabilities: [{contentType: `video/webm; codecs="vp8"`}],
}]).catch(() => 0).then(() => document.title = "clearkey asked");
</script>' > "$out/site/clearkey.html"
# A page that names the engine showing it: Safari's sends Safari's user agent, without Chrome's name.
# A text field covers it, so a click anywhere in it lands there, and the field's title says what it
# did: a press, counted, its value, what ⌘A selected and ⌘C copied, and a committed composition.
printf '%s' '<title>loading</title>
<input id=f style="position:fixed;left:0;top:0;width:100%;height:100%;box-sizing:border-box">
<script>
document.title = "engine " + (/Chrome\//.test(navigator.userAgent) ? "chromium" : "safari");
let presses = 0;
onmousedown = () => document.title = "pressed " + ++presses;
const selected = () => f.value.slice(f.selectionStart, f.selectionEnd);
f.oninput = e => e.isComposing || (document.title = "value " + f.value);
f.onselect = document.onselectionchange = () => selected() && (document.title = "select " + selected());
oncopy = () => document.title = "copy " + selected();
f.addEventListener("compositionend", e => document.title = "committed " + e.data);
</script>' > "$out/site/engine.html"
# The session page, whose title says the engine showing it, the cookies its script sees, sorted,
# and the items "item" and "back" in its localStorage. At /session?set the server sets an HttpOnly
# cookie, h, and the script a cookie, n, and the item; at /session?webkit the server sets hw and
# the script w and back. Each cookie's value is the server's port, so a run sees only its own.
printf '%s' '<title>loading</title><script>
const port = location.port;
if (location.search == "?set") {
  document.cookie = `n=${port}; path=/; max-age=3600`;
  localStorage.item = "chromium";
} else if (location.search == "?webkit") {
  document.cookie = `w=${port}; path=/; max-age=3600`;
  localStorage.back = "safari";
}
const cookies = document.cookie.split("; ").filter(Boolean).sort().join(" ") || "none";
document.title = "session " + (/Chrome\//.test(navigator.userAgent) ? "chromium" : "safari") +
  " sees " + cookies + " item " + (localStorage.item || "none") + " back " + (localStorage.back || "none");
</script>' > "$out/site/session.html"
# A page whose whole area is a link to /bounce, which the server redirects to another site,
# localhost, and from there, at hop.html, back to this one's engine page, as a sign-in through an
# identity site does, or at hop.html?away, to localhost's engine page, where it stays.
printf '%s' '<title>loading</title><a id=a style="position:fixed;inset:0"></a><script>
const other = "http://localhost:" + location.port;
const back = "http://127.0.0.1:" + location.port + "/engine.html";
const to = location.search == "?away" ? other + "/engine.html" :
  other + "/bounce?to=" + encodeURIComponent(back);
a.href = "/bounce?to=" + encodeURIComponent(to);
document.title = "hop " + (/Chrome\//.test(navigator.userAgent) ? "chromium" : "safari");
</script>' > "$out/site/hop.html"
printf '%s' iVBORw0KGgoAAAANSUhEUgAAABAAAAAQCAYAAAAf8/9hAAAAGUlEQVR42mN44GDwnxLMMGrAqAGjBgwXAwCb4E8fp7RFqwAAAABJRU5ErkJggg== |
  base64 -D > "$out/site/icon.png"
# The server logs each GET as "GET <engine> <host><path> cookie <cookies sorted, or none>", the
# engine named by the user agent as the engine page does.
python3 -I -u -c '
import functools, http.server, sys, time, urllib.parse

class Handler(http.server.SimpleHTTPRequestHandler):
    sent = 0
    # Chrome takes an extension only with its type.
    extensions_map = {
        **http.server.SimpleHTTPRequestHandler.extensions_map,
        ".crx": "application/x-chrome-extension",
    }

    def do_GET(self):
        agent = self.headers.get("User-Agent", "")
        cookies = sorted(c.strip() for c in self.headers.get("Cookie", "").split(";") if c.strip())
        print("GET", "chromium" if "Chrome/" in agent else "safari",
              self.headers.get("Host", "") + self.path, "cookie", "; ".join(cookies) or "none")
        path, _, query = self.path.partition("?")
        if path == "/bounce":
            self.send_response(302)
            self.send_header("Location", urllib.parse.parse_qs(query)["to"][0])
            self.send_header("Content-Length", "0")
            self.end_headers()
            return
        if path == "/session":
            body = open(sys.argv[1] + "/session.html", "rb").read()
            self.send_response(200)
            self.send_header("Content-Type", "text/html")
            self.send_header("Content-Length", str(len(body)))
            name = {"set": "h", "webkit": "hw"}.get(query)
            if name:
                self.send_header("Set-Cookie", "%s=%d; Path=/; Max-Age=3600; HttpOnly; SameSite=Lax"
                                 % (name, self.server.server_address[1]))
            self.end_headers()
            self.wfile.write(body)
            return
        if self.path == "/count.js":
            Handler.sent += 1
            body = b"var count = %d;" % Handler.sent
            self.send_response(200)
            self.send_header("Content-Type", "text/javascript")
            self.send_header("Content-Length", str(len(body)))
            self.send_header("Cache-Control", "max-age=3600")
            self.end_headers()
            self.wfile.write(body)
            return
        if self.path == "/wait":
            time.sleep(3600)
            return
        if self.path != "/slow.bin":
            return super().do_GET()
        self.send_response(200)
        self.send_header("Content-Type", "application/octet-stream")
        self.send_header("Content-Length", "100000000")
        self.end_headers()
        try:
            while True:
                self.wfile.write(bytes(1000))
                time.sleep(0.1)
        except OSError:
            pass

    def do_POST(self):
        self.rfile.read(int(self.headers.get("Content-Length", 0)))
        self.send_response(303)
        self.send_header("Location", "welcome.html")
        self.send_header("Content-Length", "0")
        self.end_headers()

handler = functools.partial(Handler, directory=sys.argv[1])
server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler)
print("Serving HTTP on 127.0.0.1 port", server.server_address[1], "...")
server.serve_forever()
' "$out/site" > "$out/site.log" 2>&1 &
server=$!
for _ in $(seq 50); do
  grep -q '^Serving HTTP' "$out/site.log" && break
  sleep 0.1
done
port=$(sed -n 's/^Serving HTTP on .* port \([0-9]*\) .*/\1/p' "$out/site.log")
[ -n "$port" ] || fail "the page server didn't start"
site="http://127.0.0.1:$port/"
