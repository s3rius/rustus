---
title: Configuration
description: "How to configure Rusts"
---

Rustus is highly configurable. You can adjust it with CLI or you can use environment variables.

!!! info
    Information about hooks you can find on [Hooks page](../hooks).


## Configuring server

We use actix to run server.
You can configure on which `host` and `port` rustus is listening.
Also you can configure number of actix `workers` that handle connections.

`--max-body-size` is the max number of bytes that users can send in request body.

`--url` is a base URL for all tus requests.

`--workers` by default is equal to number of physical CPU cores. Edit it carefully.

`--cors` is a list of allowed hosts with wildcards separated by commas. By default all hosts are allowed.
You can define which hosts are allowed for your particular application.

For example if you add `--cors "*.staging.domain,*.prod.domain"`, it allows all origins
like `my.staging.domain` or `my.prod.domain`, but it will refuse to serve other origins.

Also you can disable access log for `/health` endpoint, by using `--disable-health-access-log`.

=== "CLI"

    ``` bash
    rustus --host "0.0.0.0" \
        --port 1081 \
        --workers 8 \
        --max-body-size 1000000 \
        --url "/files" \
        --log-level "INFO" \
        --cors "my.*.domain.com,your.*.domain.com" \
        --disable-health-access-log
    ```

=== "ENV"

    ``` bash
    export RUSTUS_SERVER_HOST="0.0.0.0"
    export RUSTUS_SERVER_PORT="1081"
    export RUSTUS_SERVER_WORKERS="8"
    export RUSTUS_MAX_BODY_SIZE="1000000"
    export RUSTUS_URL="/files"
    export RUSTUS_LOG_LEVEL="INFO"
    export RUSTUS_CORS="my.*.domain.com,your.*.domain.com"
    export RUSTUS_DISABLE_HEALTH_ACCESS_LOG="true"

    rustus
    ```


## Sentry integration

If you have sentry and want to see all erros in your sentry project,
please provide sentry-dsn to rustus.

=== "CLI"

    ``` bash
    rustus --sentry-dsn "https://user@sentry-instance.com/11" \
        --sentry-sample-rate 1.0
    ```

=== "ENV"

    ``` bash
    export RUSTUS_SENTRY_DSN="https://user@sentry-instance.com/11"
    export RUSTUS_SENTRY_SAMPLE_RATE="1.0"

    rustus
    ```


## Configuring storage

Storages are used to actually store your files. You can configure where you want
to store files. By default in uses `file-storage` and stores every upload on
your local file system.

Availabe storages:

* `file-storage`
* `hybrid-s3`

### File storage

File storage parameters:

* `--data-dir` - path to the directory where all files are stored;
* `--dir-structure` - pattern of a directory structure inside data dir;
* `--force-fsync` - calls fsync system call after every write to disk.
``
You can use variables within the pattern.

Available variables:

* `{year}` - current year;
* `{month}` - current month number from 1 to 12;
* `{day}` - current day number from 1 to 31;
* `{hour}` - hour number from 0 to 23;
* `{minute}` - minute number from 0 to 59;

For example if you use `files/{year}/{month}/{day}` as your dir-structure, rustus stores files like:

``` bash
$ tree data
data
└── files
    └── 2022
        └── 1
            └── 8
                ├── 0bd911d4054d41c6a3ad54be67ee3e66
                └── 5bc9c62384494c439e2a064b82a39cc6
```

=== "CLI"

    ``` bash
    rustus --force-fsync \
        --storage "file-storage" \
        --data-dir "./data/" \
        --dir-structure "{year}/{month}/{day}"
    ```

=== "ENV"

    ``` bash
    export RUSTUS_STORAGE="file-storage"
    export RUSTUS_DATA_DIR="./data/"
    export RUSTUS_DIR_STRUCTURE="{year}/{month}/{day}"
    export RUSTUS_FORCE_FSYNC="true"

    rustus
    ```

### Hybrid-S3 storage

This storage stores files locally and uploads resulting file on S3 when the upload is finished.
It has no restriction on chunk size and you can make chunks less than 5MB.

!!! Danger
    When choosing this storage you still need to have a
    connected shared directory between instances.

    This storage is not intended to be used for large files,
    since it uploads files to S3 during the last request.

Hybrid-S3 uses file-storage inside, so all parameters from file storage
also applied to it.

Parameters:

* `--dir-structure` - pattern of a directory structure locally and on s3;
* `--data-dir` - path to the local directory where all files are stored;
* `--force-fsync` - calls fsync system call after every write to disk in local storage;
* `--s3-url` -  s3 endpoint URL;
* `--s3-bucket` - name of a bucket to use;
* `--s3-region` - AWS region to use;
* `--s3-access-key` - S3 access key;
* `--s3-secret-key` - S3 secret key;
* `--s3-security-token` - s3 secrity token;
* `--s3-session-token` - S3 session token;
* `--s3-profile` - Name of the section from `~/.aws/credentials` file;
* `--s3-headers` - JSON object with additional header to every S3 request (Useful for setting ACLs);
* `--s3-force-path-style` - use path style URL. It appends bucket name at the end of the URL;

Required parameter are only `--s3-url` and `--s3-bucket`.

