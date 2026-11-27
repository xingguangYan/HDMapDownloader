
#!/usr/bin/env python3
# HD Map Downloader - Web Service
import os
from flask import Flask, render_template, jsonify, send_from_directory

app = Flask(__name__, static_folder='static', template_folder='templates')

# Paths
BASE_DIR = os.path.dirname(os.path.abspath(__file__))
DONATION_QR_PATH = r"C:/Users/Administrator/Pictures/Saved Pictures/收款码1.png"

# Mock data sources
DATA_SOURCES = [
    {"name": "Esri World Imagery", "type": "XYZ", "url": "https://server.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer"},
    {"name": "OpenStreetMap", "type": "XYZ", "url": "https://tile.openstreetmap.org"},
    {"name": "天地图影像", "type": "WMTS", "url": "https://t0.tianditu.gov.cn/img_w/wmts"},
    {"name": "NASA Blue Marble", "type": "XYZ", "url": "https://gibs.earthdata.nasa.gov/wmts/epsg3857/best/BlueMarble_ShadedRelief_Bathymetry/default/500m"},
    {"name": "USGS National Map", "type": "XYZ", "url": "https://basemap.nationalmap.gov/arcgis/rest/services/USGSImageryOnly/MapServer"},
]

@app.route('/')
def index():
    return render_template('index.html')

@app.route('/api/data-sources')
def get_data_sources():
    return jsonify({
        "success": True,
        "count": len(DATA_SOURCES),
        "sources": DATA_SOURCES
    })

@app.route('/api/donation')
def donation():
    if os.path.exists(DONATION_QR_PATH):
        return jsonify({"found": True, "path": DONATION_QR_PATH})
    return jsonify({"found": False})

@app.route('/api/about')
def about():
    return jsonify({
        "name": "HD Map Downloader",
        "version": "2.0.0",
        "description": "Multi-source tile map downloader with web interface",
        "features": ["Web interface", "Database caching", "Incremental sync", "Multiple sources", "Donation QR"]
    })

@app.route('/api/status')
def status():
    return jsonify({"status": "running", "uptime": "OK"})

@app.route('/static/<path:filename>')
def static_files(filename):
    return send_from_directory(app.static_folder, filename)

@app.route('/qr/<filename>')
def qr_files(filename):
    if os.path.exists(DONATION_QR_PATH):
        return send_from_directory(os.path.dirname(DONATION_QR_PATH), filename)
    return f"Not found: {filename}", 404

if __name__ == '__main__':
    print("=" * 50)
    print("HD Map Downloader v2.0.0")
    print("=" * 50)
    print(f"Data sources: {len(DATA_SOURCES)}")
    print(f"Donation QR: {DONATION_QR_PATH}")
    print("=" * 50)
    app.run(host='0.0.0.0', port=5000, debug=True)
