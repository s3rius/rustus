---
title: Setting up hooks
desctiption: Setting up hooks to notify other systems about uploads
---

Rustus can notify about uploads using hooks.
This is useful when you integrate rustus in your architecture.
Apps can keep track of every upload using this feature.

Rustus has different event types for different moments of an upload's lifecycle.

* `pre-create` - This hook means that someone wants to create an upload;
* `post-create` - someone successfully created an upload;
* `post-receive` - someone uploaded a new part of an upload;
* `post-terminate` - someone deleted upload;
* `post-finish` - someone finished uploading file.

!!! note

    Pre-create hook is very important.
    If at least one of hooks fails, upload is canceled.

    But AMQP hooks won't cancel the upload, since it's non blocking type of hooks.

You can disable some hooks by using `--hooks` parameter.

=== "CLI"

    ``` bash
    rustus --hooks "pre-create,post-create,post-receive,post-terminate,post-finish"
    ```

=== "ENV"

    ``` bash
    export RUSTUS_HOOKS="pre-create,post-create,post-receive,post-terminate,post-finish"

    rustus
    ```


## Fomat

Information about every hook using `JSON` format.
Format can be configured using `--hooks-format` parameter or `RUSTUS_HOOKS_FORMAT` environment variable.

Available formats:

* default
* tusd

=== "default"

    ``` json
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

=== "tusd"

    ``` json
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

## Hook types

Rustus offers multiple types of Hooks. We'll take a brief look on each type.

### File hooks

Rustus can work with two types of file hooks.

* Single file hook
* Hooks directory

The main difference is that in case if use single file hook, hook name is passed as a command line argument
to an executable file, but if you use hooks directory then hook name is used to determine a file to call. Let's take a look at the examples.

Parameters:
* `--hooks-file` - path to an executable file;
* `--hooks-dir` - path to a directory with executable files.

``` bash title="single_file_hook.sh"
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

As you can see it uses first CLI parameter as a hook name and all hook data is received from the second one.
Let's make it executable and make rustus use this hook.

=== "CLI"

    ``` bash
    chmod +x "hooks/single_file_hook.sh"

    rustus --hooks-file "hooks/single_file_hook.sh"
    ```

=== "ENV"

    ``` bash
    chmod +x "hooks/single_file_hook.sh"
    export RUSTUS_HOOKS_FILE="hooks/single_file_hook.sh"

    rustus
    ```

If you would like to use directory hooks you must create directory with the following structure:

```tree
hooks
├── post-create
├── post-finish
├── post-receive
├── post-terminate
└── pre-create
```

!!! warning
    If some hook file isn't found, rustus throws an error.
    In case with `pre-create` hook it can be fatal.

### Http Hooks

Http hooks use HTTP to send `POST` requests to some endpoint.

Configuration parameters:

* `--hooks-http-proxy-headers` - list of headers to proxy (separated by commas) to listener's endpoint;
* `--hooks-http-urls` - list of absolute urls to send request to (separated by commas).

!!! note
    Hook names are passed as header called `Hook-Name`.

=== "CLI"

    ``` bash
    rustus --hooks-http-urls "https://httpbin.org/post" \
        --hooks-http-proxy-headers "Authorization"
    ```

=== "ENV"

    ``` bash
    export RUSTUS_HOOKS_HTTP_URLS="https://httpbin.org/post"
    export RUSTUS_HOOKS_HTTP_PROXY_HEADERS="Authorization"

    rustus
    ```

#### Example application

To be more verbose let's create simple web server that
handles uploads using [FastAPI](https://fastapi.tiangolo.com/).

At first we need to install dependencies using pip.

```bash
pip install fastapi uvicorn
```


```python title="server.py"
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

Now we can run this server using uvicorn.

```bash
uvicorn server:app --port 8080
```

Let's configure rustus to use this server as a hook reciever.

=== "CLI"

    ``` bash
    rustus --hooks-http-urls "http://localhost:8000/hooks" \
        --hooks-http-proxy-headers "Authorization"
    ```

=== "ENV"

    ``` bash
    export RUSTUS_HOOKS_HTTP_URLS="http://localhost:8000/hooks"
    export RUSTUS_HOOKS_HTTP_PROXY_HEADERS="Authorization"

    rustus
    ```

That's it.


### AMQP hooks

AMQP hooks are used to store information about uploads using RabbitMQ.

Configuration parameters:

* `--hooks-amqp-url` - connection string to RabbitMQ;
* `--hooks-amqp-queues-prefix` - prefix for queues for every event queue;
* `--hooks-amqp-exchange` - name of exchange to use.

This hook will send every message in an exchange with routing keys
like queues names.

Queues are named like `{prefix}.{event type}`. Eg `rustus.pre-create` and so on.

!!! warning

    Since we can't really track message delivery and responses
    Rustus doesn't stop in any case.

=== "CLI"

    ``` bash
    rustus --hooks-amqp-url "amqp://guest:guest@localhost:5672" \
        --hooks-amqp-queues-prefix "rustus_queue" \
        --hooks-amqp-exchange "rustus"
    ```

=== "ENV"

    ``` bash
    export RUSTUS_HOOKS_AMQP_URL="amqp://guest:guest@localhost:5672"
    export RUSTUS_HOOKS_AMQP_QUEUES_PREFIX="rustus_queue"
    export RUSTUS_HOOKS_AMQP_EXCHANGE="rustus"

    rustus
    ```