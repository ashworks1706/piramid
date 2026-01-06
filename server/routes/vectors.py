"""
Vector Routes

Endpoints for CRUD operations on individual vectors.
Vectors are the core data unit - each one has:
- id: Unique identifier (auto-generated UUID)
- vector: The numerical embedding (list of floats)
- text: Optional original text that was embedded
- metadata: Optional key-value pairs for filtering
"""

from fastapi import APIRouter, HTTPException

from models import (
    InsertVectorRequest, InsertVectorResponse,
    VectorResponse, UpdateVectorRequest
)

router = APIRouter(prefix="/collections/{name}/vectors", tags=["vectors"])

# Storage manager is injected via dependency
_storage_manager = None

def set_storage_manager(sm):
    """Inject the storage manager (called from main.py)"""
    global _storage_manager
    _storage_manager = sm


@router.post("", response_model=InsertVectorResponse, status_code=201)
async def insert_vector(name: str, req: InsertVectorRequest):
    """
    Insert a new vector into a collection.
    
    The vector dimension is validated against existing vectors.
    If this is the first vector, it sets the collection's dimension.
    
    Returns the auto-generated vector ID.
    """
    storage = _storage_manager.get_collection(name)
    if not storage:
        raise HTTPException(404, f"Collection '{name}' not found")
    
    # Insert and get the auto-generated ID
    vector_id = storage.insert(
        vector=req.vector,
        text=req.text,
        metadata=req.metadata or {},
    )
    
    return InsertVectorResponse(id=vector_id, status="inserted")


@router.get("/{vector_id}", response_model=VectorResponse)
async def get_vector(name: str, vector_id: str):
    """
    Retrieve a specific vector by ID.
    
    Returns the full vector data including embedding, text, and metadata.
    """
    storage = _storage_manager.get_collection(name)
    if not storage:
        raise HTTPException(404, f"Collection '{name}' not found")
    
    entry = storage.get(vector_id)
    if not entry:
        raise HTTPException(404, f"Vector '{vector_id}' not found")
    
    return VectorResponse(
        id=entry["id"],
        vector=entry["vector"],
        text=entry.get("text"),
        metadata=entry.get("metadata", {}),
    )


@router.delete("/{vector_id}", status_code=204)
async def delete_vector(name: str, vector_id: str):
    """
    Delete a vector from a collection.
    
    This is permanent. The vector ID cannot be reused.
    """
    storage = _storage_manager.get_collection(name)
    if not storage:
        raise HTTPException(404, f"Collection '{name}' not found")
    
    if not storage.delete(vector_id):
        raise HTTPException(404, f"Vector '{vector_id}' not found")


@router.put("/{vector_id}", status_code=204)
async def update_vector(name: str, vector_id: str, req: UpdateVectorRequest):
    """
    Update an existing vector.
    
    You can update the embedding, metadata, or both.
    The text field is immutable once set.
    """
    storage = _storage_manager.get_collection(name)
    if not storage:
        raise HTTPException(404, f"Collection '{name}' not found")
    
    if not storage.update(vector_id, vector=req.vector, metadata=req.metadata):
        raise HTTPException(404, f"Vector '{vector_id}' not found")
