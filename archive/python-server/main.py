"""
Piramid Server - REST API for the vector database

A clean, modular FastAPI server for vector similarity search.

Architecture:
- routes/     - API endpoints (health, collections, vectors, search)
- storage.py  - Vector storage engine with numpy
- models.py   - Pydantic request/response models

Run with: python main.py
    Or: uvicorn main:app --reload --port 6333
"""

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware
from fastapi.staticfiles import StaticFiles
from contextlib import asynccontextmanager
import os
from pathlib import Path

from storage import StorageManager
from routes import (
    health_router,
    collections_router,
    vectors_router,
    search_router,
    init_routes,
)

# =============================================================================
# APP SETUP
# =============================================================================

@asynccontextmanager
async def lifespan(app: FastAPI):
    """
    Startup and shutdown lifecycle.
    
    - Startup: Load existing collections from disk
    - Shutdown: Save all collections to disk
    """
    # Load data directory from env or use default
    data_dir = os.environ.get("PIRAMID_DATA_DIR", "./data")
    
    # Create storage manager and load existing data
    storage_manager = StorageManager(data_dir)
    storage_manager.load_all()
    
    # Inject storage manager into all routes
    init_routes(storage_manager)
    
    print(f"🔺 Piramid server starting...")
    print(f"   Data dir: {data_dir}")
    print(f"   Collections: {len(storage_manager.collections)}")
    
    yield  # Server is running
    
    # Shutdown: persist everything
    storage_manager.save_all()
    print("👋 Piramid server shutting down")


# Create FastAPI app
app = FastAPI(
    title="Piramid",
    description="Vector database for AI applications",
    version="0.1.0",
    lifespan=lifespan,
)

# CORS middleware - allow dashboard to call API
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)


# =============================================================================
# REGISTER ROUTES
# =============================================================================

# All API routes are prefixed with /api
app.include_router(health_router, prefix="/api")
app.include_router(collections_router, prefix="/api")
app.include_router(vectors_router, prefix="/api")
app.include_router(search_router, prefix="/api")


# =============================================================================
# STATIC FILES (Dashboard)
# =============================================================================

# Mount the dashboard UI if it exists (built from Next.js)
dashboard_path = os.environ.get("PIRAMID_DASHBOARD_PATH", "../dashboard/out")
if Path(dashboard_path).exists():
    app.mount("/", StaticFiles(directory=dashboard_path, html=True), name="dashboard")


# =============================================================================
# MAIN ENTRY POINT
# =============================================================================

if __name__ == "__main__":
    import uvicorn
    
    host = os.environ.get("PIRAMID_HOST", "0.0.0.0")
    port = int(os.environ.get("PIRAMID_PORT", "6333"))
    
    print(f"🔺 Starting Piramid on http://{host}:{port}")
    print(f"   API:       http://{host}:{port}/api")
    print(f"   Dashboard: http://{host}:{port}/")
    
    uvicorn.run(app, host=host, port=port)
