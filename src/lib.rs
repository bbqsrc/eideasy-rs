pub mod cli;

use std::{
    io::Read,
    path::{Path, PathBuf},
};

use file_upload_req::File;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

const URL_FILE_UPLOAD: &str = "https://id.eideasy.com/api/signatures/prepare-files-for-signing";
const URL_FILE_DOWNLOAD: &str = "https://id.eideasy.com/api/signatures/download-signed-file";
const URL_CREATE_QUEUE: &str = "https://id.eideasy.com/api/signatures/signing-queues";

fn url_push_signers(signing_queue_id: &str) -> String {
    format!("https://id.eideasy.com/api/signatures/signing-queues/{signing_queue_id}/signers/batch")
}

fn url_run_queue(signing_queue_id: &str) -> String {
    format!("https://id.eideasy.com/api/signatures/signing-queues/{signing_queue_id}/run")
}

#[derive(Debug, Serialize)]
struct CreateQueueRequest {
    client_id: String,
    secret: String,
    has_management_page: bool,
    doc_id: String,
    owner_email: String,
}

#[derive(Debug, Deserialize)]
struct CreateQueueResponse {
    id: usize,
    signing_queue_secret: String,
    management_page_url: String,
}

#[derive(Debug, Serialize)]
struct Signer {
    email: String,
    name: String,
}

async fn push_signers(
    signing_queue_id: &str,
    signing_queue_secret: String,
    signers: Vec<Signer>,
) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let res: serde_json::Value = client
        .post(url_push_signers(signing_queue_id))
        .json(&serde_json::json!({ "signers": signers }))
        .header("Authorization", format!("Bearer {signing_queue_secret}"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    println!("{:#?}", res);
    Ok(())
}

async fn run_queue(signing_queue_id: &str, signing_queue_secret: String) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let res: serde_json::Value = client
        .post(url_run_queue(signing_queue_id))
        .header("Authorization", format!("Bearer {signing_queue_secret}"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    println!("{:#?}", res);
    Ok(())
}

async fn create_queue(
    client_id: String,
    secret: String,
    doc_id: String,
    owner_email: String,
) -> anyhow::Result<()> {
    let req = CreateQueueRequest {
        client_id,
        secret,
        has_management_page: true,
        doc_id,
        owner_email,
    };

    let client = reqwest::Client::new();
    let res: CreateQueueResponse = client
        .post(URL_CREATE_QUEUE)
        .json(&req)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    println!("{:#?}", res);
    Ok(())
}

async fn download(
    client_id: String,
    secret: String,
    doc_id: String,
    output_path: &Path,
) -> anyhow::Result<()> {
    let req = FileDownloadRequest {
        doc_id,
        client_id,
        secret,
    };

    let client = reqwest::Client::new();
    let mut res: FileDownloadResponse = client
        .post(URL_FILE_DOWNLOAD)
        .json(&req)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let mut file_data = vec![];
    std::mem::swap(&mut file_data, &mut res.signed_file_contents);

    println!("{:#?}", res);
    std::fs::write(output_path, file_data)?;

    Ok(())
}

async fn upload(client_id: String, secret: String, files: Vec<PathBuf>) -> anyhow::Result<()> {
    let req = FileUploadRequest {
        client_id: client_id.to_string(),
        secret,
        container_type: "pdf".into(),
        files: files
            .into_iter()
            .map(|path| {
                let file_name = path
                    .file_name()
                    .ok_or_else(|| anyhow::anyhow!("No filename"))?
                    .to_string_lossy()
                    .to_string();

                let mut file_content = vec![];
                std::fs::File::open(&path)?.read_to_end(&mut file_content)?;

                Ok::<_, anyhow::Error>(File {
                    file_content,
                    file_name,
                    mime_type: mime_guess::from_path(&path)
                        .first_or_octet_stream()
                        .to_string(),
                })
            })
            .collect::<Result<Vec<File>, _>>()?,
    };

    let client = reqwest::Client::new();
    let res: FileUploadResponse = client
        .post(URL_FILE_UPLOAD)
        .json(&req)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    println!("{:#?}", res);
    println!(
        "https://id.eideasy.com/sign_contract_external?client_id={client_id}&doc_id={}",
        res.doc_id
    );

    // println!("{}", serde_json::to_string_pretty(&req)?);

    Ok(())
}

fn as_base64<S>(data: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&base64::encode(data))
}

fn from_base64<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    String::deserialize(deserializer)
        .and_then(|string| base64::decode(&string).map_err(|err| Error::custom(err.to_string())))
}

#[derive(Debug, Serialize)]
struct FileUploadRequest {
    files: Vec<file_upload_req::File>,
    client_id: String,
    secret: String,
    container_type: String,
    // allowed_signature_levels: Vec<String>,
    // signature_redirect: String,
}

#[derive(Debug, Serialize)]
struct FileDownloadRequest {
    doc_id: String,
    client_id: String,
    secret: String,
}

#[derive(Debug, Deserialize)]
struct FileUploadResponse {
    status: String,
    doc_id: String,
    // signature_redirect: String,
}

#[derive(Debug, Deserialize)]
struct FileDownloadResponse {
    #[serde(deserialize_with = "from_base64")]
    signed_file_contents: Vec<u8>,
    signer_country: String,
    signer_idcode: String,
    signer_lastname: String,
    signer_firstname: String,
    signing_method: String,
    status: String,
    #[serde(default)]
    verification_level: Option<String>,
    // signature_redirect: String,
}

mod file_upload_req {
    use serde::Serialize;

    #[derive(Debug, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct File {
        #[serde(serialize_with = "super::as_base64")]
        pub file_content: Vec<u8>,
        pub file_name: String,
        pub mime_type: String,
    }
}
