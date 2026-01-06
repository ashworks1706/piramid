//! Piramid HTTP Server
//!
//! Full Rust server like HelixDB - no Python server needed.
//! Dashboard and SDKs connect via HTTP.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

use piramid::{DistanceMetric, Metadata, MetadataValue, VectorEntry, VectorStorage};

// =============================================================================
// STATE
// =============================================================================

/// Shared state across all requests
struct AppState {
    /// Multiple collections, each with its own storage
    collections: RwLock<HashMap<String, VectorStorage>>,
    /// Base path for storage files
    data_dir: String,
}

impl AppState {
    fn new(data_dir: &str) -> Self {
        std::fs::create_dir_all(data_dir).ok();
        Self {
            collections: RwLock::new(HashMap::new()),
            data_dir: data_dir.to_string(),
        }
    }

    fn get_or_create_collection(&self, name: &str) -> Result<(), String> {
        let mut collections = self.collections.write().unwrap();
        if !collections.contains_key(name) {
            let path = format!("{}/{}.db", self.data_dir, name);
            let storage = VectorStorage::open(&path)
                .map_err(|e| format!("Failed to open collection: {}", e))?;
            collections.insert(name.to_string(), storage);
        }
        Ok(())
    }
}

type SharedState = Arc<AppState>;

// =============================================================================
// API TYPES
// =============================================================================

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
}

#[derive(Serialize)]
struct CollectionInfo {
    name: String,
    count: usize,
}

#[derive(Serialize)]
struct CollectionsResponse {
    collections: Vec<CollectionInfo>,
}

#[derive(Deserialize)]
struct CreateCollectionRequest {
    name: String,
}

#[derive(Deserialize)]
struct StoreVectorRequest {
    vector: Vec<f32>,
    text: String,
    #[serde(default)]
    metadata: HashMap<String, serde_json::Value>,
}

#[derive(Serialize)]
struct StoreVectorResponse {
    id: String,
}

#[derive(Serialize)]
struct VectorResponse {
    id: String,
    vector: Vec<f32>,
    text: String,
    metadata: HashMap<String, serde_json::Value>,
}

#[derive(Deserialize)]
struct SearchRequest {
    vector: Vec<f32>,
    #[serde(default = "default_k")]
    k: usize,
    #[serde(default)]
    metric: Option<String>,
}

fn default_k() -> usize { 10 }

#[derive(Serialize)]
struct SearchResultResponse {
    id: String,
    score: f32,
    text: String,
    metadata: HashMap<String, serde_json::Value>,
}

#[derive(Serialize)]
struct SearchResponse {
    results: Vec<SearchResultResponse>,
}

#[derive(Serialize)]
struct DeleteResponse {
    deleted: bool,
}

#[derive(Serialize)]
struct CountResponse {
    count: usize,
}

#[derive(Deserialize)]
struct ListVectorsQuery {
    #[serde(default = "default_limit")]
    limit: usize,
    #[serde(default)]
    offset: usize,
}

fn default_limit() -> usize { 100 }

// =============================================================================
// HELPERS
// =============================================================================

fn json_to_metadata(json: HashMap<String, serde_json::Value>) -> Metadata {
    let mut metadata = Metadata::new();
    for (k, v) in json {
        let value = match v {
            serde_json::Value::String(s) => MetadataValue::String(s),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    MetadataValue::Integer(i)
                } else if let Some(f) = n.as_f64() {
                    MetadataValue::Float(f)
                } else {
                    continue;
                }
            }
            serde_json::Value::Bool(b) => MetadataValue::Boolean(b),
            serde_json::Value::Null => MetadataValue::Null,
            _ => continue,
        };
        metadata.insert(k, value);
    }
    metadata
}

