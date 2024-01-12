## certbot

基于certbot，采用 腾讯云 dns

```shell
docker run -it --rm --name certbot  -v "/data/cert/l:/etc/letsencrypt" -v "/data/cert/ll:/etc/lib/letsencrypt" -v "/data/cert/lw:/data/letsencrypt" -v "/data/public/cert:/data/cert" -v "/data/cert/lo:/var/log/letsencrypt/" -v "/data/cert/la:/etc/letsencrypt/archive" -v "/data/cert/lk:/etc/letsencrypt/keys" ghcr.io/inkroom/certbot certonly  -d domain --cert-name name -m email --manual --preferred-challenges dns --agree-tos --manual-auth-hook "bash /root/boot.sh secretId secretKey"
```

证书位于 `/data/cert/la/`

目前只用于 单个 泛域名证书申请，多个证书未测试

