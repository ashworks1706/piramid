/**
 * Modal Components - Reusable dialogs
 */
"use client";

import { useState, ReactNode } from 'react';

interface ModalProps {
  isOpen: boolean;
  onClose: () => void;
  title: string;
  children: ReactNode;
}

export function Modal({ isOpen, onClose, title, children }: ModalProps) {
  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-[var(--bg-secondary)] p-6 rounded-xl w-96 border border-[var(--border-color)]">
        <h3 className="text-lg font-semibold mb-4">{title}</h3>
        {children}
      </div>
    </div>
  );
}

interface CreateCollectionModalProps {
  isOpen: boolean;
  onClose: () => void;
  onCreated: () => void;
}

export function CreateCollectionModal({ isOpen, onClose, onCreated }: CreateCollectionModalProps) {
  const [name, setName] = useState('');
  const [loading, setLoading] = useState(false);

  async function handleCreate() {
    if (!name.trim()) return;
    
    try {
      setLoading(true);
      const { createCollection } = await import('../lib/api');
      await createCollection(name.trim());
      setName('');
      onClose();
      onCreated();
    } catch (e) {
      alert(e instanceof Error ? e.message : 'Failed to create');
    } finally {
      setLoading(false);
    }
  }

  return (
    <Modal isOpen={isOpen} onClose={onClose} title="Create Collection">
      <input
        type="text"
        value={name}
        onChange={(e) => setName(e.target.value)}
        placeholder="Collection name"
        className="w-full px-4 py-2 bg-[var(--bg-tertiary)] border border-[var(--border-color)] rounded-lg mb-4 focus:outline-none focus:border-[var(--accent)]"
        autoFocus
      />
      <div className="flex gap-2 justify-end">
        <button
          onClick={onClose}
          className="px-4 py-2 text-[var(--text-secondary)] hover:text-white"
        >
          Cancel
        </button>
        <button
          onClick={handleCreate}
          disabled={loading || !name.trim()}
          className="px-4 py-2 bg-[var(--accent)] hover:bg-[var(--accent-hover)] rounded-lg disabled:opacity-50"
        >
          {loading ? 'Creating...' : 'Create'}
        </button>
      </div>
    </Modal>
  );
}

interface ConfirmDeleteProps {
  name: string;
  onConfirm: () => void;
  onCancel: () => void;
}

export function ConfirmDelete({ name, onConfirm, onCancel }: ConfirmDeleteProps) {
  return (
    <div className="flex items-center gap-2">
      <span className="text-sm text-[var(--text-secondary)]">Delete {name}?</span>
      <button onClick={onConfirm} className="px-2 py-1 text-sm bg-[var(--error)] rounded">
        Yes
      </button>
      <button onClick={onCancel} className="px-2 py-1 text-sm bg-[var(--bg-tertiary)] rounded">
        No
      </button>
    </div>
  );
}
