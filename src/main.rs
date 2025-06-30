use actix_cors::Cors;
use actix_web::{App, HttpResponse, HttpServer, get, http, post, web};
use serde::{Deserialize, Serialize};
use serde_json::json;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey};
use std::str::FromStr;

#[get("/balance/{pubkey}")]
async fn get_balance(pubkey: web::Path<String>) -> HttpResponse {
    let rpc_url = "https://api.devnet.solana.com".to_owned();
    let client = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

    match client.get_balance(&pubkey.parse().unwrap()).await {
        Ok(balance) => HttpResponse::Ok().json(serde_json::json!({
            "pubkey": &*pubkey,
            "balance": balance,
            "sol": balance as f64 / 1_000_000_000.0
        })),
        Err(e) => HttpResponse::BadRequest().json(serde_json::json!({
            "error": e.to_string()
        })),
    }
}

#[get("/airdrop/{pubkey}")]
async fn request_airdrop(pubkey: web::Path<String>) -> HttpResponse {
    let rpc_url = "https://api.devnet.solana.com".to_owned();
    let client = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());
    let lamports = 1_000_000_000;
    let pubkey = match Pubkey::from_str(&pubkey) {
        Ok(p) => p,
        Err(e) => return HttpResponse::BadRequest().json(json!({"error":e.to_string()})),
    };
    match client.request_airdrop(&pubkey, lamports).await {
        Ok(signature) => HttpResponse::Ok().json(json!({
             "status": "success",
            "pubkey": pubkey.to_string(),
            "airdrop_amount": lamports as f64 / 1_000_000_000.0,
            "tx_signature": signature.to_string()
        })),
        Err(e) => HttpResponse::BadRequest().json(json!({
            "error": e.to_string()
        })),
    }
}

#[derive(Serialize, Deserialize)]
struct AirdropRequest {
    pubkey: String,
    amount: f64,
}
#[post("/user/airdrop")]
async fn load_wallet(body: web::Json<AirdropRequest>) -> HttpResponse {
    let rpc_url = "https://api.devnet.solana.com";
    let client = RpcClient::new_with_commitment(rpc_url.to_owned(), CommitmentConfig::confirmed());

    let lamports = (body.amount * 1_000_000_000.0).round() as u64;

    let pubkey = match Pubkey::from_str(&body.pubkey) {
        Ok(p) => p,
        Err(_) => {
            return HttpResponse::BadRequest().json(json!({
                "error": format!("Invalid pub key:{}",&body.pubkey)
            }));
        }
    };
    match client.request_airdrop(&pubkey, lamports).await {
        Ok(signature) => HttpResponse::Ok().json(json!({
              "status": "success",
            "pubkey": pubkey.to_string(),
            "amount_sol": body.amount,
            "amount_lamports": lamports,
            "tx_signature": signature.to_string()
        })),
        Err(e) => HttpResponse::BadRequest().json(json!({
            "error": e.to_string(),
            "note": "Devnet airdrops are limited to 1 SOL per request and 5 SOL per day"
        })),
    }
}
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        // Configure CORS based on environment
        let cors = {
            Cors::default()
                .allowed_origin("http://localhost:5173")
                .allowed_origin("http://127.0.0.1:5173")
                .allowed_methods(vec!["GET", "POST", "OPTIONS"])
                .allowed_headers(vec![http::header::CONTENT_TYPE])
                .max_age(3600)
        };

        App::new()
            .wrap(cors)
            .service(get_balance)
            .service(request_airdrop)
            .service(load_wallet)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