=== "CLI"

    ``` bash
    rustus --storage "hybrid-s3" \
        --s3-url "https://localhost:9000" \
        --s3-bucket "bucket" \
        --s3-region "eu-central1" \
        --s3-access-key "fJljHcXo07rqIOzh" \
        --s3-secret-key "6BJfBUL18nLiGmF5zKW0NKrdxQVxNYWB" \
        --s3-profile "my_profile" \
        --s3-security-token "token" \
        --s3-session-token "token" \
        --s3-force-path-style \
        --s3-headers '{"x-amz-acl": "public-read"}' \
        --force-fsync \
        --data-dir "./data/" \
        --dir-structure "{year}/{month}/{day}"
    ```

=== "ENV"

    ``` bash
    export RUSTUS_STORAGE="hybrid-s3"
    export RUSTUS_S3_URL="https://localhost:9000"
    export RUSTUS_S3_BUCKET="bucket"
    export RUSTUS_S3_REGION="eu-central1"
    export RUSTUS_S3_ACCESS_KEY="fJljHcXo07rqIOzh"
    export RUSTUS_S3_SECRET_KEY="6BJfBUL18nLiGmF5zKW0NKrdxQVxNYWB"
    export RUSTUS_S3_SECURITY_TOKEN="token"
    export RUSTUS_S3_SESSION_TOKEN="token"
    export RUSTUS_S3_PROFILE="my_profile"
    export RUSTUS_S3_HEADERS='{"x-amz-acl": "public-read"}'
    export RUSTUS_DATA_DIR="./data/"
    export RUSTUS_DIR_STRUCTURE="{year}/{month}/{day}"
    export RUSTUS_FORCE_FSYNC="true"

    rustus
    ```

## Configuring info storage

Info storages are used to store information
about file uploads. These storages **must** be persistent,
because every time chunk is uploaded rustus updates information
about upload. And when someone wants to download file, information
about it requested from storage to get actual path of an upload.

Available info storages:

* `file-info-storage` - stores information in files on disk;
* `redis-info-storage` - information is stored in Redis;
* `db-info-storage` - information is stored in database;

### File info storage

file info storage stores information in files on disk.
It's default info storage. Every download has it's own associated file.
All .info files stored in flat structure so it's the least preferable way of
storing information about uploads. But if you don't plan to have many uploads, it may fit well.

`--info-dir` - directory where all .info file will be stored (default is `./data`).


=== "CLI"

    ``` bash
    rustus --info-storage "file-info-storage" \
        --info-dir "./data"
    ```

=== "ENV"

    ``` bash
    export RUSTUS_INFO_STORAGE="file-info-storage"
    export RUSTUS_INFO_DIR="./data"

    rustus
    ```

### Redis info storage

Redis db is a good way to store information.

!!! note

    If you're using Redis as a cluster
    you must provide connection string for master Redis server.
    Since rustus need to have latest information and it writes a lot.

`--info-db-dsn` - connection string for your Redis database.
It's required if `redis-info-storage` is chosen.

=== "CLI"

    ``` bash
    rustus --info-storage "redis-info-storage" \
        --info-db-dsn "redis://localhost/0"
    ```

=== "ENV"

    ``` bash
    export RUSTUS_INFO_STORAGE="redis-info-storage"
    export RUSTUS_INFO_DB_DSN="redis://localhost"

    rustus
    ```


### DB info storage

Rustus can store information about upload in a database.

It's a good and reliable option. But Rustus can't work
with replicas, since it requires the most recent information
about uploads.

You can use `postgresql`, `mysql` or even `sqlite` schemas to
connect to database.

`--info-db-dsn` - connection string for your database.

=== "CLI"

    ``` bash
    rustus --info-storage "db-info-storage" \
        --info-db-dsn "postgresql://user:password@localhost/db"
    ```

=== "ENV"

    ``` bash
    export RUSTUS_INFO_STORAGE="redis-info-storage"
    export RUSTUS_INFO_DB_DSN="postgresql://user:password@localhost/db"

    rustus
    ```

## Configuring TUS

Since TUS protocol offers extensibility you can turn off some protocol extensions.

Available extensions:

* `getting` - Rustus specific extension that helps you download uploaded files with get request;
* `creation` - helps you to create files (It's like a core feature you better have this enabled);
* `termination` - allows you to delete uploads with DELETE request;
* `creation-with-upload` - allows you to write first bytes of a file while creating;
* `creation-defer-length` - allows you to create file without specifying file length;
* `concatenation` - allows you to concatenate finished partial uploads.
* `checksum` - allows you to verify checksum of every batch.

You can read more about extensions on [official web-site](https://tus.io/protocols/resumable-upload.html#protocol-extensions).

`--tus-extensions` - a list of enabled extensions.
`--remove-parts` - remove parts files after successful concatenation (disabled by default).

By default all extensions are enabled.

=== "CLI"

    ``` bash
    rustus --remove-parts \
        --tus-extensions "getting,creation,termination,creation-with-upload,creation-defer-length,concatenation,checksum"
    ```

=== "ENV"

    ``` bash
    export RUSTUS_TUS_EXTENSIONS="getting,creation,termination,creation-with-upload,creation-defer-length,concatenation,checksum"
    export RUSTUS_REMOVE_PARTS="true"

    rustus
    ```
