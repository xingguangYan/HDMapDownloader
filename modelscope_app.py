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
            {"id": "esri", "name": "Esri 世界影像", "max": 19, "key": False},
            {"id": "osm", "name": "OpenStreetMap (de 镜像)", "max": 19, "key": False},
            {"id": "carto_light", "name": "CartoDB 浅色矢量", "max": 18, "key": False},
            {"id": "carto_dark", "name": "CartoDB 深色矢量", "max": 18, "key": False},
            {"id": "eox_s2", "name": "Sentinel-2 无云镶嵌", "max": 18, "key": False},
            {"id": "nasa_bm", "name": "NASA Blue Marble", "max": 8, "key": False},
            {"id": "modis", "name": "MODIS Terra (每日)", "max": 9, "key": False, "time": True},
            {"id": "aqua", "name": "MODIS Aqua (每日)", "max": 9, "key": False, "time": True},
            {"id": "viirs", "name": "VIIRS SNPP (每日)", "max": 9, "key": False, "time": True},
            {"id": "hls_l", "name": "Landsat 8/9 (HLS L30)", "max": 13, "key": False, "time": True},
            {"id": "hls_s", "name": "Sentinel-2 (HLS S30)", "max": 13, "key": False, "time": True},
            {"id": "tianditu", "name": "天地图影像 img_w", "max": 18, "key": True},
            {"id": "planet", "name": "Planet NICFI 月度镶嵌", "max": 18, "key": True},
        ]
    })

@app.route("/health")
def health():
    return jsonify({"status": "ok", "version": "3.0.0"})

if __name__ == "__main__":
    port = int(os.environ.get("PORT", 5000))
    app.run(host="0.0.0.0", port=port)
