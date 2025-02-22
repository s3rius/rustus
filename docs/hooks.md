---
title: Setting up hooks
description: Setting up hooks to notify other systems about uploads
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

## Proxies

If you have rustus behind proxy like nginx, please use `--behind-proxy` parameter.
This parameter helps rustus resolve ip addresses using `Forwarded` and `X-Forwarded-For`.

This option disabled by default for security purposes unless you can be sure that the `Forwarded` and `X-Forwarded-For` headers cannot be spoofed by the client.

=== "CLI"

    ``` bash
    rustus --behind-proxy
    ```

=== "ENV"

    ``` bash
    export RUSTUS_BEHIND_PROXY="true"

    rustus
    ```


## Format

Information about every event is sent using `JSON` format.
Format can be configured with `--hooks-format` parameter or `RUSTUS_HOOKS_FORMAT` environment variable.

Available formats:

* default (will be replaced by v2 in the future)
* v2 (preferred format)
* tusd

=== "default"

    === "Example"

        ``` json
        {
            "upload": {
                "id": "3cd911fe-eba0-499a-b220-b1d1b947b80f",
                "offset": 0,
                "length": 220,
                "path": null,
                "created_at": 1658671969,
                "deferred_size": false,
                "is_partial": false,
                "is_final": false,
                "parts": null,
                "storage": "file_storage",
                "metadata": {
                    "filename" "shrek2.mkv"
                }
            },
            "request": {
                "URI": "/files/",
                "method": "POST",
                "remote_addr": "127.0.0.1",
                "headers": {
                    "host": "rustus.localhost",
                    "upload-length": "220",
                    "user-agent": "curl/7.84.0",
                    "accept": "*/*",
                    "upload-metadata": "filename MTZNQl92aWRlby5tcDQ="
                }
            }
        }
        ```

    === "Pydantic models"

        ```python
        from datetime import datetime
        from typing import Dict, Optional

        from pydantic import BaseModel, IPvAnyAddress


        class Upload(BaseModel):
            """Information about the upload."""

            id: str
            offset: int
            length: int
            path: Optional[str]
            # Actually it's an int,
            # but pydantic can parse it as datetime.
            created_at: datetime
            deferred_size: bool
            is_partial: bool
            is_final: bool
            parts: Optional[List[str]]
            storage: str
            metadata: Dict[str, str]


        class Request(BaseModel):
            """
            Information about request.

            This request was direct cause of hook invocation.
            """

            URI: str
            method: str
            remote_addr: Optional[IPvAnyAddress]
            headers: Dict[str, str]


        class Hook(BaseModel):
            """Rustus hook."""

            upload: Upload
            request: Request

        ```

    === "JSON schema"

        ```json
        {
            "title": "Hook",
            "type": "object",
            "properties": {
                "upload": {
                    "$ref": "#/definitions/Upload"
                },
                "request": {
                    "$ref": "#/definitions/Request"
                }
            },
            "required": [
                "upload",
                "request"
            ],
            "definitions": {
                "Upload": {
                    "title": "Upload",
                    "type": "object",
                    "properties": {
                        "id": {
                            "title": "Id",
                            "type": "string"
                        },
                        "offset": {
                            "title": "Offset",
                            "type": "integer"
                        },
                        "length": {
                            "title": "Length",
                            "type": "integer"
                        },
                        "path": {
                            "title": "Path",
                            "type": "string"
                        },
                        "created_at": {
                            "title": "Created At",
                            "type": "integer"
                        },
                        "deferred_size": {
                            "title": "Deferred Size",
                            "type": "boolean"
                        },
                        "is_partial": {
                            "title": "Is Partial",
                            "type": "boolean"
                        },
                        "is_final": {
                            "title": "Is Final",
                            "type": "boolean"
                        },
                        "parts": {
                            "title": "Parts",
                            "type": "array",
                            "items": {
                                "type": "string"
                            }
                        },
                        "storage": {
                            "title": "Storage",
                            "type": "string"
                        },
                        "metadata": {
                            "title": "Metadata",
                            "type": "object",
                            "additionalProperties": {
                                "type": "string"
                            }
                        }
                    },
                    "required": [
                        "id",
                        "offset",
                        "length",
                        "created_at",
                        "deferred_size",
                        "is_partial",
                        "is_final",
                        "storage",
                        "metadata"
                    ]
                },
                "Request": {
                    "title": "Request",
                    "type": "object",
                    "properties": {
                        "URI": {
                            "title": "Uri",
                            "type": "string"
                        },
                        "method": {
                            "title": "Method",
                            "type": "string"
                        },
                        "remote_addr": {
                            "title": "Remote Addr",
                            "type": "string",
                            "format": "ipvanyaddress"
                        },
                        "headers": {
                            "title": "Headers",
                            "type": "object",
                            "additionalProperties": {
                                "type": "string"
                            }
                        }
                    },
                    "required": [
                        "URI",
                        "method",
                        "headers"
                    ]
                }
            }
        }
        ```

