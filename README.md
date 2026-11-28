# HD 高清地图下载器 v4.0

多源高清瓦片下载 · 行政矢量选区 · 公开影像(Sentinel/Landsat)波段检索下载 · AI 自然语言操作 · 中英双语

- 网页版: https://xingguangyan.github.io/HDMapDownloader/ (打开 `web.html` 或 `index.html`)
- 桌面版: 双击 `HDMapDownloader.exe` (Windows, Rust + egui)
- 联系: xingguangyan0703@gmail.com

## v4.0 功能

1. **多源高清瓦片**: Esri / 谷歌卫星 / 高德卫星影像 / OpenStreetMap / CartoDB / OpenTopoMap / EOX Sentinel-2 无云镶嵌 / NASA GIBS Blue Marble / MODIS / VIIRS / HLS(Landsat, Sentinel-2) / 天地图(填 tk) / Planet(填 Key) / 自定义源
2. **选区工具**: 矩形框选 / 多边形(≥3 点双击结束) / 点选(±0.005 度)
3. **行政矢量**: DataV 行政区划(国家/省/市/县)，地名或 6 位 adcode 定位，点击地图要素设为下载范围，矢量可导出 GeoJSON
4. **公开影像检索 (STAC)**: Sentinel-2 L2A / Landsat 8/9 C2 L2 / Sentinel-1 GRD / Copernicus DEM 90m; 检索前先提示选择时间范围与云量上限; 场景列表含日期/云量/平台/波段; Sentinel-2 波段(COG) 可直接勾选下载，其它(S3 requester-pays)给出路径
5. **AI 助手**: 右栏填入 OpenAI 兼容 Base URL/Key/模型，自然语言(如「帮我下载杭州市 2024-06 云量≤10% 的 Sentinel-2 B02/B03」)自动选址/检索/下载，Key 仅存本机
6. **登录后接入**: 中科星图/长光等平台登录后，粘贴其瓦片 URL 模板(含 {z}/{x}/{y}，{key} 占位)即接入地图
7. **中英双语**: 网页与桌面端一键 中/EN 切换
8. **数据门户入口**: 中科星图/长光吉林一号/资源卫星中心/地理空间数据云/Copernicus/USGS(注册下载原始数据)

## 说明

- 免 Key 数据源已实测连通; 需 Key 源请在界面填入 tk / API Key 后使用
- 数据所有权归各服务商，请遵守对应许可条款(界面内已标注)
- 桌面端同一套源与 GIBS WMS 时间序列，行政矢量/STAC/AI 与网页版一致
- 打赏: 软件底部「请作者喝杯咖啡」