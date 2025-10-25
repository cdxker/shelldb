use actix_web::{App, HttpServer, web};

pub struct GrepSearchPayload {
    pub dataset: uuid::Uuid,
    pub flags: String,
    pub folder_filter: Vec<String>,
}

async fn search(
    payload: web::JsonBody<GrepSearchPayload>
) {
    // Run grep against top level directory ${Config.data_volume_dir}/dataset/*

    // if a folder_filter is included, this will include other directories such as `private` / `user_auth`
    //
    // If the file contains 3 underscores "___", the top level directory will need to be stripped
    // from the path becuase these directorie were added to the path by the index


    // The response of this will simply just be 
}

pub struct GrepIndexPayload {
    pub dataset: uuid::Uuid,
    pub filename: String,
    pub file_payload: String,
    pub nested: Vec<String>,
}

async fn index(
    payload: web::JsonBody<GrepIndexPayload>
) {
    // store_file in ./${Config.data_volume_dir}/dataset/${file_payload}/file
    //
    // This will store the file in mount_volume
    //
    // If there are any tags for `private`. that will need to be stored in
    // ./${Config.data_volume_dir}/dataset/___private/${file_payload}/file
    //
    // or for user_auth
    //
    // ./${Config.data_volume_dir}/dataset/___private/${file_payload}/file
    // and 
    // ./${Config.data_volume_dir}/dataset/___user_auth/${file_payload}/file

}

pub struct Config {
    pub data_volume_dir: String,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {

    // Parse args, pass Config through middleware


    // TODO add routes in here to add /api/index and /api/search
    HttpServer::new(|| App::new().service(web::scope("/api")))
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}
