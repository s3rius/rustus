---
title: "Deployment"
description: "How to deploy rustus"
---

# Deployment

Deploying an application is always a challenge. Rustus was made to make deployment as easy as possible.
Since Rustus works with files you have to be careful while scaling it. All rustus instances
must have access to the same data and info storages.

!!! info

    If you want to track you rustus instances with **prometheus** you can
    always get metrics at `/metrics` endpoint.

## Docker compose

``` yaml title="docker-compose.yml"
# This is super simple configuration
version: "3.7"

services:
  rustus:
    image: s3rius/rustus
    volumes:
    # Volume mouted to default data directory
    # So it's available across multiple containers.
      - rustus_data_volume:/app/data

volumes:
  rustus_data_volume:
```

After running `docker compose up` you will see rustus startup logs.

If you want to deploy multiple rustus instances you can simply
use config as this one:

``` yaml title="docker-compose.yml"
version: "3.7"

services:
  proxy:
    image: jwilder/nginx-proxy:alpine
    container_name: proxy
    # Actual proxy ports.
    ports:
      - 8080:80
    volumes:
    # This thing helps to locate containers
    # within this composition to generate nginx config.
      - /var/run/docker.sock:/tmp/docker.sock:ro

  rustus:
    image: s3rius/rustus
    ports:
    # Ports definition
    # To generate correct nginx config.
      - 1081
    volumes:
    # Volume mouted to default data directory
    # So it's available across multiple containers.
      - rustus_data_volume:/app/data
    environment:
        # Idk why but without this variable
        # load balancing with jwilder/nginx-proxy doesn't work.
        VIRTUAL_HOST: localhost

volumes:
  rustus_data_volume: # This is named volume
```

The main idea is that traffic that comes into nginx-proxy
is routed in one of multiple rustus containers.
Here I used `jwilder/nginx-proxy` but you can use other
reverse-proxies such as [Nginx proxy](https://www.nginx.com/), [Traefik](https://traefik.io/) or [Envoy proxy](https://www.envoyproxy.io/).

Now you can run multiple rustus instnaces like this.

```bash
docker compose up --scale rustus=3
```

After that you can upload files to `http://localhost:8080/files`

## Kubernetes

Configuration for Kubernetes is almost the same as for Docker.
But the most preferable way is an official helm chart.

Load balancing is done by Kubernetes, so you just have to
create a volume to mount data and info directories.

## Helm

You can install rustus by running this set of commands:
``` bash
helm repo add "rustus" "https://s3rius.github.io/rustus/helm_releases"
helm repo update
helm install "rustus" "rustus/rustus"
```

### Configuration

Since default deplyment may not fit you.
You can adjust it to satisfy your needs.
You can do it easily with helm.


At first you need to save default values on disk.

``` bash
# You can download basic configuration by running
helm show values "rustus/rustus" > values.yml
```

!!! warning

    For production use you must provide and mount PersistentVolumeClaim
    in order to scale rustus.

    This helm chart has only one replica by default.

You can read more about configuration below.

After you done editing `values.yml`, you can apply the configuration like this:

``` bash
helm upgrade \
--install \ # Install chart if it's not installed
--namespace rustus \ # k8s namespace
--create-namespace \ # Creates namespace if it doesn't exist
--atomic \ # Ensures that everything is deployed correctly
--values "values.yml" \ # Link to values.yml file
"rustus" \ # name of a release
"rustus/rustus" # Name of the chart
```

### Persistence

You can add PVC mount by editing `persistence` section.
The most preferable way is to create `PersistentVolume` and `PersistentVolumeClaim`
before installing this chart.

After you created claim you can apply this values file to mount your claim into rustus.
``` yaml title="values.yml"
persistence:
  enabled: true
  existingClaim: "rustus-pvc"
```

!!! warning

    Currently there's no ability to create multiple mounts
    and if you use file info storage you must specify the same direcotry
    as you specified for data storage.

    But it would be better to use other type of info-storage.

### Subcharts

For example if you want to use redis as your info storage.

``` yaml title="values.yml"
env:
  RUSTUS_INFO_STORAGE: redis-info-storage
  RUSTUS_INFO_DB_DSN: redis://:pass@rustus-redis-master/0

redis:
  enabled: true
```

`redis`, `postgersql` and `mysql` are subcharts.

You can find information about configuration these subcharts here:

* [Repo](https://github.com/bitnami/charts/tree/master/bitnami/redis) for redis;
* [Repo](https://github.com/bitnami/charts/tree/master/bitnami/mysql) for mysql;
* [Repo](https://github.com/bitnami/charts/tree/master/bitnami/postgresql) for postgresql.

In production you may ignore these subcharts to deploy your own redis or mysql or postgresql.
