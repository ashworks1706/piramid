/**
 * Browse Tab - View and manage vectors in a collection
 */
"use client";

import { useState, useEffect, useCallback } from 'react';
import { Collection, deleteVector } from '../lib/api';
import { fetchAPI } from '../lib/api';

interface BrowseTabProps {
  collection: string;
}

interface VectorEntry {
  id: string;
  text: string | null;
  metadata: Record<string, unknown>;
}

export function BrowseTab({ collection }: BrowseTabProps) {
  const [vectors, setVectors] = useState<VectorEntry[]>([]);
  const [loading, setLoading] = useState(true);

  const loadVectors = useCallback(async () => {
    try {
      setLoading(true);
      const info = await fetchAPI<Collection>(`/collections/${collection}`);
      
      if (info.count > 0) {
        // Get vectors via list endpoint
        const vectors = await fetchAPI<VectorEntry[]>(`/collections/${collection}/vectors?limit=100`);
        setVectors(vectors.map(v => ({
          id: v.id,
          text: v.text,
          metadata: v.metadata,
        })));
        
      } else {
        setVectors([]);
      }
    } catch (e) {
      console.error('Failed to load vectors:', e);
    } finally {
      setLoading(false);
    }
  }, [collection]);

  useEffect(() => {
    loadVectors();
  }, [loadVectors]);

  async function handleDelete(id: string) {
    if (!confirm('Delete this vector?')) return;
    
    try {
      await deleteVector(collection, id);
      setVectors(vectors.filter(v => v.id !== id));
    } catch (e) {
      alert(e instanceof Error ? e.message : 'Failed to delete');
    }
  }

  if (loading) {
    return <div className="text-center py-8 text-[var(--text-secondary)]">Loading vectors...</div>;
  }

  if (vectors.length === 0) {
    return (
      <div className="text-center py-8 text-[var(--text-secondary)]">
        <p className="text-lg mb-2">No vectors yet</p>
        <p className="text-sm">Insert some vectors using the Overview tab</p>
      </div>
    );
  }

  return (
    <div className="bg-[var(--bg-secondary)] rounded-xl border border-[var(--border-color)] overflow-hidden">
      <table className="w-full">
        <thead>
          <tr className="border-b border-[var(--border-color)]">
            <th className="px-4 py-3 text-left text-sm font-medium text-[var(--text-secondary)]">ID</th>
            <th className="px-4 py-3 text-left text-sm font-medium text-[var(--text-secondary)]">Text</th>
            <th className="px-4 py-3 text-left text-sm font-medium text-[var(--text-secondary)]">Metadata</th>
            <th className="px-4 py-3 text-right text-sm font-medium text-[var(--text-secondary)]">Actions</th>
          </tr>
        </thead>
        <tbody>
          {vectors.map((v) => (
            <tr key={v.id} className="border-b border-[var(--border-color)] hover:bg-[var(--bg-tertiary)]">
              <td className="px-4 py-3 font-mono text-sm">{v.id.slice(0, 8)}...</td>
              <td className="px-4 py-3 text-sm">
                {v.text || <span className="text-[var(--text-secondary)]">—</span>}
              </td>
              <td className="px-4 py-3 text-sm text-[var(--text-secondary)]">
                {Object.keys(v.metadata).length > 0 
                  ? JSON.stringify(v.metadata).slice(0, 50) 
                  : '—'}
              </td>
              <td className="px-4 py-3 text-right">
                <button
                  onClick={() => handleDelete(v.id)}
                  className="text-[var(--error)] hover:underline text-sm"
                >
                  Delete
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
