#!/bin/sh
# Prints the designated requirement to sign the app with id $1 with, given the code-signing
# identity $2, for codesign's -r=: the app's id and the team in the identity's certificate, so
# any of the team's certificates Apple issued satisfies it. macOS grants Keychain items, the
# cookie key among them, to the requirement of the app that made them, and codesign's own names
# the certificate, so without this a move from Apple Development to Developer ID would ask for
# the key (platform/mac/updates.mm makes the key's item again under this one). Prints nothing for
# ad hoc or a certificate Apple didn't issue, which keep codesign's.
# Usage: tools/requirement.sh <bundle id> <identity>
set -e
[ "$2" != - ] && [ -n "$2" ] || exit 0
certificate=$(security find-certificate -c "$2" -p 2> /dev/null) || exit 0
field() {
  echo "$certificate" | openssl x509 -noout "$1" -nameopt RFC2253 |
    sed -n "s/.*[=,]$2=\([^,]*\).*/\1/p"
}
[ "$(field -issuer O)" = "Apple Inc." ] || exit 0
team=$(field -subject OU)
[ -n "$team" ] || exit 0
echo "designated => identifier \"$1\" and anchor apple generic and certificate leaf[subject.OU] = \"$team\""
