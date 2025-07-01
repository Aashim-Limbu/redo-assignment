use actix_web::{App, HttpServer};

use crate::controllers::token::{create_keypair, create_token, mint_token, send_sol, sign_message, token_transfer, verify_signature};
mod controllers;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .service(create_keypair)
            .service(create_token)
            .service(mint_token)
            .service(sign_message)
            .service(verify_signature)
            .service(send_sol)
            .service(token_transfer)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
