#ifndef BROWSER_CORE_WIDEVINE_H_
#define BROWSER_CORE_WIDEVINE_H_

#include <string>

namespace browser_core::widevine {

// Reads whether Chromium loaded Google's Widevine module at launch, and follows the component
// updater for it. Once, before the first profile.
void Start();

// Stops following it, before the component updater goes.
void Stop();

// g_browser_core_component_allowed: the component updater checks for the module, with the
// security components, only once the user said yes.
bool MayUpdate(const std::string& component);

// A BC_WIDEVINE_ value (bc_widevine).
int State();

// Whether pages can play what Widevine protects: Chromium loaded the module at launch. It still can
// after Remove, until the browser restarts.
bool Loaded();

// The user's yes, and their way back (bc_widevine_download, bc_widevine_remove).
void Download();
void Remove();

}  // namespace browser_core::widevine

#endif  // BROWSER_CORE_WIDEVINE_H_
