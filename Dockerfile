FROM debian:stable-20231218-slim as dark
RUN export DEBIAN_FRONTEND=noninteractive && sed -i 's/deb.debian.org/mirrors.ustc.edu.cn/g' /etc/apt/sources.list.d/debian.sources && apt update -y && apt install -y gcc make git 
RUN git clone https://mirror.ghproxy.com/https://github.com/emikulic/darkhttpd
RUN cd darkhttpd && make

FROM dark as aria
RUN export DEBIAN_FRONTEND=noninteractive && apt install -y nodejs npm && npm config set registry https://registry.npmmirror.com/ 
RUN git clone https://mirror.ghproxy.com/https://github.com/mayswind/AriaNg.git -b 1d253b4
COPY constants.js  AriaNg/src/scripts/config/constants.js
RUN cd AriaNg && npm i && npm i -g gulp-cli && gulp clean build

FROM p3terx/ariang
COPY --from=aria /AriaNg/dist /AriaNg
# ENTRYPOINT ["/httpd", "/aria"]
