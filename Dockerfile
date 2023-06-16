FROM rust:1.70.0


WORKDIR /app

RUN apt-get update && apt-get install -y \
	lld \
	clang

COPY . .

RUN cargo build --release 

ENTRYPOINT ["./target/relase/zero2prod"]
