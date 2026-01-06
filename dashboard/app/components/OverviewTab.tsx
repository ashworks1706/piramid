/**
 * Overview Tab - Collection stats and quick insert
 */
"use client";

import { useState } from 'react';
import { Collection, insertVector } from '../lib/api';

interface OverviewTabProps {
  collection: Collection;
}

export function OverviewTab({ collection }: OverviewTabProps) {
  return (
    <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
      <StatCard title="Vectors" value={collection.vector_count.toLocaleString()} icon="📊" />
      <StatCard title="Dimension" value={collection.dimension?.toString() || 'N/A'} icon="📐" />
      <StatCard title="Status" value="Ready" icon="✅" />
      
      <div className="md:col-span-3 bg-[var(--bg-secondary)] rounded-xl p-6 border border-[var(--border-color)]">
        <h3 className="font-semibold mb-4">Quick Insert</h3>
        <QuickInsert collection={collection.name} />
      </div>
    </div>
  );
}

function StatCard({ title, value, icon }: { title: string; value: string; icon: string }) {
  return (
    <div className="bg-[var(--bg-secondary)] rounded-xl p-6 border border-[var(--border-color)]">
      <div className="flex items-center justify-between">
        <div>
          <p className="text-[var(--text-secondary)] text-sm">{title}</p>
          <p className="text-2xl font-bold mt-1">{value}</p>
        </div>
        <span className="text-3xl">{icon}</span>
      </div>
    </div>
  );
}

function QuickInsert({ collection }: { collection: string }) {
  const [vector, setVector] = useState('');
  const [text, setText] = useState('');
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<string | null>(null);

  async function handleInsert() {
    try {
      setLoading(true);
      const vectorArray = vector.split(',').map(v => parseFloat(v.trim()));
      
      const res = await insertVector(collection, {
        vector: vectorArray,
        text: text || undefined,
      });
      
      setResult(`Inserted with ID: ${res.id}`);
      setVector('');
      setText('');
    } catch (e) {
      setResult(`Error: ${e instanceof Error ? e.message : 'Failed'}`);
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="space-y-4">
      <div>
        <label className="block text-sm text-[var(--text-secondary)] mb-1">
          Vector (comma-separated floats)
        </label>
        <input
          type="text"
          value={vector}
          onChange={(e) => setVector(e.target.value)}
          placeholder="0.1, 0.2, 0.3, 0.4"
          className="w-full px-4 py-2 bg-[var(--bg-tertiary)] border border-[var(--border-color)] rounded-lg focus:outline-none focus:border-[var(--accent)]"
        />
      </div>
      <div>
        <label className="block text-sm text-[var(--text-secondary)] mb-1">
          Text (optional)
        </label>
        <input
          type="text"
          value={text}
          onChange={(e) => setText(e.target.value)}
          placeholder="Original text content"
          className="w-full px-4 py-2 bg-[var(--bg-tertiary)] border border-[var(--border-color)] rounded-lg focus:outline-none focus:border-[var(--accent)]"
        />
      </div>
      <div className="flex items-center gap-4">
        <button
          onClick={handleInsert}
          disabled={loading || !vector.trim()}
          className="px-6 py-2 bg-[var(--accent)] hover:bg-[var(--accent-hover)] rounded-lg disabled:opacity-50"
        >
          {loading ? 'Inserting...' : 'Insert Vector'}
        </button>
        {result && (
          <span className={`text-sm ${result.startsWith('Error') ? 'text-[var(--error)]' : 'text-[var(--success)]'}`}>
            {result}
          </span>
        )}
      </div>
    </div>
  );
}
