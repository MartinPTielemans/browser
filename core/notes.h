#ifndef BROWSER_CORE_NOTES_H_
#define BROWSER_CORE_NOTES_H_

#include <string>

class Profile;

namespace browser_core::notes {

// The notes service New Note opens a new document in, which each profile keeps, as Arc's Settings ›
// Profiles › New Documents does. The core keeps the app's name for it and knows nothing of the
// services: the app has their list and their new-document URLs.

// Registers what profiles keep for it. Before the first profile is made.
void RegisterProfilePrefs();

// The profile's service, or "" for none picked.
std::string Service(Profile* profile);
void SetService(Profile* profile, const std::string& service);

}  // namespace browser_core::notes

#endif  // BROWSER_CORE_NOTES_H_
