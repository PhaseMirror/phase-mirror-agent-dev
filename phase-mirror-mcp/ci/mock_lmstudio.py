#!/usr/bin/env python3
"""Minimal LM Studio mock for CI/harness testing.

Serves the three endpoints the LmStudioClient hits:
  GET  /v1/models
  POST /v1/completions
  POST /v1/embeddings

Reads MOCK_PORT from env (default 8080).
"""

import json
import os
import sys
from http.server import HTTPServer, BaseHTTPRequestHandler

MOCK_PORT = int(os.environ.get("MOCK_PORT", "8080"))

RESPONSE_MODELS = {
    "data": [
        {"id": "test-model", "owned_by": "lmstudio", "created": 1}
    ]
}

RESPONSE_COMPLETION = {
    "id": "cmpl-mock",
    "model": "test-model",
    "choices": [
        {
            "index": 0,
            "message": {"role": "assistant", "content": "mocked"},
            "finish_reason": "stop"
        }
    ],
    "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2},
    "created": 1
}

RESPONSE_EMBEDDING = {
    "model": "test-model",
    "data": [
        {"index": 0, "embedding": [0.1, 0.2, 0.3, 0.4, 0.5]}
    ],
    "usage": {"prompt_tokens": 1, "completion_tokens": 0, "total_tokens": 1}
}


class MockHandler(BaseHTTPRequestHandler):
    def log_message(self, fmt, *args):
        sys.stderr.write("[mock-lmstudio] " + fmt % args + "\n")

    def _send_json(self, status, obj):
        body = json.dumps(obj).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        if self.path == "/v1/models":
            self._send_json(200, RESPONSE_MODELS)
        else:
            self._send_json(404, {"error": "not found"})

    def do_POST(self):
        length = int(self.headers.get("Content-Length", 0))
        body = self.rfile.read(length) if length else b"{}"
        try:
            payload = json.loads(body) if body else {}
        except json.JSONDecodeError:
            payload = {}

        if self.path == "/v1/completions":
            resp = dict(RESPONSE_COMPLETION)
            resp["model"] = payload.get("model", "test-model")
            self._send_json(200, resp)
        elif self.path == "/v1/chat/completions":
            resp = dict(RESPONSE_COMPLETION)
            resp["model"] = payload.get("model", "test-model")
            self._send_json(200, resp)
        elif self.path == "/v1/embeddings":
            resp = dict(RESPONSE_EMBEDDING)
            resp["model"] = payload.get("model", "test-model")
            self._send_json(200, resp)
        else:
            self._send_json(404, {"error": "not found"})


def main():
    port = MOCK_PORT
    if len(sys.argv) > 1:
        try:
            port = int(sys.argv[1])
        except ValueError:
            pass
    server = HTTPServer(("127.0.0.1", port), MockHandler)
    sys.stderr.write(f"[mock-lmstudio] listening on 127.0.0.1:{port}\n")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    server.server_close()


if __name__ == "__main__":
    main()
