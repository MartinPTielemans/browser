#include "browser_core/sync_guard.h"

#include <algorithm>

#include "components/sync/protocol/entity_data.h"
#include "components/sync/protocol/entity_specifics.pb.h"
#include "components/sync/protocol/password_specifics.pb.h"

namespace browser_core {

namespace {

// Whether |specifics| hold nothing but what the passphrase encrypted.
bool Ciphertext(syncer::DataType type, const sync_pb::EntitySpecifics& specifics) {
  sync_pb::EntitySpecifics rest = specifics;
  if (type == syncer::PASSWORDS) {
    // Passwords encrypt into their own specifics, beside a backup of their notes.
    if (!specifics.password().has_encrypted()) {
      return false;
    }
    sync_pb::PasswordSpecifics& password = *rest.mutable_password();
    password.clear_encrypted();
    password.clear_encrypted_notes_backup();
    if (password.ByteSizeLong() != 0) {
      return false;
    }
    rest.clear_password();
  } else {
    if (!specifics.has_encrypted()) {
      return false;
    }
    rest.clear_encrypted();
  }
  return rest.ByteSizeLong() == 0;
}

}  // namespace

bool HoldSyncCommit(syncer::DataType type, bool encrypted, syncer::PassphraseType passphrase) {
  // Sharing a password goes to another user, whom only Google's servers can find.
  if (type == syncer::OUTGOING_PASSWORD_SHARING_INVITATION) {
    return true;
  }
  return syncer::EncryptableUserTypes().Has(type) &&
         !(encrypted && syncer::IsExplicitPassphrase(passphrase));
}

bool SyncCommitIsCiphertext(syncer::DataType type, const syncer::CommitRequestDataList& commit) {
  if (!syncer::EncryptableUserTypes().Has(type)) {
    return true;
  }
  return std::ranges::all_of(commit, [type](const std::unique_ptr<syncer::CommitRequestData>& c) {
    const syncer::EntityData& entity = *c->entity;
    // A password's deletion has no name, and no deletion has specifics.
    return (entity.name.empty() || entity.name == "encrypted") &&
           (entity.is_deleted() || Ciphertext(type, entity.specifics));
  });
}

}  // namespace browser_core
