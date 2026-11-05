//! Activity operations share the source editor's inproc bridge and unknown-write policy.
use std::path::Path;

use clap::Subcommand;
use serde_json::{json, Value};

use crate::app_cmds::CmdOut;
use crate::core::{self, BackendError, Res};
use crate::inproc::client;

#[derive(Subcommand)]
pub(crate) enum ActivitySub {
    /// 只读探测建库服务(inproc;需客户端运行中)
    Probe,
    /// 当前工作区身份、模式和内存依赖(inproc)
    Current,
    /// 当前安装前置状态;引擎必须明确空闲(inproc)
    InstallState,
    /// 从 activity.zip 建真实指令库;Studio 须关闭(inproc)
    Create {
        #[arg(long)]
        name: String,
        #[arg(long)]
        code: String,
        #[arg(long)]
        description: String,
    },
    /// 按 UUID 完成已创建的新库;拒绝给已有库改名/编码(inproc)
    FinishCreate {
        #[arg(long)]
        app_id: String,
        #[arg(long)]
        name: String,
        #[arg(long)]
        code: String,
        #[arg(long)]
        description: String,
    },
    /// 从 blank.zip 建普通验收应用;Studio 须关闭(inproc)
    CreateConsumer {
        #[arg(long)]
        name: String,
        #[arg(long)]
        description: String,
    },
    /// 安装明确发布版本并核验持久化;不自动重试(inproc)
    Install {
        #[arg(long)]
        app_id: String,
        #[arg(long)]
        activity_code: String,
        #[arg(long)]
        version: String,
        #[arg(long)]
        name: String,
        /// 更新现有依赖;缺少该依赖时拒绝升级
        #[arg(long)]
        upgrade: bool,
    },
    /// Studio 关闭后按完整清单维护元数据，自动备份并回读(inproc)
    MetadataApply {
        #[arg(long)]
        app_id: String,
        /// 目标 xbot_robot 绝对路径
        #[arg(long)]
        root: String,
        /// JSON 清单文件，- 从 stdin 读取;拒绝普通源码和包身份变更
        #[arg(long)]
        manifest: String,
    },
}

pub struct PreparedActivity {
    route: &'static str,
    body: Option<Value>,
}

fn bad(message: impl Into<String>) -> BackendError {
    BackendError::new("BAD_INPUT", message)
}

fn nonempty(value: &str, field: &str) -> Res<()> {
    if value.trim().is_empty() {
        return Err(bad(format!("{field} 不能为空")));
    }
    Ok(())
}

fn uuid(value: &str) -> Res<()> {
    if !core::is_uuid(value) {
        return Err(BackendError::new("BAD_APP_ID", "app_id 必须是 UUID"));
    }
    Ok(())
}

fn activity_code(value: &str, creating: bool) -> Res<()> {
    let suffix = value.strip_prefix("activity_").unwrap_or("");
    if suffix.is_empty()
        || creating && suffix.len() < 2
        || !suffix.as_bytes()[0].is_ascii_lowercase()
        || !suffix
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        || creating && suffix.len() == 8 && suffix.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err(bad(
            "activity_code 需要 activity_ 开头的英文语义编码，建库后缀至少两个字符，拒绝随机十六进制编码",
        ));
    }
    Ok(())
}

fn allowed_metadata(path: &str) -> bool {
    matches!(
        path,
        "package.json"
            | "prototype.block.json"
            | "prototype.activity.xml"
            | "__init__.py"
            | ".dev/workspace.state.json"
    ) || path.starts_with(".dev/")
        && path.ends_with(".flow.json")
        && path.matches('/').count() == 1
        && !path.contains(['\\', ':'])
        && !path.contains("..")
}

fn validate_manifest(value: &Value) -> Res<()> {
    let files = value
        .as_object()
        .filter(|v| !v.is_empty())
        .ok_or_else(|| bad("manifest 必须是非空对象"))?;
    for (path, content) in files {
        if !allowed_metadata(path) {
            return Err(bad(format!("不允许的元数据路径: {path}")));
        }
        if path.ends_with(".json") && !content.is_object()
            || !path.ends_with(".json") && !content.is_string()
        {
            return Err(bad(format!(
                "{path}: JSON 文件须为对象，XML/初始化文件须为文本"
            )));
        }
    }
    Ok(())
}

