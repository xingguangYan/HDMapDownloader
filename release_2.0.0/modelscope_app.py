#!/usr/bin/env python3
"""
HD Map Downloader - Web Interface for ModelScope
"""
import os
from flask import Flask, render_template, request, jsonify, send_from_directory
from flask_cors import CORS

app = Flask(__name__, template_folder='templates')
CORS(app)

# Configuration
BASE_DIR = os.path.dirname(os.path.abspath(__file__))
DATA_SOURCES_PATH = os.path.join(BASE_DIR, 'data-sources.yaml')
DONATION_QR_PATH = r"C:/Users/Administrator/Pictures/Saved Pictures/收款码1.png"

# Mock data sources
DATA_SOURCES = [
    {"name": "星空图层", "type": "xyz", "url": "https://skyapi.tianditu.gov.cn/sky/wmts?z={}&x={}&y={}"},
    {"name": "Esri World Imagery", "type": "xyz"},
    {"name": "OpenStreetMap", "type": "xyz"},
    {"name": "天地图影像", "type": "wmts"},
]

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

@app.route('/api/download', methods=['POST'])
def download():
    """Handle download request (middleware layer)"""
    data = request.json
    # Implement download logic here
    return jsonify({"status": "success", "message": "Download started"})

if __name__ == '__main__':
    app.run(debug=True, port=5000)
