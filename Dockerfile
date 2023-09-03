FROM alpine as builder
RUN apk --no-cache add rustup build-base
RUN addgroup -S -g 1000 builder
RUN adduser -D -S -G builder builder

USER builder:builder
WORKDIR /home/builder/app/
RUN rustup-init --profile default --target x86_64-unknown-linux-musl --default-toolchain nightly -y
ADD --chown=builder:builder . /home/builder/app/
RUN rm -rf target/
RUN env PATH=$PATH:/home/builder/.cargo/bin/ cargo build --release

FROM eclipse-temurin:20-alpine
RUN addgroup -S -g 1000 user
RUN adduser -D -S -u 1000 -G user user

WORKDIR /app/
COPY --from=builder --chown=user:user /home/builder/app/target/release/bleeding-edge /app/
RUN chmod a+x /app/bleeding-edge

USER user:user
WORKDIR /home/user/
ENTRYPOINT [ "/app/bleeding-edge" ]