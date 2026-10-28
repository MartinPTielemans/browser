#ifndef BROWSER_CORE_AGENT_AUTOFILL_H_
#define BROWSER_CORE_AGENT_AUTOFILL_H_

#include "browser_core/public/browser_core.h"

namespace content {
class WebContents;
}

namespace browser_core {

// Keeps the user's saved passwords, addresses, cards and passkeys out of an agent's tab: a tab in
// an agent's Space, which the user doesn't hold. Chrome fills nothing there, offers nothing in its
// dropdown and offers to save nothing, through the hooks engine/patches/ours/
// chromium-core-agent-no-autofill.patch adds, as Chrome gates its own Actor. A page asking for a
// passkey is a handoff: the user holds the tab and picks theirs in Chrome's dialog.

// Whether |contents| is an agent's tab. g_browser_core_no_passwords and g_browser_core_no_autofill.
bool IsAgentTab(content::WebContents* contents);

// g_browser_core_refuse_passkey: whether Chrome is to cancel |contents|'s passkey request. One that
// |asks| the user in Chrome's dialog, in an agent's tab, holds the tab from the agent instead.
bool RefuseAgentPasskey(content::WebContents* contents, bool asks);

// |tab| has just become the user's, as when they hold it or move it out of an agent's Space: fills
// its main frame's sign-in form now, as its load would have, without reloading it.
void FillForUser(bc_id tab);

}  // namespace browser_core

#endif  // BROWSER_CORE_AGENT_AUTOFILL_H_
