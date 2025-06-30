use actix_web::{App, HttpResponse, HttpServer, get, http, post, web};
use base58::FromBase58;
use base64;
use serde::{Deserialize, Serialize};
use serde_json::json;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};
use spl_token::instruction::mint_to;
use std::str::FromStr;

#[post("/keypair")]
async fn create_keypair() -> HttpResponse {
    let key_pair = Keypair::new();
    return HttpResponse::Ok().json(json!({
        "success":true,
        "data": {
            "pubkey": &key_pair.pubkey().to_string(),
            "secret": &key_pair.to_base58_string()
        }
    }));
}

#[derive(Debug, Deserialize)]
struct MintTokenRequest {
    mint: String,
    destination: String,
    authority: String,
    amount: u64,
}

#[derive(Debug, Serialize)]
struct AccountMetaResponse {
    pubkey: String,
    is_signer: bool,
    is_writable: bool,
}

#[derive(Debug, Serialize)]
struct InstructionData {
    program_id: String,
    accounts: Vec<AccountMetaResponse>,
    instruction_data: String,
}

#[derive(Debug, Serialize)]
struct MintTokenResponse {
    success: bool,
    data: InstructionData,
}

#[post("/token/mint")]
async fn token_mint(body: web::Json<MintTokenRequest>) -> HttpResponse {
    // Parse all public keys
    let mint_pubkey = match Pubkey::from_str(&body.mint) {
        Ok(pk) => pk,
        Err(e) => return bad_request_response(format!("Invalid mint pubkey: {}", e)),
    };

    let destination_pubkey = match Pubkey::from_str(&body.destination) {
        Ok(pk) => pk,
        Err(e) => return bad_request_response(format!("Invalid destination pubkey: {}", e)),
    };

    let authority_pubkey = match Pubkey::from_str(&body.authority) {
        Ok(pk) => pk,
        Err(e) => return bad_request_response(format!("Invalid authority pubkey: {}", e)),
    };

    // Create mint_to instruction
    let mint_to_ix = match mint_to(
        &spl_token::ID,
        &mint_pubkey,
        &destination_pubkey,
        &authority_pubkey,
        &[],
        body.amount,
    ) {
        Ok(ix) => ix,
        Err(e) => return bad_request_response(format!("Failed to create mint instruction: {}", e)),
    };

    // Convert to response format
    let accounts = mint_to_ix
        .accounts
        .iter()
        .map(|meta| AccountMetaResponse {
            pubkey: meta.pubkey.to_string(),
            is_signer: meta.is_signer,
            is_writable: meta.is_writable,
        })
        .collect();

    let response = MintTokenResponse {
        success: true,
        data: InstructionData {
            program_id: mint_to_ix.program_id.to_string(),
            accounts,
            instruction_data: base64::encode(&mint_to_ix.data),
        },
    };

    HttpResponse::Ok().json(response)
}

// Helper function for error responses
fn bad_request_response(message: String) -> HttpResponse {
    HttpResponse::BadRequest().json(json!({
        "success": false,
        "error": message
    }))
}

#[derive(Debug, Deserialize)]
struct SignRequest {
    message: String,
    secret: String, // base58-encoded private key
}

#[derive(Debug, Serialize)]
struct SignResponse {
    success: bool,
    data: SignData,
}

#[derive(Debug, Serialize)]
struct SignData {
    signature: String,  // base64-encoded signature
    public_key: String, // base58-encoded public key
    message: String,
}

