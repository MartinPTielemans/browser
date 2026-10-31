#ifndef BROWSER_CORE_AUTOFILL_DATA_H_
#define BROWSER_CORE_AUTOFILL_DATA_H_

#include <string>

#include "browser_core/public/browser_core.h"

namespace browser_core {

// A profile's saved addresses and cards as the app lists them and the user removes them, in place
// of Chrome's autofill settings pages (profile_autofill_changed). A card's number stays in the core:
// the app gets its name, network and expiry.
namespace autofill_data {

// See bc_profile_read_autofill and bc_profile_remove_autofill.
void Read(bc_id profile);
void Remove(bc_id profile, const std::string& key);

}  // namespace autofill_data

}  // namespace browser_core

#endif  // BROWSER_CORE_AUTOFILL_DATA_H_
