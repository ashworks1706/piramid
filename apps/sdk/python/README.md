# piramid

Python client for [Piramid](https://piramiddb.com), an inference engine for retrieval systems.

```bash
pip install piramid
```

```python
from piramid import Piramid

with Piramid("http://127.0.0.1:6333") as db:   # key from api_key= or PIRAMID_API_KEY
    db.embed("docs", ["Compaction rewrites the record store without deleted documents."])

    answer = db.generate(
        messages=[{"role": "user", "content": "What happens to deleted documents?"}],
        collection="docs",
        k=2,
    )
    print(answer["text"], answer["retrieval"]["passages"])

    for event in db.stream(prompt="Piramid is", max_new_tokens=32):
        if event["event"] == "token":
            print(event["data"]["text"], end="")
```

Collections: `collections`, `create_collection`, `collection`, `delete_collection`, `count`,
`compact`. Documents: `embed`, `insert`, `upsert`, `get`, `list`, `delete`. Search: `search` (by
vectors), `search_text` (embedded by the server), both with a metadata `filter` such as
`{"topic": {"eq": "storage"}}`. Server: `health`, `version`, `model`.

A refused request raises `PiramidError` with the server's `message` and HTTP `status`.
`/v1/chat/completions` is OpenAI-compatible, so the `openai` package works against it directly.

Server docs: https://piramiddb.com/docs. MIT licensed.
