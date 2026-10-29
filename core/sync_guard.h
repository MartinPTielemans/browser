// What the account's sync gives its server: only what the server can't read (ilo-server's
// README, What the server can see). Chromium's sync calls these through the hooks
// engine/patches/ours/chromium-sync-hooks.patch adds, on sync's own sequences.

#ifndef BROWSER_CORE_SYNC_GUARD_H_
#define BROWSER_CORE_SYNC_GUARD_H_

#include <optional>
#include <string>

#include "browser_core/sync_words.h"
#include "components/sync/base/data_type.h"
#include "components/sync/base/passphrase_enums.h"
#include "components/sync/engine/commit_and_get_updates_types.h"

namespace browser_core {

// Whether |type|'s changes wait to commit: a type Chromium can encrypt waits until it's encrypted
// with the account's passphrase, which the server doesn't hold, and not with the keystore key it
// does, as on a first device before it has set the passphrase, or one starting over. A password
// shared with another user never commits.
bool HoldSyncCommit(syncer::DataType type, bool encrypted, syncer::PassphraseType passphrase);

// Whether each change in |commit|, of |type|, is as Chromium leaves it once the passphrase
// encrypted it, if |type| is one Chromium can encrypt: named "encrypted" or not at all, with
// specifics that hold ciphertext only.
bool SyncCommitIsCiphertext(syncer::DataType type, const syncer::CommitRequestDataList& commit);

// Gives the account's secret, which keys passwords' client tags, or takes it away.
void SetSyncPasswordTagKey(const std::optional<SyncSecret>& secret);

// A password's client tag, from Chromium's (its site and username, which the server would see
// hashed without a key): HMAC-SHA256 of it, keyed with HKDF(secret, "ilo sync client tags v1"),
// so only the account's devices can tell which site and username it is. With no secret, a key
// made for this run, so nothing the server could test guesses against goes up.
std::string SyncPasswordTag(const std::string& tag);

}  // namespace browser_core

#endif  // BROWSER_CORE_SYNC_GUARD_H_
