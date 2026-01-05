# Stage 1: Build Frontend
FROM node:20-alpine AS front-builder
WORKDIR /app/front
COPY front/package.json front/package-lock.json ./
RUN npm ci
COPY front/ .
RUN npm run build

# Stage 2: Build Backend
FROM rust:1.83-alpine AS backend-builder
WORKDIR /app
RUN apk add --no-cache musl-dev
COPY Cargo.toml Cargo.lock ./
# Create dummy main.rs to build dependencies first (caching)
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

# Copy actual source and frontend assets
COPY src/ src/
# Note: implementation of embedding will happen later, but we prepare the stage
COPY --from=front-builder /app/front/dist front/dist

# Build the actual binary
# We touch main.rs to force rebuild
RUN touch src/main.rs
RUN cargo build --release

# Stage 3: Runtime
FROM alpine:3.20
WORKDIR /app
COPY --from=backend-builder /app/target/release/bus9 /app/bus9
# Expose default port (to be implemented)
EXPOSE 8080
CMD ["./bus9"]
