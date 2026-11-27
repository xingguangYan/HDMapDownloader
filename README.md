# HD Map Downloader v2.0.0

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![GitHub release](https://img.shields.io/github/v/release/xingguangYan/HDMapDownloader)](https://github.com/xingguangYan/HDMapDownloader/releases)
[![GitHub stars](https://img.shields.io/github/stars/xingguangYan/HDMapDownloader?style=social)](https://github.com/xingguangYan/HDMapDownloader/stargazers)

## 🚀 功能特性

- 🗺️ **多源支持**: 谷歌、天地图、高德、Esri、OSM、NASA、USGS
- ✅ **Web 界面**: 浏览器直接访问，无需安装
- 💾 **数据库缓存**: SQLite/Redis 支持 (预留)
- ⚡ **增量同步**: 只下载缺失的瓦片
- 🌟 **星空图层**: 天地图星空图和观景台
- 🎨 **用户友好**: 完全中文界面操作

## 📦 快速开始

### 1. 本地桌面版

```bash
cd D:\探索\HDMapDownloader
cargo build --release
```

### 2. Web 服务版

```bash
cd D:\探索\HDMapDownloader
pip install -r requirements.txt
python modelscope_app.py
```

访问 http://localhost:5000

### 3. Docker 部署

```bash
docker build -t hdmapdl .
docker run -p 5000:5000 hdmapdl
```

## 🎯 核心功能

### 数据源
- Esri World Imagery - 全球高清影像
- OpenStreetMap - 矢量地图
- 天地图影像 - 国内高精度
- NASA Blue Marble - 卫星图
- USGS National Map - 美国地形
- 天地图星空 - 星空图层

### 导出格式
- GeoTIFF (带坐标信息)
- MBTiles (标准离线包)

### 坐标系统
- WGS84 (EPSG:4326)
- Web Mercator (EPSG:3857)
- CGCS2000

## ☕ 打赏支持

请作者喝一杯咖啡 ☕ 爬虫二维码在软件内

## 📄 许可证

MIT License - 自由使用，商业用途请询问

## 🔗 链接

- [GitHub](https://github.com/xingguangYan/HDMapDownloader)
- [GitCode](https://gitcode.com/xingguangYan/HDMapDownloader)
- [ModelScope](https://modelscope.cn/)

---
*Made with ❤️ for the geospatial community*
