"""
Routes Package

Exports all API routers for the Piramid server.
Each router handles a specific domain (collections, vectors, search).
"""

from .health import router as health_router
from .collections import router as collections_router, set_storage_manager as set_collections_storage
from .vectors import router as vectors_router, set_storage_manager as set_vectors_storage
from .search import router as search_router, set_storage_manager as set_search_storage


def init_routes(storage_manager):
    """
    Initialize all routes with the storage manager.
    
    This dependency injection pattern keeps routes testable
    and avoids circular imports.
    """
    set_collections_storage(storage_manager)
    set_vectors_storage(storage_manager)
    set_search_storage(storage_manager)


__all__ = [
    "health_router",
    "collections_router", 
    "vectors_router",
    "search_router",
    "init_routes",
]
