# build shoutrrr library
FROM docker.io/library/golang:1.25.4-trixie AS lib
WORKDIR /usr/src/app

COPY go/go.mod go/go.sum ./
RUN go mod download && go mod verify

COPY go/*.go .
RUN CGO_ENABLED=1 go build -v -ldflags '-s -w -linkmode external -extldflags "static"' -trimpath -buildmode=c-archive -o libshoutrrr.a shoutrrr.go

# chef
FROM docker.io/library/rust:1.91.1-trixie AS chef
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
FROM gcr.io/distroless/cc-debian13@sha256:54a30fb33d77e2d981f37fb34cfd2bcc124bf029ed2d73764b5b68951acabf85
COPY --from=builder /usr/src/target/x86_64-unknown-linux-gnu/release/uptimers /usr/bin/uptimers
ENTRYPOINT ["uptimers"]
