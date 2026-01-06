"""
Vector Storage Engine

This is the Python equivalent of our Rust storage.
Uses numpy for fast vector operations.

The same concepts apply:
- In-memory HashMap (dict) for fast lookups
- Brute-force O(n) search (good for <10k vectors)
- JSON persistence to disk
"""

import uuid
import os
from pathlib import Path
from typing import Optional, Any
import numpy as np
import orjson  # Fast JSON library


class VectorStorage:
    """
    Storage for a single collection.
    
    Vectors are stored in a dict: {id: {id, vector, text, metadata}}
    We also keep a numpy array for fast similarity computation.
    """
    
    def __init__(self, path: str):
        self.path = path
        self.vectors: dict[str, dict] = {}  # id -> entry
        self._vectors_array: Optional[np.ndarray] = None  # Cache for search
        self._ids_list: list[str] = []  # Parallel to _vectors_array
        self._dirty = False  # Needs rebuild of cache
    
    @property
    def dimension(self) -> Optional[int]:
        """Get vector dimension (None if empty)"""
        if not self.vectors:
            return None
        first = next(iter(self.vectors.values()))
        return len(first["vector"])
    
    def insert(
        self, 
        vector: list[float], 
        text: Optional[str] = None,
        metadata: Optional[dict] = None,
    ) -> str:
        """Insert a vector, returns its ID"""
        vector_id = str(uuid.uuid4())
        
        self.vectors[vector_id] = {
            "id": vector_id,
            "vector": vector,
            "text": text or "",
            "metadata": metadata or {},
        }
        
        self._dirty = True
        self.save()
        
        return vector_id
    
    def get(self, vector_id: str) -> Optional[dict]:
        """Get a vector by ID"""
        return self.vectors.get(vector_id)
    
    def delete(self, vector_id: str) -> bool:
        """Delete a vector, returns True if it existed"""
        if vector_id in self.vectors:
            del self.vectors[vector_id]
            self._dirty = True
            self.save()
            return True
        return False
    
    def update(
        self, 
        vector_id: str, 
        vector: Optional[list[float]] = None,
        metadata: Optional[dict] = None,
    ) -> bool:
        """Update a vector's data"""
        if vector_id not in self.vectors:
            return False
        
        entry = self.vectors[vector_id]
        
        if vector is not None:
            entry["vector"] = vector
            self._dirty = True
        
        if metadata is not None:
            entry["metadata"] = metadata
        
        self.save()
        return True
    
    def search(
        self,
        query: list[float],
        k: int = 10,
        metric: str = "cosine",
        filter: Optional[dict] = None,
    ) -> list[dict]:
        """
        Search for similar vectors.
        
        Uses numpy for fast computation:
        1. Stack all vectors into a matrix
        2. Compute distances in one operation (vectorized)
        3. Sort and return top-k
        """
        if not self.vectors:
            return []
        
        # Rebuild cache if needed
        if self._dirty or self._vectors_array is None:
            self._rebuild_cache()
        
        # Apply metadata filter first
        if filter:
            mask = self._apply_filter(filter)
            if not mask.any():
                return []
            filtered_vectors = self._vectors_array[mask]
            filtered_ids = [self._ids_list[i] for i, m in enumerate(mask) if m]
        else:
            filtered_vectors = self._vectors_array
            filtered_ids = self._ids_list
        
        # Convert query to numpy
        query_vec = np.array(query, dtype=np.float32)
        
        # Compute similarities (vectorized - this is FAST)
        if metric == "cosine":
            scores = self._cosine_similarity(query_vec, filtered_vectors)
        elif metric == "euclidean":
            # Negative because lower distance = more similar
            scores = -self._euclidean_distance(query_vec, filtered_vectors)
        elif metric in ("dot", "dot_product"):
            scores = self._dot_product(query_vec, filtered_vectors)
        else:
            scores = self._cosine_similarity(query_vec, filtered_vectors)
        
        # Get top-k indices
        if len(scores) <= k:
            top_indices = np.argsort(scores)[::-1]
        else:
            # Use argpartition for efficiency with large arrays
            top_indices = np.argpartition(scores, -k)[-k:]
            top_indices = top_indices[np.argsort(scores[top_indices])[::-1]]
        
        # Build results
        results = []
        for idx in top_indices:
            vector_id = filtered_ids[idx]
            entry = self.vectors[vector_id]
            results.append({
                "id": vector_id,
                "score": float(scores[idx]),
                "text": entry.get("text"),
                "metadata": entry.get("metadata", {}),
            })
        
        return results
    
    def _rebuild_cache(self):
        """Rebuild the numpy array cache"""
        self._ids_list = list(self.vectors.keys())
        vectors = [self.vectors[id]["vector"] for id in self._ids_list]
        self._vectors_array = np.array(vectors, dtype=np.float32)
        self._dirty = False
    
    def _apply_filter(self, filter: dict) -> np.ndarray:
        """Apply metadata filter, returns boolean mask"""
        field = filter.get("field")
        operator = filter.get("operator")
        value = filter.get("value")
        
        mask = np.zeros(len(self._ids_list), dtype=bool)
        
        for i, id in enumerate(self._ids_list):
            entry = self.vectors[id]
            actual = entry.get("metadata", {}).get(field)
            
            if actual is None:
                continue
            
            if operator == "eq":
                mask[i] = actual == value
            elif operator == "ne":
                mask[i] = actual != value
            elif operator == "gt":
                mask[i] = actual > value
            elif operator == "gte":
                mask[i] = actual >= value
            elif operator == "lt":
                mask[i] = actual < value
            elif operator == "lte":
                mask[i] = actual <= value
            elif operator == "in":
                mask[i] = actual in value
        
        return mask
    
    # =========================================================================
    # Distance functions (vectorized numpy operations)
    # =========================================================================
    
    def _cosine_similarity(self, query: np.ndarray, vectors: np.ndarray) -> np.ndarray:
        """
        Cosine similarity: dot(a,b) / (||a|| * ||b||)
        Range: -1 to 1, higher = more similar
        """
        # Normalize query
        query_norm = query / (np.linalg.norm(query) + 1e-10)
        
        # Normalize all vectors at once
        norms = np.linalg.norm(vectors, axis=1, keepdims=True) + 1e-10
        vectors_norm = vectors / norms
        
        # Dot product with normalized vectors = cosine similarity
        return vectors_norm @ query_norm
    
    def _euclidean_distance(self, query: np.ndarray, vectors: np.ndarray) -> np.ndarray:
        """
        Euclidean distance: ||a - b||
        Range: 0 to inf, lower = more similar
        """
        diff = vectors - query
        return np.linalg.norm(diff, axis=1)
    
    def _dot_product(self, query: np.ndarray, vectors: np.ndarray) -> np.ndarray:
        """
        Dot product: sum(a * b)
        Range: -inf to inf, higher = more similar (for normalized vectors)
        """
        return vectors @ query
    
    # =========================================================================
    # Persistence
    # =========================================================================
    
    def save(self):
        """Save to disk as JSON"""
        os.makedirs(os.path.dirname(self.path) or ".", exist_ok=True)
        
        data = {
            "vectors": self.vectors,
        }
        
        with open(self.path, "wb") as f:
            f.write(orjson.dumps(data))
    
    def load(self) -> bool:
        """Load from disk, returns True if file existed"""
        if not os.path.exists(self.path):
            return False
        
        try:
            with open(self.path, "rb") as f:
                data = orjson.loads(f.read())
            
            self.vectors = data.get("vectors", {})
            self._dirty = True
            return True
        except Exception as e:
            print(f"Failed to load {self.path}: {e}")
            return False


