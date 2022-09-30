# zkcreds-passport-cli

This is proof of concept code that verifies issuance requests formed from real US passports. It does not currently support creating passport proofs. The more complete version of this can be found in the `passport` bench of the `zkcreds-rs` repo.

As this is simply an academic proof-of-concept, most of the transactions between parties are handled out-of-band; the repo does not allow User and Issuer to automatically interact with each other to send or receive values.

### ⚠ Disclaimer ⚠

* This application, as well as the Arkworks ecosystem itself, is an academic proof-of-concept only, and has not been thoroughly reviewed or audited. **Do NOT use this implementation in production code.**

### Request to issue credential

In any subdirectory, run the command `cargo run --release -- help` for a list of subcommands involving the construction and parsing of issuance requests and credential lists. For a User's request:
1. A trusted party (hereafter, "Issuer") runs the `gen-crs` subcommand to generate a proving and verifying key `(pk, vk)`, specifying the destination files for each. Issuer sends `(pk, vk)` to User.
2. User runs the `issue-req` subcommand on a valid JSON-encoded passport dump to generate a base64-encoded issuance request. User hands the issuance request to the Issuer.
3. Issuer runs the `issue-grant` subcommand on the issuance request to verify the issuance criteria. If so, it outputs the base64-encoded credential itself to be issued.
