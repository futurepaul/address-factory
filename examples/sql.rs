use std::{fs, str::FromStr};

use anyhow::Result;
use bdk::{bitcoin, database::MemoryDatabase, Wallet};
use signed_address_generator::{gpg_clearsign, ColdcardJson, Database};

fn main() -> Result<()> {
    let db = Database::new()?;

    let wallet_json = fs::read_to_string("coldcard-export.json")?;
    let parsed_coldcard = ColdcardJson::from_str(&wallet_json)?;
    let desc = parsed_coldcard.get_descriptor(bitcoin::Network::Testnet);
    let wallet = Wallet::new_offline(
        desc,
        None,
        bitcoin::Network::Testnet,
        MemoryDatabase::default(),
    )?;

    let address = wallet.get_new_address()?.to_string();
    let message = gpg_clearsign(&address, "Testing 123")?;

    db.insert(&address, &message)?;

    db.print_entries()?;

    Ok(())
}
