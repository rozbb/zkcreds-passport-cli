# zkcreds-passport-cli

This is proof of concept code that verifies issuance requests formed from real US passports. It does not currently support creating passport proofs. The more complete version of this can be found in the `passport` bench of the `zkcreds-rs` repo.

As this is simply an academic proof-of-concept, most of the transactions between parties are currently handled off-chain and out-of-band; the repo does not allow User and Issuer to automatically interact with each other to send or receive values.

### ⚠ Disclaimer ⚠

* This application, as well as the Arkworks ecosystem itself, is an academic proof-of-concept only, and has not been thoroughly reviewed or audited. **Do NOT use this implementation in production code.**

In particular, **Do NOT deploy these smart contracts to the main Ethereum network (mainnet)** or any other deployed blockchain. As a reminder, smart contract "dapps" are notoriously difficult to patch, update, bugfix, or revert once they are deployed onto the blockchain and, as consistent with the [MIT License](LICENSE-MIT), the authors of this repo are NOT liable in any form for any damages, financial or otherwise, that result from this code.

### Request to issue credential

In any subdirectory, run the command `cargo run --release -- help` for a list of subcommands involving the construction and parsing of issuance requests and credential lists. For a User's request:
1. A trusted party (hereafter, "Issuer") runs the `gen-crs` subcommand to generate a proving and verifying key `(pk, vk)`, specifying the destination files for each. Issuer sends `(pk, vk)` to User.
2. User runs the `issue-req` subcommand on a valid JSON-encoded passport dump to generate a base64-encoded issuance request. User hands the issuance request to the Issuer.
3. Issuer runs the `issue-grant` subcommand on the issuance request to verify the issuance criteria. If so, it outputs the base64-encoded credential itself to be issued.

### Issue valid credential to blockchain

Then, in the `eth` subdirectory, the Issuer can set up a (test) Ethereum network environment, using the smart contract to manage a list of credentials:
1. In a terminal window, Issuer runs `MNEMONIC="<mnemonic>" npm run ganache` to setup the specified ETH account and start the RPC server hosting a development fork of the current Ethereum chain.
2. In a different terminal window, Issuer runs `truffle compile` to compile the Ethereum smart contract.
3. All parties in the system agree on whether they want to post the credential from `issue-grant` (cheaper gas costs, Issuer trusted to verify) vs. the full `IssuanceReq` (more expensive gas costs, but anyone with the CRS can verify/audit the credential list). The Issuer must ensure that this is consistent for every entry in the list. 
4. Issuer adds the appropriate validated credential to the dev chain by invoking the smart contract function `issueCred` with `truffle run issuer.js`.

### Obtain (opt. verify) credential list from blockchain

Any party who wishes to reconstruct the local credential list sends a request to Issuer:
1. Issuer obtains the current credential list and root from the contract by invoking `getCredList` and `getRoot` with `truffle run user.js` and (TODO) hands these base64-encoded list to the requesting party.
2. After receiving these values, a User or other party can invoke the `gen-tree` and/or `gen-root` subcommands, respectively, to locally reconstruct the Merkle tree representation of the Merkle list.
3. If the list entry is a full issuance request, a User or other party can verify each issuance request themselves using the `issue-grant` subcommand, (maybe) collect only the validating credentials from the list, then use the `gen-tree` and `gen-root` subcommands to check that the local view of the Merkle tree is correct.
