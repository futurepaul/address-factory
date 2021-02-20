use std::{fmt, fs::{self, File}, path::PathBuf};

use anyhow::Result;
use bdk::{
    bitcoin::{self, secp256k1::Secp256k1, Address},
    database::MemoryDatabase,
    descriptor::ExtendedDescriptor,
    Wallet,
};
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};

use crate::{gpg_clearsign, util, util::Desc, Database, Entry};

#[derive(Serialize, Deserialize, Debug)]
pub struct Factory {
    pub descriptor: Desc,
    pub next_index: u64,
    pub number_to_generate: u64,
    pub next_address: Address,
    pub message: String,
    pub network: bitcoin::Network,
}

impl fmt::Display for Factory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Descriptor: {}", self.descriptor)?;
        writeln!(f, "Next index: {}", self.next_index)?;
        writeln!(f, "Number to generate: {}", self.number_to_generate)?;
        writeln!(f, "Next address: {}", self.next_address)?;
        writeln!(f, "Message: {}", self.message)?;
        write!(f, "Network: {}", self.network)
    }

}

impl Factory {
    /// Create a struct containing all the information necessary to derive more addresses
    pub fn new(
        descriptor: String,
        network: bitcoin::Network,
        next_index: u64,
        number_to_generate: u64,
        message: String,
    ) -> Result<Self> {
        let secp = Secp256k1::new();
        let (desc, _) = ExtendedDescriptor::parse_descriptor(&secp, &descriptor)?;

        let next_address = util::nth_address(desc.clone(), network, next_index)?;

        Ok(Self {
            descriptor: desc,
            next_index,
            number_to_generate,
            next_address,
            message,
            network,
        })
    }

    /// Get path to .json from user
    pub fn from_path(path: PathBuf) -> Result<Self> {
        let state_json = fs::read_to_string(path)?;
        let gen_state = serde_json::from_str(&state_json)?;

        Ok(gen_state)
    }

    /// Increment the next_index
    pub fn finish(&mut self, peek_next_address: Address) {
        let old_next_index = self.next_index;
        self.next_index = old_next_index + self.number_to_generate;
        self.next_address = peek_next_address;
    }

    /// Save the struct as .json
    pub fn save(&self) -> Result<()> {
        let f = File::create("address-factory.json")?;
        serde_json::to_writer_pretty(f, self)?;
        Ok(())
    }

    /// Check that first address derived matches given address
    pub fn check_next_address(&self) -> Result<Address> {
        util::check_address(
            self.descriptor.clone(),
            self.network,
            self.next_address.clone(),
            self.next_index,
        )
    }

    pub fn generate_addresses(&mut self) -> Result<()> {
        self.check_next_address()?;
        let desc = self.descriptor.clone();

        let wallet = Wallet::new_offline(desc, None, self.network, MemoryDatabase::default())?;

        // Skip all these addresses by asking for them
        // TODO: figure out how to just start from an index
        if self.next_index > 0 {
            println!("Skipping addresses...");
            for _i in 0..self.next_index {
                wallet.get_new_address()?;
            }
        }

        // Create a new SQLite db file and connect to it
        let db = Database::new()?;

        // Don't want people staring at a blank prompt for minutes
        let pb = ProgressBar::new(self.number_to_generate);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("[{elapsed}] [{bar:40.green}] {pos}/{len} (eta: {eta})")
                .progress_chars("#>-"),
        );

        let mut addresses: Vec<Address> = vec![];

        println!("Generating addresses...");

        for _i in 0..self.number_to_generate {
            let address = wallet.get_new_address()?;
            addresses.push(address);
            pb.inc(1);
        }

        pb.finish();

        let message_text = &self.message;

        // Would be nice to do this in parallel with rayon but gpg doesn't like that
        println!("PGP signing addresses...");
        let pb = ProgressBar::new(self.number_to_generate);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("[{elapsed}] [{bar:40.green}] {pos}/{len} (eta: {eta})")
                .progress_chars("#>-"),
        );
        for address in addresses {
            let address = address.to_string();
            let signed_message = gpg_clearsign(&address.to_string(), message_text).unwrap();
            pb.inc(1);
            let entry = Entry::new(&address, &signed_message);
            db.insert(entry)?;
        }

        pb.finish();

        self.finish(wallet.get_new_address()?);
        self.save()?;

        println!(
            "Wrote {} addresses and PGP signed messages to {}",
            self.number_to_generate, db.filename
        );

        println!(
            "Saved this setup to address-factory.json.\nUse that file next time to pick up where you left off."
        );

        Ok(())
    }
}