pub fn prepare(sub: ActivitySub) -> Res<PreparedActivity> {
    let (route, body) = match sub {
        ActivitySub::Probe => ("/activity/probe", None),
        ActivitySub::Current => ("/studio/current", None),
        ActivitySub::InstallState => ("/activity/install-state", None),
        ActivitySub::Create {
            name,
            code,
            description,
        } => {
            nonempty(&name, "name")?;
            nonempty(&description, "description")?;
            activity_code(&code, true)?;
            (
                "/activity/create",
                Some(json!({"name":name.trim(),"code":code,"description":description})),
            )
        }
        ActivitySub::FinishCreate {
            app_id,
            name,
            code,
            description,
        } => {
            uuid(&app_id)?;
            nonempty(&name, "name")?;
            nonempty(&description, "description")?;
            activity_code(&code, true)?;
            (
                "/activity/finish-create",
                Some(
                    json!({"appId":app_id,"name":name.trim(),"code":code,"description":description}),
                ),
            )
        }
        ActivitySub::CreateConsumer { name, description } => {
            nonempty(&name, "name")?;
            nonempty(&description, "description")?;
            (
                "/activity/create-consumer",
                Some(json!({"name":name.trim(),"description":description})),
            )
        }
        ActivitySub::Install {
            app_id,
            activity_code: code,
            version,
            name,
            upgrade,
        } => {
            uuid(&app_id)?;
            activity_code(&code, false)?;
            nonempty(&name, "name")?;
            if !version.as_bytes().first().is_some_and(u8::is_ascii_digit)
                || !version
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
            {
                return Err(bad(
                    "version 需要明确的发布版本字符串，拒绝路径、空白或依赖表达式",
                ));
            }
            (
                "/activity/install",
                Some(
                    json!({"appId":app_id,"code":code,"version":version,"name":name,"upgrade":upgrade}),
                ),
            )
        }
        ActivitySub::MetadataApply {
            app_id,
            root,
            manifest,
        } => {
            uuid(&app_id)?;
            if !Path::new(&root).is_absolute() {
                return Err(bad("root 必须是 xbot_robot 绝对路径"));
            }
            let files: Value = serde_json::from_str(&core::read_file_or_stdin(&manifest)?)
                .map_err(|e| BackendError::new("BAD_JSON", format!("manifest: {e}")))?;
            validate_manifest(&files)?;
            (
                "/activity/metadata-apply",
                Some(json!({"appId":app_id,"root":root,"files":files})),
            )
        }
    };
    Ok(PreparedActivity { route, body })
}

fn current() -> Res<Value> {
    checked_data(client::get("/studio/current", 15)?)
}

fn checked_data(response: Value) -> Res<Value> {
    if response["ok"] != true || !response["data"].is_object() {
        return Err(
            BackendError::new("INPROC_BAD_RESPONSE", "inproc 响应缺少 ok=true/data 对象")
                .with_data(response),
        );
    }
    Ok(response["data"].clone())
}

fn require_target(current: &Value, id: &str) -> Res<()> {
    if current["uuid"] != id {
        return Err(
            BackendError::new("APP_MISMATCH", "当前 inproc 工作区与目标 UUID 不一致")
                .with_data(current.clone()),
        );
    }
    if current["mode"] != "edit" {
        return Err(BackendError::new(
            "VIEW_MODE_READONLY",
            "activity 写入需要已确认的编辑模式",
        ));
    }
    Ok(())
}

pub fn require_expected_app(id: &str) -> Res<()> {
    client::ensure_bridge()?;
    let snapshot = current()?;
    if snapshot["uuid"] != id {
        return Err(
            BackendError::new("APP_MISMATCH", "当前 inproc 工作区与预期 UUID 不一致")
                .with_data(snapshot),
        );
    }
    Ok(())
}

fn verification_error(result: &Value, evidence: Value) -> BackendError {
    BackendError::new(
        "ACTIVITY_VERIFY_FAILED",
        "操作已返回，后续身份或持久化核验失败;先检查实际状态",
    )
    .with_retryable(false)
    .with_data(json!({"applied":true,"result":result,"verification":evidence}))
    .with_hint(
        "不要自动重发;先 activity current / activity install-state 回读，元数据检查返回的备份目录",
    )
}

impl PreparedActivity {
    pub fn preview(&self) -> Option<Value> {
        self.body.as_ref().map(|body| json!({"action":self.route.trim_start_matches('/').replace('/',"."),"target":{"app_id":body.get("appId")},"arguments":body,"transport":"inproc"}))
    }

