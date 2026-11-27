# v3.0.0 更新说明

## 全部问题修复（对比 v2）

1. 新增「框选模式」：开启后在地图上拖出矩形即可选择下载区域，带实时经纬度标注
2. OpenStreetMap 无法接入 → 官方源国内限流，改用 tile.openstreetmap.de 镜像，实测连通
3. NASA Blue Marble 黑屏 → GIBS WMTS 低级别覆盖不全，改用 WMS 端点，实测连通
4. CartoDB 需要 API Key → 使用 basemaps.cartocdn.com b/c/d 子域名，免Key可用
5. 天地图需要个人 tk → 新增「密钥」输入框，填入即用（省去改 URL）
6. 长光卫星（吉林一号）、中科星图（星图地球）→ 已接入为「数据门户」，一键跳转注册页
7. 新增可选「影像时间」：MODIS / VIIRS / Landsat 8/9 / Sentinel-2 按日期加载（GIBS WMS TIME）
8. 底部已加入联系方式：xingguangyan0703@gmail.com

## 数据源（实测可用，2026-11）

免Key：Esri 世界影像、OSM(de 镜像)、CartoDB 浅/深、EOX Sentinel-2 无云镶嵌、NASA Blue Marble、MODIS Terra/Aqua、VIIRS、Landsat HLS L30、Sentinel-2 HLS S30
需Key：天地图 img_w、Planet NICFI
门户：中科星图、长光吉林一号、中国资源卫星中心、Copernicus

## 桌面版

- HDMapDownloader.exe：Windows 免安装，Rust + egui 重写，本地并发下载、断点续传（瓦片落盘缓存）、ZIP 导出
 - 滚轮缩放、拖拽平移、框选、坐标实时显示
 - 瓦片后台异步加载，下载进度条与日志

## 网页版

- web.html：单文件，Leaflet + JSZip，本地打开即可使用，全部功能在浏览器内完成

## 打包文件

HDMapDownloader_v3.0.0.zip：exe + web.html + 数据源清单 + README
