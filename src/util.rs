use anyhow::{bail, Context, Result};
use bdk::{
    bitcoin::{
        util::bip32::{ChildNumber, DerivationPath, ExtendedPubKey},
        Address, Network,
    },
    database::MemoryDatabase,
    descriptor::Descriptor,
    miniscript::DescriptorPublicKey,
    wallet::AddressIndex,
    Wallet,
};

pub type Desc = Descriptor<DescriptorPublicKey>;

pub enum ScriptType {
    Classic,
    NativeSegwit,
    WrappedSegwit,
}

pub fn nth_address(
    descriptor: Descriptor<DescriptorPublicKey>,
    network: Network,
    index: u32,
) -> Result<Address> {
    let wallet = Wallet::new_offline(descriptor, None, network, MemoryDatabase::default())?;
    let address = wallet.get_address(AddressIndex::Peek(index))?;

    Ok(address)
}

/// Check that first address derived matches given address
pub fn check_address(
    descriptor: Descriptor<DescriptorPublicKey>,
    network: Network,
    address: Address,
    index: u32,
) -> Result<Address> {
    let next_address = nth_address(descriptor, network, index)?;

    if address == next_address {
        Ok(address)
    } else {
        bail!("Incorrect first address derived. Check descriptor / xpub / derivation path.")
    }
}

pub fn script_type(path: &DerivationPath) -> Result<ScriptType> {
    let version_number = path.into_iter().next().context("No path")?;

    let num = match version_number {
        ChildNumber::Hardened { index } => index,
        ChildNumber::Normal { index: _ } => bail!("Non-hardened derivation path"),
    };

    match num {
        44 => Ok(ScriptType::Classic),
        49 => Ok(ScriptType::WrappedSegwit),
        84 => Ok(ScriptType::NativeSegwit),
        _ => bail!("Didn't recognize the version number: {}", version_number),
    }
}

