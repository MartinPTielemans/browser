#include "browser_core/platform/playready.h"

#include "base/win/windows_version.h"
#include "browser_core/public/browser_core.h"
#include "components/cdm/common/buildflags.h"

namespace browser_core::platform {

// What AddPlayReady in chrome/common/media/cdm_registration.cc and
// media::SupportMediaFoundationEncryptedPlayback check, in the order the user can do something
// about it. The graphics' hardware DRM is Chromium's to find, in the Media Foundation CDM's
// process, as a page asks.
int PlayReadyNeeds() {
#if BUILDFLAG(ENABLE_PLAYREADY)
  if (base::win::GetVersion() < base::win::Version::WIN11) {
    return BC_PLAYREADY_WINDOWS_11;
  }
  // The engine is x64 alone, so an ARM PC runs it emulated.
  if (base::win::OSInfo::IsRunningEmulatedOnArm64()) {
    return BC_PLAYREADY_X64;
  }
  return BC_PLAYREADY_HARDWARE;
#else
  return BC_PLAYREADY_NONE;
#endif
}

}  // namespace browser_core::platform
