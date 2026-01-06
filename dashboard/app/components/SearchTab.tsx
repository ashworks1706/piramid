/**
 * Search Tab - Query vectors and view results
 */
"use client";

import { useState } from 'react';
import { searchVectors, SearchResult } from '../lib/api';

interface SearchTabProps {
  collection: string;
}

export function SearchTab({ collection }: SearchTabProps) {
  const [vector, setVector] = useState('');
  const [limit, setLimit] = useState('10');
  const [metric, setMetric] = useState<'cosine' | 'euclidean' | 'dot'>('cosine');
  const [loading, setLoading] = useState(false);
  const [results, setResults] = useState<SearchResult[]>([]);
  const [tookMs, setTookMs] = useState<number | null>(null);

  async function handleSearch() {
    try {
      setLoading(true);
      const vectorArray = vector.split(',').map(v => parseFloat(v.trim()));
      
      const res = await searchVectors(collection, {
        vector: vectorArray,
        limit: parseInt(limit),
        metric,
      });
      
      setResults(res.results);
      setTookMs(res.took_ms);
    } catch (e) {
      alert(e instanceof Error ? e.message : 'Search failed');
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="space-y-6">
      {/* Search Form */}
      <div className="bg-[var(--bg-secondary)] rounded-xl p-6 border border-[var(--border-color)]">
        <h3 className="font-semibold mb-4">Search Query</h3>
        
        <div className="space-y-4">
          <div>
            <label className="block text-sm text-[var(--text-secondary)] mb-1">
              Query Vector
            </label>
            <input
              type="text"
              value={vector}
              onChange={(e) => setVector(e.target.value)}
              placeholder="0.1, 0.2, 0.3, 0.4"
              className="w-full px-4 py-2 bg-[var(--bg-tertiary)] border border-[var(--border-color)] rounded-lg focus:outline-none focus:border-[var(--accent)]"
            />
          </div>
          
          <div className="flex gap-4">
            <div className="flex-1">
              <label className="block text-sm text-[var(--text-secondary)] mb-1">Limit</label>
              <input
                type="number"
                value={limit}
                onChange={(e) => setLimit(e.target.value)}
                className="w-full px-4 py-2 bg-[var(--bg-tertiary)] border border-[var(--border-color)] rounded-lg focus:outline-none focus:border-[var(--accent)]"
              />
            </div>
            <div className="flex-1">
              <label className="block text-sm text-[var(--text-secondary)] mb-1">Distance Metric</label>
              <select
                value={metric}
                onChange={(e) => setMetric(e.target.value as 'cosine' | 'euclidean' | 'dot')}
                className="w-full px-4 py-2 bg-[var(--bg-tertiary)] border border-[var(--border-color)] rounded-lg focus:outline-none focus:border-[var(--accent)]"
              >
                <option value="cosine">Cosine</option>
                <option value="euclidean">Euclidean</option>
                <option value="dot">Dot Product</option>
              </select>
            </div>
          </div>
          
          <button
            onClick={handleSearch}
            disabled={loading || !vector.trim()}
            className="px-6 py-2 bg-[var(--accent)] hover:bg-[var(--accent-hover)] rounded-lg disabled:opacity-50"
          >
            {loading ? 'Searching...' : 'Search'}
          </button>
        </div>
      </div>

      {/* Results */}
      {results.length > 0 && (
        <div className="bg-[var(--bg-secondary)] rounded-xl p-6 border border-[var(--border-color)]">
          <div className="flex items-center justify-between mb-4">
            <h3 className="font-semibold">Results</h3>
            {tookMs !== null && (
              <span className="text-sm text-[var(--text-secondary)]">
                {tookMs}ms • {results.length} results
              </span>
            )}
          </div>
          
          <div className="space-y-3">
            {results.map((result, i) => (
              <div 
                key={result.id} 
                className="bg-[var(--bg-tertiary)] rounded-lg p-4 border border-[var(--border-color)]"
              >
                <div className="flex items-start justify-between">
                  <div className="flex-1">
                    <div className="flex items-center gap-2 mb-1">
                      <span className="text-[var(--text-secondary)]">#{i + 1}</span>
                      <span className="font-mono text-sm text-[var(--accent)]">
                        {result.id.slice(0, 8)}...
                      </span>
                    </div>
                    {result.text && <p className="text-sm mb-2">{result.text}</p>}
                    {Object.keys(result.metadata).length > 0 && (
                      <div className="text-xs text-[var(--text-secondary)]">
                        {JSON.stringify(result.metadata)}
                      </div>
                    )}
                  </div>
                  <div className="text-right">
                    <span className="text-lg font-semibold text-[var(--success)]">
                      {result.score.toFixed(4)}
                    </span>
                    <p className="text-xs text-[var(--text-secondary)]">similarity</p>
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
