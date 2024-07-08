FROM rust:latest
WORKDIR /workspaces/rustcracker

RUN rustup component add rustfmt
RUN rustup component add clippy

CMD ["bash"]