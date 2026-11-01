#ifndef BROWSER_CORE_BATTERY_SAVER_H_
#define BROWSER_CORE_BATTERY_SAVER_H_

// Chromium's battery saver, which Chrome's own main parts make and start: on its own below 20 % on
// battery, as Chrome's default is (#686). The host hears it turn on and off, and Settings turns it
// off with Chrome's local state's mode, which Chrome's manager follows.
namespace browser_core::battery_saver {

// From now the host hears it turn on and off. After Chrome has made its manager, which it does as
// its threads start.
void Start();

// Whether it may turn on, or never does.
bool Allowed();
void SetAllowed(bool allowed);

// For check.sh: turns it on as if the battery were low, until SetAllowed.
void TurnOnForTesting();

}  // namespace browser_core::battery_saver

#endif  // BROWSER_CORE_BATTERY_SAVER_H_
