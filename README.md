# HD Map Downloader v3.0.0

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

免费多源高清地图瓦片下载器：聚合 Esri、OpenStreetMap、CartoDB、NASA GIBS（Blue Marble / MODIS / VIIRS / Landsat HLS / Sentinel-2 HLS）、EOX Sentinel-2 无云镶嵌、天地图、Planet NICFI 等公开数据源，自带框选下载、时间范围、密钥管理、ZIP 导出。

**在线使用（免安装）**：<https://xingguangyan.github.io/HDMapDownloader/>

## 主要功能
- 框选下载：开启「框选模式」在地图上拖出矩形，自由设定缩放级别范围，自动并发下载瓦片
- 时间范围：MODIS / VIIRS / Landsat 8/9 / Sentinel-2 等时间序列源可指定影像日期（GIBS WMS TIME 参数）
- 密钥管理：天地图 tk、Planet API Key 直接在界面填写，即时启用；免Key源开箱即用
- 多源聚合：基础底图 / 卫星影像 / 时间序列 / 需Key源 / 数据门户（中科星图、长光吉林一号、资源卫星中心、Copernicus）五大类
- 离线缓存：瓦片落盘缓存，重复下载自动跳过
- ZIP 导出：一键打包已下载瓦片并打开所在文件夹
- 网页版：单文件 web.html（Leaflet），可直接本地打开或部署

## 数据源清单（全部服务端实测，2026-11）

| 数据源 | 类型 | 是否免Key | 时间序列 | 最大缩放 |
|---|---|---|---|---|
| Esri 世界影像 | XYZ | 免Key | - | 19 |
| OpenStreetMap（de 镜像） | XYZ | 免Key | - | 19 |
| CartoDB 浅色/深色矢量 | XYZ | 免Key | - | 18 |
| Sentinel-2 无云镶嵌 2020 | XYZ(WMTS) | 免Key | - | 18 |
| NASA Blue Marble | WMS | 免Key | - | 8 |
| MODIS Terra/Aqua 真彩色 | WMS | 免Key | 每日 | 9 |
| VIIRS SNPP 真彩色 | WMS | 免Key | 每日 | 9 |
| Landsat 8/9（HLS L30） | WMS | 免Key | 按日期 | 13 |
| Sentinel-2（HLS S30） | WMS | 免Key | 按日期 | 13 |
| 天地图影像 img_w | WMTS | 需 tk | - | 18 |
| Planet NICFI 月度镶嵌 | XYZ | 需 API Key | - | 18 |
| 中科星图 / 长光卫星 / 资源卫星中心 / Copernicus | 数据门户 | 需注册 | - | - |

说明：tile.openstreetmap.org 在国内被限流，改用 OSM.de 镜像；CartoDB 使用 b/c/d 子域名；NASA GIBS 的 WMTS 低级别会黑屏，已改用 WMS 端点并支持 TIME 参数——以上均已实测跑通。

## 快速开始

### 桌面版（Windows，免安装）

下载 `HDMapDownloader.exe` 或 Release 中的 `HDMapDownloader_v3.0.0.zip`，解压后双击运行。瓦片缓存写入 exe 同目录的 `hdmap_cache/`，导出 ZIP 写入 `exports/`。

### 网页版

浏览器打开 `web.html` 即可使用（框选、时间范围、密钥输入、ZIP 下载全部在浏览器本地完成）。

### 从源码构建

```bash
cargo build --release
# 产物：target/release/hdmapdl.exe
```

### Web 服务部署（ModelScope / 自托管）

```bash
pip install -r requirements.txt
python modelscope_app.py
# 访问 http://localhost:5000
```

## 使用流程

1. 左侧「数据源」选择地图（默认 Esri 世界影像）
2. 需Key源（天地图/Planet）在「密钥」填入 tk / API Key；时间序列源在「影像时间」填入日期
3. 点击「开启框选模式」，在中央地图上拖出矩形选区
4. 设置下载缩放范围，点「开始下载」，进度条与日志实时显示
5. 完成后点「导出已下载瓦片为 ZIP」

## 打赏与联系

软件内「自愿打赏」可查看收款码，请作者喝一杯咖啡（纯属自愿，功能不受任何限制）。
联系邮箱：xingguangyan0703@gmail.com

## 许可证

MIT License。各数据源版权归其所有者，使用时请遵守对应服务条款（页面与软件内均有标注）。

## 链接

- GitHub: https://github.com/xingguangYan/HDMapDownloader
- GitCode: https://gitcode.com/qq_31988139/HDMapDownloader
- ModelScope: https://modelscope.cn/models/xg346049806/HDMapDownloader
