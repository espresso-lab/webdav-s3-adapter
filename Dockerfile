FROM clux/muslrust:stable AS chef
USER root
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
ARG TARGETARCH
COPY --from=planner /app/recipe.json recipe.json
COPY platform.sh .
RUN chmod +x platform.sh
RUN ./platform.sh
RUN curl -sSL $(curl -s https://api.github.com/repos/upx/upx/releases/latest \
    | grep browser_download_url | grep $TARGETARCH | cut -d '"' -f 4) -o upx.tar.xz
RUN tar -xf upx.tar.xz \
    && cd upx-*-${TARGETARCH}_linux \
    && mv upx /bin/upx
RUN cargo chef cook --release --target $(cat /.platform) --recipe-path recipe.json
COPY . .
RUN cargo build --release --target $(cat /.platform) --bin webdav-s3-adapter
RUN mv ./target/$(cat /.platform)/release/webdav-s3-adapter ./webdav-s3-adapter
RUN upx --best --lzma ./webdav-s3-adapter

FROM scratch AS runtime
COPY --from=builder /app/webdav-s3-adapter /app
EXPOSE 3000
CMD ["/app"]