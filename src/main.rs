use lambda_http::{run, service_fn, tracing, Body, Error, Request, RequestExt, Response};
use aws_sdk_kinesis::primitives::Blob;
use aws_sdk_kinesis::operation::put_record::PutRecordError;
use aws_sdk_kinesis::config::http::HttpResponse;
use aws_smithy_runtime_api::client::result::SdkError;
use aws_sdk_kinesis::config::Region;
use serde::{Serialize, Deserialize};
use serde_json;
use chrono::{DateTime, Utc}; // 0.4.15
use std::time::SystemTime;

/// This is the main body for the function.
/// Write your code inside it.
/// There are some code example in the following URLs:
/// - https://github.com/awslabs/aws-lambda-rust-runtime/tree/main/examples
async fn function_handler(event: Request) -> Result<Response<Body>, Error> {
    // Extract some useful information from the request
    let product_id = event
        .query_string_parameters_ref()
        .and_then(|params| params.first("product_id"))
        .unwrap_or("prod1234");

    let product_avail = event
        .query_string_parameters_ref()
        .and_then(|params| params.first("product_avail"))
        .unwrap_or("true");

    let product_is_published = event
        .query_string_parameters_ref()
        .and_then(|params| params.first("product_is_published"))
        .unwrap_or("true");

    let now = SystemTime::now();
    let now: DateTime<Utc> = now.into();
    let event_timestamp = now.to_rfc3339();

    // create event
    let event = QueryEvent{
        product_id: String::from(product_id),
        product_availability: product_avail.parse::<bool>().unwrap(),
        product_is_published: product_is_published.parse::<bool>().unwrap(),
        timestamp : event_timestamp.clone()
    };

    let result: Result<(), SdkError<PutRecordError, HttpResponse>> = put_record(event).await;

    let success = match result {
        Result::Ok(_) => true,
        Result::Err(e) => {
            println!("{:?}", e);
            false
        }
    };

    let message = format!("Hello {product_id}, this is an AWS Lambda HTTP response, product status: {product_is_published}, product availability: {product_avail}, put record result --> {success} @ {event_timestamp}");

    // Return something that implements IntoResponse.
    // It will be serialized to the right response event automatically by the runtime
    let resp = Response::builder()
        .status(200)
        .header("content-type", "text/html")
        .body(message.into())
        .map_err(Box::new)?;
    Ok(resp)
}

async fn put_record(event: QueryEvent) -> Result<(), SdkError<PutRecordError, HttpResponse>> {

    let local_config = aws_config::from_env()
    .region(Region::new("us-east-1"))
    .profile_name("sf-np")
    .load()
    .await;

    let client = aws_sdk_kinesis::Client::new(&local_config);
    let json = serde_json::to_string(&event).unwrap();
    let event_bytes = json.as_bytes();
    let blob = Blob::new(event_bytes);

    client
        .put_record()
        .data(blob)
        .partition_key(event.product_id)
        .stream_arn("arn:aws:kinesis:us-east-1:441128520970:stream/middle-tier-event-stream")
        .send()
        .await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing::init_default_subscriber();
    run(service_fn(function_handler)).await
}

#[derive(Serialize, Deserialize)]
struct QueryEvent {
    product_id : String,
    product_availability : bool,
    product_is_published : bool,
    timestamp : String
    //feature_flags : Vec<(String, String)>
}
