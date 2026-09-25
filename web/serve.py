#!/usr/bin/env python3
"""Serve the burin web demo and proxy terrarium tiles (S3 has no ACAO on GET)."""

from __future__ import annotations

import http.server
import socketserver
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent
PORT = 8765
UPSTREAM = "https://s3.amazonaws.com/elevation-tiles-prod/terrarium"


class Handler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=str(ROOT), **kwargs)

    def do_GET(self):
        if self.path.startswith("/terrarium/"):
            return self.proxy_tile(self.path[len("/terrarium/") :])
        return super().do_GET()

    def proxy_tile(self, rest: str):
        # rest = "z/x/y.png"
        if not rest.endswith(".png") or rest.count("/") != 2:
            self.send_error(400, "bad tile path")
            return
        z, x, y = rest[:-4].split("/")
        if not (z.isdigit() and x.isdigit() and y.isdigit()):
            self.send_error(400, "bad tile path")
            return
        url = f"{UPSTREAM}/{z}/{x}/{y}.png"
        try:
            req = urllib.request.Request(url, headers={"User-Agent": "burin-web"})
            with urllib.request.urlopen(req, timeout=60) as r:
                data = r.read()
                ctype = r.headers.get("Content-Type", "image/png")
        except urllib.error.HTTPError as e:
            self.send_response(e.code)
            self.send_header("Access-Control-Allow-Origin", "*")
            self.end_headers()
            return
        except Exception as e:
            self.send_error(502, str(e))
            return
        self.send_response(200)
        self.send_header("Content-Type", ctype)
        self.send_header("Content-Length", str(len(data)))
        self.send_header("Cache-Control", "public, max-age=86400")
        self.send_header("Access-Control-Allow-Origin", "*")
        self.end_headers()
        self.wfile.write(data)

    def log_message(self, fmt, *args):
        # Quieter: skip tile hits.
        if isinstance(args[0], str) and "/terrarium/" in args[0]:
            return
        super().log_message(fmt, *args)


if __name__ == "__main__":
    socketserver.TCPServer.allow_reuse_address = True
    with socketserver.TCPServer(("", PORT), Handler) as httpd:
        print(f"burin → http://127.0.0.1:{PORT}")
        httpd.serve_forever()
