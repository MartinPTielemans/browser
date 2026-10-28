"""The appcast's entries for every OS, around generate_appcast, which knows only the Mac's.

Sparkle takes an enclosure marked "macos" or not marked at all, and WinSparkle one marked "windows…"
or not marked at all, so every Mac enclosure is marked. tools/appcast.sh runs it:

  appcast.py aside APPCAST ASIDE
      Moves the Windows entries to ASIDE, so generate_appcast sees only the Mac's, and prints how
      many Mac entries there are.
  appcast.py signed APPCAST BUILD
      Fails unless the Mac's entry for BUILD carries its EdDSA signature.
  appcast.py finish APPCAST ASIDE [--windows URL LENGTH SIGNATURE VERSION BUILD NOTES]
      Marks the Mac enclosures and puts the Windows entries back after them, with a new one for
      the installer at URL, which replaces one with the same build number.
"""

import email.utils
import sys
import xml.etree.ElementTree as ET

SPARKLE = "http://www.andymatuschak.org/xml-namespaces/sparkle"
ET.register_namespace("sparkle", SPARKLE)
ET.register_namespace("dc", "http://purl.org/dc/elements/1.1/")


def s(name):
    return f"{{{SPARKLE}}}{name}"


def os_of(item):
    enclosure = item.find("enclosure")
    return "" if enclosure is None else enclosure.get(s("os"), "")


def build_of(item):
    version = item.findtext(s("version"))
    enclosure = item.find("enclosure")
    if version is None and enclosure is not None:
        version = enclosure.get(s("version"))
    return version


def write(tree, path):
    ET.indent(tree, space="    ")
    tree.write(path, encoding="utf-8", xml_declaration=True)


def aside(appcast, aside_path):
    tree = ET.parse(appcast)
    channel = tree.getroot().find("channel")
    windows = [item for item in channel.findall("item") if os_of(item).startswith("windows")]
    for item in windows:
        channel.remove(item)
    rss = ET.Element("rss", {"version": "2.0"})
    ET.SubElement(rss, "channel").extend(windows)
    write(ET.ElementTree(rss), aside_path)
    write(tree, appcast)
    print(len(channel.findall("item")))


def signed(appcast, build):
    for item in ET.parse(appcast).getroot().find("channel").findall("item"):
        enclosure = item.find("enclosure")
        if build_of(item) == build and enclosure is not None and enclosure.get(s("edSignature")):
            return
    sys.exit(f"{appcast} has no signed update for build {build}")


def finish(appcast, aside_path, windows):
    tree = ET.parse(appcast)
    channel = tree.getroot().find("channel")
    for item in channel.findall("item"):
        enclosure = item.find("enclosure")
        if enclosure is not None and not enclosure.get(s("os")):
            enclosure.set(s("os"), "macos")
    kept = ET.parse(aside_path).getroot().find("channel").findall("item")
    if windows:
        url, length, signature, version, build, notes = windows
        kept = [item for item in kept if build_of(item) != build]
        item = ET.Element("item")
        ET.SubElement(item, "title").text = version
        ET.SubElement(item, "pubDate").text = email.utils.formatdate(usegmt=True)
        ET.SubElement(item, s("version")).text = build
        ET.SubElement(item, s("shortVersionString")).text = version
        # WinSparkle shows HTML, not the Mac's Markdown, so it gets the release's page.
        ET.SubElement(item, s("releaseNotesLink")).text = notes
        ET.SubElement(item, "enclosure", {
            "url": url,
            s("os"): "windows-x64",
            "length": length,
            "type": "application/octet-stream",
            s("edSignature"): signature,
        })
        kept.insert(0, item)
    channel.extend(kept)
    write(tree, appcast)


if __name__ == "__main__":
    command, args = sys.argv[1], sys.argv[2:]
    if command == "aside" and len(args) == 2:
        aside(*args)
    elif command == "signed" and len(args) == 2:
        signed(*args)
    elif command == "finish" and len(args) == 2:
        finish(*args, None)
    elif command == "finish" and len(args) == 9 and args[2] == "--windows":
        finish(args[0], args[1], args[3:])
    else:
        sys.exit(__doc__)
