/**
 * Sidebar Component - Collection list and navigation
 */
"use client";

import { Collection } from '../lib/api';

interface SidebarProps {
  collections: Collection[];
  selectedCollection: string | null;
  onSelectCollection: (name: string) => void;
  onCreateCollection: () => void;
  loading: boolean;
}

export function Sidebar({ 
  collections, 
  selectedCollection, 
  onSelectCollection,
  onCreateCollection,
  loading 
}: SidebarProps) {
  return (
    <aside className="w-64 bg-[var(--bg-secondary)] border-r border-[var(--border-color)] flex flex-col">
      {/* Header */}
      <div className="p-4 border-b border-[var(--border-color)]">
        <h1 className="text-xl font-bold flex items-center gap-2">
          <span className="text-2xl">🔺</span> Piramid
        </h1>
        <p className="text-xs text-[var(--text-secondary)] mt-1">Vector Database</p>
      </div>

      {/* Collections List */}
      <div className="flex-1 overflow-y-auto p-2">
        <div className="flex items-center justify-between px-2 py-1 mb-2">
          <span className="text-xs text-[var(--text-secondary)] uppercase tracking-wider">
            Collections
          </span>
          <button
            onClick={onCreateCollection}
            className="text-[var(--accent)] hover:text-[var(--accent-hover)] text-lg"
            title="Create collection"
          >
            +
          </button>
        </div>

        {loading ? (
          <div className="text-center py-4 text-[var(--text-secondary)]">Loading...</div>
        ) : collections.length === 0 ? (
          <div className="text-center py-4 text-[var(--text-secondary)] text-sm">
            No collections yet
          </div>
        ) : (
          <ul className="space-y-1">
            {collections.map((col) => (
              <li key={col.name}>
                <button
                  onClick={() => onSelectCollection(col.name)}
                  className={`w-full text-left px-3 py-2 rounded-lg transition-colors ${
                    selectedCollection === col.name
                      ? 'bg-[var(--accent)] text-white'
                      : 'hover:bg-[var(--bg-tertiary)]'
                  }`}
                >
                  <div className="font-medium truncate">{col.name}</div>
                  <div className="text-xs opacity-70">
                    {col.count} vectors
                  </div>
                </button>
              </li>
            ))}
          </ul>
        )}
      </div>

      {/* Server Status */}
      <div className="p-4 border-t border-[var(--border-color)]">
        <div className="flex items-center gap-2 text-sm">
          <span className="w-2 h-2 rounded-full bg-[var(--success)]"></span>
          <span className="text-[var(--text-secondary)]">Server Online</span>
        </div>
      </div>
    </aside>
  );
}
