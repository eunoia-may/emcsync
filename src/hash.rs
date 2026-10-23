//! 资源文件的哈希。
//!
//! 目前只有 SHA-1：Modrinth 用它做指纹反查，结果是小写十六进制。

use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use sha1::{Digest, Sha1};

const BUFFER_SIZE: usize = 64 * 1024;

/// 流式计算文件的 SHA-1，避免把整个 jar 读进内存。
pub fn sha1_hex(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha1::new();
    let mut buffer = [0u8; BUFFER_SIZE];

    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(to_lower_hex(&hasher.finalize()))
}

fn to_lower_hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";

    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push(DIGITS[(byte >> 4) as usize] as char);
        text.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    text
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::sha1_hex;

    fn temp_file(name: &str, content: &[u8]) -> PathBuf {
        let path = std::env::temp_dir().join(format!("emcsync-hash-{}-{name}", std::process::id()));
        fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn hashes_known_content() {
        let path = temp_file("abc.bin", b"abc");
        let digest = sha1_hex(&path).unwrap();
        fs::remove_file(&path).unwrap();

        assert_eq!(digest, "a9993e364706816aba3e25717850c26c9cd0d89d");
    }

    #[test]
    fn hashes_empty_file() {
        let path = temp_file("empty.bin", b"");
        let digest = sha1_hex(&path).unwrap();
        fs::remove_file(&path).unwrap();

        assert_eq!(digest, "da39a3ee5e6b4b0d3255bfef95601890afd80709");
    }
}
