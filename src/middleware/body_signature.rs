use crate::error::request_error::RequestError;
use crate::state::rank_config_state::RankConfigState;
use axum::extract::State;
use axum::{
    body::Body, extract::Request, http::HeaderMap, middleware::Next, response::IntoResponse,
};
use http_body_util::BodyExt;

use crate::utils::encrypt;

// use crate::utils::rencrypt::{base64_encode, md5_hash};

// pub fn body_signature_verify_fn(headers: &HeaderMap, body: &String) -> bool {
//     let signature = match headers.get("signature") {
//         Some(value) => value.to_str().unwrap_or_default().to_owned(),
//         None => return false,
//     };
//     tracing::debug!("body_signature_verify - signature:{}", signature);
//     let raw_data = format!("{body}{APPID}");
//     tracing::debug!("body_signature_verify - raw:{}", raw_data);
//     let base64 = base64_encode(&raw_data);
//     tracing::debug!("body_signature_verify - base64:{}", base64);
//     let md5 = md5_hash(&base64);
//     tracing::debug!("body_signature_verify - md5:{}", md5);
//     if signature == md5_hash(&base64_encode(&raw_data)) {
//         true
//     } else {
//         false
//     }
// }

// middleware
pub async fn body_signature_verify(
    State(state): State<RankConfigState>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<impl IntoResponse, RequestError> {
	return Ok(next.run(request).await);
    let signature = match headers.get("signature") {
        Some(value) => value.to_str().unwrap_or_default().to_owned(),
        None => String::new(),
    };
    if signature.is_empty() {
        return Err(RequestError::SignatureError);
    }
    // 提取body进行签名验证
    let request = buffer_request_body(&state, &signature, request).await?;
    Ok(next.run(request).await)
}

async fn buffer_request_body(
    state: &RankConfigState,
    signature: &String,
    request: Request,
) -> Result<Request, RequestError> {
    let (parts, body) = request.into_parts();
    let headers = &parts.headers;
    if !headers.contains_key("appid") {
        return Err(RequestError::SignatureError);
    }
    if headers["appid"].to_str().is_err() {
        return Err(RequestError::SignatureError);
    }
    let appid = headers["appid"].to_str().unwrap();

    let secret;
    {
        let secret_map = state
            .rank_config_service
            .rank_config_secret_map
            .read()
            .await;
        if secret_map.contains_key(appid) {
            secret = secret_map[appid].clone();
        } else {
            return Err(RequestError::SignatureError);
        }
    }
    // this wont work if the body is an long running stream
    let bytes = body
        .collect()
        .await
        .map_err(|err| {
            tracing::error!("req body get error,error:{}", err.to_string());
            RequestError::SignatureError
        })?
        .to_bytes();

    let body_str = String::from_utf8_lossy(&bytes);
    // tracing::debug!("body_signature_verify - signature:{}", signature);
    let mut raw_data = format!("{body_str}{secret}");
    raw_data = raw_data.chars().filter(|c| !c.is_whitespace()).collect();
    // tracing::debug!("body_signature_verify - raw:{}", raw_data);
    // let base64 = encrypt::base64_encode(&raw_data);
    // tracing::debug!("body_signature_verify - base64:{}", base64);
    // let md5 = encrypt::md5_hash(&base64);
    // tracing::debug!("body_signature_verify - md5:{}", md5);
    if *signature == encrypt::md5_hash(&encrypt::base64_encode(&raw_data)) {
        Ok(Request::from_parts(parts, Body::from(bytes)))
    } else {
        Err(RequestError::SignatureError)
    }
}
