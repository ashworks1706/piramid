"""
API Client - Helper functions for calling the Piramid REST API

This module handles all HTTP communication with the server.
Keeps API logic separate from UI components.
"""

const API_BASE = typeof window !== 'undefined' 
  ? `${window.location.protocol}//${window.location.hostname}:6333/api`
  : 'http://localhost:6333/api';

// Generic fetch wrapper with error handling
export async function fetchAPI<T>(endpoint: string, options?: RequestInit): Promise<T> {
  const res = await fetch(`${API_BASE}${endpoint}`, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options?.headers,
    },
  });
  
  if (!res.ok) {
    const error = await res.json().catch(() => ({ message: res.statusText }));
    throw new Error(error.message || 'API error');
  }
  
  if (res.status === 204) {
    return {} as T;
  }
  
  return res.json();
}

// =============================================================================
// HEALTH
// =============================================================================

export async function checkHealth(): Promise<boolean> {
  try {
    await fetchAPI('/health');
    return true;
  } catch {
    return false;
  }
}

// =============================================================================
// COLLECTIONS
// =============================================================================

export interface Collection {
  name: string;
  vector_count: number;
  dimension: number | null;
}

export async function listCollections(): Promise<Collection[]> {
  const data = await fetchAPI<{ collections: Collection[] }>('/collections');
  return data.collections;
}

export async function createCollection(name: string): Promise<Collection> {
  return fetchAPI<Collection>('/collections', {
    method: 'POST',
    body: JSON.stringify({ name }),
  });
}

export async function deleteCollection(name: string): Promise<void> {
  await fetchAPI(`/collections/${name}`, { method: 'DELETE' });
}

// =============================================================================
// VECTORS
// =============================================================================

export interface InsertVectorRequest {
  vector: number[];
  text?: string;
  metadata?: Record<string, unknown>;
}

export async function insertVector(
  collection: string, 
  data: InsertVectorRequest
): Promise<{ id: string }> {
  return fetchAPI(`/collections/${collection}/vectors`, {
    method: 'POST',
    body: JSON.stringify(data),
  });
}

export async function deleteVector(collection: string, id: string): Promise<void> {
  await fetchAPI(`/collections/${collection}/vectors/${id}`, { method: 'DELETE' });
}

// =============================================================================
// SEARCH
// =============================================================================

export interface SearchRequest {
  vector: number[];
  limit?: number;
  metric?: 'cosine' | 'euclidean' | 'dot';
}

export interface SearchResult {
  id: string;
  score: number;
  text: string | null;
  metadata: Record<string, unknown>;
}

export interface SearchResponse {
  results: SearchResult[];
  took_ms: number;
}

export async function searchVectors(
  collection: string, 
  query: SearchRequest
): Promise<SearchResponse> {
  return fetchAPI(`/collections/${collection}/search`, {
    method: 'POST',
    body: JSON.stringify(query),
  });
}
