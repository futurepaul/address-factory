use std::str::FromStr;
use anyhow::Result;

use bdk::descriptor::get_checksum;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Bip84Json {
    _pub: String,
    deriv: String,
    pub first: String,
    name: String,
    xfp: String,
    pub xpub: String,
}

#[derive(Debug, Deserialize)]
pub struct ColdcardJson {
    chain: String,
    pub xfp: String,
    xpub: String,
    // TODO: use the account, yes?
    account: u64,
    // TODO: use other address types?
    pub bip84: Bip84Json,
}

fn build_descriptor(xpub: &str, fingerprint: &str) -> String {
    // m / purpose' / coin_type' / account' / change / index
    // TODO: use the account & other address types?
    let hardened_derivation_path = "m/84h/1h/0h";

    let origin_prefix = hardened_derivation_path.replace("m", &fingerprint.to_lowercase());

    let descriptor_part = format!("[{}]{}", origin_prefix, xpub);

    let is_change = false;
    let inner = format!("{}/{}/*", descriptor_part, is_change as u32);
    // TODO: use the selected address type (not hardcoded)
    let descriptor = format!("wpkh({})", inner);

    format!("{}#{}", descriptor, get_checksum(&descriptor).unwrap())
}

impl ColdcardJson {
    pub fn build_descriptor_string(&self) -> String {
        if &self.chain != "XTN" {
            panic!("We only support tpub right now")
        }

        build_descriptor(&self.bip84.xpub, &self.xfp)
    }

    pub fn get_first_addresss(&self) -> String {
        self.bip84.first.clone() 
    }
}

impl FromStr for ColdcardJson {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
    }
}
