use anyhow::Result;
use axum::body::Body;
use axum::extract::{Extension, Json, Request, State};
use axum::http::header::{AUTHORIZATION, CONTENT_TYPE};
use axum::http::{Method, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::Router;
use axum_server::tls_rustls::RustlsConfig;
use rand::distributions::{Alphanumeric, DistString};
use rand::RngCore;
use serde::Deserialize;
use serde_json::{json, Value};
use tower_http::services::ServeDir;

use crate::config::CONFIG;
use crate::database::{Database, SdkAccount};
use crate::time;

#[derive(Clone)]
pub struct SdkState {
    db: Database,
}

#[derive(Clone, Default)]
struct AuthInfo {
    uid: Option<String>,
    token: Option<String>,
}

pub async fn serve(db: Database) -> Result<()> {
    let state = SdkState { db };
    let app = Router::new()
        .route("/", get(index))
        .route("/common/config", post(common_config))
        .route("/common/client-info", post(client_info))
        .route("/common/client-code", post(client_code))
        .route("/yostar/send-code", post(send_code))
        .route("/yostar/get-auth", post(get_auth))
        .route("/common/client-log", post(client_log))
        .route("/user/login", post(login_user))
        .route("/user/detail", post(user_detail))
        .route("/user/quick-login", post(quick_login))
        .route("/user/pgs-oauth-code", post(pgs_oauth_code))
        .route("/common/version", post(common_version))
        .route("/user/set", post(user_set))
        .nest_service("/static", ServeDir::new("assets/static"))
        .layer(middleware::from_fn(auth_middleware))
        .with_state(state);

    let tls =
        RustlsConfig::from_pem_file(CONFIG.tls_cert_path.clone(), CONFIG.tls_key_path.clone())
            .await?;

    let http_app = app.clone();
    let http = tokio::spawn(async move {
        tracing::info!("SDK HTTP listening on {}", CONFIG.sdk_http_addr);
        axum_server::bind(CONFIG.sdk_http_addr)
            .serve(http_app.into_make_service())
            .await
    });

    let https = tokio::spawn(async move {
        tracing::info!("SDK HTTPS listening on {}", CONFIG.sdk_https_addr);
        axum_server::bind_rustls(CONFIG.sdk_https_addr, tls)
            .serve(app.into_make_service())
            .await
    });

    http.await??;
    https.await??;
    Ok(())
}

async fn auth_middleware(mut req: Request<Body>, next: Next) -> Response {
    let path = req.uri().path();
    tracing::debug!(
        service = "sdk",
        method = %req.method(),
        path,
        proto = "http/json",
        "sdk request"
    );

    if (req.method() == Method::GET && path == "/")
        || path == "/common/config"
        || path == "/static"
        || path.starts_with("/static/")
    {
        return next.run(req).await;
    }

    let Some(value) = req.headers().get(AUTHORIZATION) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let Ok(value) = value.to_str() else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let Ok(auth) = serde_json::from_str::<AuthorizationHeader>(value) else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    req.extensions_mut().insert(AuthInfo {
        uid: auth.head.uid,
        token: auth.head.token,
    });
    next.run(req).await
}

async fn index() -> Response {
    api(200, json!("Hello world"), "OK")
}

async fn common_config(Json(req): Json<CommonConfigReq>) -> Response {
    if req.store_name != "googleplay" {
        return StatusCode::BAD_REQUEST.into_response();
    }

    json_body(COMMON_CONFIG)
}

async fn client_info() -> Response {
    api(200, json!({ "EuropeUnion": false }), "OK")
}

async fn client_code() -> Response {
    json_body(CLIENT_CODE)
}

async fn send_code() -> Response {
    api(200, json!({}), "OK")
}

async fn get_auth(Json(req): Json<GetAuthReq>) -> Response {
    if req.code != "114514" {
        return api(100303, json!({}), "获取授权信息失败,错误代码:%!d(string=3)");
    }

    api(
        200,
        json!({
            "UID": req.account,
            "Token": Alphanumeric.sample_string(&mut rand::thread_rng(), 77),
            "Account": req.account,
        }),
        "OK",
    )
}

async fn client_log() -> Response {
    api(200, json!({}), "OK")
}

async fn pgs_oauth_code() -> Response {
    api(
        100811,
        json!({}),
        "PGS获取Code信息失败:The provided client secret is invalid.",
    )
}

async fn login_user(State(state): State<SdkState>, Json(req): Json<LoginReq>) -> Response {
    if req.check_account != 0 && req.check_account != 1 {
        return StatusCode::BAD_REQUEST.into_response();
    }

    match state.db.get_sdk_account_by_user_name(&req.user_name).await {
        Ok(Some(account)) => {
            let is_new = account.is_new;
            let response = sdk_user_response(&account, is_new);
            if is_new != 0 {
                if let Err(err) = state.db.clear_sdk_new_flag(account.uid as u32).await {
                    tracing::error!("clear sdk new flag failed: {err}");
                }
            }
            response
        }
        Ok(None) => {
            let token = random_hex(20);
            match state
                .db
                .create_sdk_account(&req.user_name, &token, time::now_timestamp_s())
                .await
            {
                Ok(_) => api(100900, json!({}), "新账号"),
                Err(err) => {
                    tracing::error!("create sdk user failed: {err}");
                    StatusCode::INTERNAL_SERVER_ERROR.into_response()
                }
            }
        }
        Err(err) => {
            tracing::error!("load sdk user failed: {err}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn user_detail(
    State(state): State<SdkState>,
    Extension(auth): Extension<AuthInfo>,
) -> Response {
    sdk_detail(state, auth).await
}

async fn quick_login(
    State(state): State<SdkState>,
    Extension(auth): Extension<AuthInfo>,
) -> Response {
    sdk_detail(state, auth).await
}

async fn sdk_detail(state: SdkState, auth: AuthInfo) -> Response {
    let Some(uid) = auth.uid.as_deref().and_then(|uid| uid.parse::<u32>().ok()) else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };

    match state.db.get_sdk_account(uid).await {
        Ok(Some(account)) => sdk_detail_response(&account, auth.token.unwrap_or_default()),
        Ok(None) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
        Err(err) => {
            tracing::error!("load sdk user detail failed: {err}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

fn sdk_detail_response(account: &SdkAccount, token: String) -> Response {
    api(
        200,
        json!({
            "AgeVerifyMethod": 0,
            "Destroy": null,
            "IsTestAccount": false,
            "Keys": [{
                "ID": account.uid.to_string(),
                "Type": "yostar",
                "Key": account.user_name,
                "NickName": account.nick_name,
                "CreatedAt": account.created_at,
            }],
            "ServerNowAt": time::now_timestamp_s(),
            "UserInfo": user_info(account, token),
            "Yostar": yostar_info(account),
            "YostarDestroy": null,
        }),
        "OK",
    )
}

async fn common_version(Json(req): Json<VersionReq>) -> Response {
    if req.types.iter().any(|ty| {
        !matches!(
            ty.as_str(),
            "user_agreement" | "privacy_agreement" | "credit_investigation"
        )
    }) {
        return StatusCode::BAD_REQUEST.into_response();
    }

    api(
        200,
        json!({
            "Agreement": [
                {
                    "Version": "0.1",
                    "Type": "privacy_agreement",
                    "Title": "隐私政策",
                    "Content": "",
                    "Lang": "ja"
                },
                {
                    "Version": "0.1",
                    "Type": "user_agreement",
                    "Title": "用户协议",
                    "Content": "",
                    "Lang": "ja"
                }
            ],
            "ErrorCode": "5.5"
        }),
        "OK",
    )
}

async fn user_set(
    State(state): State<SdkState>,
    Extension(auth): Extension<AuthInfo>,
    Json(req): Json<UserSetReq>,
) -> Response {
    if req.key != "Nickname" {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let Some(uid) = auth.uid.as_deref().and_then(|uid| uid.parse::<u32>().ok()) else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };

    match state.db.update_sdk_nickname(uid, &req.value).await {
        Ok(true) => api(200, json!({}), "OK"),
        Ok(false) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
        Err(err) => {
            tracing::error!("update sdk nickname failed: {err}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

fn sdk_user_response(account: &SdkAccount, is_new: i64) -> Response {
    api(
        200,
        json!({
            "AgeVerifyMethod": 0,
            "IsNew": is_new,
            "UserInfo": user_info(account, account.token.clone()),
            "Yostar": yostar_info(account),
        }),
        "OK",
    )
}

fn user_info(account: &SdkAccount, token: String) -> Value {
    json!({
        "ID": account.uid.to_string(),
        "UID2": 0,
        "PID": "JP-AZURLANE",
        "Token": token,
        "Birthday": "",
        "RegChannel": "googleplay",
        "TransCode": "",
        "State": 1,
        "DeviceID": "",
        "CreatedAt": account.created_at,
    })
}

fn yostar_info(account: &SdkAccount) -> Value {
    json!({
        "ID": format!("Y{}", account.uid),
        "Country": "JP",
        "Nickname": account.nick_name,
        "Picture": "",
        "State": 1,
        "AgreeAd": 0,
        "CreatedAt": account.created_at,
    })
}

fn api(code: i32, data: Value, msg: &str) -> Response {
    Json(json!({
        "Code": code,
        "Data": data,
        "Msg": msg,
    }))
    .into_response()
}

fn json_body(body: &'static str) -> Response {
    ([(CONTENT_TYPE, "application/json")], body).into_response()
}

fn random_hex(bytes: usize) -> String {
    let mut data = vec![0; bytes];
    rand::thread_rng().fill_bytes(&mut data);
    data.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[derive(Deserialize)]
struct AuthorizationHeader {
    #[serde(rename = "Head")]
    head: AuthorizationHead,
}

#[derive(Deserialize)]
struct AuthorizationHead {
    #[serde(rename = "UID")]
    uid: Option<String>,
    #[serde(rename = "Token")]
    token: Option<String>,
}

#[derive(Deserialize)]
struct CommonConfigReq {
    #[serde(rename = "StoreName")]
    store_name: String,
}

#[derive(Deserialize)]
struct GetAuthReq {
    #[serde(rename = "Account")]
    account: String,
    #[serde(rename = "Code")]
    code: String,
}

#[derive(Deserialize)]
struct LoginReq {
    #[serde(rename = "CheckAccount")]
    check_account: i32,
    #[serde(rename = "UserName")]
    user_name: String,
}

#[derive(Deserialize)]
struct VersionReq {
    #[serde(rename = "Type")]
    types: Vec<String>,
}

#[derive(Deserialize)]
struct UserSetReq {
    #[serde(rename = "Key")]
    key: String,
    #[serde(rename = "Value")]
    value: String,
}

const COMMON_CONFIG: &str = r#"{
    "Code": 200,
    "Data": {
        "AppConfig": {
            "ACCOUNT_RETRIEVAL": {
                "FIRST_LOGIN_POPUP": true,
                "LOGIN_POPUP": true,
                "PAGE_URL": "https://migration.yo-star.com"
            },
            "AGREEMENT_POPUP_TYPE": "Browser",
            "APPLE_CURRENCY_BLOCK_LIST": null,
            "APP_CLIENT_LANG": [
                "en"
            ],
            "APP_DEBUG": 0,
            "APP_GL": "en",
            "AUTO_LOGIN_URL": null,
            "BIND_METHOD": [
                "google",
                "apple_hk",
                "facebook",
                "steam"
            ],
            "CAPTCHA_ENABLED": false,
            "CLIENT_LOG_REPORTING": {
                "ENABLE": true
            },
            "CREDIT_INVESTIGATION": "0.0",
            "DESTROY_USER_DAYS": 15,
            "DESTROY_USER_ENABLE": 1,
            "DETECTION_ADDRESS": {
                "AUTO": {
                    "DNS": [
                        "https://blhxusstatic.yo-star.com/img_test1.png",
                        "https://en-sdk-api.yostarplat.com",
                        "http://blhxusgate.yo-star.com/?cmd=test?"
                    ],
                    "HTTP": [
                        "https://blhxusstatic.yo-star.com/img_test1.png",
                        "https://en-sdk-api.yostarplat.com",
                        "http://blhxusgate.yo-star.com/?cmd=test?"
                    ],
                    "MTR": [
                        "https://blhxusstatic.yo-star.com/img_test1.png",
                        "https://en-sdk-api.yostarplat.com",
                        "http://blhxusgate.yo-star.com/?cmd=test?"
                    ],
                    "PING": [
                        "https://blhxusstatic.yo-star.com/img_test1.png",
                        "https://en-sdk-api.yostarplat.com",
                        "http://blhxusgate.yo-star.com/?cmd=test?"
                    ],
                    "TCP": [
                        "https://blhxusstatic.yo-star.com/img_test1.png",
                        "https://en-sdk-api.yostarplat.com",
                        "http://blhxusgate.yo-star.com/?cmd=test?"
                    ]
                },
                "ENABLE": true,
                "ENABLE_MANUAL": true,
                "INTERNET": "https://www.google.com",
                "INTERNET_ADDRESS": "https://www.google.com",
                "NETWORK_ENDPORINT": "https://ap-southeast-1.log.aliyuncs.com",
                "NETWORK_PROJECT": "yostar-oversea-netsdk-logging",
                "NETWORK_SECRET_KEY": "eyJhbGl5dW5fdWlkIjoiMTI5NDA1ODU3MDYyMTk5MCIsImlwYV9hcHBfaWQiOiJMNFFSSG1zNzdqdW5WSGNCWTZVd1ZLIiwic2VjX2tleSI6ImM2YzAxZDZhNTZkZjdlMTY3Yjg2MmFjM2EwYzQ5MTJlN2RmZTRmNjIxMTc1YTZkOGI5ZjcxYWJhYWY2YWNjYmQ2MTg1ZjVmMmYxMTVkMTczNjg5MGRlYWU0Nzg0MTI0NzFmZGNjMmRlOWUwMWMyNmJhOTdmZDA0YTJkM2IxZjUwIiwic2lnbiI6ImQxOGQwZTc0YjFhYWIzZmVlYWNmNDY2ZTYyYjQyMDZmYzA4NWFmMjJiN2ZjODQ1MDYzMjM3MDNlOGVkOGUxNGU5ZWI0ZGM3YjllOTFiNzE3NmUxZTBmYjBhOTU1OWQxMTFhM2QyMzU2YTQyNWQ1YTlkNGI1ZWMxMWQxYjY0NTBjIn0="
            },
            "ENABLE_AGREEMENT": true,
            "ENABLE_MULTI_LANG_AGREEMENT": false,
            "ENABLE_TEXT_REVIEW": true,
            "ERROR_CODE": "5.3",
            "FILE_DOMAIN": "https://storage.googleapis.com/sdkplat-en-prod",
            "GEETEST_ENABLE": false,
            "GEETEST_ID": "",
            "GOOGLE_ANALYTICS_MEASUREMENT_ID": "",
            "MIGRATE_POPUP": true,
            "NICKNAME_REG": "^[A-Za-z0-9]{2,20}$",
            "PASSPORT_DESTROY_DAYS": 15,
            "POPUP": {
                "Data": [
                    {
                        "Lang": "ja",
                        "Text": "Yostar IDを作成"
                    },
                    {
                        "Lang": "en",
                        "Text": "Create a Yostar account"
                    },
                    {
                        "Lang": "kr",
                        "Text": "YOSTAR 계정 가입하기"
                    },
                    {
                        "Lang": "fr",
                        "Text": "Créez votre compte Yostar"
                    },
                    {
                        "Lang": "de",
                        "Text": "Einen Yostar-Account erstellen"
                    }
                ],
                "Enable": true
            },
            "PRIVACY_AGREEMENT": "0.2",
            "RECHARGE_LIMIT": {
                "Enable": false,
                "IsOneLimit": false,
                "Items": [],
                "OneLimitAmount": 0
            },
            "SHARE": {
                "CaptureScreen": {
                    "AutoCloseDelay": 0,
                    "Enabled": false
                },
                "Facebook": {
                    "AppID": "",
                    "Enabled": true
                },
                "Instagram": {
                    "Enabled": true
                },
                "Kakao": {
                    "AppKey": "",
                    "Enabled": false
                },
                "Naver": {
                    "Enabled": false
                },
                "Twitter": {
                    "Enabled": true
                }
            },
            "SLS": {
                "ACCESS_KEY_ID": "7b5d0ffd0943f26704fc547a871c68b1b5d56b5c9caeb354205b81f445d7af59",
                "ACCESS_KEY_SECRET": "4a5e9cc8a50819290c9bfa1fedc79da7c50e85189a05eb462a3d28a7688eabb0",
                "ENABLE": true
            },
            "SURVEY_POPUP_TYPE": "Browser",
            "UDATA": {
                "Enable": false,
                "URL": ""
            },
            "USER_AGREEMENT": "0.1",
            "YOSTAR_PREFIX": "yoyo"
        },
        "EuropeUnion": false,
        "StoreConfig": {
            "ADJUST_APPID": "9ldfqpo1zx1c",
            "ADJUST_CHARGEEVENTTOKEN": "",
            "ADJUST_ENABLED": 1,
            "ADJUST_EVENTTOKENS": {
                "7d_retention": "dhtuhc",
                "account_create": "binqg9",
                "behavior_check": "wgjks1",
                "behavior_verification_failed": "q6psau",
                "completed_registration": "9og2k7",
                "purchase": "wopxnk",
                "purchase_click": "v3vmv8",
                "purchase_click_diamond": "bmrzqo",
                "purchase_click_giftbag": "xt7jbc",
                "purchase_click_monthlycard": "b3z5at",
                "purchase_first": "mqg9kg",
                "role_create": "2j5npu",
                "role_levelup": "h3ohkb",
                "role_login": "ax1v0t",
                "role_logout": "cmyuyx",
                "tutorial_complete": "d5ydk2",
                "tutorial_complete_1": "b3vh0x",
                "tutorial_complete_2": "fa1z6r",
                "tutorial_complete_3": "2gdwsh",
                "tutorial_complete_4": "pmg7ny",
                "user_charge": "wpf4hu",
                "user_charge_first": "u912no",
                "ysdk_err": "dmpp3y"
            },
            "ADJUST_ISDEBUG": 0,
            "AIRWALLEX_ENABLED": false,
            "AI_HELP": {
                "AihelpAppID": "yostar_platform_236d3992-13c7-4cad-ab05-025353a464f4",
                "AihelpAppKey": "YOSTAR_app_a9bba09f659e44a19dfc9a395ea0cb25",
                "AihelpDomain": "yostar.aihelp.net",
                "CustomerEmailAddr": "",
                "CustomerServiceURL": "",
                "CustomerWay": 1,
                "DisplayType": "Browser",
                "Enable": 1,
                "Mode": "robot"
            },
            "CODA_ENABLED": false,
            "ENABLED_PAY": {
                "AIRWALLEX_ENABLED": false,
                "CODA_ENABLED": false,
                "GMOAlipay": false,
                "GMOAu": false,
                "GMOCreditcard": false,
                "GMOCvs": false,
                "GMODocomo": false,
                "GMOPaypal": false,
                "GMOPaypay": false,
                "GMOSoftbank": false,
                "MYCARD_ENABLED": false,
                "PAYPAL_ENABLED": false,
                "PINGPONG_ENABLED": false,
                "RAZER_ENABLED": false,
                "STEAM_ENABLED": false,
                "STRIPE_ENABLED": false,
                "TOSS_ENABLED": false,
                "WEBMONEY_ENABLED": false
            },
            "FACEBOOK_APPID": "962529950581548",
            "FACEBOOK_CLIENT_TOKEN": "",
            "FACEBOOK_SECRET": "9f0aeefa566b78173ecddb45e500c635",
            "FIREBASE_ENABLED": 1,
            "GMO_CC_JS": "https://",
            "GMO_CC_KEY": "",
            "GMO_CC_SHOPID": "",
            "GMO_PAY_CHANNEL": {
                "GMOAlipay": false,
                "GMOAu": false,
                "GMOCreditcard": false,
                "GMOCvs": false,
                "GMODocomo": false,
                "GMOPaypal": false,
                "GMOPaypay": false,
                "GMOSoftbank": false
            },
            "GMO_PAY_ENABLED": false,
            "GOOGLE_CLIENT_ID": "150452741869-sojo18oh2am6u573v5achpi78kau4s52.apps.googleusercontent.com",
            "GUEST_CREATE_METHOD": 0,
            "GUIDE_POPUP": {
                "DATA": null,
                "ENABLE": 0
            },
            "GoogleClientID": "150452741869-sojo18oh2am6u573v5achpi78kau4s52.apps.googleusercontent.com",
            "LOGIN": {
                "DEFAULT": "yostar",
                "ICON_SIZE": "big",
                "SORT": [
                    "facebook",
                    "google",
                    "device"
                ]
            },
            "MYCARD_ENABLED": false,
            "ONE_STORE_LICENSE_KEY": "",
            "PAYPAL_ENABLED": false,
            "PINGPONG_ENABLED": false,
            "RAZER_ENABLED": false,
            "REMOTE_CONFIG": [],
            "SAMSUNG_SANDBOX_MODE": false,
            "STEAM_APPID": "",
            "STEAM_ENABLED": false,
            "STEAM_PAY_APPID": "",
            "STRIPE_ENABLED": false,
            "TOSS_ENABLED": false,
            "TWITTER_KEY": "",
            "TWITTER_SECRET": "",
            "WEBMONEY_ENABLED": false
        }
    },
    "Msg": "OK"
}"#;

const CLIENT_CODE: &str = include_str!("client-code.json");
