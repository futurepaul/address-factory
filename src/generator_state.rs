use std::{fs::{self, File}, path::PathBuf};

use anyhow::{bail, anyhow, Result};
use bdk::{Wallet, bitcoin::{self, Address}, database::MemoryDatabase, descriptor::{ExtendedDescriptor, KeyMap, ToWalletDescriptor}};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct GeneratorState {
    // TODO: could this be more strongly typed?
    descriptor: String,
    pub next_index: u64,
    pub number_to_generate: u64,
    next_address: String,
    pub message: String,
}

impl GeneratorState {
    pub fn new(descriptor: String, next_index: u64, number_to_generate: u64, next_address: String, message: String) -> Self {
        Self {
            descriptor,
            next_index,
            number_to_generate,
            next_address,
            message,
        }
    }
    pub fn from_path(path: PathBuf) -> Result<Self> {
        let state_json = fs::read_to_string(path)?;
        let gen_state = serde_json::from_str(&state_json)?;

        Ok(gen_state)
    }

    pub fn finish(&mut self, peek_next_address: String) {
        let old_next_index = self.next_index;
        self.next_index = old_next_index + self.number_to_generate; 
        self.next_address = peek_next_address;
    }

    pub fn save(&self) -> Result<()> {
        let mut f = File::create("signed-address-generator-state.json")?;
        serde_json::to_writer_pretty(f, self)?;
        Ok(())
    }

pub fn check_first_address(&self, wallet: &Wallet<(), MemoryDatabase>) -> Result<Address> {
        let address = wallet.get_new_address()?;
        let next = self.next_address.clone(); 
        println!("Checking for a match\naddy: {}\nshould be: {}", address.clone(), next.clone());
        if address.to_string() == next {
            Ok(address) } else {
            bail!("The first generated address isn't what we expected it to be. There might be something wrong with the descriptor / xpub / derivation path.\nGot {}\nExpected {}", address, next)
        }
    }

    pub fn get_descriptor(
        &self,
        network: bitcoin::Network,
    ) -> Result<(ExtendedDescriptor, KeyMap)> {
        match self.descriptor.to_wallet_descriptor(network) {
            Ok(descriptor_pair) => Ok(descriptor_pair),
            Err(e) => Err(anyhow!("{}", e)),
        }
    }
}
