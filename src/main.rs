use actix_multipart::Multipart;
use actix_web::{post, web, App, HttpRequest, HttpResponse, HttpServer};
use dotenv::dotenv;
use futures::{StreamExt, TryStreamExt};
use lazy_static::lazy_static;
use std::{
    env,
    fs::{self, File},
    io::Write,
    path::Path,
};

lazy_static! {
    static ref SECRET: String = {
        dotenv().ok();
        env::var("CDN_SECRET").expect("No secret provided.")
    };
}

#[post("/files/shiba")]
async fn send_shiba(mut payload: Multipart, request: HttpRequest) -> HttpResponse {
    let header = request.headers().get("Authorization");
    if header.is_none() || header.unwrap().to_str().ok() != Some(&SECRET) {
        return HttpResponse::Unauthorized().finish();
    }

    if let Ok(Some(mut field)) = payload.try_next().await {
        if let Some(content) = field.content_disposition() {
            if let Some(name) = content.get_filename() {
                let path = Path::new("./files/shiba").join(sanitize_filename::sanitize(name));
                if path.exists() {
                    return HttpResponse::Conflict().finish();
                }
                if let Ok(mut f) = File::create(path) {
                    while let Some(chunk) = field.next().await {
                        match web::block(move || f.write_all(chunk.as_ref().unwrap()).map(|_| f))
                            .await
                        {
                            Ok(file) => f = file,
                            Err(why) => {
                                eprintln!("Error writing file {}: {:?}", name, why);
                                return HttpResponse::InternalServerError().finish();
                            }
                        }
                    }
                    return HttpResponse::Created().finish();
                };
            }
        }
    }
    HttpResponse::BadRequest().finish()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    fs::create_dir_all("./files/shiba").ok();
    HttpServer::new(|| {
        App::new()
            .service(send_shiba)
            .service(actix_files::Files::new("/", "./files"))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