fn metadata_to_json(metadata: &Metadata) -> HashMap<String, serde_json::Value> {
    metadata
        .iter()
        .map(|(k, v)| {
            let json_val = match v {
                MetadataValue::String(s) => serde_json::Value::String(s.clone()),
                MetadataValue::Integer(i) => serde_json::json!(*i),
                MetadataValue::Float(f) => serde_json::json!(*f),
                MetadataValue::Boolean(b) => serde_json::Value::Bool(*b),
                MetadataValue::Null => serde_json::Value::Null,
                MetadataValue::Array(arr) => {
                    serde_json::Value::Array(arr.iter().map(|v| match v {
                        MetadataValue::String(s) => serde_json::Value::String(s.clone()),
                        MetadataValue::Integer(i) => serde_json::json!(*i),
                        MetadataValue::Float(f) => serde_json::json!(*f),
                        MetadataValue::Boolean(b) => serde_json::Value::Bool(*b),
                        _ => serde_json::Value::Null,
                    }).collect())
                }
            };
            (k.clone(), json_val)
        })
        .collect()
}

fn parse_metric(s: Option<String>) -> DistanceMetric {
    match s.as_deref() {
        Some("euclidean") => DistanceMetric::Euclidean,
        Some("dot") | Some("dot_product") => DistanceMetric::DotProduct,
        _ => DistanceMetric::Cosine,
    }
}

// =============================================================================
// HANDLERS
// =============================================================================

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}

async fn list_collections(State(state): State<SharedState>) -> Json<CollectionsResponse> {
    let collections = state.collections.read().unwrap();
    let infos: Vec<CollectionInfo> = collections
        .iter()
        .map(|(name, storage)| CollectionInfo {
            name: name.clone(),
            count: storage.count(),
        })
        .collect();
    Json(CollectionsResponse { collections: infos })
}

async fn create_collection(
    State(state): State<SharedState>,
    Json(req): Json<CreateCollectionRequest>,
) -> Result<Json<CollectionInfo>, (StatusCode, String)> {
    state.get_or_create_collection(&req.name)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    
    let collections = state.collections.read().unwrap();
    let count = collections.get(&req.name).map(|s| s.count()).unwrap_or(0);
    
    Ok(Json(CollectionInfo { name: req.name, count }))
}

async fn get_collection(
    State(state): State<SharedState>,
    Path(name): Path<String>,
) -> Result<Json<CollectionInfo>, (StatusCode, String)> {
    state.get_or_create_collection(&name)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    
    let collections = state.collections.read().unwrap();
    let count = collections.get(&name).map(|s| s.count()).unwrap_or(0);
    
    Ok(Json(CollectionInfo { name, count }))
}

async fn delete_collection(
    State(state): State<SharedState>,
    Path(name): Path<String>,
) -> Result<Json<DeleteResponse>, (StatusCode, String)> {
    let mut collections = state.collections.write().unwrap();
    let existed = collections.remove(&name).is_some();
    
    if existed {
        let path = format!("{}/{}.db", state.data_dir, name);
        std::fs::remove_file(&path).ok();
    }
    
    Ok(Json(DeleteResponse { deleted: existed }))
}

async fn store_vector(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
    Json(req): Json<StoreVectorRequest>,
) -> Result<Json<StoreVectorResponse>, (StatusCode, String)> {
    state.get_or_create_collection(&collection)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    
    let metadata = json_to_metadata(req.metadata);
    let entry = VectorEntry::with_metadata(req.vector, req.text, metadata);
    
    let mut collections = state.collections.write().unwrap();
    let storage = collections.get_mut(&collection)
        .ok_or((StatusCode::NOT_FOUND, "Collection not found".to_string()))?;
    
    let id = storage.store(entry)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    Ok(Json(StoreVectorResponse { id: id.to_string() }))
}

async fn get_vector(
    State(state): State<SharedState>,
    Path((collection, id)): Path<(String, String)>,
) -> Result<Json<VectorResponse>, (StatusCode, String)> {
    state.get_or_create_collection(&collection)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid UUID".to_string()))?;
    
    let collections = state.collections.read().unwrap();
    let storage = collections.get(&collection)
        .ok_or((StatusCode::NOT_FOUND, "Collection not found".to_string()))?;
    
    let entry = storage.get(&uuid)
        .ok_or((StatusCode::NOT_FOUND, "Vector not found".to_string()))?;
    
    Ok(Json(VectorResponse {
        id: entry.id.to_string(),
        vector: entry.vector,
        text: entry.text,
        metadata: metadata_to_json(&entry.metadata),
    }))
}

