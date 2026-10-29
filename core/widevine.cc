#include "browser_core/widevine.h"

#include "browser_core/public/browser_core.h"
#include "build/build_config.h"
#include "third_party/widevine/cdm/buildflags.h"

// Offered only on Linux. The Mac plays these sites in its system engine and Windows with
// PlayReady, and Netflix-class sites want a signed browser's Widevine on both (decision 1).
#if BUILDFLAG(IS_LINUX) && BUILDFLAG(ENABLE_WIDEVINE_CDM_COMPONENT)
#define BROWSER_CORE_OFFERS_WIDEVINE 1
#endif

#if defined(BROWSER_CORE_OFFERS_WIDEVINE)
#include <algorithm>

#include "base/files/file_path.h"
#include "base/files/file_util.h"
#include "base/functional/bind.h"
#include "base/functional/callback_helpers.h"
#include "base/memory/weak_ptr.h"
#include "base/no_destructor.h"
#include "base/path_service.h"
#include "base/scoped_observation.h"
#include "base/task/thread_pool.h"
#include "base/version.h"
#include "browser_core/host.h"
#include "browser_core/session.h"
#include "chrome/browser/browser_process.h"
#include "chrome/browser/component_updater/widevine_cdm_component_installer.h"
#include "chrome/common/chrome_paths.h"
#include "chrome/common/media/cdm_registration.h"
#include "components/component_updater/component_updater_service.h"
#include "components/update_client/crx_update_item.h"
#include "components/update_client/update_client_errors.h"
#endif

namespace browser_core::widevine {

#if defined(BROWSER_CORE_OFFERS_WIDEVINE)

namespace {

// The component's id, from its CRX's hash (widevine_cdm_component_installer.cc).
constexpr char kComponentId[] = "oimompecagnajdejgnnjijobebaeigek";

component_updater::ComponentUpdateService* Updater() {
  return g_browser_process ? g_browser_process->component_updater() : nullptr;
}

}  // namespace

// Follows the component updater for the module, and downloads it at the user's yes, which
// component_updater_service.h lets this class alone of the core's ask for.
class Downloader : public component_updater::ServiceObserver {
 public:
  static Downloader& Get() {
    static base::NoDestructor<Downloader> downloader;
    return *downloader;
  }

  void Start() {
    if (component_updater::ComponentUpdateService* updater = Updater()) {
      observation_.Observe(updater);
    }
    // What Chromium loaded, as cdm_registration.cc found it before the zygote: the module the hint
    // file names, if it's there. Nothing has changed it since, as the component updater's first
    // check is a minute after launch, and nothing has asked yet: a page's first request comes once
    // the Spaces are read and a tab has loaded.
    base::ThreadPool::PostTaskAndReplyWithResult(
        FROM_HERE, {base::MayBlock(), base::TaskPriority::USER_VISIBLE},
        base::BindOnce([] { return !GetSoftwareSecureWidevine().empty(); }),
        base::BindOnce(&Downloader::Read, weak_factory_.GetWeakPtr()));
  }

  void Stop() { observation_.Reset(); }

  int state() const { return state_; }
  bool loaded() const { return loaded_; }

  void Begin() {
    if (state_ != BC_WIDEVINE_ABSENT && state_ != BC_WIDEVINE_FAILED) {
      return;
    }
    component_updater::ComponentUpdateService* updater = Updater();
    if (!updater) {
      return Set(BC_WIDEVINE_FAILED);
    }
    UserSettings().widevine = true;
    SessionChanged();
    Set(BC_WIDEVINE_DOWNLOADING);
    // Removing it unregistered it, and a component's registration reads its folder first.
    if (std::ranges::contains(updater->GetComponentIDs(), kComponentId)) {
      Update();
    } else {
      component_updater::RegisterWidevineCdmComponent(
          updater, base::BindOnce(&Downloader::Update, weak_factory_.GetWeakPtr()));
    }
  }

