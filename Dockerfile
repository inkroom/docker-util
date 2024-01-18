FROM golang:1.21.6-alpine AS build
ADD . /go
WORKDIR /go/cla
#ADD ./clash.tar.gz /go/cla/clash
#RUN go get -u  github.com/kr328/clash
RUN apk add git && cd /go/cla/ && rm -rf clash && git clone https://github.com/mokitoo/kr328-clash clash  && go env -w GO111MODULE=on && go env -w GOPROXY=https://goproxy.cn,direct && go get && go build -ldflags="-s -w" main.go && chmod +x main


FROM alpine
COPY --from=build /go/cla/main /clash/
WORKDIR /clash
ENTRYPOINT ["/clash/main","out"]

# 使用样例 docker run -it --rm -v /clashConfig:/clash/out clash:config url
# 在 /clashConfig下有个config.yaml，这就是生成的文件

