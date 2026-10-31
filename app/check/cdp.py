"""The passkeys and passwords runs' DevTools client (check/passkeys.sh, check/passwords.sh), over
the pipe --remote-debugging-pipe gives the browser: the browser reads from IN and writes to OUT,
each message JSON ending in NUL.

It attaches to the page whose address ends in PAGE, logging "cdp: attached", then takes a command
a line on its standard input:
  authenticator   gives the page a virtual authenticator, through DevTools' WebAuthn domain, with
                  the browser's own passkey dialogs: a USB security key that keeps passkeys,
                  verifies its user and is touched for each request. It logs
                    cdp: authenticator ready
                    cdp: credential added | credential asserted   as the page registers, signs in
  presence 0|1    stops or starts the key's touches, then logs "cdp: presence 0|1"
  reveal ID       logs "cdp: reveal shown|hidden|none": whether the eye Blink draws in the
                  password field with the id ID shows, from the field's user-agent shadow tree,
                  which the page's script can't see
  shows PREFIX TEXT
                  logs "cdp: shows yes|no|none": whether the first page at an address starting
                  with PREFIX has TEXT in a text node or an attribute, shadow trees included, or
                  none for no such page, such as Chrome's password manager page, a WebUI page
The domains are for the checks alone. Agents never get DevTools: it runs any script in any page.

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


def page_target(wanted=lambda url: url.endswith(page)):
    for event in events:
        if event is None:
            raise SystemExit("cdp: the browser closed the pipe")
        if event["method"] in ("Target.targetCreated", "Target.targetInfoChanged"):
            info = event["params"]["targetInfo"]
            if info["type"] == "page" and wanted(info["url"]):
                return info["targetId"]
    return None


with answered:
    answered.wait_for(page_target)
    target = page_target()
session = call("Target.attachToTarget", {"targetId": target, "flatten": True})["sessionId"]
log("attached")
authenticator = None
# Sessions on the other pages shows looked in, by their targets.
sessions = {}


def attributes(node):
    values = node.get("attributes", [])
    return dict(zip(values[::2], values[1::2]))


def find(node, wanted):
    """The first node in NODE's tree, shadow trees and frames' documents included, WANTED takes."""
    if wanted(node):
        return node
    inner = node.get("shadowRoots", []) + node.get("children", [])
    if "contentDocument" in node:
        inner.append(node["contentDocument"])
    for child in inner:
        found = find(child, wanted)
        if found:
            return found
    return None


def reveal(field_id):
    root = call("DOM.getDocument", {"depth": -1, "pierce": True}, session)["root"]
    field = find(root, lambda node: node.get("nodeName") == "INPUT"
                 and attributes(node).get("id") == field_id)
    eye = field and find(field, lambda node: attributes(node).get("id") == "password-reveal")
    if not eye:
        return "none"
    # Blink hides it with an inline display: none, and takes that away to show it.
    return "hidden" if "display: none" in attributes(eye).get("style", "") else "shown"


def shows(prefix, text):
    with answered:
        target = page_target(lambda url: url.startswith(prefix))
    if not target:
        return "none"
    if target not in sessions:
        attached = call("Target.attachToTarget", {"targetId": target, "flatten": True})
        sessions[target] = attached["sessionId"]
    root = call("DOM.getDocument", {"depth": -1, "pierce": True}, sessions[target])["root"]
    found = find(root, lambda node: text in node.get("nodeValue", "")
                 or any(text in value for value in attributes(node).values()))
    return "yes" if found else "no"


for line in sys.stdin:
    words = line.split()
    if words == ["authenticator"] and not authenticator:
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
    elif words[:1] == ["presence"] and words[1:] in (["0"], ["1"]) and authenticator:
        call("WebAuthn.setAutomaticPresenceSimulation",
             {"authenticatorId": authenticator, "enabled": words[1] == "1"}, session)
        log("presence " + words[1])
    elif words[:1] == ["reveal"] and len(words) == 2:
        log("reveal " + reveal(words[1]))
    elif words[:1] == ["shows"] and len(words) == 3:
        log("shows " + shows(words[1], words[2]))
