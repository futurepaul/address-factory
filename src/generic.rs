use anyhow::{bail, Result};
use bdk::{bitcoin::Network, database::MemoryDatabase, descriptor::get_checksum, Wallet};

#[derive(Debug)]
pub struct GenericXpub {
    network: Network,
    xpub: String,
    derivation_path: String,
    fingerprint: String,
    first_address: String,
    pub descriptor: String,
}

pub enum ScriptType {
    Classic,
    NativeSegwit,
    WrappedSegwit,
}

fn script_type_from_path(path: &str) -> Result<ScriptType> {
    let split_path: Vec<&str> = path.split("/").collect();

    // TODO: learn a better Rust-ey way of handling this
    let version = if split_path.get(1).is_some() {
        split_path[1]
    } else {
        bail!("Something is wrong with this derivation path: {}", path)
    };

    let version = version.trim_end_matches(|c| c == '\'' || c == 'h');

    match version {
        "44" => Ok(ScriptType::Classic),
        "49" => Ok(ScriptType::WrappedSegwit),
        "84" => Ok(ScriptType::NativeSegwit),
        _ => bail!("Didn't recognize the version number: {}", version),
    }
}

fn build_descriptor(
    xpub: &str,
    derivation_path: &str,
    fingerprint: &str,
) -> Result<String> {
    // m / purpose' / coin_type' / account' / change / index
    // TODO: if network is regtest or testnet, make sure coin_type is 1
    let origin_prefix = derivation_path.replace("m", &fingerprint.to_lowercase());
    let hardened = origin_prefix.replace("'", "h");

    let descriptor_part = format!("[{}]{}", hardened, xpub);

    let is_change = false;

    let inner = format!("{}/{}/*", descriptor_part, is_change as u32);

    let script_type = script_type_from_path(derivation_path)?;

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
        xpub: String,
        derivation_path: String,
        fingerprint: String,
        first_address: String,
        network: Network,
    ) -> Result<Self> {
        let descriptor = build_descriptor(
            &xpub.trim(),
            &derivation_path.trim(),
            &fingerprint.trim(),
        )?;

        // Check the first address just to be safe
        let wallet = Wallet::new_offline(&descriptor, None, network, MemoryDatabase::default())?;
        let wallet_first = wallet.get_new_address()?;
        dbg!(wallet_first.to_string());

        if wallet_first.to_string() == first_address {
            Ok(Self {
                xpub,
                derivation_path,
                fingerprint,
                first_address,
                network,
                descriptor
            })
        } else {
            bail!("Incorrect first address derived. Check descriptor / xpub / derivation path.")
        }
    }
}

// TODO: write some tests using these sample addresses:
// https://github.com/satoshilabs/slips/blob/master/slip-0132.md
