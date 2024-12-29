use base64::prelude::{Engine, BASE64_STANDARD};
use http::{HeaderName, HeaderValue};

// https://cloud.google.com/storage/docs/xml-api/reference-headers#xgooghash
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct XGoogHash {
    pub crc32c: Option<[u8; 4]>,
    pub md5: Option<[u8; 16]>,
}

impl headers::Header for XGoogHash {
    fn name() -> &'static HeaderName {
        static NAME: HeaderName = HeaderName::from_static("x-goog-hash");
        &NAME
    }

    fn decode<'i, I>(values: &mut I) -> Result<Self, headers::Error>
    where
        Self: Sized,
        I: Iterator<Item = &'i HeaderValue>,
    {
        values.try_fold(
            Self {
                crc32c: None,
                md5: None,
            },
            |mut this, value| {
                if let Some(value) = value.as_bytes().strip_prefix(b"crc32c=") {
                    this.crc32c = Some(
                        BASE64_STANDARD
                            .decode(value)
                            .map_err(|_| headers::Error::invalid())?
                            .try_into()
                            .map_err(|_| headers::Error::invalid())?,
                    );
                }
                if let Some(value) = value.as_bytes().strip_prefix(b"md5=") {
                    this.md5 = Some(
                        BASE64_STANDARD
                            .decode(value)
                            .map_err(|_| headers::Error::invalid())?
                            .try_into()
                            .map_err(|_| headers::Error::invalid())?,
                    );
                }
                Ok(this)
            },
        )
    }

    fn encode<E>(&self, values: &mut E)
    where
        E: Extend<HeaderValue>,
    {
        let crc32c = self.crc32c.iter().map(|value| {
            HeaderValue::from_str(&format!("crc32c={}", BASE64_STANDARD.encode(value))).unwrap()
        });
        let md5 = self.md5.iter().map(|value| {
            HeaderValue::from_str(&format!("md5={}", BASE64_STANDARD.encode(value))).unwrap()
        });
        values.extend(crc32c.chain(md5));
    }
}

#[cfg(test)]
mod tests {
    use headers::HeaderMapExt;
    use http::{HeaderMap, HeaderValue};

    #[test]
    fn test_x_goo_hash_decode() {
        {
            let headers = HeaderMap::new();
            assert_eq!(headers.typed_get::<super::XGoogHash>(), None);
        }
        {
            let mut headers = HeaderMap::new();
            headers.append("x-goog-hash", HeaderValue::from_static("crc32c=n03x6A=="));
            assert_eq!(
                headers.typed_get::<super::XGoogHash>(),
                Some(super::XGoogHash {
                    crc32c: Some(hex_literal::hex!("9f4df1e8")),
                    ..super::XGoogHash::default()
                }),
            );
        }
        {
            let mut headers = HeaderMap::new();
            headers.append(
                "x-goog-hash",
                HeaderValue::from_static("md5=Ojk9c3dhfxgoKVVHYwFbHQ=="),
            );
            assert_eq!(
                headers.typed_get(),
                Some(super::XGoogHash {
                    md5: Some(hex_literal::hex!("3a393d7377617f182829554763015b1d")),
                    ..super::XGoogHash::default()
                }),
            );
        }
        {
            let mut headers = HeaderMap::new();
            headers.append("x-goog-hash", HeaderValue::from_static("crc32c=n03x6A=="));
            headers.append(
                "x-goog-hash",
                HeaderValue::from_static("md5=Ojk9c3dhfxgoKVVHYwFbHQ=="),
            );
            assert_eq!(
                headers.typed_get(),
                Some(super::XGoogHash {
                    crc32c: Some(hex_literal::hex!("9f4df1e8")),
                    md5: Some(hex_literal::hex!("3a393d7377617f182829554763015b1d")),
                }),
            );
        }
    }

    #[test]
    fn test_x_goo_hash_encode() {
        {
            let mut headers = HeaderMap::new();
            headers.typed_insert(super::XGoogHash {
                crc32c: Some(hex_literal::hex!("9f4df1e8")),
                md5: Some(hex_literal::hex!("3a393d7377617f182829554763015b1d")),
            });
            let mut values = headers.get_all("x-goog-hash").into_iter();
            assert_eq!(
                values.next(),
                Some(&HeaderValue::from_static("crc32c=n03x6A==")),
            );
            assert_eq!(
                values.next(),
                Some(&HeaderValue::from_static("md5=Ojk9c3dhfxgoKVVHYwFbHQ==")),
            );
        }
    }
}
