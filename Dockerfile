# Stage 1: Build the game statically
FROM rust:1.97-slim AS builder

# Install musl tools and target
RUN apt-get update && apt-get install -y \
    musl-tools \
    && rm -rf /var/lib/apt/lists/*
RUN rustup target add x86_64-unknown-linux-musl

WORKDIR /usr/src/sshnake

# Cache dependencies by doing a dummy build
COPY Cargo.toml ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release --target x86_64-unknown-linux-musl
RUN rm -rf src

# Copy source code and build the real binary
COPY src ./src
RUN touch src/main.rs
RUN cargo build --release --target x86_64-unknown-linux-musl

# Stage 2: Runtime SSH Server
FROM debian:bookworm-slim

# Install SSH daemon
RUN apt-get update && apt-get install -y \
    openssh-server \
    && rm -rf /var/lib/apt/lists/*

# Required directory for sshd to run
RUN mkdir /var/run/sshd

# Copy compiled static binary from builder
COPY --from=builder /usr/src/sshnake/target/x86_64-unknown-linux-musl/release/sshnake /usr/local/bin/sshnake

# Add the sshnake TUI to allowable shells
RUN echo "/usr/local/bin/sshnake" >> /etc/shells

# Create user 'snake' with sshnake as login shell
RUN useradd -m -s /usr/local/bin/sshnake snake

# Enable passwordless login for user 'snake'
RUN passwd -d snake

# Configure SSH daemon settings:
# - Listen on port 2222 (allows running rootless or avoiding host port 22 conflicts)
# - Enable passwordless logins for the snake user
# - Force command execution of sshnake (prevents shell escape attacks)
RUN echo "Port 2222" >> /etc/ssh/sshd_config \
    && echo "PermitEmptyPasswords yes" >> /etc/ssh/sshd_config \
    && echo "PasswordAuthentication yes" >> /etc/ssh/sshd_config \
    && echo "PermitRootLogin no" >> /etc/ssh/sshd_config \
    && echo "Match User snake" >> /etc/ssh/sshd_config \
    && echo "    ForceCommand /usr/local/bin/sshnake" >> /etc/ssh/sshd_config

# Generate host keys
RUN ssh-keygen -A

# Expose port 2222
EXPOSE 2222

# Start SSH daemon in foreground
CMD ["/usr/sbin/sshd", "-D", "-e"]