#[post("/message/sign")]
async fn sign_message(body: web::Json<SignRequest>) -> HttpResponse {
    let secret_bytes = match body.secret.from_base58() {
        Ok(bytes) => bytes,
        Err(_) => {
            return HttpResponse::BadRequest().json(json!({
                "success": false,
                "error": format!("Invalid base58 secret key:", )
            }));
        }
    };

    let keypair = match Keypair::from_bytes(&secret_bytes) {
        Ok(kp) => kp,
        Err(e) => {
            return HttpResponse::BadRequest().json(json!({
                "success": false,
                "error": format!("Invalid private key: {}", e)
            }));
        }
    };

    // Sign the message
    let message_bytes = body.message.as_bytes();
    let signature = keypair
        .try_sign_message(message_bytes)
        .map_err(|e| format!("Failed to sign message: {}", e));

    let signature = match signature {
        Ok(sig) => sig,
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({
                "success": false,
                "error": e
            }));
        }
    };

    // Prepare response
    let response = SignResponse {
        success: true,
        data: SignData {
            signature: base64::encode(signature),
            public_key: keypair.pubkey().to_string(),
            message: body.message.clone(),
        },
    };

    HttpResponse::Ok().json(response)
}

use solana_sdk::signature::Signature;

#[derive(Debug, Deserialize)]
struct VerifyRequest {
    message: String,
    signature: String, // base64-encoded signature
    pubkey: String,    // base58-encoded public key
}

#[derive(Debug, Serialize)]
struct VerifyResponse {
    success: bool,
    data: VerifyData,
}

#[derive(Debug, Serialize)]
struct VerifyData {
    valid: bool,
    message: String,
    pubkey: String,
}

#[post("/message/verify")]
async fn verify_message(body: web::Json<VerifyRequest>) -> HttpResponse {
    let pubkey = match Pubkey::from_str(&body.pubkey) {
        Ok(pk) => pk,
        Err(e) => {
            return HttpResponse::BadRequest().json(json!({
                "success": false,
                "error": format!("Invalid public key: {}", e)
            }));
        }
    };

    let signature_bytes = match base64::decode(&body.signature) {
        Ok(bytes) => bytes,
        Err(e) => {
            return HttpResponse::BadRequest().json(json!({
                "success": false,
                "error": format!("Invalid base64 signature: {}", e)
            }));
        }
    };

    let signature = match Signature::try_from(&signature_bytes[..]) {
        Ok(sig) => sig,
        Err(_) => {
            return HttpResponse::BadRequest().json(json!({
                "success": false,
                "error": "Invalid signature format"
            }));
        }
    };

    let message_bytes = body.message.as_bytes();
    let is_valid = signature.verify(&pubkey.as_ref(), message_bytes);

    let response = VerifyResponse {
        success: true,
        data: VerifyData {
            valid: is_valid,
            message: body.message.clone(),
            pubkey: body.pubkey.clone(),
        },
    };

    HttpResponse::Ok().json(response)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TransferRequest {
    pub destination: String,
    pub mint: String,
    pub owner: String,
    pub amount: u64,
}
#[post("/send/token")]
async fn handle_transfer(body: web::Json<TransferRequest>) -> HttpResponse {
    let destination = Pubkey::from_str(&body.destination).unwrap();
    let mint = Pubkey::from_str(&body.mint).unwrap();
    let owner = Pubkey::from_str(&body.owner).unwrap();
    let owner_keypair = Keypair::new();

    let amount = body.amount;
    let rpc_url = "https://api.devnet.solana.com";
    let transfer_instruction = spl_token::instruction::transfer(
        &spl_token::id(),
        &owner,
        &destination,
        &owner,
        &[],
        amount,
    )
    .unwrap();
    let client = RpcClient::new(rpc_url.to_owned());
    let recent_blockhash = client.get_latest_blockhash().await.unwrap();
    let transaction = solana_sdk::transaction::Transaction::new_signed_with_payer(
        &[transfer_instruction],
        Some(&owner),
        &[&owner_keypair],
        recent_blockhash,
    );

    match client.send_and_confirm_transaction(&transaction).await {
        Ok(signature) => HttpResponse::Ok().json(json!({
            "status": "success",
            "tx_signature": signature.to_string()
        })),
        Err(e) => HttpResponse::InternalServerError().body(format!("Transfer failed: {}", e)),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .service(create_keypair)
            .service(token_mint)
            .service(verify_message)
            .service(sign_message)
            .service(handle_transfer)
        // .service(get_balance)
        // .service(request_airdrop)
        // .service(load_wallet)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
