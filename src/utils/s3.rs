use aws_sdk_s3::Client as S3Client;
use aws_config::ConfigLoader;
use aws_types::region::Region;
use aws_config::BehaviorVersion; // Import BehaviorVersion

pub async fn create_s3_client() -> S3Client {
    let aws_config = ConfigLoader::default()
        .region(std::env::var("AWS_REGION").ok().map(Region::new))
        .behavior_version(BehaviorVersion::latest()) // Set behavior version here
        .load()
        .await;

    S3Client::new(&aws_config)
}