=== "v2"


    === "Example"

        ``` json
        {
            "upload": {
                "id": "3cd911fe-eba0-499a-b220-b1d1b947b80f",
                "offset": 0,
                "length": 220,
                "path": null,
                "created_at": 1658671969,
                "deferred_size": false,
               "is_partial": false,
                "is_final": false,
                "parts": null,
                "storage": "file_storage",
                "metadata": {
                    "filename" "shrek2.mkv"
                }
            },
            "request": {
                "uri": "/files/",
                "method": "POST",
                "remote_addr": "127.0.0.1",
                "headers": {
                    "host": "rustus.localhost",
                    "upload-length": "220",
                    "user-agent": "curl/7.84.0",
                    "accept": "*/*",
                    "upload-metadata": "filename MTZNQl92aWRlby5tcDQ="
                }
            }
        }
        ```

    === "Pydantic models"

        ```python
        from datetime import datetime
        from typing import Dict, Optional

        from pydantic import BaseModel, IPvAnyAddress


        class Upload(BaseModel):
            """Information about the upload."""

            id: str
            offset: int
            length: int
            path: Optional[str]
            # Actually it's an int,
            # but pydantic can parse it as datetime.
            created_at: datetime
            deferred_size: bool
            is_partial: bool
            is_final: bool
            parts: Optional[List[str]]
            storage: str
            metadata: Dict[str, str]


        class Request(BaseModel):
            """
            Information about request.

            This request was direct cause of hook invocation.
            """

            uri: str
            method: str
            remote_addr: Optional[IPvAnyAddress]
            headers: Dict[str, str]


        class Hook(BaseModel):
            """Rustus hook."""

            upload: Upload
            request: Request

        ```

    === "JSON schema"

        ```json
        {
            "title": "Hook",
            "type": "object",
            "properties": {
                "upload": {
                    "$ref": "#/definitions/Upload"
                },
                "request": {
                    "$ref": "#/definitions/Request"
                }
            },
            "required": [
                "upload",
                "request"
            ],
            "definitions": {
                "Upload": {
                    "title": "Upload",
                    "type": "object",
                    "properties": {
                        "id": {
                            "title": "Id",
                            "type": "string"
                        },
                        "offset": {
                            "title": "Offset",
                            "type": "integer"
                        },
                        "length": {
                            "title": "Length",
                            "type": "integer"
                        },
                        "path": {
                            "title": "Path",
                            "type": "string"
                        },
                        "created_at": {
                            "title": "Created At",
                            "type": "integer"
                        },
                        "deferred_size": {
                            "title": "Deferred Size",
                            "type": "boolean"
                        },
                        "is_partial": {
                            "title": "Is Partial",
                            "type": "boolean"
                        },
                        "is_final": {
                            "title": "Is Final",
                            "type": "boolean"
                        },
                        "parts": {
                            "title": "Parts",
                            "type": "array",
                            "items": {
                                "type": "string"
                            }
                        },
                        "storage": {
                            "title": "Storage",
                            "type": "string"
                        },
                        "metadata": {
                            "title": "Metadata",
                            "type": "object",
                            "additionalProperties": {
                                "type": "string"
                            }
                        }
                    },
                    "required": [
                        "id",
                        "offset",
                        "length",
                        "created_at",
                        "deferred_size",
                        "is_partial",
                        "is_final",
                        "storage",
                        "metadata"
                    ]
                },
                "Request": {
                    "title": "Request",
                    "type": "object",
                    "properties": {
                        "uri": {
                            "title": "Uri",
                            "type": "string"
                        },
                        "method": {
                            "title": "Method",
                            "type": "string"
                        },
                        "remote_addr": {
                            "title": "Remote Addr",
                            "type": "string",
                            "format": "ipvanyaddress"
                        },
                        "headers": {
                            "title": "Headers",
                            "type": "object",
                            "additionalProperties": {
                                "type": "string"
                            }
                        }
                    },
                    "required": [
                        "uri",
                        "method",
                        "headers"
                    ]
                }
            }
        }
        ```
