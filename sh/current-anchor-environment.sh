#!/bin/sh

set -euo pipefail

rustc --version
solana --version
# The avm release version installed
AVM=$(avm --version)
echo "avm release version: $AVM"
# The 'default' avm version currently used
CURRENT_AVM=$(avm list | grep 'current' | cut -f 1)
echo "Current avm used: $CURRENT_AVM"
