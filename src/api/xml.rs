pub mod get_object;

pub use get_object::builder as get_object;
use std::fmt;

fn uri<B, O>(bucket_name: B, object_name: O) -> String
where
    B: fmt::Display,
    O: AsRef<[u8]>,
{
    format!(
        "https://{bucket_name}.storage.googleapis.com/{}",
        percent_encoding::percent_encode(object_name.as_ref(), percent_encoding::NON_ALPHANUMERIC),
    )
}
