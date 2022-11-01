FROM alpine:3.15.0 as base

ARG app_version

ADD "https://github.com/s3rius/rustus/releases/download/${app_version}/rustus-${app_version}-linux-musl-x86_64.tar.gz" "."

RUN tar xvf *.tar.gz
RUN rm *.tar.gz
RUN mv rustus /bin

ENTRYPOINT ["/bin/rustus"]

FROM base as rootless

RUN adduser -u 1000 --disabled-password rustus
WORKDIR /home/rustus
USER rustus