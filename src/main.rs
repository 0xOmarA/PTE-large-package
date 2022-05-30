
use radix_engine::model::{SignedTransaction, Instruction};
use radix_engine::transaction::TransactionBuilder;
use scrypto::prelude::*;

use std::io::Read;

// Used to handle the JSON serialization and deserialization
use serde::{Deserialize, Serialize};

mod utils;
use utils::{DecompileError, decompile};

fn main() {
    let f = std::fs::File::open("2mb_package.wasm").unwrap();
    let mut reader = std::io::BufReader::new(f);
    let mut buffer: Vec<u8> = Vec::new();
    
    // Read file into vector.
    reader.read_to_end(&mut buffer).unwrap();

    // Publishing the package to the PTE
    let package_publish_tx: SignedTransaction = TransactionBuilder::new()
        .publish_package(&buffer[..])
        .build(12)
        .sign([]);
    let package_publish_receipt: Receipt = submit_transaction(&package_publish_tx).unwrap();

    println!("Receipt from package is: {:?}", package_publish_receipt);
}

// =====================================================================================================================
// Additional code required to support the above function
// =====================================================================================================================


/// Submits the transaction to the PTE01 server.
pub fn submit_transaction(transaction: &SignedTransaction) -> Result<Receipt, TransactionSubmissionError> {
    // Getting the nonce used in the transaction from the transaction object itself
    let nonce: u64 = {
        let nonce_instructions: Vec<Instruction> = transaction.transaction.instructions
            .iter()
            .filter(|x| {
                match x {
                    Instruction::Nonce { nonce: _ } => true,
                    _ => false
                }
            })
            .cloned()
            .collect();

        if nonce_instructions.len() == 0 {
            Err(TransactionSubmissionError::NoNonceFound)
        } 
        else if nonce_instructions.len() == 1{ 
            if let Instruction::Nonce { nonce } = nonce_instructions[0] {
                Ok(nonce)
            } else {
                panic!("Expected a nonce");
            }
        } 
        else {
            Err(TransactionSubmissionError::MultipleNonceFound)
        }
    }?;
    let nonce: Nonce = Nonce { value: nonce };

    let signatures: Vec<Signature> = transaction.signatures
        .iter()
        .map(|x| Signature{
            public_key: x.0.to_string(), 
            signature: x.1.to_string()
        })
        .collect();

    // Creating the transaction body object which is what will be submitted to the PTE
    let transaction_body: TransactionBody = TransactionBody {
        manifest: decompile(&transaction.transaction)?,
        nonce: nonce,
        signatures: signatures
    };

    // Submitting the transaction to the PTE's `/transaction` endpoint
    let response = reqwest::blocking::Client::new()
        .post("https://pte01.radixdlt.com/transaction")
        .json(&transaction_body)
        .send()?;

    let response_body: String = response.text().unwrap();
    if let Ok(receipt) = serde_json::from_str(&response_body) {
        return Ok(receipt)
    } else {
        return Err(TransactionSubmissionError::JsonDeserializationError(format!("Can not deserialize string: {}", response_body)));
    };
}

/// A struct which describes the Nonce. Required for the TransactionBody struct
#[derive(Serialize, Deserialize, Debug)]
pub struct Nonce {
    value: u64,
}

/// A struct which defines the signature used in the TransactionBody struct.
#[derive(Serialize, Deserialize, Debug)]
pub struct Signature {
    public_key: String,
    signature: String,
}

/// A struct which defines the transaction payload that the PTE's API accepts.
#[derive(Serialize, Deserialize, Debug)]
pub struct TransactionBody {
    manifest: String,
    nonce: Nonce,
    signatures: Vec<Signature>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Receipt {
    pub transaction_hash: String,
    pub status: String,
    pub outputs: Vec<String>,
    pub logs: Vec<String>,
    pub new_packages: Vec<String>,
    pub new_components: Vec<String>,
    pub new_resources: Vec<String>,
}

impl Receipt {
    pub fn new_packages(&self) -> Vec<PackageAddress> {
        return self.new_packages
            .iter()
            .map(|x| PackageAddress::from_str(x).unwrap())
            .collect()
    }
    
    pub fn new_components(&self) -> Vec<ComponentAddress> {
        return self.new_components
            .iter()
            .map(|x| ComponentAddress::from_str(x).unwrap())
            .collect()
    }
    
    pub fn new_resources(&self) -> Vec<ResourceAddress> {
        return self.new_resources
            .iter()
            .map(|x| ResourceAddress::from_str(x).unwrap())
            .collect()
    }
}

/// An enum of the errors which could occur when submitting a transaction to the PTE API.
#[derive(Debug)]
pub enum TransactionSubmissionError {
    NoNonceFound,
    MultipleNonceFound,
    DecompileError(DecompileError),
    HttpRequestError(reqwest::Error),
    JsonDeserializationError(String)
}

impl From<utils::DecompileError> for TransactionSubmissionError {
    fn from(error: DecompileError) -> TransactionSubmissionError {
        TransactionSubmissionError::DecompileError(error)
    }
}

impl From<reqwest::Error> for TransactionSubmissionError {
    fn from(error: reqwest::Error) -> TransactionSubmissionError {
        TransactionSubmissionError::HttpRequestError(error)
    }
}