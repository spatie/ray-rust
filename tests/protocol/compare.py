"""Capture real PHP and Rust HTTP requests and compare portable Ray payloads.

Run from the crate root:
    python3 tests/protocol/compare.py --php-root /path/to/spatie/ray
The PHP checkout needs `composer install --no-dev`. No Ray app is required.
Use --write-fixture only after reviewing changes to the reference or scenarios.
"""

import argparse
import json
import subprocess
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

REFERENCE = "dd0a28063c15e847f5d67a5855c97c273ddfe693"


def capture(command):
    requests = []

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *_):
            pass

        def do_GET(self):
            self.send_response(404)
            self.end_headers()

        def do_POST(self):
            assert self.path == "/", self.path
            assert self.headers["Content-Type"].startswith("application/json")
            assert self.headers["User-Agent"] == "Ray 1.0"
            body = self.rfile.read(int(self.headers["Content-Length"]))
            requests.append(json.loads(body))
            self.send_response(200)
            self.end_headers()
            self.wfile.write(b"{}")

    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    worker = threading.Thread(target=server.serve_forever)
    worker.start()
    try:
        subprocess.run(command + [str(server.server_port)], check=True, timeout=120)
    finally:
        server.shutdown()
        worker.join()
        server.server_close()
    assert requests, "client sent no requests"
    return requests


def normalize(requests):
    uuids = {}
    normalized = []
    for request in requests:
        uuid = request["uuid"]
        assert isinstance(uuid, str) and len(uuid) == 36, uuid
        request["uuid"] = uuids.setdefault(uuid, len(uuids))
        # Language/version, source file and line are runtime-specific. Preserve
        # project name and UUID relationships so broken updates cannot pass.
        request["meta"] = {"project_name": request["meta"].get("project_name", "")} if isinstance(request["meta"], dict) else []
        for payload in request["payloads"]:
            origin = payload.pop("origin")
            assert set(origin) == {"file", "line_number", "hostname"}, origin
            assert origin["file"] and origin["line_number"] > 0 and origin["hostname"], origin
        normalized.append(request)
    return normalized


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--php-root", required=True, type=Path)
    parser.add_argument("--write-fixture", action="store_true")
    args = parser.parse_args()
    revision = subprocess.check_output(["git", "-C", str(args.php_root), "rev-parse", "HEAD"], text=True).strip()
    assert revision == REFERENCE, f"expected PHP {REFERENCE}, got {revision}"
    subprocess.run(["cargo", "build", "--locked", "--example", "protocol"], check=True)
    php = normalize(capture(["php", "tests/protocol/php.php", str(args.php_root.resolve())]))
    rust = normalize(capture([str(Path("target/debug/examples/protocol").resolve())]))
    for index, (expected, actual) in enumerate(zip(php, rust)):
        assert actual == expected, f"request {index}:\nPHP {json.dumps(expected, ensure_ascii=False, indent=2)}\nRust {json.dumps(actual, ensure_ascii=False, indent=2)}"
    assert len(php) == len(rust), (len(php), len(rust))
    fixture = Path("tests/fixtures/php-wire.json")
    if args.write_fixture:
        fixture.write_text(json.dumps(php, indent=2, ensure_ascii=False) + "\n")
    else:
        assert json.loads(fixture.read_text()) == php, "checked-in fixture differs from PHP"
    print(f"PASS: {len(php)} HTTP requests, {sum(len(r['payloads']) for r in php)} payloads match PHP {REFERENCE}")


if __name__ == "__main__":
    main()
