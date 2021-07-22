use std::str::FromStr;
use bitcoin::Address;
use bitcoin::util::psbt::PartiallySignedTransaction;
use bitcoin::consensus::{deserialize, encode::serialize};
use std::u32;
use bitcoin::{Network};
use bitcoin::secp256k1::Secp256k1;
use bdk::Error;
use anyhow::{bail, ensure, Context, Result};
use bdk::{Wallet, database::MemoryDatabase};
use bdk::wallet::{coin_selection::DefaultCoinSelectionAlgorithm, AddressIndex};
use bdk::keys::bip39::{Mnemonic, MnemonicType, Language};
use bdk::keys::{DerivableKey, ExtendedKey};
use bdk::blockchain::{electrum, noop_progress};
use bdk::electrum_client::Client;
use bdk::KeychainKind;
use bdk::SignOptions;
use std::collections::BTreeMap;

#[derive(Debug)]
pub struct Keys {
    xprv: String,
    fingerprint: String,
    phrase: String,
}
#[derive(Debug, Clone)]
enum Mode {
    Descriptor,
    Balance {
        descriptor: String,
        change_descriptor: String,
    },
    Receive {
        descriptor: String,
        index: u32,
    },
    Build {
        descriptor: String,
        change_descriptor: String,
        amount: u64,
        destination: String,
    },
    Send {
        descriptor: String,
        psbt: String,
    },
}

const HELP: &str = "\
USAGE:
  mori <MODE>
FLAGS:
  -h, --help            Prints help information
MODE:
  descriptor            Get a new wallet descriptor
  balance               Show user balance
  receive               Return a bitcoin address
  build                 Create bitcoin psbt
  send                  Broadcast psbt
";

fn main() {
    let mode = match parse_args() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Error: {}.", e);
            std::process::exit(1);
        }
    };

    match execute(mode) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Error: {}.", e);
            std::process::exit(1);
        }
    }
}

fn generate_xpriv() -> Result<Keys, Error> {
    let mnemonic = Mnemonic::new(MnemonicType::Words12, Language::English);
    let phrase = mnemonic.phrase().to_string();
    let secp = Secp256k1::new();
    let xkey: ExtendedKey = mnemonic.into_extended_key()?;
    let xprv = xkey.into_xprv(Network::Testnet).unwrap();
    let fingerprint = xprv.fingerprint(&secp);
    Ok(Keys {
        xprv: xprv.to_string(),
        fingerprint: fingerprint.to_string(),
        phrase,
    })
}

fn create_wallet(
    desc_string: &str,
    change_desc: Option<&str>,
) -> Result<Wallet<electrum::ElectrumBlockchain, MemoryDatabase>> {
    // Create a SSL-encrypted Electrum client
    let client = Client::new("ssl://electrum.blockstream.info:60002")?;

    // Create a BDK wallet
    let wallet = Wallet::new(
        // Our wallet descriptor
        desc_string,
        // Descriptor used for generating change addresses
        change_desc,
        // Which network we'll using. If you change this to `Bitcoin` things get real.
        bitcoin::Network::Testnet,
        // In-memory ephemeral database. There's also a default key value storage provided by BDK if you want persistence.
        MemoryDatabase::default(),
        // This wrapper implements the blockchain traits BDK needs for this specific client type
        electrum::ElectrumBlockchain::from(client),
    )?;

    println!("Syncing...");

    // Important! We have to sync our wallet with the blockchain.
    // Because our wallet is ephemeral we need to do this on each run, so I put it in `create_wallet` for convenience.
    wallet.sync(noop_progress(), None)?;

    Ok(wallet)
}

fn get_descriptor(alice: &Keys, bob: &Keys, change: &char) -> Result<String> {
    let alice_derivation_path = format!(
        "[{}/87h/1h/0h]{}/{}/*",
        alice.fingerprint,
        alice.xprv,
        change
    );
    let bob_derivation_path = format!(
        "[{}/87h/1h/0h]{}/{}/*",
        bob.fingerprint,
        bob.xprv,
        change,
    );
    let descriptor_rcv = format!(
        "wsh(or_d(pk({}),and_v(v:pk({}),older(25920))))",
        alice_derivation_path,
        bob_derivation_path,
    );
    Ok(descriptor_rcv)
}

