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
    signature::read_keypair_file,
    signer::{Signer, keypair::Keypair},
    transaction::Transaction,
};
use spl_token::{ID, instruction::initialize_mint, state::Mint};
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
    // client
    //     .request_airdrop(&payer.pubkey(), 1_000_000_000)
    //     .await
    //     .unwrap();

    // let payer = get_funded_payer(rent_exempt_amount).await.unwrap();

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
// async fn get_funded_payer(min_balance: u64) -> Result<Keypair, HttpResponse> {
//     let payer = Keypair::new();
//     let rpc_url = "https://api.devnet.solana.com ";
//     let client = RpcClient::new(rpc_url.to_owned());

//     if client.get_balance(&payer.pubkey()).await.unwrap() < min_balance {
//         client
//             .request_airdrop(&payer.pubkey(), 1_000_000_000)
//             .await
//             .unwrap();
//     }
//     Ok(payer)
// }
