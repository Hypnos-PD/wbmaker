#!/usr/bin/env python3
"""Static server with a same-origin proxy for WBArts data and art."""

from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import urlsplit
from urllib.request import Request, urlopen
import os

ROOT = os.path.join(os.path.dirname(__file__), "..", "web")
UPSTREAM = "https://sva.hypd.asia/"


class Handler(SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=ROOT, **kwargs)

    def do_GET(self):
        path = urlsplit(self.path).path
        prefix = "/api/wbarts/"
        if path.startswith(prefix):
            upstream_path = path[len(prefix):]
            if ".." in upstream_path.split("/"):
                self.send_error(403)
                return
            try:
                request = Request(UPSTREAM + upstream_path, headers={"User-Agent": "wbmaker"})
                with urlopen(request, timeout=30) as response:
                    body = response.read()
                    self.send_response(response.status)
                    self.send_header("Content-Type", response.headers.get_content_type())
                    self.send_header("Content-Length", str(len(body)))
                    self.end_headers()
                    self.wfile.write(body)
            except Exception as error:
                self.send_error(502, str(error))
            return
        super().do_GET()


if __name__ == "__main__":
    server = ThreadingHTTPServer(("127.0.0.1", 8000), Handler)
    print("wbmaker dev server: http://127.0.0.1:8000")
    server.serve_forever()