class StorageManager:
    """
    Manages multiple collections.
    Like having multiple tables in a traditional database.
    """
    
    def __init__(self, data_dir: str):
        self.data_dir = data_dir
        self.collections: dict[str, VectorStorage] = {}
        
        os.makedirs(data_dir, exist_ok=True)
    
    def load_all(self):
        """Load all existing collections from disk"""
        for filename in os.listdir(self.data_dir):
            if filename.endswith(".json"):
                name = filename[:-5]  # Remove .json
                path = os.path.join(self.data_dir, filename)
                storage = VectorStorage(path)
                if storage.load():
                    self.collections[name] = storage
                    print(f"  Loaded: {name} ({len(storage.vectors)} vectors)")
    
    def save_all(self):
        """Save all collections"""
        for storage in self.collections.values():
            storage.save()
    
    def create_collection(self, name: str) -> VectorStorage:
        """Create a new empty collection"""
        path = os.path.join(self.data_dir, f"{name}.json")
        storage = VectorStorage(path)
        storage.save()  # Create empty file
        self.collections[name] = storage
        return storage
    
    def get_collection(self, name: str) -> Optional[VectorStorage]:
        """Get a collection by name"""
        return self.collections.get(name)
    
    def delete_collection(self, name: str) -> bool:
        """Delete a collection and its file"""
        if name not in self.collections:
            return False
        
        storage = self.collections.pop(name)
        try:
            os.remove(storage.path)
        except:
            pass
        return True