=== "tusd"

    === "Example"

        ``` json
        {
            "Upload": {
                "ID": "317e1429-61f3-4631-a480-c50207b69ee4",
                "Offset": 0,
                "Size": 16392985,
                "IsFinal": false,
                "IsPartial": false,
                "PartialUploads": null,
                "SizeIsDeferred": false,
                "MetaData": {
                    "filename": "shrek2.mkv"
                },
                "Storage": {
                    "Type": "file_storage",
                    "Path": null
                }
            },
            "HTTPRequest": {
                "URI": "/files/",
                "Method": "POST",
                "RemoteAddr": "127.0.0.1",
                "Header": {
                    "content-length": [
                        "0"
                    ],
                    "upload-length": [
                        "16392985"
                    ],
                    "user-agent": [
                        "python-requests/2.27.1"
                    ],
                    "host": [
                        "rustus.localhost"
                    ],
                    "accept": [
                        "*/*"
                    ],
                    "upload-metadata": [
                        "filename MTZNQl92aWRlby5tcDQ="
                    ],
                    "tus-resumable": [
                        "1.0.0"
                    ]
                }
            }
        }
        ```
    === "Pydantic models"

        ```python
        from typing import Dict, List, Optional

        from pydantic import BaseModel, IPvAnyAddress


        class Request(BaseModel):
            """
            Information about request.

            This request was direct cause of hook invocation.
            """

            URI: str
            Method: str
            RemoteAddr: IPvAnyAddress
            Header: Dict[str, List[str]]


        class Storage(BaseModel):
            """Information where upload is stored."""

            Type: str
            Path: Optional[str]


        class Upload(BaseModel):
            """Information about the upload."""

            ID: str
            Offset: int
            Size: int
            IsFinal: bool
            IsPartial: bool
            PartialUploads: Optional[List[str]]
            SizeIsDeferred: bool
            MetaData: Dict[str, str]
            Storage: Storage


        class Hook(BaseModel):
            """Rustus hook."""

            Upload: Upload
            HTTPRequest: Request

        ```

    === "JSON schema"

        ```json
        {
            "title": "Hook",
            "type": "object",
            "properties": {
                "Upload": {
                    "$ref": "#/definitions/Upload"
                },
                "HTTPRequest": {
                    "$ref": "#/definitions/Request"
                }
            },
            "required": [
                "Upload",
                "HTTPRequest"
            ],
            "definitions": {
                "Storage": {
                    "title": "Storage",
                    "type": "object",
                    "properties": {
                        "Type": {
                            "title": "Type",
                            "type": "string"
                        },
                        "Path": {
                            "title": "Path",
                            "type": "string"
                        }
                    },
                    "required": [
                        "Type"
                    ]
                },
                "Upload": {
                    "title": "Upload",
                    "type": "object",
                    "properties": {
                        "ID": {
                            "title": "Id",
                            "type": "string"
                        },
                        "Offset": {
                            "title": "Offset",
                            "type": "integer"
                        },
                        "Size": {
                            "title": "Size",
                            "type": "integer"
                        },
                        "IsFinal": {
                            "title": "Isfinal",
                            "type": "boolean"
                        },
                        "IsPartial": {
                            "title": "Ispartial",
                            "type": "boolean"
                        },
                        "PartialUploads": {
                            "title": "Partialuploads",
                            "type": "array",
                            "items": {
                                "type": "string"
                            }
                        },
                        "SizeIsDeferred": {
                            "title": "Sizeisdeferred",
                            "type": "boolean"
                        },
                        "MetaData": {
                            "title": "MetaData",
                            "type": "object",
                            "additionalProperties": {
                                "type": "string"
                            }
                        },
                        "Storage": {
                            "$ref": "#/definitions/Storage"
                        }
                    },
                    "required": [
                        "ID",
                        "Offset",
                        "Size",
                        "IsFinal",
                        "IsPartial",
                        "SizeIsDeferred",
                        "MetaData",
                        "Storage"
                    ]
                },
                "Request": {
                    "title": "Request",
                    "type": "object",
                    "properties": {
                        "URI": {
                            "title": "Uri",
                            "type": "string"
                        },
                        "Method": {
                            "title": "Method",
                            "type": "string"
                        },
                        "RemoteAddr": {
                            "title": "Remoteaddr",
                            "type": "string",
                            "format": "ipvanyaddress"
                        },
                        "Header": {
                            "title": "Header",
                            "type": "object",
                            "additionalProperties": {
                                "type": "array",
                                "items": {
                                    "type": "string"
                                }
                            }
                        }
                    },
                    "required": [
                        "URI",
                        "Method",
                        "RemoteAddr",
                        "Header"
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
* `--http-hook-timeout` - Timeout for all http requests in seconds. By default it's 2 seconds.

!!! note
    Hook names are passed as header called `Hook-Name`.

=== "CLI"

    ``` bash
    rustus --hooks-http-urls "https://httpbin.org/post" \
        --hooks-http-proxy-headers "Authorization" \
        --http-hook-timeout 1
    ```

=== "ENV"

    ``` bash
    export RUSTUS_HOOKS_HTTP_URLS="https://httpbin.org/post"
    export RUSTUS_HOOKS_HTTP_PROXY_HEADERS="Authorization"
    export RUSTUS_HTTP_HOOK_TIMEOUT="1"

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

