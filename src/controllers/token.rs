use std::{path::Path, str::FromStr};

use actix_web::{
    HttpResponse, post,
    web::{self},
};
use serde::Deserialize;
use serde_json::json;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    program_pack::Pack,
    pubkey::Pubkey,
    signature::{Signature, read_keypair_file},
    signer::{Signer, keypair::Keypair},
    transaction::Transaction,
};
use spl_associated_token_account::{
    get_associated_token_address, instruction::create_associated_token_account,
};
use spl_token::{
    ID,
    instruction::{self, initialize_mint},
    state::Mint,
};
#[post("/keypair")]
async fn create_keypair() -> HttpResponse {
    let pair = Keypair::new();
    HttpResponse::Ok().json(json!({
       "success":true,
       "data" : {
           "pubkey" : pair.pubkey().to_string(),
           "secret": pair.to_base58_string()
       }
    }))
}

#[derive(Deserialize)]
struct CreateToken {
    mint_authority: String,
    decimals: u8,
}

#[post("/token/create")]
async fn create_token(body: web::Json<CreateToken>) -> HttpResponse {
    let mint_authority = match Pubkey::from_str(&body.mint_authority) {
        Ok(pubkey) => pubkey,
        Err(e) => {
            return HttpResponse::BadRequest().json(json!({
                "success": false,
                "error": format!("Invalid mint_authority: {}", e),
            }));
        }
    };
    let decimals = body.decimals;
    let mint_keypair = Keypair::new();
    let mint = mint_keypair.pubkey();

    let rpc_url = "https://api.devnet.solana.com";
    let client = RpcClient::new(rpc_url.to_owned());

    let rent_exempt_amount = match client
        .get_minimum_balance_for_rent_exemption(Mint::LEN)
        .await
    {
        Ok(amount) => amount,
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({
                "success": false,
                "error": format!("Failed to calculate rent: {}", e),
            }));
        }
    };
    let payer = match read_keypair_file(Path::new("/home/aashim/.config/solana/id.json")) {
        Ok(p) => p,
        Err(e) => {
            return HttpResponse::BadRequest().json(json!({
                 "success": false,
                "error": format!("Failed to read keypair file: {}", e),
            }));
        }
    };
    let create_account_instruction = solana_sdk::system_instruction::create_account(
        &payer.pubkey(),
        &mint,
        rent_exempt_amount,
        Mint::LEN as u64,
        &ID,
    );

    let initialize_mint_instruction =
        initialize_mint(&ID, &mint, &mint_authority, None, decimals).unwrap();

    let recent_blockhash = match client.get_latest_blockhash().await {
        Ok(hash) => hash,
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({
                "success": false,
                "error": format!("Failed to get blockhash: {}", e),
            }));
        }
    };

    let transaction = Transaction::new_signed_with_payer(
        &[create_account_instruction, initialize_mint_instruction],
        Some(&payer.pubkey()),
        &[&payer, &mint_keypair],
        recent_blockhash,
    );

    match client
        .send_and_confirm_transaction_with_spinner(&transaction)
        .await
    {
        Ok(signature) => HttpResponse::Ok().json(json!({
            "success": true,
            "data": {
                "mint": mint.to_string(),
                "mint_authority": mint_authority.to_string(),
                "transaction_signature": signature.to_string(),
            }
        })),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "success": false,
            "error": format!("Transaction failed: {}", e),
        })),
    }
}

#[derive(Debug, Deserialize)]
pub struct TokenTransferRequest {
    pub mint: String,
    pub destination: String,
    pub authority: String,
    pub amount: u64,
}
#[post("/token/mint")]
async fn mint_token(body: web::Json<TokenTransferRequest>) -> HttpResponse {
    let mint_address = match Pubkey::from_str(&body.mint) {
        Ok(p) => p,
        Err(e) => {
            return HttpResponse::BadRequest().json(json!({
                "success":false,
                "error": format!("Error: {}",e)
            }));
        }
    };
    let destination = match Pubkey::from_str(&body.destination) {
        Ok(p) => p,
        Err(e) => {
            return HttpResponse::BadRequest().json(json!({
                "success":false,
                "error": format!("Invalid destination: {}", e)
            }));
        }
    };
    let authority = match Pubkey::from_str(&body.authority) {
        Ok(p) => p,
        Err(e) => {
            return HttpResponse::BadRequest().json(json!({
                "success":false,
                "error": format!("Invalid authority: {}", e)
            }));
        }
    };
    let amount = body.amount;
    let signer = match read_keypair_file(Path::new("/home/aashim/.config/solana/id.json")) {
        Ok(p) => p,
        Err(e) => {
            return HttpResponse::BadRequest().json(json!({
                 "success": false,
                "error": format!("Failed to read keypair file: {}", e),
            }));
        }
    };
    let rpc_url = "https://api.devnet.solana.com";
    let client = RpcClient::new(rpc_url.to_owned());
    let associated_account = get_associated_token_address(&destination, &mint_address);
    let associated_instruction =
        create_associated_token_account(&signer.pubkey(), &destination, &mint_address, &ID);
    let mint_instruction = instruction::mint_to(
        &ID,
        &mint_address,
        &associated_account,
        &authority,
        &[&signer.pubkey()],
        amount,
    )
    .unwrap();
    let recent_blockhash = client.get_latest_blockhash().await.unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[associated_instruction, mint_instruction],
        Some(&signer.pubkey()),
        &[&signer],
        recent_blockhash,
    );
    client
        .send_and_confirm_transaction_with_spinner(&transaction)
        .await
        .unwrap();
    HttpResponse::Ok().json(json!({
        "status": true,
        "data":{
            "program_id": ID,
            "accounts": [{
                "pubkey": associated_account,
                "is_signer":false,
                "is_writable":true}
            ],
        }
    }))
}

#[derive(Debug, Deserialize)]
pub struct SignRequest {
    pub message: String,
    pub secret: String,
}
#[post("/message/sign")]
async fn sign_message(body: web::Json<SignRequest>) -> HttpResponse {
    let signer = match read_keypair_file(Path::new("/home/aashim/.config/solana/id.json")) {
        Ok(p) => p,
        Err(e) => {
            return HttpResponse::BadRequest().json(json!({
                 "success": false,
                "error": format!("Failed to read keypair file: {}", e),
            }));
        }
    };
    let combined_message = format!("{}{}", &body.message, &body.secret);
    let signature = signer.sign_message(&combined_message.as_bytes());
    HttpResponse::Ok().json(json!({
        "success":true,
        "signature": signature,
        "message":&body.message,
        "public_key":signer.pubkey()
    }))
}

#[derive(Debug, Deserialize)]
pub struct VerifyRequest {
    pub message: String,
    pub signature: Signature,
    pub pubkey: String,
    pub secret: Option<String>,
}
#[post("/verify")]
async fn verify_signature(body: web::Json<VerifyRequest>) -> HttpResponse {
    let pubkey = match Pubkey::from_str(&body.pubkey) {
        Ok(p) => p,
        Err(e) => {
            return HttpResponse::BadRequest().json(json!({
                "success": false,
                "error": format!("Invalid public key: {}", e),
            }));
        }
    };
    let signature = &body.signature;
    let message_to_verify = match &body.secret {
        Some(secret) => format!("{}{}", &body.message, secret),
        None => body.message.clone(),
    };
    let is_valid = signature.verify(pubkey.as_ref(), message_to_verify.as_bytes());
    HttpResponse::Ok().json(json!({
        "success": true,
        "verified": is_valid,
        "details": {
            "message": body.message,
            "public_key": pubkey.to_string(),
            "used_secret": body.secret.is_some()
        }
    }))
}
