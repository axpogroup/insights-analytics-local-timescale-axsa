# Use an official Rust image as the base
FROM --platform=linux/arm64 rust:latest as rust_build

# Set environment variables for non-interactive installation
ENV DEBIAN_FRONTEND=noninteractive

# Set the working directory inside the container
WORKDIR /workspace

# Install necessary system dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    git \
    clang \
    libclang-dev \
    build-essential \
    libreadline-dev \
    zlib1g-dev \
    flex \
    bison \
    libxml2-dev \
    libxslt-dev \
    libssl-dev \
    libxml2-utils \
    xsltproc \
    ccache \
    pkg-config && \
    apt-get clean && rm -rf /var/lib/apt/lists/*

# Install Rustfmt (if not already installed)
RUN rustup component add rustfmt
# RUN cargo install cargo-edit --version 0.12.0
# Install cargo-pgrx (current fix due to https://github.com/pgcentralfoundation/pgrx/issues/2009)
RUN cargo install cargo-pgrx --version 0.13.0 --locked
# Initialize pgrx
RUN cargo pgrx init

# Create a new extension
RUN cargo pgrx new my_extension
# Copy the project files into the container

COPY rust_code/Cargo.toml my_extension/Cargo.toml
COPY rust_code/src/main.rs my_extension/src/lib.rs

RUN cd my_extension && cargo pgrx install


FROM --platform=linux/arm64 timescale/timescaledb-ha:pg15 AS timescale_db
RUN mkdir -p /usr/share/postgresql/15/extension /usr/lib/postgresql/15/lib
COPY --from=rust_build /usr/share/postgresql/15/extension/my_extension.control /usr/share/postgresql/15/extension/
COPY --from=rust_build /my_extension/ /my_extension
COPY --from=rust_build /usr/share/postgresql/15/extension/my_extension--0.0.0.sql /usr/share/postgresql/15/extension/
COPY --from=rust_build /usr/lib/postgresql/15/lib/my_extension.so /usr/lib/postgresql/15/lib/