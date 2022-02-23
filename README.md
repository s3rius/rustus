<div align="center">
    <img src="./imgs/logo_horizontal.svg" alt="logo" width="500">
    <div>
        <p></p>
        <img alt="Docker Image Size (latest by date)" src="https://img.shields.io/docker/image-size/s3rius/rustus?sort=date&style=for-the-badge">
        <img alt="Docker Image Version (latest semver)" src="https://img.shields.io/docker/v/s3rius/rustus?style=for-the-badge">
        <img alt="GitHub" src="https://img.shields.io/github/license/s3rius/rustus?style=for-the-badge">
    </div>
    <p><a href="https://tus.io/">TUS</a> protocol implementation written in Rust.</p>
</div>

## Features

This implementation has several features to make usage as simple as possible.

* Rustus is robust, since it uses asynchronous Rust;
* It can store information about files in databases;
* You can specify directory structure to organize your uploads;
* It has a lot of hooks options, and hooks can be combined.
* Highly configurable;

## Installation

You can download binaries from a [releases page](https://github.com/s3rius/rustus/releases).

If you want to use docker, you can use official images from [s3rius/rustus](https://hub.docker.com/r/s3rius/rustus/):

```bash
docker run --rm -it -p 1081:1081 s3rius/rustus:latest
```

If we don't have a binary file for your operating system you can build it
with [cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html).

```bash
git clone https://github.com/s3rius/rustus.git
cd rustus
cargo install --path . --features=all
```

### Supported data storages

Right now you can only use `file-storage` to store uploads data. The only two options you can adjust are:

* uploads directory
* directory structure

To upload files in a custom directory other than `./data`
you can provide a `--data-dir` parameter.

```bash
rustus --data-dir "./files"
```

If you have a lot of uploads, you don't want to store all your files in a flat structure. So you can set a directory
structure for your uploads.

```bash
rustus --dir-structure="{env[HOSTNAME]}/{year}/{month}/{day}"
```

```bash
tree data
data
├── 0bd911d4054d41c6a3ad54be67ee3e66.info
├── 5bc9c62384494c439e2a064b82a39cc6.info
└── rtus-68cb5b8746-5mgw9
    └── 2022
        └── 1
            └── 8
                ├── 0bd911d4054d41c6a3ad54be67ee3e66
                └── 5bc9c62384494c439e2a064b82a39cc6

```

**Important note:** if you use variable that doesn't exist or incorrect like invalid env variable, it results in an
error and the directory structure will become flat again.

As you can see all info files are stored in a flat structure. It cannot be changed if you use file info storage. In
order to get rid of those `.info` files use different info storages.

## Info storages

The info storage is a database or directory. The main goal is to keep track of uploads. Rustus stores information about
download in json format inside database.

File storage is used by default. You can customize the directory of an .info files by providing `--info-dir` parameter.

```bash
rustus --info-dir "./info_dir"
```

If you want to choose different storage you have to specify its type and connection string.

```bash
# Redis info storage
rustus --info-storage redis-info-storage --info-db-dsn "redis://localhost"
# PostgreSQL info storage
rustus --info-storage db-info-storage --info-db-dsn "postgres://rustus:rustus@192.168.1.89:5440/rustus"
# SQLite3 info storage
rustus --info-storage db-info-storage --info-db-dsn "sqlite:////test.sqlite3"
# MySQL
rustus --info-storage db-info-storage --info-db-dsn "mysql://rustus:rustus@192.168.1.89:3306/rustus"
```

## Hooks

Rustus supports several event hooks, such as:

* File hooks;
* HTTP hooks;
* AMQP hooks.

You can combine them, but you have to be careful, since AMQP hooks won't block uploading.

If you want to check the "Authorization" header value or validate some information, you have to use webhooks or File
hooks.

Hooks have priorities: file hooks are the most important, then goes webhooks and AMQP hooks have the least priority. If
pre-create hook failed, the upload would not start. Of course, since AMQP is a protocol that doesn't allow you to track
responses we can't validate anything to stop uploading.

Hooks can have 2 formats

default:

```json
{
  "upload": {
    "id": "",
    "offset": 0,
    "length": 39729945,
    "path": null,
    "created_at": 1641620821,
    "deferred_size": false,
    "metadata": {
      "filename": "38MB_video.mp4",
      "meme": "hehe2"
    }
  },
  "request": {
    "URI": "/files",
    "method": "POST",
    "remote_addr": "127.0.0.1",
    "headers": {
      "accept-encoding": "gzip, deflate",
      "connection": "keep-alive",
      "host": "localhost:1081",
      "upload-metadata": "meme aGVoZTI=,filename MzhNQl92aWRlby5tcDQ=",
      "tus-resumable": "1.0.0",
      "content-length": "0",
      "upload-length": "39729945",
      "user-agent": "python-requests/2.26.0",
      "accept": "*/*"
    }
  }
}
```

tusd:

```json
{
  "Upload": {
    "ID": "",
    "Offset": 0,
    "Size": 39729945,
    "IsFinal": true,
    "IsPartial": false,
    "PartialUploads": null,
    "SizeIsDeferred": false,
    "Metadata": {
      "filename": "38MB_video.mp4",
      "meme": "hehe2"
    },
    "Storage": {
      "Type": "filestore",
      "Path": null
    }
  },
  "HTTPRequest": {
    "URI": "/files",
    "Method": "POST",
    "RemoteAddr": "127.0.0.1",
    "Header": {
      "host": [
        "localhost:1081"
      ],
      "user-agent": [
        "python-requests/2.26.0"
      ],
      "accept": [
        "*/*"
      ],
      "content-length": [
        "0"
      ],
      "upload-metadata": [
        "meme aGVoZTI=,filename MzhNQl92aWRlby5tcDQ="
      ],
      "connection": [
        "keep-alive"
      ],
      "tus-resumable": [
        "1.0.0"
      ],
      "upload-length": [
        "39729945"
      ],
      "accept-encoding": [
        "gzip, deflate"
      ]
    }
  }
}
```

### File hooks

Rustus can work with two types of file hooks.

1. Single file hook;
2. Hooks directory.

The main difference is that hook name is passed as a command line parameter to a single file hook, but if you use hooks
directory then hook name is used to determine a file to call. Let's take a look at the examples

Example of a single file hook:

```bash
#!/bin/bash

# Hook name would be "pre-create", "post-create" and so on.
HOOK_NAME="$1"
HOOK_INFO="$2"
MEME="$(echo "$HOOK_INFO" | jq ".upload .metadata .meme" | xargs)"

# Here we check if name in metadata is equal to pepe.
if [[ $MEME = "pepe" ]]; then
  echo "This meme isn't allowed" 1>&2;
  exit 1
fi
```

As you can see it uses first CLI parameter as a hook name and all hook data is received from stdin.

Let's make it executable

```bash
chmod +x "hooks/unified_hook"
```

To use it you can add parameter

```bash
rustus --hooks-file "hooks/unified_hook"
```

This hook is going to ignore any file that has "pepe" in metadata.

Let's create a hook directory.

```bash
❯ tree hooks
hooks
├── post-create
├── post-finish
├── post-receive
├── post-terminate
└── pre-create
```

Every file in this directory has an executable flag. So you can specify a parameter to use hooks directory.

```bash
rustus --hooks-dir "hooks"
```

In this case rustus will append a hook name to the directory you pointed at and call it as an executable.

Information about hook is passed as a first parameter, as if you call script by running:

```bash
./hooks/pre-create '{"id": "someid", ...}'
```

### Http Hooks

Http hooks use http protocol to notify you about an upload. You can use HTTP hooks to verify Authorization.

Let's create a FastAPI application that listens to hooks and checks the authorization header.

```bash
# Installing dependencies
pip install fastapi uvicorn
```

```python
# server.py
from fastapi import FastAPI, Header, HTTPException
from typing import Optional

app = FastAPI()


@app.post("/hooks")
def hook(
        authorization: Optional[str] = Header(None),
        hook_name: Optional[str] = Header(None),
):
    print(f"Received: {hook_name}")
    if authorization != "Bearer jwt":
        raise HTTPException(401)
    return None
```

Now we can start a server.

```bash
uvicorn server:app --port 8080
```

Now you can start rustus, and it will check if Authorization header has a correct value.

```bash
rustus --hooks-http-urls "http://localhost:8000/hooks" --hooks-http-proxy-headers "Authorization"
```

### AMQP hooks

All hooks can be sent with an AMQP protocol.

For example if you have a rabbitMQ you can use it.

```bash
rustus --hooks-amqp-url "amqp://guest:guest@localhost" --hooks-amqp-exchange "my_exchange"
```

This command will create an exchange called "rustus" and queues for every hook.

Every hook is published with routing key "rustus.{hook_name}" like
"rustus.post-create" or "rustus.pre-create" and so on.

The problem with AMQP hooks is that you can't block the upload. To do this you have to use HTTP or File hooks. But with
AMQP your uploads become non-blocking which is definitely a good thing.