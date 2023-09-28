FROM python:alpine3.16

RUN mkdir -p /light/data

WORKDIR /light
RUN sed -i 's/dl-cdn.alpinelinux.org/mirrors.ustc.edu.cn/g' /etc/apk/repositories
RUN apk add nodejs && apk add npm && npm --registry https://registry.npm.taobao.org install node-mailer

COPY requirement.txt /light/requirement.txt
RUN cd /light && pip3 install -r requirement.txt -i https://mirrors.aliyun.com/pypi/simple



VOLUME /light/data
COPY r.py /light/
ENTRYPOINT ["python3","r.py"]

## 执行命令 docker run -it --rm -v "/data/docker/书籍id:/light/data" ili-light id号 第二个参数随便传，可以实现不分卷输出

## 可以使用 shell 函数 缩短命令
