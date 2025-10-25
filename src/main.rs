use actix_web::{App, HttpServer, web, HttpResponse, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::process::Command;
use clap::Parser;

#[derive(Deserialize)]
pub struct GrepSearchPayload {
    pub dataset: uuid::Uuid,
    pub flags: String,
    pub folder_filter: Vec<String>,
}

async fn search(
    payload: web::Json<GrepSearchPayload>,
    config: web::Data<Config>,
) -> Result<HttpResponse> {
    let dataset = payload.dataset.to_string();
    let base_path = format!("{}/{}", config.data_volume_dir, dataset);

    // Parse flags (everything except "grep" itself)
    let flags_vec: Vec<String> = payload.flags
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

#[derive(Deserialize)]
pub struct GrepIndexPayload {
    pub dataset: uuid::Uuid,
    pub filename: String,
    pub file_payload: String,
    pub nested: Vec<String>,
}

async fn index(
    payload: web::Json<GrepIndexPayload>,
    config: web::Data<Config>,
) -> Result<HttpResponse> {
    let dataset = payload.dataset.to_string();
    let base_path = format!("{}/{}", config.data_volume_dir, dataset);

    // Create the base dataset directory
    fs::create_dir_all(&base_path)
        .expect("Failed to create dataset directory");

    // Determine storage paths based on nested tags
    let mut storage_paths = Vec::new();
    let has_private = payload.nested.contains(&"private".to_string());
    let has_user_auth = payload.nested.contains(&"user_auth".to_string());

    if has_private {
        let private_path = format!("{}/___private", base_path);
        fs::create_dir_all(&private_path)
            .expect("Failed to create private directory");
        storage_paths.push(format!("{}/{}", private_path, payload.filename));
    }

    if has_user_auth {
        let user_auth_path = format!("{}/___user_auth", base_path);
        fs::create_dir_all(&user_auth_path)
            .expect("Failed to create user_auth directory");
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
            fs::create_dir_all(parent)
                .expect("Failed to create parent directory");
        }

        fs::write(&path, &payload.file_payload)
            .unwrap_or_else(|e| panic!("Failed to write file to {}: {}", path, e));

        println!("Indexed file to: {}", path);
    }

    Ok(HttpResponse::Ok().body("File indexed successfully"))
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Config {
    pub data_volume_dir: String,
}

#[derive(Parser)]
#[command(name = "grepdb")]
#[command(about = "A database for grepping", long_about = None)]
struct Cli {
    /// Config file path
    #[arg(short, long, default_value = "./config.yaml")]
    config: String,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Parse command line arguments
    let cli = Cli::parse();

    // Load config from YAML file
    let config_path = &cli.config;
    let config_file = std::fs::read_to_string(config_path)
        .unwrap_or_else(|_| {
            // If config file doesn't exist, create default config
            let default_config = Config {
                data_volume_dir: "data_volume_dir".to_string(),
            };
            let yaml = serde_yaml::to_string(&default_config).unwrap();
            std::fs::write(config_path, &yaml).unwrap();
            yaml
        });

    let config: Config = serde_yaml::from_str(&config_file)
        .expect("Failed to parse config file");

    println!("Starting GrepDB with data volume: {}", config.data_volume_dir);

    // Ensure data volume directory exists
    fs::create_dir_all(&config.data_volume_dir)
        .expect("Failed to create data volume directory");

    let config_data = web::Data::new(config);

    // TODO add routes in here to add /api/index and /api/search
    HttpServer::new(move || {
        App::new()
            .app_data(config_data.clone())
            .service(web::scope("/api")
                .route("/search", web::post().to(search))
                .route("/index", web::post().to(index))
            )
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
