#include "browser_core/error_page.h"

#include <string>

#include "base/json/json_writer.h"
#include "base/strings/escape.h"
#include "base/strings/string_util.h"

namespace browser_core {

namespace {

// The page's look is ours (docs/ux-spec.md › Pages that break): the words in the middle of the
// pane, on its colour, which the tab gives the page (Tab::SetPageColor), at the sizes and colours of
// the app's cards. One script draws it from Chromium's strings, at first and again when Chromium's
// DNS probe sends new ones.
constexpr char kPage[] = R"(<!doctype html>
<html dir="$dir" lang="$lang">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>$title</title>
<style>
html, body { height: 100%; margin: 0; background: transparent; }
body {
  display: flex; align-items: center; justify-content: center;
  font: 14px system-ui; color: #fff; cursor: default; -webkit-user-select: none;
}
main {
  display: flex; flex-direction: column; align-items: center;
  max-width: 400px; padding: 0 24px; text-align: center;
}
h1 { margin: 0; font-size: 14px; font-weight: 500; }
#note { margin-top: 4px; font-size: 13px; color: rgb(255 255 255 / 0.6); }
button {
  margin-top: 12px; height: 28px; padding: 0 12px; border: 0; border-radius: 6px;
  background: rgb(255 255 255 / 0.1); color: #fff; font: 500 13px system-ui;
}
button:hover { background: rgb(255 255 255 / 0.12); }
#code { margin-top: 12px; font-size: 11px; color: rgb(255 255 255 / 0.32); }
[hidden] { display: none; }
</style>
</head>
<body>
<main>
<h1 id="heading"></h1>
<div id="note" hidden></div>
<button id="reload" hidden>Reload</button>
<div id="code"></div>
</main>
<script>
let reloadUrl = '';
// Chromium's heading is HTML, with the host in a tag: its text, which an inert template can't run.
function text(html) {
  const template = document.createElement('template');
  template.innerHTML = html || '';
  return template.content.textContent;
}
function show(strings) {
  const offline = !!strings.isOfflineError;
  document.getElementById('heading').textContent =
      offline ? 'You’re offline' : text(strings.heading && strings.heading.msg);
  const note = document.getElementById('note');
  // Chromium's auto-reload loads the page once the network is back, so there's nothing to press.
  note.textContent = 'Reloads when you’re back';
  note.hidden = !offline;
  reloadUrl = strings.reloadButton ? strings.reloadButton.reloadUrl : '';
  document.getElementById('reload').hidden = offline || !reloadUrl;
  document.getElementById('code').textContent = strings.errorCode || '';
}
document.getElementById('reload').addEventListener('click', () => {
  // Chromium's controller reloads as its own page's button does, else the page goes to the URL.
  if (window.errorPageController) {
    errorPageController.reloadButtonClick();
  } else if (reloadUrl) {
    location.href = reloadUrl;
  }
});
window.updateForDnsProbe = show;
show($strings);
</script>
</body>
</html>
)";

std::string Text(const base::DictValue& strings, const char* key) {
  const std::string* value = strings.FindString(key);
  return value ? *value : std::string();
}

}  // namespace

std::string ErrorPageHtml(const base::DictValue& strings) {
  std::string json = base::WriteJson(strings).value_or("{}");
  // In a script element, where "</script>" in a host or URL would end it.
  base::ReplaceSubstringsAfterOffset(&json, 0, "<", "\\u003c");
  std::string dir = Text(strings, "textdirection");
  if (dir != "rtl") {
    dir = "ltr";
  }
  std::string html = kPage;
  // Last first, so each placeholder's first match is its own, not one in a value put in before it.
  base::ReplaceFirstSubstringAfterOffset(&html, 0, "$strings", json);
  base::ReplaceFirstSubstringAfterOffset(&html, 0, "$title",
                                         base::EscapeForHTML(Text(strings, "title")));
  base::ReplaceFirstSubstringAfterOffset(&html, 0, "$lang",
                                         base::EscapeForHTML(Text(strings, "language")));
  base::ReplaceFirstSubstringAfterOffset(&html, 0, "$dir", dir);
  return html;
}

}  // namespace browser_core
