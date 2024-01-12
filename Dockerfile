FROM certbot/certbot
RUN pip install -i https://mirrors.aliyun.com/pypi/simple tccli
COPY boot.sh /root/boot.sh
RUN chmod +x /root/boot.sh
