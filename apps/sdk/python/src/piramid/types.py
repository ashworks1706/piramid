"""The shapes the server sends and accepts, as typed dictionaries."""

from __future__ import annotations

from typing import Any, Dict, List, Optional

from typing_extensions import NotRequired, TypedDict

Metadata = Dict[str, Any]
"""Field names to JSON values a search can filter on."""

Filter = Dict[str, Dict[str, Any]]
"""A metadata field to an operator (eq, ne, gt, gte, lt, lte, in) and its value."""


class Message(TypedDict):
    """One chat message."""

    role: str
    content: str


class Collection(TypedDict):
    """A collection's summary."""

    name: str
    count: int
    created_at: Optional[int]
    updated_at: Optional[int]
    dimensions: Optional[int]
    metric: str


class CompactResult(TypedDict):
    """What a compaction did."""

    documents: int
    bytes_before: int
    bytes_after: int
    latency_ms: float


class Document(TypedDict):
    """A stored document."""

    id: str
    vector: List[float]
    text: str
    metadata: Metadata


class Hit(TypedDict):
    """One search result."""

    id: str
    score: float
    text: str
    metadata: Metadata


class InsertResult(TypedDict):
    """The ids of inserted documents."""

    ids: List[str]
    count: int
    latency_ms: float


class UpsertResult(TypedDict):
    """The id of an upserted document and whether it was new."""

    id: str
    created: bool
    latency_ms: float


class EmbedResult(TypedDict):
    """The ids and embeddings of embedded texts."""

    ids: List[str]
    embeddings: List[List[float]]
    total_tokens: Optional[int]


class Passage(TypedDict):
    """A retrieved passage."""

    id: str
    score: float
    text: str


class Retrieval(TypedDict):
    """What a generation retrieved."""

    collection: str
    passages: List[Passage]
    embed_ms: float
    search_ms: float


class Usage(TypedDict):
    """Token counts and timings of a generation."""

    prompt_tokens: int
    cached_prompt_tokens: int
    completion_tokens: int
    time_to_first_token_ms: NotRequired[float]
    total_ms: float


class GenerateResult(TypedDict):
    """A finished generation."""

    text: str
    finish_reason: str
    usage: Usage
    retrieval: NotRequired[Retrieval]


class Model(TypedDict):
    """The loaded model."""

    name: str
    architecture: str
    device: str
    max_sequence_length: int
    hook: str


class StreamEvent(TypedDict):
    """One server-sent event: retrieval, token or done, with its payload."""

    event: str
    data: Dict[str, Any]
