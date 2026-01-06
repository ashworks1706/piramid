"""
Search Routes

The core functionality of a vector database - finding similar vectors!

Vector similarity search works by:
1. Taking a query vector (your search embedding)
2. Computing distance to every stored vector
3. Returning the K nearest neighbors

Supports multiple distance metrics:
- cosine: Angular similarity (most common for text embeddings)
- euclidean: Straight-line distance in vector space
- dot: Dot product (similar to cosine but not normalized)
"""

import time
from fastapi import APIRouter, HTTPException

from models import SearchRequest, SearchResponse, SearchResult

router = APIRouter(prefix="/collections/{name}/search", tags=["search"])

# Storage manager is injected via dependency
_storage_manager = None

def set_storage_manager(sm):
    """Inject the storage manager (called from main.py)"""
    global _storage_manager
    _storage_manager = sm


@router.post("", response_model=SearchResponse)
async def search_vectors(name: str, req: SearchRequest):
    """
    Search for vectors similar to the query vector.
    
    This is the main operation you'll use for:
    - Semantic search (find text similar to a query)
    - Recommendations (find items similar to what user liked)
    - Deduplication (find near-duplicate entries)
    - Clustering (group similar items together)
    
    Parameters:
    - vector: The query embedding to search for
    - limit: Maximum number of results (default 10)
    - metric: Distance function - "cosine", "euclidean", or "dot"
    - filter: Optional metadata filter (e.g., {"category": "sports"})
    
    Returns results sorted by similarity (highest first for cosine/dot,
    lowest first for euclidean).
    """
    storage = _storage_manager.get_collection(name)
    if not storage:
        raise HTTPException(404, f"Collection '{name}' not found")
    
    # Time the search for performance monitoring
    start = time.time()
    
    results = storage.search(
        query=req.vector,
        k=req.limit or 10,
        metric=req.metric or "cosine",
        filter=req.filter,
    )
    
    # Convert to milliseconds
    took_ms = int((time.time() - start) * 1000)
    
    return SearchResponse(
        results=[
            SearchResult(
                id=r["id"],
                score=r["score"],
                text=r.get("text"),
                metadata=r.get("metadata", {}),
            )
            for r in results
        ],
        took_ms=took_ms,
    )
