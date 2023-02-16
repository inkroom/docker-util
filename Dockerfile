FROM golang:1.20.0-alpine3.16 AS build
ADD . /go
WORKDIR /go/cla
RUN apk add git && cd /go/cla/ && rm -rf clash && git clone https://github.com/Kr328/clash.git clash && go env -w GO111MODULE=on && go env -w GOPROXY=https://goproxy.cn,direct && go get && go build -ldflags="-s -w" main.go && chmod +x main


FROM alpine
COPY --from=build /go/cla/main /clash/
WORKDIR /clash
ENTRYPOINT ["/clash/main","out"]

# 使用样例 docker run -it --rm -v /clashConfig:/clash/out clash:config url
# 在 /clashConfig下有个config.yaml，这就是生成的文件

