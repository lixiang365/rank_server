use base64::{engine::general_purpose, Engine as _};
use bcrypt::DEFAULT_COST;

// 计算MD5
pub fn md5_hash(pwd: &str) -> String {
    let digest = md5::compute(pwd);
    format!("{:?}", digest)
}

// 验证MD5
pub fn md5_verify(pwd: &str, md5_pwd: &str) -> bool {
    format!("{:?}", md5::compute(pwd)) == md5_pwd
}

// base64 编码
pub fn base64_encode(data: &String) -> String {
    general_purpose::STANDARD.encode(data)
}

// base64 解码
pub fn base64_decode(data: &String) -> Result<String, String> {
    match general_purpose::STANDARD.decode(data) {
        Ok(decode_vec) => {
            if let Ok(decode_str) = String::from_utf8(decode_vec) {
                Ok(decode_str)
            } else {
                Err("base64 decode fiailed".to_string())
            }
        }
        Err(err) => Err(err.to_string()),
    }
}
