# Use a specific Rust version for consistency
FROM rustlang/rust:nightly-bookworm AS builder

# Install additional tools needed for building
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    postgresql-client \
    && rm -rf /var/lib/apt/lists/*

# Set the working directory
WORKDIR /app

# Copy the Cargo.toml and Cargo.lock files
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to allow cargo to download dependencies
RUN mkdir src && echo "fn main() {println!(\"Hello, world!\");}" > src/main.rs

# Build the dependencies
RUN cargo build --release
RUN rm -rf src

# Copy the actual source code
COPY src ./src

# Build the application
# Set environment variables for Diesel
ENV DATABASE_URL=postgresql://user:password@localhost/database
RUN touch src/main.rs && cargo build --release

# Create a minimal runtime image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libpq5 \
    postgresql-client \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Create a non-root user
RUN groupadd -r appuser && useradd -r -g appuser appuser

# Set the working directory
WORKDIR /app

# Create the feeds output directory and set proper permissions
RUN mkdir -p /app/avito_feeds_output && chown -R appuser:appuser /app

# Copy the compiled binary from the builder stage
COPY --from=builder /app/target/release/crawler /usr/local/bin/app

# Change ownership of the app directory to the non-root user
RUN chown -R appuser:appuser /app

# Switch to the non-root user
USER appuser

# Command to run the application
CMD ["/bin/sh", "-c", "mkdir -p /app/avito_feeds_output && chown appuser:appuser /app/avito_feeds_output && exec app"]