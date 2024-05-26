use std::env;

use aws_config::BehaviorVersion;
use aws_sdk_s3::{
    operation::{
        delete_object::DeleteObjectOutput, get_object::GetObjectOutput,
        list_objects_v2::ListObjectsV2Output, put_object::PutObjectOutput,
    },
    primitives::ByteStream,
    Client,
};
use tracing::info;

pub async fn init_client() -> Client {
    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let s3_endpoint = env::var("S3_ENDPOINT").unwrap_or("".to_string());
    if s3_endpoint.is_empty() {
        return aws_sdk_s3::Client::new(&config);
    }
    let local_config = aws_sdk_s3::config::Builder::from(&config)
        .endpoint_url(s3_endpoint)
        .force_path_style(
            env::var("S3_FORCE_PATH_STYLE")
                .unwrap_or("".to_string())
                .eq("true"),
        )
        .build();
    return aws_sdk_s3::Client::from_conf(local_config);
}

pub async fn fetch_file_from_s3(
    client: &Client,
    bucket: &str,
    key: &str,
) -> Result<(Vec<u8>, String), aws_sdk_s3::Error> {
    let result = client.get_object().bucket(bucket).key(key).send().await?;
    let content_type = result
        .content_type()
        .unwrap_or("application/octet-stream")
        .to_string();
    let body = result.body.collect().await.unwrap().into_bytes();
    Ok((body.to_vec(), content_type))
}

pub async fn list_objects_in_s3(
    client: &Client,
    bucket: &str,
    prefix: &str,
    delimiter: Option<&str>,
) -> Result<ListObjectsV2Output, aws_sdk_s3::Error> {
    let result = client
        .list_objects_v2()
        .bucket(bucket)
        .prefix(prefix)
        .delimiter(delimiter.unwrap_or_default())
        .send()
        .await?;
    Ok(result)
}

pub async fn upload_file_to_s3(
    client: &Client,
    bucket: &str,
    key: &str,
    body: ByteStream,
) -> Result<PutObjectOutput, aws_sdk_s3::Error> {
    let result = client
        .put_object()
        .bucket(bucket)
        .key(key)
        .body(body)
        .send()
        .await?;
    Ok(result)
}

pub async fn delete_file_from_s3(
    client: &Client,
    bucket: &str,
    key: &str,
) -> Result<DeleteObjectOutput, aws_sdk_s3::Error> {
    let result = client
        .delete_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await?;
    Ok(result)
}

pub async fn delete_files_from_s3(
    client: &Client,
    bucket: &str,
    keys: Vec<String>,
) -> Result<(), aws_sdk_s3::Error> {
    for key in keys {
        delete_file_from_s3(client, bucket, &key).await?;
    }
    Ok(())
}

pub async fn delete_folder_from_s3(
    client: &Client,
    bucket: &str,
    prefix: &str,
) -> Result<(), aws_sdk_s3::Error> {
    let result = list_objects_in_s3(client, bucket, prefix, None).await?;
    let mut keys = Vec::new();
    if let Some(contents) = result.contents {
        for object in contents {
            if let Some(key) = object.key() {
                keys.push(key.to_string());
            }
        }
    }
    delete_files_from_s3(client, bucket, keys).await?;
    Ok(())
}

pub async fn get_object(
    client: &Client,
    bucket: &str,
    key: &str,
) -> Result<GetObjectOutput, aws_sdk_s3::Error> {
    let result = client.get_object().bucket(bucket).key(key).send().await?;
    Ok(result)
}

pub async fn is_folder(
    client: &Client,
    bucket: &str,
    key: &str,
) -> Result<bool, aws_sdk_s3::Error> {
    if key.is_empty() {
        return Ok(true);
    }
    let prefix = if key.ends_with('/') {
        key.to_string()
    } else {
        format!("{}/", key)
    };
    let result: ListObjectsV2Output = client
        .list_objects_v2()
        .bucket(bucket)
        .prefix(&prefix)
        .max_keys(1)
        .send()
        .await?;
    Ok(result.key_count().unwrap_or_default() > 0)
}
