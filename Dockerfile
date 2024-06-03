ARG CROSS # aarch64 (default) or x86_64
FROM messense/rust-musl-cross:${CROSS:-aarch64}-musl as chef
RUN rustup update && \
    rustup target add ${CROSS:-aarch64}-unknown-linux-musl
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
ARG TARGETARCH
COPY --from=planner /app/recipe.json recipe.json
RUN curl -sSL $(curl -s https://api.github.com/repos/upx/upx/releases/latest \
    | grep browser_download_url | grep $TARGETARCH | cut -d '"' -f 4) -o upx.tar.xz
RUN tar -xf upx.tar.xz \
    && cd upx-*-${TARGETARCH}_linux \
    && mv upx /bin/upx
RUN cargo chef cook --release --target ${CROSS:-aarch64}-unknown-linux-musl --recipe-path recipe.json
COPY . .
RUN cargo build --release --target ${CROSS:-aarch64}-unknown-linux-musl --bin webdav-s3-adapter
RUN mv ./target/${CROSS:-aarch64}-unknown-linux-musl/release/webdav-s3-adapter ./webdav-s3-adapter
RUN upx --best --lzma ./webdav-s3-adapter

FROM scratch AS runtime
COPY --from=builder /app/webdav-s3-adapter /app
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
EXPOSE 3000
CMD ["/app"]
