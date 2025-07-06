# novel

爬取[轻小说文库](https://www.wenku8.net)小说，并上传到[cos](https://console.cloud.tencent.com/cos/bucket)


## 使用方法

采用selenium方案，并通过vnc技术附带虚拟桌面，所以最好再装一个vnc viewer

由于有cf防护，所以第一次使用或者重新部署的时候，建议手动打开一下网页，避免可能出现的人机验证，同时容器最好是常驻不关闭



部署
- docker build . -t novel -f Dockerfile
- docker run -itd -e COS_REGION= -e COS_BUCKET= -e COS_APP_SECERT= -e COS_APP_ID= -p 5901:5901  -v epub-out:/app/out/ -v epub-mozilla:/root/.mozilla -v epub-temp:/app/temp --name novel novel 
- 通过vnc连接 5901 端口，打开网页看下效果
- docker exec -it -e DISPLAY=":1" novel exec [网址]

输出文件在 /app/out 目录下
缓存文件在 /app/temp 目录下