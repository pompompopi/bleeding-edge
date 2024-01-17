FROM alpine as builder
RUN apk --no-cache add rustup build-base
RUN addgroup -S -g 1000 builder
RUN adduser -D -S -u 1000 -G builder builder

USER builder:builder
WORKDIR /home/builder/app/
ADD --chown=builder:builder . /home/builder/app/
RUN rustup-init --profile default --target x86_64-unknown-linux-musl --default-toolchain nightly -y
RUN --mount=type=cache,target=/home/builder/app/target,uid=1000,gid=1000 env PATH=$PATH:/home/builder/.cargo/bin/ cargo build --release; cp /home/builder/app/target/release/bleeding-edge /home/builder/app/build-output

FROM eclipse-temurin:21-alpine
RUN addgroup -S -g 1000 user
RUN adduser -D -S -u 1000 -G user user

WORKDIR /app/
COPY --from=builder /home/builder/app/build-output /app/bleeding-edge
RUN chmod a+x /app/bleeding-edge

USER user:user
WORKDIR /home/user/
ENTRYPOINT [ "/app/bleeding-edge" ]