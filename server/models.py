"""
API Request/Response Models

Pydantic models define the JSON shape of our API.
They handle validation automatically - if someone sends
bad data, they get a clear error message.
"""

from pydantic import BaseModel
from typing import Optional, Any


# =============================================================================
# COLLECTION MODELS
# =============================================================================

class CreateCollectionRequest(BaseModel):
    name: str
    dimension: Optional[int] = None


class CollectionInfo(BaseModel):
    name: str
    vector_count: int
    dimension: Optional[int] = None


class CollectionsResponse(BaseModel):
    collections: list[CollectionInfo]


# =============================================================================
# VECTOR MODELS
# =============================================================================

class InsertVectorRequest(BaseModel):
    vector: list[float]
    text: Optional[str] = None
    metadata: Optional[dict[str, Any]] = None


class InsertVectorResponse(BaseModel):
    id: str
    status: str


class VectorResponse(BaseModel):
    id: str
    vector: list[float]
    text: Optional[str] = None
    metadata: dict[str, Any] = {}


class UpdateVectorRequest(BaseModel):
    vector: Optional[list[float]] = None
    text: Optional[str] = None
    metadata: Optional[dict[str, Any]] = None


# =============================================================================
# SEARCH MODELS
# =============================================================================

class FilterRequest(BaseModel):
    """Metadata filter for search"""
    field: str
    operator: str  # eq, ne, gt, gte, lt, lte, in
    value: Any


class SearchRequest(BaseModel):
    vector: list[float]
    limit: Optional[int] = 10
    metric: Optional[str] = "cosine"  # cosine, euclidean, dot
    filter: Optional[FilterRequest] = None


class SearchResult(BaseModel):
    id: str
    score: float
    text: Optional[str] = None
    metadata: dict[str, Any] = {}


class SearchResponse(BaseModel):
    results: list[SearchResult]
    took_ms: int
