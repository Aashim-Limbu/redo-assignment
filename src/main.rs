use actix_web::{App, HttpServer};

use crate::controllers::token::{create_keypair, create_token, mint_token};
mod controllers;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .service(create_keypair)
            .service(create_token)
            .service(mint_token)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
