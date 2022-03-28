---
title: Configuration
description: "How to configure Rusts"
---

Rustus is highly configurable you can configure rustus with CLI or you can use environment variables.

!!! info
    Some options can be passed only through as CLI parameters

!!! info

    Information about hooks you can find on [Hooks page](../hooks).


## Configuring server

We use actix to run server.
You can configure on wich `host` and `port` rustus is listenging.
Also you can configure number of actix `workers` that handle connections.

`--max-body-size` is the max number of bytes that users can send in request body.

`--url` is a base URL for all tus requests.

`--workers` by default is euqal to number of physical CPU cores. Edit it carefully.

=== "CLI"

    ``` bash
    rustus --host "0.0.0.0" \
        --port 1081 \
        --workers 8 \
        --max-body-size 1000000 \
        --url "/files" \
        --log-level "INFO"
    ```

=== "ENV"

    ``` bash
    export RUSTUS_SERVER_HOST="0.0.0.0"
    export RUSTUS_SERVER_PORT="1081"
    export RUSTUS_SERVER_WORKERS="8"
    export RUSTUS_MAX_BODY_SIZE="1000000"
    export RUSTUS_URL="/files"
    export RUSTUS_LOG_LEVEL="INFO"

    rustus
    ```


## Configuring data storage


!!!info

    Currently only file storage is available,
    so if you pass to `--storage` parameter other than `file-storage` you will get an error.

    Also you **can not** pass `--force-fsync` through environment variables.


`--storage` is a type of data storage to be used.

`--data-dir` is a path to the directory where all files are stored.

`--dir-structure` is a pattern of a directory structure inside data dir.
You can use variables within the pattern.

Available variables:

* `{year}` - current year;
* `{month}` - current month number from 1 to 12;
* `{day}` - current day number from 1 to 31;
* `{hour}` - hour number from 0 to 23;
* `{minute}` - minute number from 0 to 59;
* `{env[ENV_NAME]}` - environment variable where `ENV_NAME` is name of your variable.

!!! note

    All environment variables are saved in memory during rustus startup.
    So you cannot change variable dynamically. Even if you change env used in
    structure pattern it won't change.

For example if you use `{env[HOSTNAME]}/{year}/{month}/{day}` as your dir-structure, rustus stores files like:

``` bash
$ tree data
data
└── rtus-68cb5b8746-5mgw9
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

    rustus --force-fsync
    ```

## Configuring info storage

Info storages are used to store information
about file uploads. These storages **must** be persistent,
because every time chunk is uploaded rustus updates information
about upload. And when someone wants to download file, information
about it requested from storage to get actual path of an upload.

Available info storages:

* `file-info-storage` - stores information in files on disk;
* `redis-info-storage` - information is stored in redis;
* `db-info-storage` - information is stored in database;

### File info storage

file info storage stores information in files on disk.
It's default info storage. Every download has it's own associated file.
All .info files stored in flat structure so it's the least preferable way of
storing information about uploads. But if you don't plan to have many uploads, it may fit well.

`--info-dir` - directory where all .info file will be stored (default is `./data`).


=== "CLI"

    ``` bash
    rustus --force-fsync \
        --storage "file-info-storage" \
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

    If you're using redis as a cluster
    you must provide connection string for master redis server.
    Since rustus need to have latest information and it writes a lot.

`--info-db-dsn` - connection string for your redis database.
It's required if redis-info-storage is chosen.

=== "CLI"

    ``` bash
    rustus --force-fsync \
        --storage "redis-info-storage" \
        --info-db-dsn "redis://localhost/0"
    ```

=== "ENV"

    ``` bash
    export RUSTUS_INFO_STORAGE="redis-info-storage"
    export RUSTUS_INFO_DB_DSN="redis://localhost"

    rustus
    ```


### DB info storage

Rustus can store information about upload in database.

It's a good and reliable option. But rustus can't work
with replicas since it requires most recent information
about uploads.

You can use `postgresql`, `mysql` or even `sqlite` schemas to
connect to database.

`--info-db-dsn` - connection string for your database.

=== "CLI"

    ``` bash
    rustus --force-fsync \
        --storage "db-info-storage" \
        --info-db-dsn "postgresql://user:password@localhost/db"
    ```

=== "ENV"

    ``` bash
    export RUSTUS_INFO_STORAGE="redis-info-storage"
    export RUSTUS_INFO_DB_DSN="postgresql://user:password@localhost/db"

    rustus
    ```

## Configuring TUS

Since tus protocol offers extensibility you can turn off some protocol extensions.

Available extensions:

* `getting` - rustus specific extension that helps you download uploaded files with get request;
* `creation` - helps you to create files (It's like a core feature you better have this enabled);
* `termination` - allows you to delete uploads with DELETE request;
* `creation-with-upload` - allows you to write first bytes of a file while creating;
* `creation-defer-length` - allows you to create file without specifying file length;
* `concatenation` - allows you to concatenate finished partial uploads.
* `checksum` - allows you to verify checksum of every batch.

You can read more about extensions on [official web-site](https://tus.io/protocols/resumable-upload.html#protocol-extensions).

`--tus-extensions` - a list of enabled extensions.

By default all extensions are enabled.

=== "CLI"

    ``` bash
    rustus --tus-extensions "getting,creation,termination,creation-with-upload,creation-defer-length,concatenation,checksum"
    ```

=== "ENV"

    ``` bash
    export RUSTUS_TUS_EXTENSIONS="getting,creation,termination,creation-with-upload,creation-defer-length,concatenation,checksum"

    rustus
    ```
