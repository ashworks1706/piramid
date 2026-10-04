"""The HTTP client for a Piramid server."""

from __future__ import annotations

import json
import os
from typing import Any, Dict, Iterator, List, Mapping, Optional, Sequence

import httpx

from .errors import PiramidError
from .types import (
    Collection,
    CompactResult,
    Document,
    EmbedResult,
    Filter,
    GenerateResult,
    Hit,
    InsertResult,
    Message,
    Metadata,
    Model,
    StreamEvent,
    UpsertResult,
)

DEFAULT_BASE_URL = "http://127.0.0.1:6333"


class Piramid:
    """A client for one Piramid server.

    The key comes from ``api_key`` or, when that is not given, from ``PIRAMID_API_KEY``. A server on
    the default loopback bind needs none.
    """

    def __init__(
        self,
        base_url: str = DEFAULT_BASE_URL,
        api_key: Optional[str] = None,
        timeout: Optional[float] = 120.0,
        http: Optional[httpx.Client] = None,
    ) -> None:
        key = api_key if api_key is not None else os.environ.get("PIRAMID_API_KEY")
        headers = {"Authorization": f"Bearer {key}"} if key else {}
        self._http = http or httpx.Client(
            base_url=base_url.rstrip("/"), headers=headers, timeout=timeout
        )

    def close(self) -> None:
        """Closes the connection pool."""
        self._http.close()

    def __enter__(self) -> "Piramid":
        return self

    def __exit__(self, *_: object) -> None:
        self.close()

    # Server

    def health(self) -> bool:
        """Whether the server answers its liveness check."""
        return self._http.get("/api/health").is_success

    def version(self) -> Dict[str, Any]:
        """The server's version and build."""
        return self._request("GET", "/api/version")

    def model(self) -> Model:
        """The loaded model."""
        return self._request("GET", "/api/model")

    # Collections

    def collections(self) -> List[Collection]:
        """Every open collection."""
        return self._request("GET", "/api/collections")["collections"]

    def create_collection(self, name: str) -> Collection:
        """Creates an empty collection."""
        return self._request("POST", "/api/collections", {"name": name})

    def collection(self, name: str) -> Collection:
        """One collection's summary."""
        return self._request("GET", f"/api/collections/{_seg(name)}")

    def delete_collection(self, name: str) -> bool:
        """Deletes a collection and its files."""
        return self._request("DELETE", f"/api/collections/{_seg(name)}")["deleted"]

    def count(self, collection: str) -> int:
        """How many documents a collection holds."""
        return self._request("GET", f"/api/collections/{_seg(collection)}/count")["count"]

    def compact(self, collection: str) -> CompactResult:
        """Rewrites a collection's record store without deleted documents."""
        return self._request("POST", f"/api/collections/{_seg(collection)}/compact")

    # Documents

    def embed(
        self,
        collection: str,
        texts: Sequence[str],
        metadata: Optional[Sequence[Metadata]] = None,
    ) -> EmbedResult:
        """Embeds texts with the server's provider and stores them, creating the collection."""
        body: Dict[str, Any] = {"texts": list(texts)}
        if metadata is not None:
            body["metadata"] = list(metadata)
        return self._request("POST", f"/api/collections/{_seg(collection)}/embed", body)

    def insert(
        self,
        collection: str,
        vectors: Sequence[Sequence[float]],
        texts: Sequence[str],
        metadata: Optional[Sequence[Metadata]] = None,
        normalize: bool = False,
    ) -> InsertResult:
        """Stores documents with vectors computed by the caller."""
        body: Dict[str, Any] = {
            "vectors": [list(v) for v in vectors],
            "texts": list(texts),
            "normalize": normalize,
        }
        if metadata is not None:
            body["metadata"] = list(metadata)
        return self._request("POST", f"/api/collections/{_seg(collection)}/vectors", body)

    def upsert(
        self,
        collection: str,
        vector: Sequence[float],
        text: str,
        metadata: Optional[Metadata] = None,
        id: Optional[str] = None,
        normalize: bool = False,
    ) -> UpsertResult:
        """Inserts one document, or replaces the one with the same id."""
        body: Dict[str, Any] = {"vector": list(vector), "text": text, "normalize": normalize}
        if metadata is not None:
            body["metadata"] = dict(metadata)
        if id is not None:
            body["id"] = id
        return self._request("POST", f"/api/collections/{_seg(collection)}/upsert", body)

    def get(self, collection: str, id: str) -> Document:
        """One document by id."""
        return self._request("GET", f"/api/collections/{_seg(collection)}/vectors/{_seg(id)}")

    def list(self, collection: str, limit: int = 100, offset: int = 0) -> List[Document]:
        """A page of a collection's documents."""
        return self._request(
            "GET",
            f"/api/collections/{_seg(collection)}/vectors",
            params={"limit": limit, "offset": offset},
        )

    def delete(self, collection: str, ids: Sequence[str]) -> int:
        """Deletes documents by id and returns how many were deleted."""
        return self._request(
            "DELETE", f"/api/collections/{_seg(collection)}/vectors", {"ids": list(ids)}
        )["deleted_count"]

    # Search

    def search(
        self,
        collection: str,
        vectors: Sequence[Sequence[float]],
        k: int = 10,
        filter: Optional[Filter] = None,
    ) -> List[List[Hit]]:
        """The best k documents for each query vector."""
        body: Dict[str, Any] = {"vectors": [list(v) for v in vectors], "k": k}
        if filter is not None:
            body["filter"] = filter
        return self._request("POST", f"/api/collections/{_seg(collection)}/search", body)[
            "results"
        ]

    def search_text(
        self,
        collection: str,
        query: str,
        k: int = 10,
        filter: Optional[Filter] = None,
    ) -> List[Hit]:
        """The best k documents for a text query, embedded by the server."""
        body: Dict[str, Any] = {"query": query, "k": k}
        if filter is not None:
            body["filter"] = filter
        results = self._request(
            "POST", f"/api/collections/{_seg(collection)}/search/text", body
        )["results"]
        return results[0] if results else []

    # Generation

    def generate(
        self,
        messages: Optional[Sequence[Message]] = None,
        prompt: Optional[str] = None,
        collection: Optional[str] = None,
        k: int = 4,
        query: Optional[str] = None,
        **sampling: Any,
    ) -> GenerateResult:
        """Generates an answer, retrieving from ``collection`` first when one is named.

        ``sampling`` takes ``max_new_tokens``, ``temperature``, ``top_p``, ``top_k``,
        ``repetition_penalty``, ``seed`` and ``stop``.
        """
        body = _generate_body(messages, prompt, collection, k, query, sampling, stream=False)
        return self._request("POST", "/api/generate", body)

    def stream(
        self,
        messages: Optional[Sequence[Message]] = None,
        prompt: Optional[str] = None,
        collection: Optional[str] = None,
        k: int = 4,
        query: Optional[str] = None,
        **sampling: Any,
    ) -> Iterator[StreamEvent]:
        """Generates as a stream of events: ``retrieval``, then ``token`` per token, then ``done``."""
        body = _generate_body(messages, prompt, collection, k, query, sampling, stream=True)
        with self._http.stream("POST", "/api/generate", json=body) as response:
            if not response.is_success:
                response.read()
                raise PiramidError.from_response(response)
            event = "message"
            data: List[str] = []
            for line in response.iter_lines():
                if line.startswith("event:"):
                    event = line[6:].strip()
                elif line.startswith("data:"):
                    data.append(line[5:].lstrip())
                elif line == "" and data:
                    payload = json.loads("\n".join(data))
                    if event == "error":
                        raise PiramidError(payload.get("error", "generation failed"), 500)
                    yield {"event": event, "data": payload}
                    event, data = "message", []

    # Transport

    def _request(
        self,
        method: str,
        path: str,
        body: Optional[Mapping[str, Any]] = None,
        params: Optional[Mapping[str, Any]] = None,
    ) -> Any:
        response = self._http.request(method, path, json=body, params=params)
        if not response.is_success:
            raise PiramidError.from_response(response)
        return response.json()


def _seg(value: str) -> str:
    """A path segment with reserved characters escaped."""
    return httpx.URL("/" + value).raw_path.decode()[1:].replace("/", "%2F")


def _generate_body(
    messages: Optional[Sequence[Message]],
    prompt: Optional[str],
    collection: Optional[str],
    k: int,
    query: Optional[str],
    sampling: Mapping[str, Any],
    stream: bool,
) -> Dict[str, Any]:
    if (messages is None) == (prompt is None):
        raise ValueError("give exactly one of messages or prompt")
    body: Dict[str, Any] = {"stream": stream, **sampling}
    if messages is not None:
        body["messages"] = [dict(m) for m in messages]
    else:
        body["prompt"] = prompt
    if collection is not None:
        retrieval: Dict[str, Any] = {"collection": collection, "k": k}
        if query is not None:
            retrieval["query"] = query
        body["retrieval"] = retrieval
    return body
