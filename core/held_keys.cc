#include "browser_core/held_keys.h"

#include <cstdint>
#include <cstdio>
#include <set>
#include <string>
#include <utility>
#include <vector>

#include "base/functional/bind.h"
#include "base/no_destructor.h"
#include "base/time/time.h"
#include "base/timer/timer.h"
#include "browser_core/tab.h"
#include "components/input/native_web_keyboard_event.h"
#include "content/public/browser/render_frame_host.h"
#include "content/public/browser/render_widget_host.h"
#include "content/public/browser/web_contents.h"
#include "third_party/blink/public/common/input/web_input_event.h"

namespace browser_core::held_keys {

namespace {

// The Mac's host holds them as long (kHoldKeys): past it, the page kept the shortcut.
constexpr base::TimeDelta kHoldFor = base::Milliseconds(400);

struct State {
  // The page whose keys are held, or 0.
  bc_id tab = 0;
  // The presses held, in the order typed.
  std::vector<input::NativeWebKeyboardEvent> keys;
  // Their keys, whose releases are held too, so the page never gets a release before its press.
  std::set<int> down;
  base::OneShotTimer timer;
};

bc_id g_asking = 0;

State& Held() {
  static base::NoDestructor<State> state;
  return *state;
}

// Ends the hold. Returns the page whose keys were held and the presses.
std::pair<bc_id, std::vector<input::NativeWebKeyboardEvent>> End() {
  State& held = Held();
  held.timer.Stop();
  held.down.clear();
  return {std::exchange(held.tab, 0), std::move(held.keys)};
}

// The page's focused frame's widget, which a key goes to, or null once the page went.
content::RenderWidgetHost* WidgetOf(bc_id tab) {
  content::WebContents* contents = Tab::FromId(tab);
  if (!contents) {
    return nullptr;
  }
  content::RenderFrameHost* frame = contents->GetFocusedFrame();
  return (frame ? frame : contents->GetPrimaryMainFrame())->GetRenderWidgetHost();
}

// Nothing took the keys, as the page kept the shortcut, so the page gets them as typed. Each press
// comes with the character it types and its release, which never reached the page: Chromium drops
// what follows a press the browser kept.
void GiveToPage() {
  auto [tab, keys] = End();
  if (!keys.empty()) {
    // check.sh reads this line.
    fprintf(stderr, "browser: held %zu keys went to the page\n", keys.size());
  }
  for (const input::NativeWebKeyboardEvent& press : keys) {
    // A key can close the page.
    content::RenderWidgetHost* widget = WidgetOf(tab);
    if (!widget) {
      return;
    }
    widget->ForwardKeyboardEvent(press);
    const char16_t typed = press.text[0];
    const int shortcut = blink::WebInputEvent::kControlKey | blink::WebInputEvent::kMetaKey;
    if (typed >= 0x20 && typed != 0x7f && !(press.GetModifiers() & shortcut)) {
      input::NativeWebKeyboardEvent character = press;
      character.SetType(blink::WebInputEvent::Type::kChar);
      character.windows_key_code = typed;
      widget->ForwardKeyboardEvent(character);
    }
    input::NativeWebKeyboardEvent release = press;
    release.SetType(blink::WebInputEvent::Type::kKeyUp);
    widget->ForwardKeyboardEvent(release);
  }
}

}  // namespace

base::AutoReset<bc_id> AskingAbout(bc_id tab) {
  return base::AutoReset<bc_id>(&g_asking, tab);
}

void Hold() {
  if (!g_asking) {
    return;
  }
  // Another page's hold, which can't be the field's, ends as if nothing took its keys.
  if (Held().tab && Held().tab != g_asking) {
    GiveToPage();
  }
  State& held = Held();
  held.tab = g_asking;
  held.timer.Start(FROM_HERE, kHoldFor, base::BindOnce(&GiveToPage));
}

bool Holds(bc_id tab, const input::NativeWebKeyboardEvent& event) {
  State& held = Held();
  if (!held.tab || tab != held.tab) {
    return false;
  }
  switch (event.GetType()) {
    case blink::WebInputEvent::Type::kRawKeyDown:
      held.keys.push_back(event);
      held.down.insert(event.dom_code);
      return true;
    case blink::WebInputEvent::Type::kKeyUp:
      return held.down.contains(event.dom_code);
    default:
      // The character a press the page got types, such as the shortcut's.
      return false;
  }
}

void Give(void (*take)(void* context, const bc_key* keys, size_t count), void* context) {
  auto [tab, keys] = End();
  if (keys.empty() || !take) {
    return;
  }
  std::vector<std::string> names;
  std::vector<uint32_t> modifiers(keys.size());
  for (size_t i = 0; i < keys.size(); i++) {
    names.push_back(Tab::KeyName(keys[i], &modifiers[i]));
  }
  // Not the page's: they go to the app's field.
  std::vector<bc_key> given;
  for (size_t i = 0; i < keys.size(); i++) {
    given.push_back({names[i].c_str(), modifiers[i], false});
  }
  take(context, given.data(), given.size());
}

}  // namespace browser_core::held_keys
