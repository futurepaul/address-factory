use anyhow::Result;
use std::str::FromStr;

use bdk::bitcoin::{
    util::bip32::{DerivationPath, ExtendedPubKey},
    Address,
};
use serde::Deserialize;

use crate::{util, Desc};

#[derive(Debug, Deserialize)]
pub struct Bip84Json {
    _pub: String,
    pub deriv: String,
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

impl ColdcardJson {
    pub fn build_descriptor_string(&self) -> Result<Desc> {
        if &self.chain != "XTN" {
            panic!("We only support tpub right now")
        }

        let derivation_path = DerivationPath::from_str(&self.bip84.deriv)?;
        let xpub = ExtendedPubKey::from_str(&self.bip84.xpub)?;

        util::build_descriptor(xpub, derivation_path)
    }

    pub fn get_first_addresss(&self) -> Result<Address> {
        let address = Address::from_str(&self.bip84.first)?;
        Ok(address)
    }
}

impl FromStr for ColdcardJson {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
    }
}
