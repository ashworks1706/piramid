# Piramid Server (Python)

A FastAPI REST server for the Piramid vector database.

## Structure

```
server/
├── main.py           # Entry point - FastAPI app setup
├── storage.py        # Vector storage engine (numpy-based)
├── models.py         # Pydantic request/response models
├── requirements.txt  # Python dependencies
└── routes/           # API endpoint handlers
    ├── __init__.py   # Route exports & initialization
    ├── health.py     # Health check endpoint
    ├── collections.py # Collection CRUD
    ├── vectors.py    # Vector CRUD
    └── search.py     # Similarity search
```

## Running

```bash
# Install dependencies
pip install -r requirements.txt

# Start server (port 6333)
python main.py

# Or with auto-reload for development
uvicorn main:app --reload --port 6333
```

## API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/health` | Health check |
| GET | `/api/collections` | List all collections |
| POST | `/api/collections` | Create collection |
| DELETE | `/api/collections/{name}` | Delete collection |
| POST | `/api/collections/{name}/vectors` | Insert vector |
| GET | `/api/collections/{name}/vectors/{id}` | Get vector |
| DELETE | `/api/collections/{name}/vectors/{id}` | Delete vector |
| POST | `/api/collections/{name}/search` | Search vectors |

## Distance Metrics

The search endpoint supports three distance metrics:

- **cosine** - Angular similarity (default, best for text embeddings)
- **euclidean** - Straight-line distance
- **dot** - Dot product (use for normalized vectors)

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `PIRAMID_HOST` | `0.0.0.0` | Server host |
| `PIRAMID_PORT` | `6333` | Server port |
| `PIRAMID_DATA_DIR` | `./data` | Data persistence directory |
| `PIRAMID_DASHBOARD_PATH` | `../dashboard/out` | Static dashboard files |

## Data Format

Collections are stored as JSON files in the data directory:

```json
{
  "vectors": {
    "uuid-1": {
      "id": "uuid-1",
      "vector": [0.1, 0.2, 0.3, ...],
      "text": "original text",
      "metadata": {"category": "example"}
    }
  }
}
```
