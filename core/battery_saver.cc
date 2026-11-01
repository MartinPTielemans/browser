#include "browser_core/battery_saver.h"

#include "base/no_destructor.h"
#include "base/scoped_observation.h"
#include "browser_core/host.h"
#include "chrome/browser/browser_process.h"
#include "chrome/browser/performance_manager/public/user_tuning/battery_saver_mode_manager.h"
#include "components/performance_manager/public/user_tuning/prefs.h"
#include "components/prefs/pref_service.h"

namespace browser_core::battery_saver {

namespace {

using performance_manager::user_tuning::BatterySaverModeManager;
using performance_manager::user_tuning::prefs::BatterySaverModeState;
using performance_manager::user_tuning::prefs::GetCurrentBatterySaverModeState;
using performance_manager::user_tuning::prefs::kBatterySaverModeState;

class Observer : public BatterySaverModeManager::Observer {
 public:
  void Start() {
    if (BatterySaverModeManager::HasInstance()) {
      observation_.Observe(BatterySaverModeManager::GetInstance());
    }
  }

 private:
  // BatterySaverModeManager::Observer:
  void OnBatterySaverActiveChanged(bool is_active) override {
    if (Host().battery_saver_changed) {
      Host().battery_saver_changed(Host().context, is_active);
    }
  }

  // Chrome lets go of its manager as its run loop ends, before the core's parts hear it has.
  void OnBatterySaverModeManagerDestroyed() override { observation_.Reset(); }

  base::ScopedObservation<BatterySaverModeManager, BatterySaverModeManager::Observer>
      observation_{this};
};

Observer& TheObserver() {
  static base::NoDestructor<Observer> observer;
  return *observer;
}

void SetMode(BatterySaverModeState state) {
  g_browser_process->local_state()->SetInteger(kBatterySaverModeState, static_cast<int>(state));
}

}  // namespace

void Start() {
  TheObserver().Start();
}

bool Allowed() {
  return GetCurrentBatterySaverModeState(g_browser_process->local_state()) !=
         BatterySaverModeState::kDisabled;
}

void SetAllowed(bool allowed) {
  // Chrome's default, below 20 % on battery, which its manager reads as the battery changes.
  SetMode(allowed ? BatterySaverModeState::kEnabledBelowThreshold
                  : BatterySaverModeState::kDisabled);
}

void TurnOnForTesting() {
  SetMode(BatterySaverModeState::kEnabled);
}

}  // namespace browser_core::battery_saver
