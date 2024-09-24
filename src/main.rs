use google_cloud_storage::{
    client::{Client as storage_client, ClientConfig as storage_config},
    http::objects::upload::{Media, UploadObjectRequest, UploadType},
};

use google_cloud_storage::http::objects::download::Range;
use google_cloud_storage::http::objects::get::GetObjectRequest;

use google_cloud_kms::{
    client::{Client as kms_client, ClientConfig as kms_config},
    grpc::kms::v1::{DecryptRequest, EncryptRequest},
};
use std::env;

const MEDIA_NAME: &str = "seed.bin";

#[tokio::main]
async fn main() {
    // Collect the arguments into a vector
    let args: Vec<String> = env::args().collect();

    // Check if the user passed the correct number of arguments
    if args.len() != 3 {
        eprintln!("Usage: {} <key_name> <butcket_name>", args[0]);
        return;
    }

    // Parse the first and second arguments as string
    let key_name = &args[1];
    let bucket_name = &args[2];
    println!("Key name: {}, Bucket name: {}", key_name, bucket_name);

    println!("generating a seed and upload it to Google Cloud Storage");
    let seed: [u8; 32] = {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        rng.gen::<[u8; 32]>()
    };

    let encrypted_seed = encrypt_seed(&seed, &key_name).await;
    let decrypted_seed = decrypt_encrypted_seed(&encrypted_seed, &key_name).await;

    //Sanit check: encrypt the seed using the GCP KMS and then decrypt it
    // This should match the original seed.
    assert_eq!(&seed[..], &decrypted_seed[..]);

    // Upload the encrypted seed to GCS
    upload_seed_to_gcs(&encrypted_seed, &bucket_name).await;

    // Download the seed from GCS
    let downloaded_seed = download_seed_from_gcs(&bucket_name).await;

    // Verify the integrity of the downloaded seed
    assert_eq!(&encrypted_seed[..], &downloaded_seed[..]);
}

// Uploads the given seed to the specified GCS bucket.
async fn upload_seed_to_gcs(seed: &[u8], bucket_name: &String) {
    let config = storage_config::default().with_auth().await.unwrap();
    let client = storage_client::new(config);

    let upload_type = UploadType::Simple(Media::new(MEDIA_NAME));
    let _uploaded = client
        .upload_object(
            &UploadObjectRequest {
                bucket: bucket_name.to_string(),
                ..Default::default()
            },
            seed.to_vec(),
            &upload_type,
        )
        .await;
}
// Downloads the seed from the specified GCS bucket.
async fn download_seed_from_gcs(bucket_name: &String) -> Vec<u8> {
    let config = storage_config::default().with_auth().await.unwrap();
    let client = storage_client::new(config);

    // Download the file
    client
        .download_object(
            &GetObjectRequest {
                bucket: bucket_name.to_string(),
                object: MEDIA_NAME.to_string(),
                ..Default::default()
            },
            &Range::default(),
        )
        .await
        .unwrap()
}

// Decrypts the encrypted seed using the GCP KMS.
async fn decrypt_encrypted_seed(encrypted_seed: &[u8], key_name: &String) -> Vec<u8> {
    let config = kms_config::default().with_auth().await.unwrap();
    let client = kms_client::new(config).await.unwrap();

    let request = DecryptRequest {
        name: key_name.to_string(),
        ciphertext: encrypted_seed.to_vec(),
        additional_authenticated_data: vec![],
        ciphertext_crc32c: None,
        additional_authenticated_data_crc32c: None,
    };

    let decrypted_seed = client
        .decrypt(request, None)
        .await
        .expect("Failed to decrypt seed");

    decrypted_seed.plaintext
}

async fn encrypt_seed(_seed: &[u8], key_name: &String) -> Vec<u8> {
    let config = kms_config::default().with_auth().await.unwrap();
    let client = kms_client::new(config).await.unwrap();

    let request = EncryptRequest {
        name: key_name.to_string(),
        plaintext: _seed.to_vec(),
        additional_authenticated_data: vec![],
        plaintext_crc32c: None,
        additional_authenticated_data_crc32c: None,
    };

    let encrypted_seed = client
        .encrypt(request, None)
        .await
        .expect("Failed to encrypt seed");

    encrypted_seed.ciphertext
}