    pub fn execute(self) -> Res<CmdOut> {
        client::ensure_bridge()?;
        let Some(body) = self.body else {
            return Ok((client::get(self.route, 15)?, vec![]));
        };
        let before = match self.route {
            "/activity/create" | "/activity/create-consumer" | "/activity/metadata-apply" => {
                let probe = client::get("/activity/probe", 15)?;
                if probe["hasStudio"] != false || probe["isBusy"] != false {
                    return Err(BackendError::new(
                        "STUDIO_BUSY",
                        "操作需要 Studio 已关闭且客户端空闲",
                    )
                    .with_data(probe));
                }
                None
            }
            _ => {
                let snapshot = current()?;
                require_target(&snapshot, body["appId"].as_str().unwrap())?;
                if self.route == "/activity/finish-create"
                    && snapshot["robotType"]
                        .as_str()
                        .is_none_or(|s| !s.eq_ignore_ascii_case("activity"))
                {
                    return Err(bad("finish-create 仅用于 activity 工作区"));
                }
                if self.route == "/activity/install" {
                    let idle = checked_data(client::get("/activity/install-state", 15)?)?;
                    if idle["uuid"] != body["appId"] {
                        return Err(BackendError::new(
                            "APP_MISMATCH",
                            "安装前置状态的 UUID 不一致",
                        ));
                    }
                    let deps = snapshot["dependencies"].as_array().ok_or_else(|| {
                        BackendError::new("INPROC_BAD_RESPONSE", "工作区缺少 dependencies")
                    })?;
                    let prefix = format!("{}==", body["code"].as_str().unwrap());
                    let pins: Vec<_> = deps
                        .iter()
                        .filter_map(Value::as_str)
                        .filter(|d| d.starts_with(&prefix))
                        .collect();
                    if body["upgrade"] == true && pins.is_empty()
                        || body["upgrade"] == false
                            && pins.iter().any(|p| {
                                *p != format!("{}{}", prefix, body["version"].as_str().unwrap())
                            })
                    {
                        return Err(bad("升级需要已安装依赖;切换已安装版本请加 --upgrade"));
                    }
                }
                Some(snapshot)
            }
        };
        let result = client::post(self.route, &body, 170)?;
        let data = checked_data(result.clone())
            .map_err(|e| verification_error(&result, json!({"code":e.code,"detail":e.message})))?;
        if self.route == "/activity/metadata-apply" {
            let requested: Vec<_> = body["files"].as_object().unwrap().keys().cloned().collect();
            let returned: Vec<_> = data["files"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect();
            if data["appId"] != body["appId"]
                || data["verification"] != "disk_readback"
                || data["backup"].as_str().is_none_or(str::is_empty)
                || requested
                    .into_iter()
                    .collect::<std::collections::BTreeSet<_>>()
                    != returned.into_iter().collect()
            {
                return Err(verification_error(&result, data));
            }
            return Ok((
                json!({"verified":true,"result":data}),
                vec!["xbot-cli app open <app-id>  # 重开后保存编译，再核对元数据与公开清单".into()],
            ));
        }
        let after = current()
            .map_err(|e| verification_error(&result, json!({"code":e.code,"detail":e.message})))?;
        let id = if matches!(self.route, "/activity/install" | "/activity/finish-create") {
            &body["appId"]
        } else {
            &data["uuid"]
        };
        if !id.as_str().is_some_and(core::is_uuid)
            || after["uuid"] != *id
            || after["mode"] != "edit"
        {
            return Err(verification_error(&result, after));
        }
        if self.route == "/activity/install" {
            let code = body["code"].as_str().unwrap();
            let version = body["version"].as_str().unwrap();
            let pin = format!("{code}=={version}");
            let prefix = format!("{code}==");
            let other = |v: &Value| -> Vec<String> {
                let mut values: Vec<_> = v
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .filter(|p| !p.starts_with(&prefix))
                    .map(str::to_owned)
                    .collect();
                values.sort();
                values
            };
            let deps = after["dependencies"]
                .as_array()
                .ok_or_else(|| verification_error(&result, after.clone()))?;
            if data["appId"] != *id
                || data["activityCode"] != body["code"]
                || data["version"] != body["version"]
                || data["installedVersion"] != body["version"]
                || data["dependencies"] != after["dependencies"]
                || deps
                    .iter()
                    .filter_map(Value::as_str)
                    .filter(|p| p.starts_with(&prefix))
                    .collect::<Vec<_>>()
                    != vec![pin.as_str()]
                || other(&before.unwrap()["dependencies"]) != other(&after["dependencies"])
            {
                return Err(verification_error(&result, after));
            }
        } else {
            let kind = after["robotType"].as_str().unwrap_or("");
            let consumer = self.route == "/activity/create-consumer";
            if kind.is_empty()
                || kind.eq_ignore_ascii_case("activity") == consumer
                || after["name"] != body["name"]
                || after["description"] != body["description"]
                || !consumer && after["activityCode"] != body["code"]
            {
                return Err(verification_error(&result, after));
            }
        }
        Ok((
            json!({"verified":true,"result":data,"current":after}),
            vec![],
        ))
    }
}
