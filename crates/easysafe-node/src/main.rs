use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{Path as AxumPath, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, put};
use axum::{Json, Router};
use clap::Parser;
use easysafe_core::{HeadRef, HealthResponse, ObjectStore, WorkspaceConfig, derive_object_id};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::info;

#[derive(Debug, Parser)]
struct Args {
    #[arg(long, default_value = "127.0.0.1:8080")]
    listen: String,
    #[arg(long, default_value = ".easysafe-node")]
    data_dir: PathBuf,
}

#[derive(Clone)]
struct AppState {
    objects: ObjectStore,
    data_dir: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "easysafe_node=info,tower_http=info".to_string()),
        )
        .init();

    let args = Args::parse();
    fs::create_dir_all(&args.data_dir)?;
    let state = Arc::new(AppState {
        objects: ObjectStore::new(args.data_dir.join("objects")),
        data_dir: args.data_dir.clone(),
    });
    state.objects.init()?;

    let app = Router::new()
        .route("/api/health", get(health))
        .route("/api/objects/:object_id", put(put_object).get(get_object))
        .route(
            "/api/workspaces/:workspace_id/config",
            put(put_config).get(get_config),
        )
        .route(
            "/api/workspaces/:workspace_id/head",
            put(put_head).get(get_head),
        )
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr: SocketAddr = args.listen.parse()?;
    info!("listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
    })
}

async fn put_object(
    State(state): State<Arc<AppState>>,
    AxumPath(object_id): AxumPath<String>,
    bytes: Bytes,
) -> Result<StatusCode, ApiError> {
    let actual = derive_object_id(&bytes);
    if actual != object_id {
        return Err(ApiError::bad_request(format!(
            "object id mismatch; expected {object_id}, got {actual}"
        )));
    }
    state.objects.put(&object_id, &bytes)?;
    Ok(StatusCode::CREATED)
}

async fn get_object(
    State(state): State<Arc<AppState>>,
    AxumPath(object_id): AxumPath<String>,
) -> Result<(StatusCode, Bytes), ApiError> {
    let bytes = state.objects.get(&object_id).map_err(ApiError::from)?;
    Ok((StatusCode::OK, Bytes::from(bytes)))
}

async fn put_config(
    State(state): State<Arc<AppState>>,
    AxumPath(workspace_id): AxumPath<String>,
    Json(config): Json<WorkspaceConfig>,
) -> Result<StatusCode, ApiError> {
    if config.workspace_id != workspace_id {
        return Err(ApiError::bad_request(
            "workspace id in path did not match config payload".to_string(),
        ));
    }
    write_workspace_file(&state.data_dir, &workspace_id, "config.json", &config)?;
    Ok(StatusCode::CREATED)
}

async fn get_config(
    State(state): State<Arc<AppState>>,
    AxumPath(workspace_id): AxumPath<String>,
) -> Result<Json<WorkspaceConfig>, ApiError> {
    let config = read_workspace_file(&state.data_dir, &workspace_id, "config.json")?;
    Ok(Json(config))
}

async fn put_head(
    State(state): State<Arc<AppState>>,
    AxumPath(workspace_id): AxumPath<String>,
    Json(head_ref): Json<HeadRef>,
) -> Result<StatusCode, ApiError> {
    if head_ref.workspace_id != workspace_id {
        return Err(ApiError::bad_request(
            "workspace id in path did not match head payload".to_string(),
        ));
    }
    write_workspace_file(&state.data_dir, &workspace_id, "head.json", &head_ref)?;
    Ok(StatusCode::CREATED)
}

async fn get_head(
    State(state): State<Arc<AppState>>,
    AxumPath(workspace_id): AxumPath<String>,
) -> Result<Json<HeadRef>, ApiError> {
    let head_ref = read_workspace_file(&state.data_dir, &workspace_id, "head.json")?;
    Ok(Json(head_ref))
}

fn workspace_dir(root: &Path, workspace_id: &str) -> PathBuf {
    root.join("workspaces").join(workspace_id)
}

fn write_workspace_file<T: serde::Serialize>(
    root: &Path,
    workspace_id: &str,
    name: &str,
    value: &T,
) -> Result<(), ApiError> {
    let dir = workspace_dir(root, workspace_id);
    fs::create_dir_all(&dir)?;
    fs::write(dir.join(name), serde_json::to_vec_pretty(value)?)?;
    Ok(())
}

fn read_workspace_file<T: serde::de::DeserializeOwned>(
    root: &Path,
    workspace_id: &str,
    name: &str,
) -> Result<T, ApiError> {
    let path = workspace_dir(root, workspace_id).join(name);
    if !path.exists() {
        return Err(ApiError::not_found(format!(
            "{name} for workspace {workspace_id} was not found"
        )));
    }
    let bytes = fs::read(path)?;
    Ok(serde_json::from_slice(&bytes)?)
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn bad_request(message: String) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message,
        }
    }

    fn not_found(message: String) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message,
        }
    }
}

impl From<std::io::Error> for ApiError {
    fn from(value: std::io::Error) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: value.to_string(),
        }
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(value: serde_json::Error) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: value.to_string(),
        }
    }
}

impl From<easysafe_core::EasySafeError> for ApiError {
    fn from(value: easysafe_core::EasySafeError) -> Self {
        match value {
            easysafe_core::EasySafeError::MissingObject(_) => Self {
                status: StatusCode::NOT_FOUND,
                message: value.to_string(),
            },
            _ => Self {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: value.to_string(),
            },
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        (self.status, self.message).into_response()
    }
}
