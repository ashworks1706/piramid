/**
 * Server Offline Component - Shown when API is not available
 */
"use client";

interface ServerOfflineProps {
  onRetry: () => void;
}

export function ServerOffline({ onRetry }: ServerOfflineProps) {
  return (
    <div className="min-h-screen flex items-center justify-center bg-[var(--bg-primary)]">
      <div className="text-center">
        <div className="text-6xl mb-4">🔺</div>
        <h1 className="text-2xl font-bold mb-2">Piramid Server Offline</h1>
        <p className="text-[var(--text-secondary)] mb-6">
          Cannot connect to the Piramid server at port 6333
        </p>
        <div className="space-y-3">
          <p className="text-sm text-[var(--text-secondary)]">
            Start the server with:
          </p>
          <code className="block bg-[var(--bg-tertiary)] px-4 py-2 rounded-lg text-sm">
            cd server && python main.py
          </code>
          <button
            onClick={onRetry}
            className="mt-4 px-6 py-2 bg-[var(--accent)] hover:bg-[var(--accent-hover)] rounded-lg transition-colors"
          >
            Retry Connection
          </button>
        </div>
      </div>
    </div>
  );
}
