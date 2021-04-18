# Address Factory 

This is a tool to generate a ton of addresses from an xpub. The idea is you generate the addresses in a batch, then put that stack of addresses on a server somewhere for receiving payments. This way your xpub itself doesn't have to be on the server.

To get the xpub I'm using the bip84 from [this file](https://github.com/Coldcard/firmware/blob/c1d78d12528d7c4b0f12c3a4ea6c18453d424f5e/docs/generic-wallet-export.md) which can be exported by a Coldcard:

```
Advanced > MicroSD Card > Export Wallet > Generic JSON
```

## How to use this

First you need GPG installed and set up. If you're new to GPG [this is a nice guide](https://medium.com/@acparas/gpg-quickstart-guide-d01f005ca99).

Here are the basics:

Generate a Key

`gpg --full-generate-key`

Get Fingerprint to put in twitter bio

`gpg --list-secret-keys --fingerprint`

Get Public PGP Key to put on website

`gpg --armor --export <key ID>`

OR

`gpg --armor --output <file> --export <key ID>`

Now you can run Address Factory and follow the instructions and everything should work out great.


## TODO

- [x] come up with a better name
- [x] sign the addresses with gpg
- [x] write the signed addresses to an sqlite db
- [x] parse non-coldcard xpubs 
- [x] fix all the obvious usability issues
- [ ] audit the bitcoin code 
- [ ] switch to mainnet
- [ ] reduce dependencies
- [ ] deploy a donation page using this