Let's configure rustus to use this server as a hook receiver.

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
* `--hooks-amqp-routing-key` - routing key for all messages passed to exchange;
* `--hooks-amqp-connection-pool-size` - maximum number of opened connections to RabbitMQ;
* `--hooks-amqp-channel-pool-size` - maximum number of opened channels for each connection to RabbitMQ;
* `--hooks-amqp-idle-connection-timeout` - timeout for idle connection in seconds. If the connection isn't used, it's dropped;
* `--hooks-amqp-idle-channels-timeout` - timeout for idle channels in seconds. If the channel isn't used, it's dropped.

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
        --hooks-amqp-declare-exchange \
        --hooks-amqp-declare-queues \
        --hooks-amqp-durable-exchange \
        --hooks-amqp-durable-queues \
        --hooks-amqp-celery \
        --hooks-amqp-connection-pool-size 10 \
        --hooks-amqp-channel-pool-size 10 \
        --hooks-amqp-idle-connection-timeout 20 \
        --hooks-amqp-idle-channels-timeout 10
    ```

=== "ENV"

    ``` bash
    export RUSTUS_HOOKS_AMQP_URL="amqp://guest:guest@localhost:5672"
    export RUSTUS_HOOKS_AMQP_QUEUES_PREFIX="rustus_prefix"
    export RUSTUS_HOOKS_AMQP_EXCHANGE="rustus"
    export RUSTUS_HOOKS_AMQP_EXCHANGE_KIND="topic"
    export RUSTUS_HOOKS_AMQP_ROUTING_KEY="route66"
    export RUSTUS_HOOKS_AMQP_DECLARE_EXCHANGE="true"
    export RUSTUS_HOOKS_AMQP_DECLARE_QUEUES="true"
    export RUSTUS_HOOKS_AMQP_DURABLE_EXCHANGE="true"
    export RUSTUS_HOOKS_AMQP_DURABLE_QUEUES="true"
    export RUSTUS_HOOKS_AMQP_CELERY="true"
    export RUSTUS_HOOKS_AMQP_CONNECTION_POOL_SIZE="10"
    export RUSTUS_HOOKS_AMQP_CHANNEL_POOL_SIZE="10"
    export RUSTUS_HOOKS_AMQP_IDLE_CONNECTION_TIMEOUT="20"
    export RUSTUS_HOOKS_AMQP_IDLE_CHANNELS_TIMEOUT="10"

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
def post_receive(data):
    print(f"POST RECEIVE: {data}")
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
        --hooks-amqp-celery
    ```

=== "ENV"

    ``` bash
    export RUSTUS_HOOKS_AMQP_URL="amqp://guest:guest@localhost:5672"
    export RUSTUS_HOOKS_AMQP_EXCHANGE="celery"
    export RUSTUS_HOOKS_AMQP_EXCHANGE_KIND="direct"
    export RUSTUS_HOOKS_AMQP_ROUTING_KEY="celery"
    export RUSTUS_HOOKS_AMQP_CELERY="true"

    rustus
    ```

### Kafka hooks

Rustus support sending hooks to kafka cluster. We use [rust-rdkafka](https://github.com/fede1024/rust-rdkafka) as a driver. Since it uses a C++ library, it's configuration. If you have any question about specific parameter, please refer to the C++ library [configuration](https://github.com/confluentinc/librdkafka/blob/master/CONFIGURATION.md).

!!! info

    All messages are sent with a key, which is equals to upload_id.

Configuration parameters:

* `--hooks-kafka-urls` - Kafka urls. List of brokers to connect to in the format `host:port`. If you have multiple brokers, separate them with commas. Corresponds to `bootstrap.servers` in kafka config.
* `--hooks-kafka-client-id` - Kafka producer client.id
* `--hooks-kafka-topic` - Kafka topic. If specified, all events will be sent to this topic.
* `--hooks-kafka-prefix` - Kafka prefix. If specified, all hook-names will be prepended with this string and used as a topic.
* `--hooks-kafka-required-acks` - Kafka required acks. This parameter is used to configure how many replicas must acknowledge the message. Corresponds to `request.required.acks`.
* `--hooks-kafka-compression` - Compression codec. This parameter is used to compress messages before sending them to Kafka. Corresponds to `compression.codec` in Kafka configuration.
* `--hooks-kafka-idle-timeout` - Kafka idle timeout in seconds. After this amount of time in seconds, the connection will be dropped. Corresponds to `connections.max.idle.ms` in Kafka configuration, but in seconds.
* `--hooks-kafka-send-timeout` - Kafka send timeout in seconds. After this amount of time in seconds, the message will be dropped
* `--hooks-kafka-extra-options` - Extra options for Kafka. This parameter is used to pass additional options to Kafka. All options must be in the format `key=value`, separated by semicolon. Example: `key1=value1;key2=value2`.

=== "CLI"

    ``` bash
    rustus --hooks-kafka-urls "localhost:9094" \
        --hooks-kafka-client-id "client-1" \
        --hooks-kafka-topic "topic" \
        --hooks-kafka-prefix "my-prefix" \
        --hooks-kafka-required-acks \
        --hooks-kafka-compression "none" \
        --hooks-kafka-idle-timeout "10" \
        --hooks-kafka-send-timeout "10" \
        --hooks-kafka-extra-options "allow.auto.create.topics=true;security.protocol=plaintext"
    ```

=== "ENV"

    ``` bash
    export RUSTUS_HOOKS_KAFKA_URLS="localhost:9094"
    export RUSTUS_HOOKS_KAFKA_CLIENT_ID="client-1"
    export RUSTUS_HOOKS_KAFKA_TOPIC="my-topic";
    export RUSTUS_HOOKS_KAFKA_PREFIX="pref"
    export RUSTUS_HOOKS_KAFKA_COMPRESSION="gzip"
    export RUSTUS_HOOKS_KAFKA_IDLE_TIMEOUT="10"
    export RUSTUS_HOOKS_KAFKA_SEND_TIMEOUT="10"
    export RUSTUS_HOOKS_KAFKA_EXTRA_OPTIONS="allow.auto.create.topics=true;security.protocol=plaintext"
    

    rustus
    ```

!!! warning

    Since we can't really track message delivery and responses
    Rustus won't stop a current upload in any case.
