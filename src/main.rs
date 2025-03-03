use std::{env::args, fs};

use actix_web::{get, middleware::Logger, web::{self, PayloadConfig}, App, HttpResponse, HttpServer, Responder};
use opendal::{services, Operator};
use routes::{management, pull, push};
mod routes;
mod storage;
mod appconfig;

#[actix_web::main]
async fn main() -> std::io::Result<()> {

    let mut config_path = String::from(".");

    let mut args: Vec<String> = args().collect();

    if args.len() > 1 {
        config_path = args.remove(1);
    }
    
    let config_fs = fs::read_to_string(config_path)?;

    let app_cfg: appconfig::AppConfig = serde_yaml::from_str(config_fs.as_str()).unwrap();
    
    let cache_builder = services::Fs::default().root(&app_cfg.storage.get_local());
    let cache_op = Operator::new(cache_builder).unwrap().finish();
    
    let primary_storage =  match app_cfg.storage.create_s3_op() {
    Ok(s3cfg) => {
        Operator::new(s3cfg).unwrap().finish()        
    },
    Err(_) => {
        cache_op.clone()
    },
    };

    let store = storage::new(primary_storage,cache_op);
   

    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let app_data = web::Data::new(store);
    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(app_data.clone())
            .app_data(PayloadConfig::new(1073741824))
            .service(get_status)
            .service(
                web::scope("/v2").service(get_status)
                .configure(push::config)
                .configure(pull::config)
                .configure(management::config)
            )
    })
    .bind((app_cfg.server.address,8080))?
    .run()
    .await
    
}

#[get("/")]
async fn get_status() -> impl Responder {
    HttpResponse::Ok().body("true")
}