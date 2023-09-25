FROM dart:latest AS build
ENV PUB_HOSTED_URL=https://pub.flutter-io.cn
ENV FLUTTER_STORAGE_BASE_URL=https://storage.flutter-io.cn
COPY . /data
WORKDIR /data
RUN dart pub get && dart compile exe bin/main.dart -o packer && chmod +x ./packer 
RUN mkdir -p /data/so/lib/x86_64-linux-gnu && cd /data/so/ && mkdir lib64 \
   && cp /lib64/ld-linux-x86-64.so.2 lib64 \
   && cp /lib/x86_64-linux-gnu/libdl.so.2 lib/x86_64-linux-gnu/ \
   && cp /lib/x86_64-linux-gnu/libpthread.so.0 lib/x86_64-linux-gnu/ \
   && cp /lib/x86_64-linux-gnu/libm.so.6 lib/x86_64-linux-gnu/ \
   && cp /lib/x86_64-linux-gnu/libc.so.6 lib/x86_64-linux-gnu/ \
   && cp ../packer ./



FROM scratch
COPY --from=build /data/so/ /
WORKDIR /
VOLUME /data/out
ENTRYPOINT ["/packer","-d","/data/out"]

## docker run -it --rm -v ./out:/data/out  packer -u [url]

