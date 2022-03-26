use crate::{errors::RustusError, RustusResult};
use actix_web::http::header::HeaderValue;
use digest::Digest;

/// Checks if hash-sum of a slice matches the given checksum.
fn checksum_verify(algo: &str, bytes: &[u8], checksum: &[u8]) -> RustusResult<bool> {
    match algo {
        "sha1" => {
            let sum = sha1::Sha1::digest(bytes);
            Ok(sum.as_slice() == checksum)
        }
        "sha256" => {
            let sum = sha2::Sha256::digest(bytes);
            Ok(sum.as_slice() == checksum)
        }
        "sha512" => {
            let sum = sha2::Sha512::digest(bytes);
            Ok(sum.as_slice() == checksum)
        }
        "md5" => {
            let sum = md5::Md5::digest(bytes);
            Ok(sum.as_slice() == checksum)
        }
        _ => Err(RustusError::UnknownHashAlgorithm),
    }
}

/// Verify checksum of a given chunk based on header's value.
///
/// This function decodes given header value.
/// Format of the header is:
/// <algorithm name> <base64 encoded checksum value>
///
/// It tries decode header value to string,
/// splits it in two parts and after decoding base64 checksum
/// verifies it.
///
/// # Errors
///
/// It may return error if header value can't be represented as string,
/// if checksum can't be decoded with base64 or if unknown algorithm is used.
pub fn verify_chunk_checksum(header: &HeaderValue, data: &[u8]) -> RustusResult<bool> {
    if let Ok(val) = header.to_str() {
        let mut split = val.split(' ');
        if let Some(algo) = split.next() {
            if let Some(checksum_base) = split.next() {
                let checksum = base64::decode(checksum_base).map_err(|_| {
                    log::error!("Can't decode checksum value");
                    RustusError::WrongHeaderValue
                })?;
                return checksum_verify(algo, data, checksum.as_slice());
            }
        }
        Err(RustusError::WrongHeaderValue)
    } else {
        log::error!("Can't decode checksum header.");
        Err(RustusError::WrongHeaderValue)
    }
}

#[cfg(test)]
mod tests {
    use super::{checksum_verify, verify_chunk_checksum};
    use actix_web::http::header::HeaderValue;

    #[test]
    fn test_success_checksum_verify() {
        let res = checksum_verify(
            "sha1",
            b"hello",
            b"\xaa\xf4\xc6\x1d\xdc\xc5\xe8\xa2\xda\xbe\xde\x0f;H,\xd9\xae\xa9CM",
        )
        .unwrap();
        assert!(res);
        let res = checksum_verify(
            "sha256",
            b"hello",
            b",\xf2M\xba_\xb0\xa3\x0e&\xe8;*\xc5\xb9\xe2\x9e\x1b\x16\x1e\\\x1f\xa7B^s\x043b\x93\x8b\x98$",
        ).unwrap();
        assert!(res);
        let res = checksum_verify(
            "sha512",
            b"hello",
            b"\x9bq\xd2$\xbdb\xf3x]\x96\xd4j\xd3\xea=s1\x9b\xfb\xc2\x89\x0c\xaa\xda\xe2\xdf\xf7%\x19g<\xa7##\xc3\xd9\x9b\xa5\xc1\x1d|z\xccn\x14\xb8\xc5\xda\x0cFcG\\.\\:\xde\xf4os\xbc\xde\xc0C",
        ).unwrap();
        assert!(res);
        let res =
            checksum_verify("md5", b"hello", b"]A@*\xbcK*v\xb9q\x9d\x91\x10\x17\xc5\x92").unwrap();
        assert!(res);
    }

    #[test]
    fn test_sum_unknown_algo_checksum_verify() {
        let res = checksum_verify("base64", "test".as_bytes(), b"dGVzdAo=");
        assert!(res.is_err());
    }

    #[test]
    fn test_success_verify_chunk_checksum() {
        let res = verify_chunk_checksum(
            &HeaderValue::from_str("md5 XUFAKrxLKna5cZ2REBfFkg==").unwrap(),
            b"hello",
        )
        .unwrap();
        assert!(res);
    }

    #[test]
    fn test_wrong_checksum() {
        let res = verify_chunk_checksum(&HeaderValue::from_str("md5 memes==").unwrap(), b"hello");
        assert!(res.is_err());
    }

    #[test]
    fn test_bytes_header() {
        let res = verify_chunk_checksum(
            &HeaderValue::from_bytes(b"ewq ]A@*\xbcK*v").unwrap(),
            b"hello",
        );
        assert!(res.is_err());
    }

    #[test]
    fn test_badly_formatted_header() {
        let res = verify_chunk_checksum(&HeaderValue::from_str("md5").unwrap(), b"hello");
        assert!(res.is_err());
    }
}
