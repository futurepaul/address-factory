use anyhow::{bail, Context, Result};
use bdk::{Wallet, bitcoin::{Network, util::bip32::{ChildNumber, DerivationPath, ExtendedPubKey}}, database::MemoryDatabase, descriptor::get_checksum};

#[derive(Debug)]
pub struct GenericXpub {
    network: Network,
    xpub: ExtendedPubKey,
    derivation_path: DerivationPath,
    fingerprint: String,
    first_address: String,
    pub descriptor: String,
}

pub enum ScriptType {
    Classic,
    NativeSegwit,
    WrappedSegwit,
}

fn script_type(path: &DerivationPath) -> Result<ScriptType> {
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

fn build_descriptor(
    xpub: ExtendedPubKey,
    derivation_path: DerivationPath,
    fingerprint: &str,
) -> Result<String> {
    // m / purpose' / coin_type' / account' / change / index
    // TODO: if network is regtest or testnet, make sure coin_type is 1

    let derivation_path_string = derivation_path
        .to_string()
        .replace("m", &fingerprint.to_lowercase())
        .replace("'", "h");

    let descriptor_part = format!("[{}]{}", derivation_path_string, xpub);

    let is_change = false;

    let inner = format!("{}/{}/*", descriptor_part, is_change as u32);

    let script_type = script_type(&derivation_path)?;

    let descriptor = match script_type {
        ScriptType::Classic => {
            format!("pkh({})", inner)
        }
        ScriptType::NativeSegwit => {
            format!("wpkh({})", inner)
        }
        ScriptType::WrappedSegwit => {
            format!("sh(wpkh({}))", inner)
        }
    };

    let checksum = get_checksum(&descriptor)?;

    Ok(format!("{}#{}", descriptor, checksum))
}

impl GenericXpub {
    pub fn new(
        xpub: ExtendedPubKey,
        derivation_path: DerivationPath,
        fingerprint: String,
        first_address: String,
        network: Network,
    ) -> Result<Self> {
        let descriptor = build_descriptor(xpub, derivation_path.clone(), &fingerprint.trim())?;

        // Check the first address just to be safe
        let wallet = Wallet::new_offline(&descriptor, None, network, MemoryDatabase::default())?;
        let wallet_first = wallet.get_new_address()?;

        if wallet_first.to_string() == first_address {
            Ok(Self {
                xpub,
                derivation_path,
                fingerprint,
                first_address,
                network,
                descriptor,
            })
        } else {
            bail!("Incorrect first address derived. Check descriptor / xpub / derivation path.")
        }
    }
}

// TODO: write some tests using these sample addresses:
// https://github.com/satoshilabs/slips/blob/master/slip-0132.md
#[cfg(test)]
mod tests {
    use anyhow::Result;
    use std::str::FromStr;

    use bdk::bitcoin::{self, util::bip32::ExtendedPubKey};
    use slip132::FromSlip132;

    use crate::GenericXpub;

    fn test_vector(path: &str, extended_public_key: &str, first_address: &str) -> Result<GenericXpub> {
        let xpub =
            ExtendedPubKey::from_slip132_str(&extended_public_key).expect("Failed to make an xpub");
        let derivation_path = bitcoin::util::bip32::DerivationPath::from_str(path)
            .expect("Failed to make a derivation path");
        let fingy = xpub.parent_fingerprint.to_string();

        GenericXpub::new(
            xpub,
            derivation_path,
            fingy,
            first_address.to_string(),
            bitcoin::Network::Bitcoin,
        )

    }

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
}
