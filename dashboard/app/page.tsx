/**
 * Piramid Dashboard - Main Page
 * 
 * A clean, modular dashboard for managing vector collections.
 * Components are split into separate files for maintainability.
 */
"use client";

import { useState, useEffect, useCallback } from 'react';

// API client
import { checkHealth, listCollections, deleteCollection, Collection } from './lib/api';

// UI Components
import { Sidebar } from './components/Sidebar';
import { ServerOffline } from './components/ServerOffline';
import { CreateCollectionModal } from './components/Modal';
import { OverviewTab } from './components/OverviewTab';
import { SearchTab } from './components/SearchTab';
import { BrowseTab } from './components/BrowseTab';

// =============================================================================
// MAIN DASHBOARD
// =============================================================================

type Tab = 'overview' | 'search' | 'browse';

export default function Dashboard() {
  // Server state
  const [serverOnline, setServerOnline] = useState<boolean | null>(null);
  
  // Collections state
  const [collections, setCollections] = useState<Collection[]>([]);
  const [selectedCollection, setSelectedCollection] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  
  // UI state
  const [activeTab, setActiveTab] = useState<Tab>('overview');
  const [showCreateModal, setShowCreateModal] = useState(false);

  // ---------------------------------------------------------------------------
  // Data Loading
  // ---------------------------------------------------------------------------
  
  const checkServer = useCallback(async () => {
    const online = await checkHealth();
    setServerOnline(online);
    return online;
  }, []);

  const loadCollections = useCallback(async () => {
    try {
      setLoading(true);
      const cols = await listCollections();
      setCollections(cols);
      
      // Auto-select first collection if none selected
      if (cols.length > 0 && !selectedCollection) {
        setSelectedCollection(cols[0].name);
      }
    } catch (e) {
      console.error('Failed to load collections:', e);
    } finally {
      setLoading(false);
    }
  }, [selectedCollection]);

  // Initial load
  useEffect(() => {
    async function init() {
      const online = await checkServer();
      if (online) {
        await loadCollections();
      }
    }
    init();
  }, [checkServer, loadCollections]);

  // ---------------------------------------------------------------------------
  // Event Handlers
  // ---------------------------------------------------------------------------

  async function handleDeleteCollection() {
    if (!selectedCollection) return;
    if (!confirm('Delete collection "' + selectedCollection + '"?')) return;
    
    try {
      await deleteCollection(selectedCollection);
      setSelectedCollection(null);
      await loadCollections();
    } catch (e) {
      alert(e instanceof Error ? e.message : 'Failed to delete');
    }
  }

  // ---------------------------------------------------------------------------
  // Render
  // ---------------------------------------------------------------------------

  // Loading state
  if (serverOnline === null) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-[var(--bg-primary)]">
        <div className="text-xl">Connecting to Piramid...</div>
      </div>
    );
  }

  // Server offline
  if (!serverOnline) {
    return <ServerOffline onRetry={checkServer} />;
  }

  // Get current collection data
  const currentCollection = collections.find(c => c.name === selectedCollection);

  return (
    <div className="min-h-screen bg-[var(--bg-primary)] text-[var(--text-primary)] flex">
      {/* Sidebar */}
      <Sidebar
        collections={collections}
        selectedCollection={selectedCollection}
        onSelectCollection={setSelectedCollection}
        onCreateCollection={() => setShowCreateModal(true)}
        loading={loading}
      />

      {/* Main Content */}
      <main className="flex-1 p-6 overflow-auto">
        {selectedCollection && currentCollection ? (
          <>
            {/* Header */}
            <header className="mb-6">
              <div className="flex items-center justify-between">
                <div>
                  <h2 className="text-2xl font-bold">{selectedCollection}</h2>
                  <p className="text-[var(--text-secondary)]">
                    {currentCollection.vector_count} vectors
                    {currentCollection.dimension && ' • ' + currentCollection.dimension + ' dimensions'}
                  </p>
                </div>
                <button
                  onClick={handleDeleteCollection}
                  className="px-4 py-2 text-[var(--error)] hover:bg-[var(--error)]/10 rounded-lg transition-colors"
                >
                  Delete Collection
                </button>
              </div>

              {/* Tabs */}
              <nav className="flex gap-2 mt-6">
                {(['overview', 'search', 'browse'] as Tab[]).map((tab) => (
                  <button
                    key={tab}
                    onClick={() => setActiveTab(tab)}
                    className={'px-4 py-2 rounded-lg capitalize transition-colors ' +
                      (activeTab === tab
                        ? 'bg-[var(--accent)] text-white'
                        : 'text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)]')
                    }
                  >
                    {tab}
                  </button>
                ))}
              </nav>
            </header>

            {/* Tab Content */}
            {activeTab === 'overview' && <OverviewTab collection={currentCollection} />}
            {activeTab === 'search' && <SearchTab collection={selectedCollection} />}
            {activeTab === 'browse' && <BrowseTab collection={selectedCollection} />}
          </>
        ) : (
          <div className="h-full flex items-center justify-center text-[var(--text-secondary)]">
            <div className="text-center">
              <p className="text-lg mb-4">Select or create a collection to get started</p>
              <button
                onClick={() => setShowCreateModal(true)}
                className="px-6 py-3 bg-[var(--accent)] hover:bg-[var(--accent-hover)] rounded-lg text-white"
              >
                Create Collection
              </button>
            </div>
          </div>
        )}
      </main>

      {/* Modals */}
      <CreateCollectionModal
        isOpen={showCreateModal}
        onClose={() => setShowCreateModal(false)}
        onCreated={loadCollections}
      />
    </div>
  );
}
