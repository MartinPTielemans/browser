#include "browser_core/platform/playready.h"

#include "browser_core/public/browser_core.h"

namespace browser_core::platform {

// Only Windows plays these sites with PlayReady (decision 1).
int PlayReadyNeeds() {
  return BC_PLAYREADY_NONE;
}

}  // namespace browser_core::platform
