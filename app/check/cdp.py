"""The passkeys run's DevTools client (check/passkeys.sh), over the pipe --remote-debugging-pipe
gives the browser: the browser reads from IN and writes to OUT, each message JSON ending in NUL.

It attaches to the page whose address ends in PAGE and gives it a virtual authenticator, through
DevTools' WebAuthn domain, with the browser's own passkey dialogs: a USB security key that keeps
passkeys, verifies its user and is touched for each request. It logs on its standard output:
  cdp: authenticator ready
  cdp: credential added | credential asserted     as the page registers and signs in
  cdp: presence 0|1                                once a line "presence 0|1" on its standard
                                                   input stops or starts the touches
The domain is for the check alone. Agents never get DevTools: it runs any script in any page.

Usage: cdp.py IN OUT PAGE
"""

import json
import sys
import threading

to_browser = open(sys.argv[1], "wb")
from_browser = open(sys.argv[2], "rb")
page = sys.argv[3]
lock = threading.Lock()
replies = {}
answered = threading.Condition(lock)
events = []
sent = 0


def log(line):
    print("cdp: " + line, flush=True)


def call(method, params=None, session=None):
    global sent
    with lock:
        sent += 1
        number = sent
    message = {"id": number, "method": method, "params": params or {}}
    if session:
        message["sessionId"] = session
    to_browser.write(json.dumps(message).encode() + b"\0")
    to_browser.flush()
    with answered:
        answered.wait_for(lambda: number in replies)
        reply = replies.pop(number)
    if "error" in reply:
        raise SystemExit("cdp: %s failed: %s" % (method, reply["error"]))
    return reply["result"]


def read():
    buffer = b""
    while True:
        chunk = from_browser.read1(65536)
        if not chunk:
            with answered:
                events.append(None)
                answered.notify_all()
            return
        buffer += chunk
        while b"\0" in buffer:
            raw, buffer = buffer.split(b"\0", 1)
            message = json.loads(raw)
            with answered:
                if "id" in message:
                    replies[message["id"]] = message
                else:
                    events.append(message)
                    if message["method"] == "WebAuthn.credentialAdded":
                        log("credential added")
                    elif message["method"] == "WebAuthn.credentialAsserted":
                        log("credential asserted")
                answered.notify_all()


threading.Thread(target=read, daemon=True).start()
# The page's target, once it's there: the browser may start before the page has.
call("Target.setDiscoverTargets", {"discover": True})


def page_target():
    for event in events:
        if event is None:
            raise SystemExit("cdp: the browser closed the pipe")
        if event["method"] in ("Target.targetCreated", "Target.targetInfoChanged"):
            info = event["params"]["targetInfo"]
            if info["type"] == "page" and info["url"].endswith(page):
                return info["targetId"]
    return None


with answered:
    answered.wait_for(page_target)
    target = page_target()
session = call("Target.attachToTarget", {"targetId": target, "flatten": True})["sessionId"]
call("WebAuthn.enable", {"enableUI": True}, session)
authenticator = call("WebAuthn.addVirtualAuthenticator", {"options": {
    "protocol": "ctap2",
    "transport": "usb",
    "hasResidentKey": True,
    "hasUserVerification": True,
    "isUserVerified": True,
    "automaticPresenceSimulation": True,
}}, session)["authenticatorId"]
log("authenticator ready")
for line in sys.stdin:
    words = line.split()
    if words[:1] == ["presence"] and words[1:] in (["0"], ["1"]):
        call("WebAuthn.setAutomaticPresenceSimulation",
             {"authenticatorId": authenticator, "enabled": words[1] == "1"}, session)
        log("presence " + words[1])
