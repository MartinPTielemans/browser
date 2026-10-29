// Pairing's cryptography (ilo-server's docs/protocol.md, Pairing): a device that holds the account
// secret seals it to a new device through the relay, which can't read it, and both show a match
// code the user compares, which the relay can't forge, as the new device commits to its key first.

#ifndef BROWSER_CORE_SYNC_PAIRING_H_
#define BROWSER_CORE_SYNC_PAIRING_H_

#include <array>
#include <cstdint>
#include <optional>
#include <string>
#include <vector>

#include "browser_core/sync_words.h"

namespace browser_core {

using PairingKey = std::array<uint8_t, 32>;

// An X25519 key pair made for one pairing and dropped after it.
struct PairingKeys {
  PairingKeys();
  // Made from |private_key|, for the protocol's worked example.
  explicit PairingKeys(const PairingKey& private_key);
  ~PairingKeys();

  PairingKey private_key;
  PairingKey public_key;
};

// SHA-256("ilo pairing v1 commitment" || requester's key).
std::array<uint8_t, 32> PairingCommitment(const PairingKey& requester);

// The 6 digits both devices show, from the requester's key and the approver's.
std::string PairingCode(const PairingKey& requester, const PairingKey& approver);

// The secret sealed by the approver to the requester, the ciphertext and its tag, for |pairing|.
// None if |requester| is a low-order key.
std::optional<std::vector<uint8_t>> SealPairing(const PairingKeys& approver,
                                                const PairingKey& requester,
                                                const std::string& pairing,
                                                const SyncSecret& secret);

// The secret the approver sealed, opened by the requester, or none if it doesn't open.
std::optional<SyncSecret> OpenPairing(const PairingKeys& requester,
                                      const PairingKey& approver,
                                      const std::string& pairing,
                                      const std::vector<uint8_t>& sealed);

}  // namespace browser_core

#endif  // BROWSER_CORE_SYNC_PAIRING_H_
