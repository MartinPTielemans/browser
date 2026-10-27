#include "browser_core/platform/keychain.h"

#import <Foundation/Foundation.h>

#include <string>

#include "base/strings/sys_string_conversions.h"
#include "components/os_crypt/common/keychain_password_mac.h"

namespace browser_core::platform {

void NameKeychainItem() {
  NSString* name = [NSBundle.mainBundle objectForInfoDictionaryKey:@"CFBundleName"];
  if (name.length == 0) {
    return;
  }
  std::string app = base::SysNSStringToUTF8(name);
  KeychainPassword::GetServiceName() = app + " Safe Storage";
  KeychainPassword::GetAccountName() = app;
}

}  // namespace browser_core::platform
