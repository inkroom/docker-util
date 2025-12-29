FROM inkbox/rust:1.88.0 as build
RUN cd / && wget https://github.com/mozilla/geckodriver/releases/download/v0.35.0/geckodriver-v0.35.0-linux32.tar.gz && tar -xf geckodriver-v0.35.0-linux32.tar.gz && rm -rf geckodriver-v0.35.0-linux32.tar.gz
RUN  cd / && cargo new app
COPY Cargo.toml /app/
WORKDIR /app
RUN cargo build --release && rm -rf target/release/deps/wenku-*
COPY . /app
RUN cargo build --release

FROM fullaxx/ubuntu-desktop

RUN dpkgArch="$(dpkg --print-architecture)"; \
    case "${dpkgArch##*-}" in \
        amd64) sed -i "s@http://.*archive.ubuntu.com@http://mirrors.huaweicloud.com@g" /etc/apt/sources.list && sed -i "s@http://.*security.ubuntu.com@http://mirrors.huaweicloud.com@g" /etc/apt/sources.list ;; \
        arm64) sed -i "s@http://ports.ubuntu.com@https://mirrors.huaweicloud.com@g" /etc/apt/sources.list  ;; \
        *) echo >&2 "unsupported architecture: ${dpkgArch}" ;; \
    esac; apt update -y
RUN apt install -y language-pack-zh-hans && locale-gen zh_CN.UTF-8
ENV LANG=zh_CN.UTF-8
ENV LC_ALL=zh_CN.UTF-8
ENV TZ=Asia/Shanghai
RUN apt install -y git zsh vim fonts-wqy-zenhei
RUN echo "LC_ALL=zh_CN.UTF-8" >> /etc/environment && \
    echo "LANG=zh_CN.UTF-8" > /etc/locale.conf && \
    echo "zh_CN.UTF-8 UTF-8" >> /etc/locale.gen && \
    echo "export LC_ALL=zh_CN.UTF-8" >> /etc/profile && \
    echo "export LANG=zh_CN.UTF-8" >> /etc/profile && \
    echo "export LANGUAGE=zh_CN:en_US" >> /etc/profile 

RUN sh /app/scripts/prepare_firefox_ppa.sh && apt update -y && apt install -y firefox build-essential cmake perl pkg-config libclang-dev musl-tools 
COPY --from=build /app/target/release/wenku /app/exec
COPY --from=build /geckodriver /app/
WORKDIR /app/
ENV PATH ${PATH}:/app/
CMD /app/app.sh

