ARG PYTHON_VERSION=3.14
FROM python:${PYTHON_VERSION}-alpine3.23

# Native musl toolchain, plus build requirements for spaCy's Python dependencies
# when a musllinux wheel is unavailable for a selected CPython ABI.
RUN apk add --no-cache bash build-base curl git linux-headers \
    cmake ninja pkgconf gfortran openblas-dev lapack-dev

ENV RUSTUP_HOME=/opt/rustup CARGO_HOME=/opt/cargo
ENV PATH="/opt/cargo/bin:${PATH}"
RUN curl --proto '=https' --tlsv1.2 -fsSL https://sh.rustup.rs -o /tmp/rustup-init.sh \
    && sh /tmp/rustup-init.sh -y --profile minimal --default-toolchain stable \
    && rm /tmp/rustup-init.sh

# Shared CPython and Python extension modules require dynamic musl linking.
ENV CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_RUSTFLAGS="-C target-feature=-crt-static"
ENV PYO3_PYTHON=/usr/local/bin/python
WORKDIR /work
