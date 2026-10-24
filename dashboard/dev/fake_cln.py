"""A simulated Core Lightning clnrest endpoint, for dashboard work without Polar.

Answers getinfo and listconfigs; refuses every other method the way a rune
restricted to those two does (code 1502), so Firewatch's read-only check passes.
The version is read from a file on every request, so writing a new version to
it simulates an upgrade.

    python3 fake_cln.py <port> <version-file>

Not a Lightning node: it holds no keys and speaks no Lightning protocol.
"""

import json
import sys
from http.server import BaseHTTPRequestHandler, HTTPServer

port, version_file = int(sys.argv[1]), sys.argv[2]


class Handler(BaseHTTPRequestHandler):
    def do_POST(self):
        self.rfile.read(int(self.headers.get("Content-Length") or 0))
        method = self.path.rsplit("/", 1)[-1]
        if method == "getinfo":
            with open(version_file) as f:
                version = f.read().strip()
            code, body = 200, {"version": version, "network": "regtest"}
        elif method == "listconfigs":
            code, body = 200, {
                "configs": {"experimental-splicing": {"set": False, "source": "default"}}
            }
        else:
            code, body = 403, {"code": 1502, "message": "Not permitted: method not allowed by rune"}
        data = json.dumps(body).encode()
        self.send_response(code)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def log_message(self, *args):
        pass


HTTPServer(("127.0.0.1", port), Handler).serve_forever()
