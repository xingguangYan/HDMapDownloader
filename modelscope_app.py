#!/usr/bin/env python3
"""
HD Map Downloader - Web Interface for ModelScope
"""
import os
from flask import Flask, render_template, request, jsonify, send_from_directory, send_file
from flask_cors import CORS
import yaml

app = Flask(__name__, static_folder='static')
CORS(app)

# Configuration
BASE_DIR = os.path.dirname(os.path.abspath(__file__))
DATA_SOURCES_PATH = os.path.join(BASE_DIR, 'data-sources.yaml')
DONATION_QR_PATH = r"C:/Users/Administrator/Pictures/Saved Pictures/收款码1.png"

# Ensure static directory exists
os.makedirs(os.path.join(BASE_DIR, 'static'), exist_ok=True)

# Create placeholder QR image
qr_placeholder = r"""
<!DOCTYPE html>
<html><body style="display:flex;align-items:center;justify-content:center;height:100vh;background:#f0f0f0;">
<h2 style="color:#666;">收款码图片</h2>
</body></html>
"""
with open(os.path.join(BASE_DIR, 'static', 'qr_placeholder.png'), 'w') as f:
    f.write(qr_placeholder)

# Load data sources
def load_data_sources():
    try:
        with open(DATA_SOURCES_PATH, 'r', encoding='utf-8') as f:
            data = yaml.safe_load(f)
            return [{"name": s["name"], "type": s["type"], "url": s.get("url_template", "")} for s in data.get("sources", [])]
    except Exception as e:
        return [
            {"name": "Esri World Imagery", "type": "xyz", "url": "https://server.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile/{z}/{y}/{x}"},
            {"name": "OpenStreetMap", "type": "xyz", "url": "https://tile.openstreetmap.org/{z}/{x}/{y}.png"},
            {"name": "天地图影像", "type": "wmts", "url": "https://t0.tianditu.gov.cn/img_w/wmts"},
            {"name": "NASA Blue Marble", "type": "xyz", "url": "https://gibs.earthdata.nasa.gov/wmts/epsg3857/best/BlueMarble_ShadedRelief_Bathymetry/default/500m/{z}/{y}/{x}.jpeg"},
        ]

DATA_SOURCES = load_data_sources()

@app.route('/')
def index():
    return render_template('index.html')

@app.route('/api/data-sources')
def get_data_sources():
    return jsonify({
        "sources": DATA_SOURCES,
        "total": len(DATA_SOURCES)
    })

@app.route('/api/donation-qr')
def get_donation():
    """Return donation QR code filename"""
    if os.path.exists(DONATION_QR_PATH):
        return jsonify({
            "qr_found": True,
            "qr_path": os.path.basename(DONATION_QR_PATH)
        })
    return jsonify({"qr_found": False})

@app.route('/static/<path:filename>')
def serve_static(filename):
    return send_from_directory(app.static_folder, filename)

@app.route('/qr/<path:filename>')
def serve_qr(filename):
    """Serve donation QR code"""
    if os.path.exists(DONATION_QR_PATH):
        return send_file(DONATION_QR_PATH)
    return f"QR code not found: {filename}", 404

@app.route('/api/download', methods=['POST'])
def download():
    """Handle download request"""
    data = request.json or {}
    print(f"Download request: {data}")
    return jsonify({
        "status": "success",
        "message": "Download started",
        "jobs": data.get("jobs", 0),
        "source": data.get("source", "unknown")
    })

if __name__ == '__main__':
    print("Starting HD Map Downloader Web...")
    print(f"Data sources: {len(DATA_SOURCES)}")
    print(f"QR code path: {DONATION_QR_PATH}")
    print("Access at: http://localhost:5000")
    app.run(debug=True, port=5000, host='0.0.0.0')
