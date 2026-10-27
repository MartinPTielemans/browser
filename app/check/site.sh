# The check's web server, which serve in lib.sh starts for the runs that load its pages. Its
# page's text field, along its bottom, puts what's in it in the title, and an unseen link in its
# top-left 40 points goes to a text file. "d" typed in the page downloads a file, "q" the same
# file as another, "s" one that takes longer than the run to come, and "k" opens the cookie page,
# once a check, as later runs type in it too. "x" and "z" each open check.crx, an extension the
# extensions run puts there, which Chrome installs from. "a" asks to manage windows on all the
# displays, a permission with no question from macOS, and puts the answer in the title. Keys with
# ⌘, ⌃ or ⌥ are the browser's. /wait is a page that never comes. /sign-in, with any query, asks
# for HTTP basic sign-in and takes ada with the password lovelace.
mkdir "$out/site"
printf '%s' '<title>loading</title><link rel="icon" href="icon.png">
<textarea id=text style="position:fixed;left:0;bottom:0;width:100%;height:60px"></textarea>
<a id=d download href="notes.txt"></a><a id=s download href="slow.bin"></a>
<a id=q download="trash.txt" href="notes.txt"></a>
<a id=k href="clear.html"></a><a id=x href="check.crx"></a><a id=z href="check.crx?again"></a>
<a href="notes.txt" style="position:fixed;left:0;top:0;width:40px;height:40px"></a>
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
# The dialogs page asks with "a" alert(), "c" confirm() and "p" prompt(), each from a key in the
# page, and puts what the user answered in the title. "u" has it ask before it's left.
printf '%s' '<title>loading</title>
<script>
requestAnimationFrame(() => setTimeout(() => document.title = "ready"));
onkeydown = e => {
  if (e.metaKey || e.ctrlKey || e.altKey) {
    return;
  }
  if (e.key == "a") {
    alert("The check says hello.");
    document.title = "alerted";
  }
  if (e.key == "c") {
    document.title = "confirm " + confirm("Go on?");
  }
  if (e.key == "p") {
    document.title = "prompt " + prompt("Your name?", "Ada");
  }
  if (e.key == "u") {
    onbeforeunload = e => e.preventDefault();
    document.title = "armed";
  }
};
</script>' > "$out/site/dialogs.html"
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
printf '%s' iVBORw0KGgoAAAANSUhEUgAAABAAAAAQCAYAAAAf8/9hAAAAGUlEQVR42mN44GDwnxLMMGrAqAGjBgwXAwCb4E8fp7RFqwAAAABJRU5ErkJggg== |
  base64 -D > "$out/site/icon.png"
python3 -I -u -c '
import base64, functools, http.server, sys, time

class Handler(http.server.SimpleHTTPRequestHandler):
    sent = 0
    # Chrome takes an extension only with its type.
    extensions_map = {
        **http.server.SimpleHTTPRequestHandler.extensions_map,
        ".crx": "application/x-chrome-extension",
    }

    def do_GET(self):
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
        if self.path.split("?")[0] == "/sign-in":
            signed_in = self.headers.get("Authorization") == "Basic " + base64.b64encode(
                b"ada:lovelace").decode()
            body = b"<title>signed in ada</title>" if signed_in else b"<title>signed out</title>"
            self.send_response(200 if signed_in else 401)
            if not signed_in:
                self.send_header("WWW-Authenticate", "Basic realm=\"check\"")
            self.send_header("Content-Type", "text/html")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
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
