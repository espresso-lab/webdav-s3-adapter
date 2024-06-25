# `WebDAV` to `S3` Adapter

[![GitHub tag](https://img.shields.io/github/tag/espresso-lab/webdav-s3-adapter?include_prereleases=&sort=semver&color=blue)](https://github.com/espresso-lab/webdav-s3-adapter/tags/)
[![License](https://img.shields.io/badge/License-MIT-blue)](#license)
[![Rust Report Card](https://rust-reportcard.xuri.me/badge/github.com/espresso-lab/webdav-s3-adapter)](https://rust-reportcard.xuri.me/report/github.com/espresso-lab/webdav-s3-adapter)

This container acts as adapter to integrate S3 with WebDAV. Initially this solution was created to link Enpass to a self-hosted Minio (S3-Storage).

## Features

- Blazing fast ⚡️ and written in Rust ⚙️
- Secure implementation 🔐
- Easy to deploy to a Kubernetes environment via Helm or to use it with Docker Compose
- Simple configuration via environment variables or Helm values

## Usage

### Usage in Kubernetes / Helm

First, install the `webdav-s3-adapter` Helm chart:

```
helm install oci://ghcr.io/espresso-lab/helm-charts/webdav-s3-adapter
```

### Usage in Docker Compose

## Architecture

## Configuration

### Environment variables

#### General settings

## License

Released under [MIT](/LICENSE) by [@espresso-lab](https://github.com/espresso-lab).
