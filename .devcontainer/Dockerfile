# See here for image contents: https://github.com/microsoft/vscode-dev-containers/tree/v0.231.3/containers/rust/.devcontainer/base.Dockerfile

ARG VARIANT="bullseye"
FROM mcr.microsoft.com/vscode/devcontainers/rust:0-${VARIANT}

RUN sudo -H -u vscode bash -lic 'rustup target install thumbv6m-none-eabi && cargo install flip-link'