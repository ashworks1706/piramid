"""
Collection Routes

Endpoints for managing vector collections.
Collections are like tables in a database - they group related vectors together.
"""

from fastapi import APIRouter, HTTPException

from models import CollectionInfo, CollectionsResponse, CreateCollectionRequest

router = APIRouter(prefix="/collections", tags=["collections"])

# Storage manager is injected via dependency
_storage_manager = None

def set_storage_manager(sm):
    """Inject the storage manager (called from main.py)"""
    global _storage_manager
    _storage_manager = sm


@router.get("", response_model=CollectionsResponse)
async def list_collections():
    """
    List all collections in the database.
    
    Returns collection names along with stats like vector count
    and dimension (if vectors have been inserted).
    """
    infos = []
    for name, storage in _storage_manager.collections.items():
        infos.append(CollectionInfo(
            name=name,
            vector_count=len(storage.vectors),
            dimension=storage.dimension,
        ))
    return CollectionsResponse(collections=infos)


@router.post("", response_model=CollectionInfo, status_code=201)
async def create_collection(req: CreateCollectionRequest):
    """
    Create a new collection.
    
    Collections start empty. The dimension is set automatically
    when you insert the first vector.
    """
    # Validate name (no path separators to prevent directory traversal)
    if not req.name or "/" in req.name or "\\" in req.name:
        raise HTTPException(400, "Invalid collection name")
    
    # Check if already exists
    if req.name in _storage_manager.collections:
        raise HTTPException(409, f"Collection '{req.name}' already exists")
    
    # Create it
    _storage_manager.create_collection(req.name)
    
    return CollectionInfo(
        name=req.name,
        vector_count=0,
        dimension=req.dimension,
    )


@router.get("/{name}", response_model=CollectionInfo)
async def get_collection_info(name: str):
    """
    Get detailed info about a specific collection.
    
    Includes vector count and dimension.
    """
    storage = _storage_manager.get_collection(name)
    if not storage:
        raise HTTPException(404, f"Collection '{name}' not found")
    
    return CollectionInfo(
        name=name,
        vector_count=len(storage.vectors),
        dimension=storage.dimension,
    )


@router.delete("/{name}", status_code=204)
async def delete_collection(name: str):
    """
    Delete a collection and all its vectors.
    
    This is permanent and cannot be undone!
    """
    if not _storage_manager.delete_collection(name):
        raise HTTPException(404, f"Collection '{name}' not found")
