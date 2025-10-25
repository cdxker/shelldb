use actix_web::{App, HttpResponse, HttpServer, Result, web};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::process::Command;
use utoipa::{OpenApi, ToSchema};

#[derive(Deserialize, ToSchema)]
pub struct GrepSearchPayload {
    /// The UUID of the dataset to search
    pub dataset: uuid::Uuid,
    /// Grep flags to use (e.g., "-r -n pattern")
    pub flags: String,
    /// Optional folders to filter the search (e.g., ["___private", "___user_auth"])
    pub folder_filter: Vec<String>,
}

#[utoipa::path(
    post,
    path = "/api/search",
    request_body = GrepSearchPayload,
    responses(
        (status = 200, description = "Search results returned successfully", body = String),
        (status = 500, description = "Internal server error")
    ),
    tag = "Search"
)]
async fn search(
    payload: web::Json<GrepSearchPayload>,
    config: web::Data<Config>,
) -> Result<HttpResponse> {
    let dataset = payload.dataset.to_string();
    let base_path = format!("{}/{}", config.data_volume_dir, dataset);

    // Parse flags (everything except "grep" itself)
    let flags_vec: Vec<String> = payload
        .flags
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();

    // Build search paths based on folder_filter
    let mut search_paths = vec![base_path.clone()];

    for folder in &payload.folder_filter {
        // User passes the folder name directly (e.g., "___private", "___user_auth")
        search_paths.push(format!("{}/{}", base_path, folder));
    }

    // Execute grep asynchronously using web::block
    let output = web::block(move || {
        let mut cmd = Command::new("grep");

        // Add all flags
        for flag in flags_vec {
            cmd.arg(flag);
        }

        // Add all search paths
        for path in search_paths {
            cmd.arg(path);
        }

        cmd.output()
    })
    .await
    .unwrap()
    .unwrap();

    // Convert output to string
    let mut result = String::from_utf8_lossy(&output.stdout).to_string();

    // Strip the top-level directory from paths with "___" prefix
    result = result
        .lines()
        .map(|line| {
            if line.contains("/___") {
                // Find the dataset UUID and strip everything before it
                if let Some(pos) = line.find(&dataset) {
                    &line[pos..]
                } else {
                    line
                }
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    Ok(HttpResponse::Ok().body(result))
}

#[derive(Deserialize, ToSchema)]
pub struct GrepIndexPayload {
    /// The UUID of the dataset to index into
    pub dataset: uuid::Uuid,
    /// The filename to store
    pub filename: String,
    /// The content of the file
    pub file_payload: String,
    /// Tags for nested storage (e.g., ["private", "user_auth"])
    pub nested: Vec<String>,
}

#[utoipa::path(
    post,
    path = "/api/index",
    request_body = GrepIndexPayload,
    responses(
        (status = 200, description = "File indexed successfully", body = String),
        (status = 500, description = "Internal server error")
    ),
    tag = "Index"
)]
async fn index(
    payload: web::Json<GrepIndexPayload>,
    config: web::Data<Config>,
) -> Result<HttpResponse> {
    let dataset = payload.dataset.to_string();
    let base_path = format!("{}/{}", config.data_volume_dir, dataset);

    // Create the base dataset directory
    fs::create_dir_all(&base_path).expect("Failed to create dataset directory");

    // Determine storage paths based on nested tags
    let mut storage_paths = Vec::new();
    let has_private = payload.nested.contains(&"private".to_string());
    let has_user_auth = payload.nested.contains(&"user_auth".to_string());

    if has_private {
        let private_path = format!("{}/___private", base_path);
        fs::create_dir_all(&private_path).expect("Failed to create private directory");
        storage_paths.push(format!("{}/{}", private_path, payload.filename));
    }

    if has_user_auth {
        let user_auth_path = format!("{}/___user_auth", base_path);
        fs::create_dir_all(&user_auth_path).expect("Failed to create user_auth directory");
        storage_paths.push(format!("{}/{}", user_auth_path, payload.filename));
    }

    // If no special tags, store in base directory
    if storage_paths.is_empty() {
        storage_paths.push(format!("{}/{}", base_path, payload.filename));
    }

    // Write the file content to all determined paths
    for path in storage_paths {
        // Create parent directory if it doesn't exist
        if let Some(parent) = Path::new(&path).parent() {
            fs::create_dir_all(parent).expect("Failed to create parent directory");
        }

        fs::write(&path, &payload.file_payload)
            .unwrap_or_else(|e| panic!("Failed to write file to {}: {}", path, e));

        println!("Indexed file to: {}", path);
    }

    Ok(HttpResponse::Ok().body("File indexed successfully"))
}

#[derive(Deserialize, Serialize, Clone, ToSchema)]
pub struct Config {
    /// The directory where datasets are stored
    pub data_volume_dir: String,
}

#[derive(OpenApi)]
#[openapi(
    paths(search, index),
    components(schemas(GrepSearchPayload, GrepIndexPayload)),
    tags(
        (name = "Search", description = "Search operations on indexed data"),
        (name = "Index", description = "Index new files into datasets")
    ),
    info(
        title = "GrepDB API",
        version = "0.1.0",
        description = "A database for grepping - search indexed content with grep-like functionality",
        license(name = "MIT")
    )
)]
pub struct ApiDoc;

#[derive(Parser)]
#[command(name = "grepdb")]
#[command(about = "A database for grepping", long_about = None)]
struct Cli {
    /// Config file path
    #[arg(short, long, default_value = "./config.yaml")]
    config: String,
}

#[actix_web::main]
pub async fn start_server() -> std::io::Result<()> {
    // Parse command line arguments
    let cli = Cli::parse();

    // Load config from YAML file
    let config_path = &cli.config;
    let config_file = std::fs::read_to_string(config_path).unwrap_or_else(|_| {
        // If config file doesn't exist, create default config
        let default_config = Config {
            data_volume_dir: "data_volume_dir".to_string(),
        };
        let yaml = serde_yaml::to_string(&default_config).unwrap();
        std::fs::write(config_path, &yaml).unwrap();
        yaml
    });

    let config: Config = serde_yaml::from_str(&config_file).expect("Failed to parse config file");

    println!(
        "Starting GrepDB with data volume: {}",
        config.data_volume_dir
    );

    // Ensure data volume directory exists
    fs::create_dir_all(&config.data_volume_dir).expect("Failed to create data volume directory");

    let config_data = web::Data::new(config);

    let openapi_json = ApiDoc::openapi()
        .to_pretty_json()
        .expect("Failed to generate openapi json");

    fs::write("docs/openapi.yaml", openapi_json).expect("Failed to write openapi_json");

    HttpServer::new(move || {
        App::new().app_data(config_data.clone()).service(
            web::scope("/api")
                .route("/search", web::post().to(search))
                .route("/index", web::post().to(index)),
        )
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