fn parse_args() -> Result<Mode> {
    let mut pargs = pico_args::Arguments::from_env();

    // Help has a higher priority and should be handled separately.
    if pargs.contains(["-h", "--help"]) {
        print!("{}", HELP);
        std::process::exit(0);
    }

    let subcommand = pargs.subcommand()?;
    ensure!(
        subcommand.is_some(),
        "Need to pick a mode: balance || receive || send || create"
    );

    let info = match subcommand.unwrap().as_str() {
        "descriptor" => Mode::Descriptor,
        "balance" => Mode::Balance {
            descriptor: pargs
                .value_from_str("--desc")
                .context("Missing descriptor")?,
            change_descriptor: pargs
                .value_from_str("--change")
                .context("Missing change descriptor")?,
        },
        "receive" => Mode::Receive {
            descriptor: pargs
                .value_from_str("--desc")
                .context("Missing descriptor")?,
            index: pargs
                .value_from_str("--index")
                .context("Missing address index")?,
        },
        "build" => Mode::Build {
            descriptor: pargs
                .value_from_str("--desc")
                .context("Missing descriptor")?,
            change_descriptor: pargs
                .value_from_str("--change")
                .context("Missing change descriptor")?,
            amount: pargs
                .value_from_str("--amount")
                .context("Missing amount")?,
            destination: pargs
                .value_from_str("--destination")
                .context("Missing destination addresss")?,
        },
        "send" => Mode::Send {
            descriptor: pargs
                .value_from_str("--desc")
                .context("Missing descriptor")?,
            psbt: pargs
                .value_from_str("--psbt")
                .context("Missing Partially signed bitcoin transactions")?,
        },
        _ => bail!("Unknown mode"),
    };

    Ok(info)
}

fn execute(mode: Mode) -> Result<()> {
    match mode {
        Mode::Descriptor => {
            let alice = generate_xpriv().unwrap();
            let bob = generate_xpriv().unwrap();
            let descriptor_rcv = get_descriptor(&alice, &bob, &'0').unwrap();
            let descriptor_chg = get_descriptor(&alice, &bob, &'1').unwrap();
            println!("Receiving descriptor: {}", descriptor_rcv);
            println!("Change descriptor: {}", descriptor_chg);
            Ok(())
        }
        Mode::Balance {
            descriptor,
            change_descriptor,
        } => {
            // We need to include the change descriptor to correctly calculate the balance
            let wallet = create_wallet(&descriptor, Some(&change_descriptor))?;
            let balance = wallet.get_balance()?;
            println!("{} sats", balance);
        
            // List unspent ouputs
            println!("{:#?}", wallet.list_unspent());
        
            Ok(())
        }
        Mode::Receive { descriptor, index } => {
            let wallet = create_wallet(&descriptor, None)?;
            // Derives an address based on the descriptor
            let info = wallet.get_address(AddressIndex::Peek(index))?;

            // Show the address
            println!("{}", info.address);

            Ok(())
        }
        Mode::Build {
            descriptor,
            change_descriptor,
            amount,
            destination,
        } => {
            let wallet = create_wallet(&descriptor, Some(&change_descriptor))?;
            // We need the policy to get the root id
            let policy = wallet.policies(KeychainKind::External).unwrap().unwrap();

            let mut path = BTreeMap::new();
            path.insert(policy.id, vec![0]);
            // We parse the address and convert it to a script pubkey
            let dest_script = Address::from_str(destination.as_str())?.script_pubkey();

            // Create a transaction builder TxBuilder
            let mut tx_builder = wallet
            .build_tx()
            .coin_selection(DefaultCoinSelectionAlgorithm::default());
      
            // Add our script and the amount in sats to send
            tx_builder
                .add_recipient(dest_script, amount)
                .policy_path(path.clone(), KeychainKind::External)
                .policy_path(path.clone(), KeychainKind::Internal);

            // "Finish" the builder which returns a tuple:
            // A `PartiallySignedTransaction` which serializes as a psbt
            // And `TransactionDetails` which has helpful info about the transaction we just built
            let (psbt, details) = tx_builder.finish()?;
            println!("{:#?}", details);
            println!("================ PSBT ================");
            println!("{}", base64::encode(&serialize(&psbt)));
            println!("================ PSBT ================");

            Ok(())
        }
        Mode::Send {
            descriptor,
            psbt,
         } => {
            let wallet = create_wallet(&descriptor, None)?;

            // Deserialize the psbt. First as a Vec of bytes, then as a strongly typed `PartiallySignedTransaction`
            let psbt = base64::decode(&psbt)?;
            let mut psbt: PartiallySignedTransaction = deserialize(&psbt)?;
      
            // Default options for finalizing the transaction
            let sign_options = SignOptions::default();

            wallet.sign(&mut psbt, sign_options)?;
      
            // Get the transaction out of the PSBT so we can broadcast it
            let tx = psbt.extract_tx();
      
            // Broadcast the transaction using our chosen backend, returning a `Txid` or an error
            let txid = wallet.broadcast(tx)?;
      
            println!("TxId: {:#?}", txid);
      
            Ok(())
        }
    }
}