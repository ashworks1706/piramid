/**
 * ErrorDisplay - Show API errors in a user-friendly way
 */
"use client";

import { APIError } from '../lib/api';

interface ErrorDisplayProps {
  error: Error | APIError | string | null;
  onDismiss?: () => void;
}

export function ErrorDisplay({ error, onDismiss }: ErrorDisplayProps) {
  if (!error) return null;

  const isAPIError = error instanceof APIError;
  const errorMessage = typeof error === 'string' ? error : error.message;
  
  // Determine error severity by status code
  const getSeverity = (status: number) => {
    if (status === 0) return 'network'; // Network error
    if (status >= 500) return 'server'; // Server error
    if (status === 404) return 'notfound'; // Not found
    if (status === 503) return 'unavailable'; // Service unavailable
    if (status >= 400) return 'client'; // Client error
    return 'unknown';
  };

  const severity = isAPIError ? getSeverity((error as APIError).status) : 'unknown';

  const severityColors = {
    network: 'border-orange-500 bg-orange-500/10',
    server: 'border-red-500 bg-red-500/10',
    notfound: 'border-yellow-500 bg-yellow-500/10',
    unavailable: 'border-purple-500 bg-purple-500/10',
    client: 'border-yellow-500 bg-yellow-500/10',
    unknown: 'border-red-500 bg-red-500/10',
  };

  const severityTitles = {
    network: 'Network Error',
    server: 'Server Error',
    notfound: 'Not Found',
    unavailable: 'Service Unavailable',
    client: 'Request Error',
    unknown: 'Error',
  };

  return (
    <div className={`rounded-lg border-2 p-4 ${severityColors[severity]}`}>
      <div className="flex items-start justify-between">
        <div className="flex-1">
          <h4 className="font-semibold mb-1">
            {severityTitles[severity]}
            {isAPIError && (error as APIError).status > 0 && (
              <span className="ml-2 text-sm font-mono opacity-75">
                [{(error as APIError).status}]
              </span>
            )}
          </h4>
          <p className="text-sm mb-2">{errorMessage}</p>
          
          {isAPIError && (
            <div className="text-xs opacity-75 space-y-1 mt-2 font-mono">
              <div>Endpoint: <span className="text-[var(--accent)]">{(error as APIError).endpoint}</span></div>
              {(error as APIError).status === 503 && (
                <div className="mt-2 text-sm">
                  <strong>Note:</strong> This service requires configuration. Check if embedding provider is set up correctly.
                </div>
              )}
              {(error as APIError).status === 404 && (
                <div className="mt-2 text-sm">
                  <strong>Note:</strong> The requested resource doesn't exist. Check the collection name or vector ID.
                </div>
              )}
              {(error as APIError).status === 0 && (
                <div className="mt-2 text-sm">
                  <strong>Note:</strong> Can't reach the server. Make sure Piramid is running on port 6333.
                </div>
              )}
            </div>
          )}
        </div>
        
        {onDismiss && (
          <button
            onClick={onDismiss}
            className="text-xl opacity-50 hover:opacity-100 transition-opacity px-2"
            aria-label="Dismiss error"
          >
            ×
          </button>
        )}
      </div>
    </div>
  );
}
