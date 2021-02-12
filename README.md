# Signed address generator

This is a tool to generate a ton of addresses from an xpub. The idea is you generate the addresses in a batch, then put that stack of addresses on a server somewhere for receiving payments. This way your xpub itself doesn't have to be on the server.

To get the xpub I'm using the bip84 from [this file](https://github.com/Coldcard/firmware/blob/c1d78d12528d7c4b0f12c3a4ea6c18453d424f5e/docs/generic-wallet-export.md) which can be exported by a Coldcard:

```
Advanced > MicroSD Card > Export Wallet > Generic JSON
```

## TODO

- [ ] come up with a better name
- [x] sign the addresses with gpg
- [ ] write the signed addresses to an sqlite db