async fn list_vectors(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
    Query(params): Query<ListVectorsQuery>,
) -> Result<Json<Vec<VectorResponse>>, (StatusCode, String)> {
    state.get_or_create_collection(&collection)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    
    let collections = state.collections.read().unwrap();
    let storage = collections.get(&collection)
        .ok_or((StatusCode::NOT_FOUND, "Collection not found".to_string()))?;
    
    let vectors: Vec<VectorResponse> = storage.get_all()
        .into_iter()
        .skip(params.offset)
        .take(params.limit)
        .map(|e| VectorResponse {
            id: e.id.to_string(),
            vector: e.vector.clone(),
            text: e.text.clone(),
            metadata: metadata_to_json(&e.metadata),
        })
        .collect();
    
    Ok(Json(vectors))
}

async fn delete_vector(
    State(state): State<SharedState>,
    Path((collection, id)): Path<(String, String)>,
) -> Result<Json<DeleteResponse>, (StatusCode, String)> {
    state.get_or_create_collection(&collection)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid UUID".to_string()))?;
    
    let mut collections = state.collections.write().unwrap();
    let storage = collections.get_mut(&collection)
        .ok_or((StatusCode::NOT_FOUND, "Collection not found".to_string()))?;
    
    let deleted = storage.delete(&uuid)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    Ok(Json(DeleteResponse { deleted }))
}

async fn search_vectors(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
    Json(req): Json<SearchRequest>,
) -> Result<Json<SearchResponse>, (StatusCode, String)> {
    state.get_or_create_collection(&collection)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    
    let metric = parse_metric(req.metric);
    
    let collections = state.collections.read().unwrap();
    let storage = collections.get(&collection)
        .ok_or((StatusCode::NOT_FOUND, "Collection not found".to_string()))?;
    
    let results: Vec<SearchResultResponse> = storage
        .search(&req.vector, req.k, metric)
        .into_iter()
        .map(|r| SearchResultResponse {
            id: r.id.to_string(),
            score: r.score,
            text: r.text,
            metadata: metadata_to_json(&r.metadata),
        })
        .collect();
    
    Ok(Json(SearchResponse { results }))
}

async fn collection_count(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
) -> Result<Json<CountResponse>, (StatusCode, String)> {
    state.get_or_create_collection(&collection)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    
    let collections = state.collections.read().unwrap();
    let count = collections.get(&collection).map(|s| s.count()).unwrap_or(0);
    
    Ok(Json(CountResponse { count }))
}

// =============================================================================
// MAIN
// =============================================================================

#[tokio::main]
async fn main() {
    let port = std::env::var("PORT").unwrap_or_else(|_| "6333".to_string());
    let data_dir = std::env::var("DATA_DIR").unwrap_or_else(|_| "./piramid_data".to_string());
    
    let state = Arc::new(AppState::new(&data_dir));
    
    // CORS for dashboard
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
    
    let app = Router::new()
        // Health
        .route("/api/health", get(health))
        
        // Collections
        .route("/api/collections", get(list_collections))
        .route("/api/collections", post(create_collection))
        .route("/api/collections/{name}", get(get_collection))
        .route("/api/collections/{name}", delete(delete_collection))
        .route("/api/collections/{name}/count", get(collection_count))
        
        // Vectors
        .route("/api/collections/{collection}/vectors", get(list_vectors))
        .route("/api/collections/{collection}/vectors", post(store_vector))
        .route("/api/collections/{collection}/vectors/{id}", get(get_vector))
        .route("/api/collections/{collection}/vectors/{id}", delete(delete_vector))
        
        // Search
        .route("/api/collections/{collection}/search", post(search_vectors))
        
        .layer(cors)
        .with_state(state);
    
    let addr = format!("0.0.0.0:{}", port);
    println!("🔺 Piramid server running on http://{}", addr);
    println!("   Data directory: {}", data_dir);
    
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
