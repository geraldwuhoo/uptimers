# build shoutrrr library
FROM docker.io/library/golang:1.27.0-trixie AS lib
WORKDIR /usr/src/app

COPY go/go.mod go/go.sum ./
RUN go mod download && go mod verify

COPY go/*.go .
RUN CGO_ENABLED=1 go build -buildmode=c-archive -trimpath -ldflags '-s -w' -o libshoutrrr.a shoutrrr.go

# chef
FROM docker.io/library/rust:1.98.0-trixie AS chef
RUN cargo install cargo-chef
WORKDIR /usr/src

# planner
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Builder
FROM chef AS builder
COPY --from=planner /usr/src/recipe.json recipe.json
RUN cargo chef cook --release --target x86_64-unknown-linux-gnu --recipe-path recipe.json

COPY . .
# Reuse the archive from the `lib` stage so this image needs no Go toolchain;
# build.rs builds it itself when SHOUTRRR_LIB_DIR is unset.
COPY --from=lib /usr/src/app/libshoutrrr.a ./go/
ENV SHOUTRRR_LIB_DIR=/usr/src/go
RUN cargo build --release --target x86_64-unknown-linux-gnu --bin uptimers

# Clean image
FROM gcr.io/distroless/cc-debian13@sha256:8ad216d570d12730747a98573b6cedbad7c55a5ffd5452014a501902b5877d06
COPY --from=builder /usr/src/target/x86_64-unknown-linux-gnu/release/uptimers /usr/bin/uptimers
ENTRYPOINT ["uptimers"]
