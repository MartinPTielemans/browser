#include "browser_core/sync_guard.h"

#include <memory>
#include <string>

#include "base/base64url.h"
#include "base/hash/sha1.h"
#include "components/sync/protocol/entity_data.h"
#include "components/sync/protocol/entity_specifics.pb.h"
#include "components/sync/protocol/password_specifics.pb.h"
#include "testing/gtest/include/gtest/gtest.h"

namespace browser_core {
namespace {

using syncer::PassphraseType;

std::unique_ptr<syncer::CommitRequestData> Change(const std::string& name,
                                                  const sync_pb::EntitySpecifics& specifics) {
  auto change = std::make_unique<syncer::CommitRequestData>();
  change->entity = std::make_unique<syncer::EntityData>();
  change->entity->name = name;
  change->entity->specifics = specifics;
  return change;
}

sync_pb::EntitySpecifics Encrypted() {
  sync_pb::EntitySpecifics specifics;
  specifics.mutable_encrypted()->set_key_name("key");
  specifics.mutable_encrypted()->set_blob("ciphertext");
  return specifics;
}

sync_pb::EntitySpecifics EncryptedPassword() {
  sync_pb::EntitySpecifics specifics;
  specifics.mutable_password()->mutable_encrypted()->set_key_name("key");
  specifics.mutable_password()->mutable_encrypted()->set_blob("ciphertext");
  specifics.mutable_password()->mutable_encrypted_notes_backup()->set_blob("notes");
  return specifics;
}

bool Ciphertext(syncer::DataType type, const sync_pb::EntitySpecifics& specifics) {
  syncer::CommitRequestDataList commit;
  commit.push_back(Change("encrypted", specifics));
  return SyncCommitIsCiphertext(type, commit);
}

// browser#639: a type Chromium can encrypt waits for the account's passphrase, which the server
// doesn't hold, and not the keystore key it does.
TEST(SyncGuardTest, HoldsUntilThePassphraseEncrypts) {
  for (syncer::DataType type : {syncer::BOOKMARKS, syncer::PASSWORDS, syncer::SESSIONS}) {
    EXPECT_TRUE(HoldSyncCommit(type, false, PassphraseType::kKeystorePassphrase));
    EXPECT_TRUE(HoldSyncCommit(type, true, PassphraseType::kKeystorePassphrase));
    EXPECT_TRUE(HoldSyncCommit(type, true, PassphraseType::kImplicitPassphrase));
    EXPECT_TRUE(HoldSyncCommit(type, false, PassphraseType::kCustomPassphrase));
    EXPECT_FALSE(HoldSyncCommit(type, true, PassphraseType::kCustomPassphrase));
  }
  // What sets the passphrase, and what the server reads, go up as they are.
  EXPECT_FALSE(HoldSyncCommit(syncer::NIGORI, false, PassphraseType::kKeystorePassphrase));
  EXPECT_FALSE(HoldSyncCommit(syncer::DEVICE_INFO, false, PassphraseType::kKeystorePassphrase));
  EXPECT_TRUE(HoldSyncCommit(syncer::OUTGOING_PASSWORD_SHARING_INVITATION, true,
                             PassphraseType::kCustomPassphrase));
}

// browser#639: a commit of a type Chromium can encrypt carries ciphertext only.
TEST(SyncGuardTest, CommitsCiphertextOnly) {
  EXPECT_TRUE(Ciphertext(syncer::BOOKMARKS, Encrypted()));
  EXPECT_TRUE(Ciphertext(syncer::PASSWORDS, EncryptedPassword()));

  sync_pb::EntitySpecifics bookmark;
  bookmark.mutable_bookmark()->set_url("https://example.test/");
  EXPECT_FALSE(Ciphertext(syncer::BOOKMARKS, bookmark));
  sync_pb::EntitySpecifics both = Encrypted();
  both.mutable_bookmark()->set_url("https://example.test/");
  EXPECT_FALSE(Ciphertext(syncer::BOOKMARKS, both));

  sync_pb::EntitySpecifics password = EncryptedPassword();
  password.mutable_password()->mutable_unencrypted_metadata()->set_url("https://example.test/");
  EXPECT_FALSE(Ciphertext(syncer::PASSWORDS, password));
  password = EncryptedPassword();
  password.mutable_password()->mutable_client_only_encrypted_data()->set_origin(
      "https://example.test/");
  EXPECT_FALSE(Ciphertext(syncer::PASSWORDS, password));
  // Encrypted the way other types are, a password's own ciphertext is missing.
  EXPECT_FALSE(Ciphertext(syncer::PASSWORDS, Encrypted()));

  // A deletion has nothing to encrypt, and Nigori is the server's to read.
  EXPECT_TRUE(Ciphertext(syncer::BOOKMARKS, sync_pb::EntitySpecifics()));
  sync_pb::EntitySpecifics nigori;
  nigori.mutable_nigori()->set_keybag_is_frozen(true);
  EXPECT_TRUE(Ciphertext(syncer::NIGORI, nigori));
}

// browser#639: a change's name goes up beside its specifics, so it's "encrypted" too, as Chromium
// names what it encrypts, where a password's would be its site.
TEST(SyncGuardTest, NamesNothing) {
  syncer::CommitRequestDataList commit;
  commit.push_back(Change("encrypted", EncryptedPassword()));
  commit.push_back(Change("encrypted", sync_pb::EntitySpecifics()));
  commit.push_back(Change("", sync_pb::EntitySpecifics()));
  EXPECT_TRUE(SyncCommitIsCiphertext(syncer::PASSWORDS, commit));
  commit.push_back(Change("https://example.test/", EncryptedPassword()));
  EXPECT_FALSE(SyncCommitIsCiphertext(syncer::PASSWORDS, commit));

  syncer::CommitRequestDataList deleted;
  deleted.push_back(Change("Example", sync_pb::EntitySpecifics()));
  EXPECT_FALSE(SyncCommitIsCiphertext(syncer::BOOKMARKS, deleted));

  syncer::CommitRequestDataList device;
  sync_pb::EntitySpecifics info;
  info.mutable_device_info()->set_client_name("laptop");
  device.push_back(Change("laptop", info));
  EXPECT_TRUE(SyncCommitIsCiphertext(syncer::DEVICE_INFO, device));
}

// browser#640: a password's client tag, which the server sees, tells it neither the site nor the
// username, and only the account's devices agree on it.
TEST(SyncGuardTest, PasswordTagsAreKeyed) {
  const std::string tag = "https%3A//example.test/|username|alice|password|https://example.test/";
  SyncSecret secret;
  secret.fill(0x33);
  SyncSecret other;
  other.fill(0x44);

  SetSyncPasswordTagKey(std::nullopt);
  const std::string unkeyed = SyncPasswordTag(tag);
  EXPECT_NE(unkeyed, tag);
  EXPECT_EQ(SyncPasswordTag(tag), unkeyed);

  SetSyncPasswordTagKey(secret);
  const std::string keyed = SyncPasswordTag(tag);
  EXPECT_EQ(keyed.find("example"), std::string::npos);
  EXPECT_EQ(keyed.find("alice"), std::string::npos);
  EXPECT_NE(keyed, unkeyed);
  EXPECT_NE(keyed, SyncPasswordTag(tag + "x"));
  std::string sha1;
  base::Base64UrlEncode(base::SHA1HashString(tag), base::Base64UrlEncodePolicy::OMIT_PADDING,
                        &sha1);
  EXPECT_NE(keyed, sha1);

  SetSyncPasswordTagKey(other);
  EXPECT_NE(SyncPasswordTag(tag), keyed);
  SetSyncPasswordTagKey(secret);
  EXPECT_EQ(SyncPasswordTag(tag), keyed);
  SetSyncPasswordTagKey(std::nullopt);
  EXPECT_EQ(SyncPasswordTag(tag), unkeyed);
}

}  // namespace
}  // namespace browser_core
