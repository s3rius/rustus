FROM alpine:3.15.0

ARG app_version

ADD "https://github.com/s3rius/rustus/releases/download/${app_version}/rustus-${app_version}-linux-musl-x86_64.tar.gz" "."

RUN tar xvf *.tar.gz
RUN rm *.tar.gz
RUN mv rustus /bin
WORKDIR /app

ENTRYPOINT ["/bin/rustus"]