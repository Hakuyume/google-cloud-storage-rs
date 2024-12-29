pub mod patch_object;

use std::fmt;

fn uri<B, O>(bucket_name: B, object_name: O) -> String
where
    B: fmt::Display,
    O: AsRef<[u8]>,
{
    format!(
        "https://storage.googleapis.com/storage/v1/b/{bucket_name}/o/{}",
        percent_encoding::percent_encode(object_name.as_ref(), percent_encoding::NON_ALPHANUMERIC),
    )
}
