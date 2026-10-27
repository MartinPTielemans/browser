// The Keychain item that holds the profiles' encryption key.

#ifndef BROWSER_CORE_PLATFORM_KEYCHAIN_H_
#define BROWSER_CORE_PLATFORM_KEYCHAIN_H_

namespace browser_core::platform {

// Names the item after the app, as "<name> Safe Storage", so the app keeps a key of its own
// rather than sharing Chromium's with every other Chromium on the Mac. Call it before anything
// reads the key.
void NameKeychainItem();

}  // namespace browser_core::platform

#endif  // BROWSER_CORE_PLATFORM_KEYCHAIN_H_
