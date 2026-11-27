#!/usr/bin/env python3
"""
HD Map Downloader - ModelScope Web 服务
直接提供完整单页应用（二维码已 base64 内嵌于 web.html）
"""
import os
from flask import Flask, send_from_directory, jsonify

BASE_DIR = os.path.dirname(os.path.abspath(__file__))
app = Flask(__name__, static_folder=None)
WEB_HTML = os.path.join(BASE_DIR, "web.html")

@app.route("/")
def index():
    if os.path.exists(WEB_HTML):
        return send_from_directory(BASE_DIR, "web.html")
    return "web.html not found", 404

@app.route("/api/sources")
def sources():
    return jsonify({
        "sources": [
            {"id": "esri", "name": "Esri World Imagery", "max": 19},
            {"id": "osm", "name": "OpenStreetMap", "max": 19},
            {"id": "nasa", "name": "NASA Blue Marble", "max": 8},
            {"id": "usgs", "name": "USGS National Map", "max": 16},
            {"id": "tianditu", "name": "天地图影像", "max": 18}
        ]
    })

@app.route("/health")
def health():
    return jsonify({"status": "ok", "version": "2.0.0"})

if __name__ == "__main__":
    port = int(os.environ.get("PORT", 5000))
    app.run(host="0.0.0.0", port=port)
