use std::env;
use std::str::FromStr;
use cosmrs::tx::SignDoc;
use cosmrs::crypto::secp256k1;
use k256::Secp256k1;
use bip39::{Mnemonic, Language};

/*
    cosmos uses bip39 mnemonics https://docs.cosmos.network/master/basics/accounts.html 

    https://docs.rs/cosmrs/latest/cosmrs/tx/index.html

    1. Read from file, parse JSON. Get mnemonic word array
    2. Convert mnemonic to seed bytes
    3. Paraphrase used by cosmos is empty string
    4. 

*/

fn main() {
    let args: Vec<String> = env::args().collect();
    let param = &args[1];
    println!("First Param lolz {}", param);   
    

    // example:
    // {"name":"user","type":"local","address":"wasm1etqun8gp9hqu0jf4w859syhdr8jhke79gva3uc"
    // "pubkey":"{\"@type\":\"/cosmos.crypto.secp256k1.PubKey\",\"key\":\"AjXphehCZ41aW6FfDMCkZvVHEJ7DrjGeXEn0/P4FN1Sp\"}",}
    // "mnemonic":"sound icon awful viable true material napkin trade lizard taste arrange moment right genius feel clown obey genius armed piece empower gadget axis whisper"

    let mnemonic_str = "sound icon awful viable true material napkin trade lizard taste arrange moment right genius feel clown obey genius armed piece empower gadget axis whisper";
    let mnemonic = Mnemonic::parse_in_normalized(Language::English, mnemonic_str).unwrap();

    let seed = mnemonic.to_seed_normalized("");

    println!("{:?}", seed);

    println!("{:#02x}", seed[0]);
    println!("{:#02x}", seed[1]);
    println!("{:#02x}", seed[2]);

    // let sender_private_key = secp256k1::SigningKey::random();
    // let sender_public_key = sender_private_key.public_key();
    // let sender_account_id = sender_public_key.account_id("wasm").unwrap();
    
    let result = secp256k1::SigningKey::from_bytes(&seed);
    match result {
        Ok(key) => {
            let result = key.sign(b"kekw");
            match result {
                Ok(sign) => println!("Signature of kekw is: {:?}", sign.to_string()),
                Err(err) => println!("Error trying to sign: {:?}", err),
            };        
        }
        Err(err) => {
            println!("Error trying to create signing key: {:?}", err);
            println!("TO_STR = {:?}", err.to_string());
        },
    }


}

