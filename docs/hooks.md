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
* `pre-terminate` - someone wants to delete the upload;
* `post-terminate` - someone deleted upload;
* `post-finish` - someone finished uploading file.

!!! note

    `Pre-create` and `Pre-terminate` hooks are very important.
    If at least one of hooks fails, upload creation or removal is canceled.

    But AMQP hooks won't cancel the upload, since it's non blocking type of hooks.

!!! warning
    After creating final upload with concatenation extension,
    you won't receive `post-create` hook, but `post-finish` instead.

!!! warning
    If you uploaded a whole file within one request with
    `creation-with-upload` extension,
    you won't receive `post-create` hook, but `post-finish` instead.


You can disable some hooks by using `--hooks` parameter.

=== "CLI"

    ``` bash
    rustus --hooks "pre-create,post-create,post-receive,pre-terminate,post-terminate,post-finish"
    ```

=== "ENV"

    ``` bash
    export RUSTUS_HOOKS="pre-create,post-create,post-receive,pre-terminate,post-terminate,post-finish"

    rustus
    ```


## Format

Information about every event is sent using `JSON` format.
Format can be configured with `--hooks-format` parameter or `RUSTUS_HOOKS_FORMAT` environment variable.

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
* `--hooks-amqp-exchange` - name of exchange to use;
* `--hooks-amqp-declare-exchange` - creates exchange on startup;
* `--hooks-amqp-exchange-kind` - kind of exchange to connect to;
* `--hooks-amqp-declare-queues` - creates all queues and binds them to exchange;
* `--hooks-amqp-durable-exchange` - adds durability to created exchange;
* `--hooks-amqp-durable-queues` - adds durability to created;
* `--hooks-amqp-celery` - adds headers required by [Celery](https://docs.celeryq.dev/en/stable/index.html);
* `--hooks-amqp-routing-key` - routing key for all messages passed to exchange.

If no hooks_amqp_routing_key specified, rustus will send all messages with
different routing keys. Named like `{prefix}.{event type}`. Eg `rustus.pre-create` and so on.
Otherwise, it will use only one routing key and only one queue!

!!! warning

    Since we can't really track message delivery and responses
    Rustus won't stop a current upload in any case.

=== "CLI"

    ``` bash
    rustus --hooks-amqp-url "amqp://guest:guest@localhost:5672" \
        --hooks-amqp-queues-prefix "rustus_prefix" \
        --hooks-amqp-exchange "rustus" \
        --hooks-amqp-exchange-kind "topic" \
        --hooks-amqp-routing-key "route66" \
        --hooks-amqp-declare-exchange "yes" \
        --hooks-amqp-declare-queues "yes" \
        --hooks-amqp-durable-exchange "yes" \
        --hooks-amqp-durable-queues "yes" \
        --hooks-amqp-celery "yes"
    ```

=== "ENV"

    ``` bash
    export RUSTUS_HOOKS_AMQP_URL="amqp://guest:guest@localhost:5672"
    export RUSTUS_HOOKS_AMQP_QUEUES_PREFIX="rustus_prefix"
    export RUSTUS_HOOKS_AMQP_EXCHANGE="rustus"
    export RUSTUS_HOOKS_AMQP_EXCHANGE_KIND="topic"
    export RUSTUS_HOOKS_AMQP_ROUTING_KEY="route66"
    export RUSTUS_HOOKS_AMQP_DECLARE_EXCHANGE="yes"
    export RUSTUS_HOOKS_AMQP_DECLARE_QUEUES="yes"
    export RUSTUS_HOOKS_AMQP_DURABLE_EXCHANGE="yes"
    export RUSTUS_HOOKS_AMQP_DURABLE_QUEUES="yes"
    export RUSTUS_HOOKS_AMQP_CELERY="yes"

    rustus
    ```

#### Using Rustus with Celery

Rustus has a cool integration with [Celery](https://docs.celeryq.dev/en/stable/index.html).
Let's build a Celery application that handles rustus hooks.

At first, we need to install Celery itself.
```bash
pip install celery
```

Now we can create a file called "celery.py" in a directory "rustus_celery".
This file contains code that handles celery tasks.

```python title="rustus_celery/celery.py"
import celery

app = celery.Celery("rustus_celery")
app.conf.update(
    broker_url="amqp://guest:guest@localhost:5672",
)


@app.task(name="rustus.pre-create")
def post_create(data):
    print(f"PRE CREATE: {data}")


@app.task(name="rustus.post-create")
def post_create(data):
    print(f"POST CREATE: {data}")


@app.task(name="rustus.post-finish")
def post_finish(data):
    print(f"POST FINISH: {data}")


@app.task(name="rustus.post-terminate")
def post_terminate(data):
    print(f"POST TERMINATE: {data}")


@app.task(name="rustus.post-receive")
def post_recieve(data):
    print(f"POST RECIEVE: {data}")
```

!!! info
    Every task has its name. You must use these names
    in order to handle tasks.

Now we can run our celery worker to start executing tasks.

```
celery -A rustus_celery
```

After starting celery worker you can run Rustus with these
parameters.

The most important parameter is `--hooks-amqp-celery`, because it
adds required by Celery headers to every message.

=== "CLI"

    ``` bash
    rustus --hooks-amqp-url "amqp://guest:guest@localhost:5672" \
        --hooks-amqp-exchange "celery" \
        --hooks-amqp-exchange-kind "direct" \
        --hooks-amqp-routing-key "celery" \
        --hooks-amqp-celery "yes"
    ```

=== "ENV"

    ``` bash
    export RUSTUS_HOOKS_AMQP_URL="amqp://guest:guest@localhost:5672"
    export RUSTUS_HOOKS_AMQP_EXCHANGE="celery"
    export RUSTUS_HOOKS_AMQP_EXCHANGE_KIND="direct"
    export RUSTUS_HOOKS_AMQP_ROUTING_KEY="celery"
    export RUSTUS_HOOKS_AMQP_CELERY="yes"

    rustus
    ```
