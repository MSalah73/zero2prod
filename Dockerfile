# Compute recipe file 
FROM lukemathwalker/cargo-chef:latest-rust-1.70.0 as chef

WORKDIR /app

RUN apt-get update && apt-get install -y \
	lld \
	clang

# Compute lcok-like files for caching
FROM chef as planner

COPY . . 
# Compute a lock-like file for the project
RUN cargo chef prepare --recipe-path recipe.json

# Build UPX Stage
FROM debian:bullseye-slim as upx
#
RUN apt-get update && apt-get install -y build-essential curl cmake \
	&& mkdir -p /upx \
	&& curl -# -L https://github.com/upx/upx/releases/download/v4.0.1/upx-4.0.1-src.tar.xz | tar xJ --strip 1 -C /upx \
	&& make -C /upx build/release-gcc -j$(nproc) \
	&& cp -v /upx/build/release-gcc/upx /usr/bin/upx

# Caching and Building Stage 
FROM chef as builder

COPY --from=planner /app/recipe.json recipe.json

# Build our project dependencies, not out application 
RUN cargo chef cook --release --recipe-path recipe.json 
# Up to this point, if our dependency tree stays the same,
# all layer should be cached.
COPY . . 

ENV SQLX_OFFLINE true

# Build the application
RUN cargo build --release --bin zero2prod 

# Optimazize the applization binary for lower size
COPY --from=upx /usr/bin/upx /usr/bin/upx

RUN upx /app/target/release/zero2prod 

# Runtime Stage 
FROM debian:bullseye-slim as runtime

WORKDIR /app

# Install OpenSSL - it is a dynamically linked by some of our dependencies  
# Install certificates -- it is needed to verify TLS certificates when establishing HTTP connections
RUN apt-get update && apt-get install -y \
	--no-install-recommends openssl ca-certificates \
	# Clean up
	&& apt-get autoremove -y \
	&& apt-get clean -y \
	&& rm -rf /var/lib/apt/lists/*

# Copy the compiled binary from the builder environment
# to the runtime 
COPY --from=builder /app/target/release/zero2prod zero2prod

# We need the configuration file at runtime 
COPY configuration configuration

ENV APP_ENVIRONMENT production

ENTRYPOINT ["./zero2prod"]
