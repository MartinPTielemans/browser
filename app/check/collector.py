# A crash collector for the crash run, as GlitchTip's minidump endpoint takes reports: a multipart
# POST whose upload_file_minidump part is the minidump. It listens on a free port of 127.0.0.1,
# which it writes to the file it's given, and logs each request on stdout: "report <bytes> <path>"
# for one it takes, "refused <why>" for one it doesn't, and refuses everything under /refuse, as a
# collector that's down does.
import email.parser
import email.policy
import http.server
import sys


class Collector(http.server.BaseHTTPRequestHandler):
    def do_POST(self):
        body = self.rfile.read(int(self.headers.get("Content-Length", 0)))
        why = self.refusal(body)
        print(f"refused {why}" if why else f"report {len(body)} {self.path}", flush=True)
        self.send_response(503 if why else 200)
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        self.wfile.write(b"{}" if why else b'{"id": "check"}')

    def refusal(self, body):
        if self.path.startswith("/refuse"):
            return "as asked"
        # A report goes without the user's cookies.
        if "Cookie" in self.headers:
            return "a cookie"
        message = email.parser.BytesParser(policy=email.policy.HTTP).parsebytes(
            b"Content-Type: " + self.headers.get("Content-Type", "").encode() + b"\r\n\r\n" + body
        )
        if not message.is_multipart():
            return "no multipart body"
        parts = list(message.iter_parts())
        names = [part.get_param("name", header="content-disposition") for part in parts]
        if names != ["upload_file_minidump"]:
            return f"parts {names}"
        if not parts[0].get_payload(decode=True).startswith(b"MDMP"):
            return "no minidump"
        return None

    def log_message(self, *_):
        pass


server = http.server.HTTPServer(("127.0.0.1", 0), Collector)
with open(sys.argv[1], "w") as port:
    port.write(str(server.server_port))
server.serve_forever()
