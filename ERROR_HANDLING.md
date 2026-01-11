# Error Handling Guide

## Overview
The dashboard now includes comprehensive error handling with detailed error messages to help debug API issues quickly.

## What's New

### 1. Enhanced API Error Class
Located in `dashboard/app/lib/api.ts`:
- **APIError class**: Captures HTTP status, status text, endpoint, and error details
- **Network error detection**: Status 0 indicates network/connection issues
- **Error body parsing**: Attempts to parse JSON error responses from the server

### 2. ErrorDisplay Component
Located in `dashboard/app/components/ErrorDisplay.tsx`:
- **Visual error messages**: Color-coded by error type (network, server, client, etc.)
- **Status code display**: Shows HTTP status codes prominently
- **Endpoint information**: Shows which API endpoint failed
- **Helpful tips**: Context-specific advice based on error type

### Error Severities:
- 🌐 **Network Error** (Status 0): Can't reach the server
- 🔥 **Server Error** (5xx): Backend crashed or misconfigured
- ⚠️ **Service Unavailable** (503): Feature not configured (e.g., embedder)
- 🔍 **Not Found** (404): Resource doesn't exist
- ❌ **Client Error** (4xx): Bad request or invalid input

### 3. Updated Components
All components now use ErrorDisplay instead of `alert()`:
- **OverviewTab**: Insert vector errors
- **BrowseTab**: Loading and delete errors
- **SearchTab**: Vector and text search errors
- **EmbedTab**: Single and batch embedding errors

## Debugging Common Errors

### 503 Service Unavailable on `/embed` endpoints
**Symptoms**: Embedding features return 503 error

**Causes**:
1. Embedder not initialized in AppState
2. Missing environment variables (EMBEDDING_PROVIDER, OPENAI_API_KEY, etc.)
3. Embedding provider configuration failed

**How to Debug**:
1. Check the error display message - it will show the exact endpoint
2. Look at Docker logs: `docker compose logs piramid`
3. Verify environment variables are set in `docker-compose.yml`
4. Check if embedder is created in `src/bin/server.rs`

**Fix**:
```yaml
# Add to docker-compose.yml:
environment:
  - EMBEDDING_PROVIDER=openai
  - OPENAI_API_KEY=your_key_here
  - OPENAI_MODEL=text-embedding-3-small
```

### Network Errors (Status 0)
**Symptoms**: Can't reach server, all requests fail

**Causes**:
1. Server not running
2. Wrong port (dashboard expects 6333)
3. CORS issues
4. Firewall blocking connection

**How to Debug**:
1. Check if server is running: `docker compose ps`
2. Check server logs: `docker compose logs piramid`
3. Test API directly: `curl http://localhost:6333/api/health`

### 404 Not Found
**Symptoms**: Specific collection or vector not found

**Causes**:
1. Collection name typo
2. Vector ID doesn't exist
3. Wrong API endpoint

**How to Debug**:
1. Error display shows the exact endpoint that failed
2. Verify collection name is correct
3. Use Browse tab to see existing vectors

## Testing Error Display

### Test 503 Error (Embedding not configured):
1. Don't set EMBEDDING_PROVIDER environment variable
2. Try to use Embed tab
3. Should see: "⚠️ Service Unavailable [503]" with helpful tip

### Test Network Error:
1. Stop the server: `docker compose stop`
2. Try any API operation
3. Should see: "🌐 Network Error" with tip to check server

### Test 404 Error:
1. Try to search in a non-existent collection
2. Should see: "🔍 Not Found [404]" with helpful tip

## Implementation Details

### API Client Error Flow:
```typescript
try {
  const response = await fetch(endpoint);
  if (!response.ok) {
    // Parse error body
    const errorBody = await response.text();
    // Throw APIError with all details
    throw new APIError(message, status, statusText, endpoint, errorBody);
  }
  return response.json();
} catch (error) {
  // Network error or parsing failure
  if (error instanceof APIError) throw error;
  throw new APIError('Network error', 0, 'Network Error', endpoint);
}
```

### Component Error Handling:
```typescript
const [error, setError] = useState<Error | APIError | null>(null);

async function handleAction() {
  try {
    setError(null); // Clear previous errors
    const result = await apiFunction();
    // Handle success
  } catch (e) {
    setError(e instanceof Error ? e : new Error('Failed'));
  }
}

// In JSX:
{error && <ErrorDisplay error={error} onDismiss={() => setError(null)} />}
```

## Next Steps

### To Fix 503 Errors on Embed Endpoints:
1. Check `src/bin/server.rs` - verify embedder is being created
2. Add environment variable validation on startup
3. Add `/api/embed/status` endpoint to check embedder availability
4. Log embedder initialization status

### To Improve Error Handling Further:
1. Add toast notifications for non-blocking errors
2. Add error logging/reporting system
3. Add retry logic for transient errors
4. Add more context-specific tips for common issues