/// Build the descriptor string
pub fn build_descriptor(
    xpub: ExtendedPubKey,
    derivation_path: DerivationPath,
) -> Result<Descriptor<DescriptorPublicKey>> {
    let script_type = script_type(&derivation_path)?;

    // Change step of the derivation path
    let derivation_path = vec![ChildNumber::Normal { index: 0 }].into();

    let (desc, _, _) = match script_type {
        ScriptType::Classic => {
            bdk::descriptor!(pkh((xpub, derivation_path)))
        }
        ScriptType::NativeSegwit => {
            bdk::descriptor!(wpkh((xpub, derivation_path)))
        }
        ScriptType::WrappedSegwit => {
            bdk::descriptor!(sh(wpkh((xpub, derivation_path))))
        }
    }?;

    Ok(desc)
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use std::str::FromStr;

    use bdk::bitcoin::{self, util::bip32::ExtendedPubKey, Address};
    use slip132::FromSlip132;

    use crate::ColdcardJson;

    use super::build_descriptor;

    fn test_vector(path: &str, extended_public_key: &str, first_address: &str) -> Result<()> {
        let xpub =
            ExtendedPubKey::from_slip132_str(&extended_public_key).expect("Failed to make an xpub");
        let derivation_path = bitcoin::util::bip32::DerivationPath::from_str(path)
            .expect("Failed to make a derivation path");

        let descriptor = build_descriptor(xpub, derivation_path);

        super::check_address(
            descriptor?,
            bitcoin::Network::Bitcoin,
            Address::from_str(first_address)?,
            0,
        )?;

        Ok(())
    }

    // Tests from here: https://github.com/satoshilabs/slips/blob/master/slip-0132.md
    #[test]
    fn slip132_test_vectors() -> Result<()> {
        let path_44 = r"m/44'/0'/0'";
        let pub_44 = "xpub6BosfCnifzxcFwrSzQiqu2DBVTshkCXacvNsWGYJVVhhawA7d4R5WSWGFNbi8Aw6ZRc1brxMyWMzG3DSSSSoekkudhUd9yLb6qx39T9nMdj";
        let addr_44 = "1LqBGSKuX5yYUonjxT5qGfpUsXKYYWeabA";

        test_vector(path_44, pub_44, addr_44)?;

        let path_49 = r"m/49'/0'/0'";
        let pub_49 = "ypub6Ww3ibxVfGzLrAH1PNcjyAWenMTbbAosGNB6VvmSEgytSER9azLDWCxoJwW7Ke7icmizBMXrzBx9979FfaHxHcrArf3zbeJJJUZPf663zsP";
        let addr_49 = "37VucYSaXLCAsxYyAPfbSi9eh4iEcbShgf";

        test_vector(path_49, pub_49, addr_49)?;

        let path_84 = r"m/84'/0'/0'";
        let pub_84 = "zpub6rFR7y4Q2AijBEqTUquhVz398htDFrtymD9xYYfG1m4wAcvPhXNfE3EfH1r1ADqtfSdVCToUG868RvUUkgDKf31mGDtKsAYz2oz2AGutZYs";
        let addr_84 = "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu";

        test_vector(path_84, pub_84, addr_84)?;

        Ok(())
    }

    // Sample coldcard json from here: https://github.com/Coldcard/firmware/blob/c1d78d12528d7c4b0f12c3a4ea6c18453d424f5e/docs/generic-wallet-export.md
    #[test]
    fn coldcard_import() -> Result<()> {
        let coldcard_json = r#"
            {
    "chain": "XTN",
    "xfp": "0F056943",
    "xpub": "tpubD6NzVbkrYhZ4XzL5Dhayo67Gorv1YMS7j8pRUvVMd5odC2LBPLAygka9p7748JtSq82FNGPppFEz5xxZUdasBRCqJqXvUHq6xpnsMcYJzeh",
    "account": 123,
    "bip44": {
        "deriv": "m/44'/1'/123'",
        "first": "n44vs1Rv7T8SANrg2PFGQhzVkhr5Q6jMMD",
        "name": "p2pkh",
        "xfp": "B7908B26",
        "xpub": "tpubDCiHGUNYdRRGoSH22j8YnruUKgguCK1CC2NFQUf9PApeZh8ewAJJWGMUrhggDNK73iCTanWXv1RN5FYemUH8UrVUBjqDb8WF2VoKmDh9UTo"
    },
    "bip49": {
        "_pub": "upub5DMRSsh6mNak9KbcVjJ7xAgHJvbE3Nx22CBTier5C35kv8j7g2q58ywxskBe6JCcAE2VH86CE2aL4MifJyKbRw8Gj9ay7SWvUBkp2DJ7y52",
        "deriv": "m/49'/1'/123'",
        "first": "2N87V39riUUCd4vmXfDjMWAu9gUCiBji5jB",
        "name": "p2wpkh-p2sh",
        "xfp": "CEE1D809",
        "xpub": "tpubDCDqt7XXvhAdy1MpSze5nMJA9x8DrdRaKALRRPasfxyHpiqWWEAr9cbDBQ9BcX7cB3up98Pk97U2QQ3xrvQsi5dNPmRYYhdcsKY9wwEY87T"
    },
    "bip84": {
        "_pub": "vpub5Y5a91QvDT45EnXQaKeuvJupVvX8f9BiywDcadSTtaeJ1VgJPPXMitnYsqd9k7GnEqh44FKJ5McJfu6KrihFXhAmvSWgm7BAVVK8Gupu4fL",
        "deriv": "m/84'/1'/123'",
        "first": "tb1qc58ys2dphtphg6yuugdf3d0kufmk0tye044g3l",
        "name": "p2wpkh",
        "xfp": "78CF94E5",
        "xpub": "tpubDC7jGaaSE66VDB6VhEDFYQSCAyugXmfnMnrMVyHNzW9wryyTxvha7TmfAHd7GRXrr2TaAn2HXn9T8ep4gyNX1bzGiieqcTUNcu2poyntrET"
    }
}
"#;

        let parsed_coldcard = ColdcardJson::from_str(coldcard_json)?;
        let desc = parsed_coldcard.build_descriptor_string()?;

        // TODO: this only makes sense when we're starting from zero yeah?
        // Regardless of the start index this must be checked
        let next_address = parsed_coldcard.get_first_addresss()?;

        let _address =
            super::check_address(desc.clone(), bitcoin::Network::Testnet, next_address, 0)?;

        Ok(())
    }
}
