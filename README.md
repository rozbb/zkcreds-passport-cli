# zkcreds-passport-cli

This is proof of concept code that verifies issuance requests formed from real US passports. It does not currently support creating passport proofs. The more complete version of this can be found in the `passport` bench of the `zkcreds-rs` repo.

As this is simply an academic proof-of-concept, most of the transactions between parties are currently handled off-chain and out-of-band; the repo does not allow User and Issuer to automatically interact with each other to send or receive values.

### ⚠ Disclaimer ⚠

* This application, as well as the Arkworks ecosystem itself, is a (currently incomplete) academic proof-of-concept only, and has NOT been thoroughly reviewed for production use. **Do NOT use this implementation in production code.**

Especially, **Do NOT deploy these smart contracts to the main Ethereum network (mainnet)**. As a reminder, smart contract "dapps" are notoriously difficult to patch, update, bugfix, or revert once they are deployed onto the blockchain and, as consistent with the [MIT License](LICENSE-MIT), the authors of this repo are NOT liable in any form for any damages, financial or otherwise, that result from this code.

### Request to issue user's cred

In any subdirectory, run the command `cargo run --release -- help` for a list of subcommands involving the construction and parsing of issuance requests and credential lists. For a User's request:
1. A trusted party (hereafter, "Issuer") runs the `gen-crs` subcommand to generate proving and verifying key `(pk, vk)`, specifying the destination files for each. Issuer sends `(pk, vk)` to User.
2. User runs the `issue-req` subcommand on a valid JSON-encoded passport dump to generate a base64-encoded issuance request (committed credential + hash of passport + signature + proof that it can be issued). User hands the issuance request to the Issuer.
3. Issuer runs the `issue-grant` subcommand on the issuance request to verify that it can be issued. If so, it outputs the corresponding credential.

### Issue valid cred to blockchain

Then, in the `eth` subdirectory, the Issuer can sets up a development Ethereum network where they can use a smart contract to manage a list of base64-encoded credentials:
1. In a terminal window, Issuer runs `MNEMONIC="<mnemonic>" npm run ganache` to start the development RPC server.
2. In a different terminal window, Issuer runs `truffle compile` to compile the Ethereum smart contract.
3. Parties decides whether to post just the credential (cheaper gas costs, Issuer trusted with verifying it) from `issue-grant` vs. the full `IssuanceReq` (more expensive gas costs, but anyone with the CRS can verify/audit the credential list) for all requests. 
4. TODO: Using `truffle` JS scripts, Issuer calls the `issueCred` function using its smart contract to add the aformentioned validated  to the dev chain.

### Obtain (optionally, verify) credential list from blockchain

Any party who wishes to reconstruct the local credential list sends a request to Issuer:
1. TODO: Using `truffle` JS scripts, Issuer calls the `getCredList`, and `getRoot` functions using its smart contract to obtain the respective base64-encocded credential list, and hands these values to the requesting party.
2. After receiving these values, a User or other party can invoke the `gen-tree` and/or `gen-root` subcommands, respectively, to locally reconstruct the Merkle tree representation of the Merkle list.
3. TODO: If Issuer agreed to post the full issuance requests, a User or other party can verify each issuance request themselves using the `issue-grant` subcommand, (maybe) collect only the validating credentials, then use the `gen-tree` and `gen-root` subcommands to check that the local view of the Merkle tree is correct.
