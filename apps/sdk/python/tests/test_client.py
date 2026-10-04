import json

import httpx
import pytest

from piramid import Piramid, PiramidError


def client(handler):
    http = httpx.Client(base_url="http://test", transport=httpx.MockTransport(handler))
    return Piramid(http=http)


def test_search_sends_vectors_k_and_filter():
    seen = {}

    def handler(request):
        seen["path"] = request.url.path
        seen["body"] = json.loads(request.content)
        return httpx.Response(200, json={"results": [[{"id": "a", "score": 1.0, "text": "t", "metadata": {}}]], "latency_ms": 0.1})

    hits = client(handler).search("docs", [[0.1, 0.2]], k=3, filter={"topic": {"eq": "x"}})
    assert seen["path"] == "/api/collections/docs/search"
    assert seen["body"] == {"vectors": [[0.1, 0.2]], "k": 3, "filter": {"topic": {"eq": "x"}}}
    assert hits[0][0]["id"] == "a"


def test_generate_with_retrieval_builds_the_request():
    seen = {}

    def handler(request):
        seen["body"] = json.loads(request.content)
        return httpx.Response(200, json={"text": "ok", "finish_reason": "stop", "usage": {}})

    result = client(handler).generate(
        messages=[{"role": "user", "content": "q"}], collection="docs", k=2, max_new_tokens=8
    )
    assert result["text"] == "ok"
    assert seen["body"] == {
        "stream": False,
        "max_new_tokens": 8,
        "messages": [{"role": "user", "content": "q"}],
        "retrieval": {"collection": "docs", "k": 2},
    }


def test_generate_needs_exactly_one_of_messages_or_prompt():
    with pytest.raises(ValueError):
        client(lambda r: httpx.Response(200)).generate()


def test_stream_yields_events_in_order():
    body = (
        'event: retrieval\ndata: {"collection": "docs", "passages": []}\n\n'
        'event: token\ndata: {"token": 1, "text": "Hi"}\n\n'
        'event: done\ndata: {"finish_reason": "stop", "usage": {}}\n\n'
    )

    def handler(request):
        return httpx.Response(200, text=body, headers={"content-type": "text/event-stream"})

    events = list(client(handler).stream(prompt="p", collection="docs"))
    assert [e["event"] for e in events] == ["retrieval", "token", "done"]
    assert events[1]["data"]["text"] == "Hi"


def test_stream_error_event_raises():
    body = 'event: error\ndata: {"error": "boom"}\n\n'
    handler = lambda r: httpx.Response(200, text=body)
    with pytest.raises(PiramidError, match="boom"):
        list(client(handler).stream(prompt="p"))


def test_refused_request_raises_with_status_and_message():
    handler = lambda r: httpx.Response(404, json={"error": "collection not found", "code": 404})
    with pytest.raises(PiramidError) as caught:
        client(handler).count("missing")
    assert caught.value.status == 404
    assert caught.value.message == "collection not found"


def test_api_key_is_sent_as_bearer(monkeypatch):
    monkeypatch.setenv("PIRAMID_API_KEY", "secret")
    db = Piramid("http://test")
    assert db._http.headers["Authorization"] == "Bearer secret"


def test_names_are_escaped_in_paths():
    seen = {}

    def handler(request):
        seen["raw"] = request.url.raw_path
        return httpx.Response(200, json={"count": 0})

    client(handler).count("a b/c")
    assert seen["raw"] == b"/api/collections/a%20b%2Fc/count"
