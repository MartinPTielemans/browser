#ifndef BROWSER_CORE_HOST_H_
#define BROWSER_CORE_HOST_H_

#include "browser_core/public/browser_core.h"

namespace browser_core {

// The app's callbacks, from bc_main.
const bc_host& Host();

// A window's or tab's id, unique among both.
bc_id NewId();

}  // namespace browser_core

#endif  // BROWSER_CORE_HOST_H_
