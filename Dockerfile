FROM rust:1.98.1-bookworm
ARG TARGETARCH

RUN apt-get update \
    && apt-get install -y --no-install-recommends cmake gawk pkg-config python3 \
    && rm -rf /var/lib/apt/lists/*
RUN rustup target add armv7-unknown-linux-gnueabi
RUN set -eu; \
    case "$TARGETARCH" in \
        arm64) sdk_arch=aarch64; sdk_sha=3a9330f3789f8008ece7c8a48f58c3a71d0307a9a1f81896db938b785524c3ce ;; \
        amd64) sdk_arch=x86_64; sdk_sha=2230a6351e5cce6fcb9d4fdcb4697335f223995401ef969c81744aaeb14c5344 ;; \
        *) echo "Unsupported SDK host architecture: $TARGETARCH" >&2; exit 1 ;; \
    esac; \
    curl --fail --location --retry 2 \
        "https://github.com/openlgtv/buildroot-nc4/releases/download/2026.08-webos/arm-webos-linux-gnueabi_sdk-buildroot-$sdk_arch.tar.gz" \
        --output /tmp/webos-sdk.tar.gz; \
    echo "$sdk_sha  /tmp/webos-sdk.tar.gz" | sha256sum -c -; \
    mkdir /opt/webos-sdk; \
    tar -xzf /tmp/webos-sdk.tar.gz --strip-components=1 -C /opt/webos-sdk; \
    rm /tmp/webos-sdk.tar.gz; \
    /opt/webos-sdk/relocate-sdk.sh

ENV WEBOS_SDK=/opt/webos-sdk
ENV CARGO_TARGET_ARMV7_UNKNOWN_LINUX_GNUEABI_LINKER=/opt/webos-sdk/bin/arm-webos-linux-gnueabi-gcc
WORKDIR /work
CMD ["cargo", "build", "--locked", "--release", "--target", "armv7-unknown-linux-gnueabi", "--example", "counter"]
