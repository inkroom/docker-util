FROM python:3.11.3-alpine as build
WORKDIR /app
RUN mkdir /plib && cd /plib && sed -i 's/dl-cdn.alpinelinux.org/mirrors.ustc.edu.cn/g' /etc/apk/repositories && apk add gcc g++ git jpeg jpeg-dev zlib zlib-dev && git clone https://github.com/lightnovel-center/linovelib2epub.git && cd linovelib2epub && git checkout d5885af
COPY modify/ /plib/linovelib2epub/
RUN cd /plib && cd linovelib2epub && python3 -m venv venv && chmod +x ./venv/bin/activate && ./venv/bin/activate && python3 -m pip install -r requirements.txt -i https://mirrors.aliyun.com/pypi/simple && python3 -m pip install -e . -i https://mirrors.aliyun.com/pypi/simple && pip3 install argparse -i https://mirrors.aliyun.com/pypi/simple
COPY lib.py /app/lib.py
RUN pip3 install pyinstaller -i https://mirrors.aliyun.com/pypi/simple && apk add binutils && pyinstaller lib.py && cd /app/dist/lib && cp -r /usr/local/lib/python3.11/site-packages/fake_useragent ./_internal/ && mkdir _internal/linovelib2epub && cp -r /plib/linovelib2epub/src/linovelib2epub/styles  _internal/linovelib2epub


FROM alpine
RUN sed -i 's/dl-cdn.alpinelinux.org/mirrors.ustc.edu.cn/g' /etc/apk/repositories && apk add tzdata \
    && cp /usr/share/zoneinfo/Asia/Shanghai /etc/localtime \
    && echo "Asia/Shanghai" > /etc/timezone
COPY --from=build /app/dist/lib /app
WORKDIR /app
ENTRYPOINT ["/app/lib"]

# -h 查看用法

# docker run -v $PWD/out:/app/data -v $PWD/temp:/app/temp -e uaa_token= -e uaa_cookie= -h



