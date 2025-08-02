FROM rust:1.88 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates
COPY --from=builder /app/target/release/line-dolphin /usr/local/bin/
EXPOSE 3000
CMD ["line-dolphin"] 