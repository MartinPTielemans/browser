// Spike (#589): sync with the browser's own account and relay, in place of Google's sign-in and
// FCM, through the hooks engine/patches/ours/chromium-sync-hooks.patch adds.
#ifndef BROWSER_CORE_SYNC_H_
#define BROWSER_CORE_SYNC_H_

#include <string>

class Profile;

namespace browser_core {

// Sets the sync hooks, before any profile is made. Only when BROWSER_SYNC_ACCOUNT is set: the
// spike takes the account and its token from the environment, where the product takes them from
// the app through the C API.
void InstallSyncHooks();

// Hands |payload|, a SyncInvalidationsPayload as the relay passed it on, to |profile|'s sync, as
// FCM's message would be.
void DeliverSyncInvalidation(Profile* profile, const std::string& payload);

}  // namespace browser_core

#endif  // BROWSER_CORE_SYNC_H_