  void Remove() {
    UserSettings().widevine = false;
    SessionChanged();
    // Its installer deletes each version it has. The hint file goes too, which would name a module
    // no longer there at the next launch.
    if (component_updater::ComponentUpdateService* updater = Updater()) {
      updater->UnregisterComponent(kComponentId);
    }
    base::FilePath dir;
    if (base::PathService::Get(chrome::DIR_COMPONENT_UPDATED_WIDEVINE_CDM, &dir)) {
      base::ThreadPool::PostTask(
          FROM_HERE, {base::MayBlock(), base::TaskPriority::USER_VISIBLE},
          base::BindOnce(base::IgnoreResult(&base::DeletePathRecursively), dir));
    }
    Set(BC_WIDEVINE_ABSENT);
  }

 private:
  friend class base::NoDestructor<Downloader>;

  Downloader() = default;
  ~Downloader() override = default;

  void Read(bool loaded) {
    loaded_ = loaded;
    if (loaded && state_ == BC_WIDEVINE_ABSENT) {
      Set(BC_WIDEVINE_READY);
    }
  }

  void Update() {
    component_updater::ComponentUpdateService* updater = Updater();
    if (!updater) {
      return Set(BC_WIDEVINE_FAILED);
    }
    updater->GetOnDemandUpdater().OnDemandUpdate(
        kComponentId, component_updater::OnDemandUpdater::Priority::FOREGROUND,
        base::BindOnce(&Downloader::Updated, weak_factory_.GetWeakPtr()));
  }

  // The on-demand update is done, after its events. One that ends still downloading never began:
  // another check was going, or the component wasn't registered.
  void Updated(update_client::Error) {
    if (state_ == BC_WIDEVINE_DOWNLOADING) {
      Set(BC_WIDEVINE_FAILED);
    }
  }

  // component_updater::ServiceObserver:
  void OnEvent(const update_client::CrxUpdateItem& item) override {
    if (item.id != kComponentId) {
      return;
    }
    switch (item.state) {
      // The user's yes, or a check since, as after a yes whose download the last launch never
      // finished. Linux loads the module at launch, so a new one plays after a restart.
      case update_client::ComponentState::kUpdated:
        return Set(loaded_ ? BC_WIDEVINE_READY : BC_WIDEVINE_RESTART);
      // Google's service has none newer than the one in place, if there is one.
      case update_client::ComponentState::kUpToDate:
        if (state_ == BC_WIDEVINE_DOWNLOADING) {
          const bool installed = item.component &&
                                 item.component->version.IsValid() &&
                                 item.component->version > base::Version("0.0.0.0");
          Set(!installed ? BC_WIDEVINE_FAILED
                         : (loaded_ ? BC_WIDEVINE_READY : BC_WIDEVINE_RESTART));
        }
        return;
      case update_client::ComponentState::kUpdateError:
        if (state_ == BC_WIDEVINE_DOWNLOADING) {
          Set(BC_WIDEVINE_FAILED);
        }
        return;
      default:
        return;
    }
  }

  void Set(int state) {
    if (state == state_) {
      return;
    }
    state_ = state;
    if (Host().widevine_changed) {
      Host().widevine_changed(Host().context, state);
    }
  }

  int state_ = BC_WIDEVINE_ABSENT;
  bool loaded_ = false;
  base::ScopedObservation<component_updater::ComponentUpdateService,
                          component_updater::ServiceObserver>
      observation_{this};
  base::WeakPtrFactory<Downloader> weak_factory_{this};
};

void Start() {
  Downloader::Get().Start();
}

void Stop() {
  Downloader::Get().Stop();
}

bool MayUpdate(const std::string& component) {
  return component == kComponentId && UserSettings().widevine;
}

int State() {
  return Downloader::Get().state();
}

bool Loaded() {
  return Downloader::Get().loaded();
}

void Download() {
  Downloader::Get().Begin();
}

void Remove() {
  Downloader::Get().Remove();
}

#else

void Start() {}

void Stop() {}

bool MayUpdate(const std::string&) {
  return false;
}

int State() {
  return BC_WIDEVINE_NONE;
}

bool Loaded() {
  return false;
}

void Download() {}

void Remove() {}

#endif  // defined(BROWSER_CORE_OFFERS_WIDEVINE)

}  // namespace browser_core::widevine
