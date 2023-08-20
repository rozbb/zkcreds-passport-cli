# zkcreds-passport-cli

This is proof of concept code that verifies issuance requests formed from real US passports. It does not currently support creating passport proofs. The more complete version of this can be found in the `passport` bench of the `zkcreds-rs` repo.

As this is simply an academic proof-of-concept, most of the transactions between parties are handled out-of-band; the repo does not allow User and Issuer to automatically interact with each other to send or receive values.

### ⚠ Disclaimer ⚠

* This application, as well as the Arkworks ecosystem itself, is an academic proof-of-concept only, and has not been thoroughly reviewed or audited. **Do NOT use this implementation in production code.**

## Usage

Here's a list of tasks you can do with this CLI, and example commands to do them. The CLI has help built in too. Run `cargo run --release -- help` for a list of subcommands involving the construction and parsing of issuance requests and credential lists.

### Generating the proving key material

A trusted party (the "Issuer") generates a proving and verifying key `(pk, vk)`. In a real deployment, these are made public.

```shell
cargo run --release gen-crs --proving-key pk.key --verifying-key vk.key
```

### Generating an issuance request

A user who has dumped their passport using the [passport dumping utility](https://github.com/rozbb/zkcreds-passport-dumper) submits an issuance request to the issuer. That is, it proves that it has a valid passport and asks that a commitment to it be included in the issuer's Merkle tree.

```shell
cargo run --release issue-req --proving-key pk.key --dump-file passport_dump.json > issuereq.bin
```

### Granting an issuance request

An issuer receives an issuance request and verifies that it's valid. On success, it will save the credential (the aforementioned commitment).

```shell
cargo run --release issue-grant --verifying-key vk.key < issuereq.bin > cred.bin
```

### Forming a tree from all the credentials

An issuer represents its list of issued credentials as a Merkle tree whose leaves are the credentials. It takes a newline-separated list of credentials and outputs its tree representation.

```shell
cargo run --release gen-tree --creds creds.bin > tree.bin
```

### Getting the root of a tree

The root of a Merkle tree is a succinct representation of the entire tree. The issuer can calculate it as follows.

```shell
cargo run --release get-root --tree tree.bin > root.bin
```
