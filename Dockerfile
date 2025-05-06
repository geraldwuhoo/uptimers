# build shoutrrr library
FROM docker.io/library/golang:1.24.3-bookworm AS lib
WORKDIR /usr/src/app

COPY go/go.mod go/go.sum ./
RUN go mod download && go mod verify

COPY go/*.go .
RUN CGO_ENABLED=1 go build -v -ldflags '-s -w -linkmode external -extldflags "static"' -trimpath -buildmode=c-archive -o libshoutrrr.a shoutrrr.go

# chef
FROM docker.io/library/rust:1.84.1 AS chef
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
COPY --from=lib /usr/src/app/libshoutrrr.a /usr/src/app/libshoutrrr.h ./go/
RUN cargo build --release --target x86_64-unknown-linux-gnu --bin uptimers

# Clean image
FROM gcr.io/distroless/cc-debian12@sha256:de4789799c7c27e3f172f81313adc30f100d632e53fac755a3965f799b685860
COPY --from=builder /usr/src/target/x86_64-unknown-linux-gnu/release/uptimers /usr/bin/uptimers
ENTRYPOINT ["uptimers"]
