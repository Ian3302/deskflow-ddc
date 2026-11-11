//! Short-lived authenticated lifecycle requests, independent of the input session.
use super::*;
use futures_util::{SinkExt, StreamExt};

pub(crate) fn port(control: u16) -> u16 {
    if control == u16::MAX {
        control - 1
    } else {
        control + 1
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Command {
    id: uuid::Uuid,
    fingerprint: String,
    action: String,
}

impl Runtime {
    pub(super) fn start_standby(self: &Arc<Self>) {
        let this = self.clone();
        tokio::spawn(async move {
            let limit = Arc::new(tokio::sync::Semaphore::new(4));
            let mut listener: Option<(u16, TcpListener)> = None;
            let mut failed_port = None;
            let mut _advertisement = None;
            loop {
                if this.shutdown.is_cancelled() {
                    break;
                }
                let state = this.snapshot().await;
                let wanted = (state.config.mode == OperationMode::Sharing
                    && state.has_saved_pairing)
                    .then(|| port(state.config.control_port));
                if listener.as_ref().map(|(p, _)| *p) != wanted {
                    listener = None;
                    _advertisement = None;
                    if let Some(p) = wanted {
                        match TcpListener::bind(("0.0.0.0", p)).await {
                            Ok(socket) => {
                                listener = Some((p, socket));
                                failed_port = None;
                                match Discovery::new_standby().and_then(|mut d| {
                                    d.advertise(&state.device_id, &state.config)?;
                                    Ok(d)
                                }) {
                                    Ok(d) => _advertisement = Some(d),
                                    Err(error) => {
                                        this.log(
                                            "error",
                                            &format!(
                                                "待命设备发现不可用，仍可使用已认证地址：{error}"
                                            ),
                                        )
                                        .await
                                    }
                                }
                            }
                            Err(error) => {
                                if failed_port != Some(p) {
                                    this.log(
                                        "error",
                                        &format!("远程启停待命端口 {p} 不可用：{error}"),
                                    )
                                    .await;
                                    failed_port = Some(p);
                                }
                            }
                        }
                    }
                }
                let Some((_, socket)) = &listener else {
                    tokio::select! {
                        _ = this.shutdown.cancelled() => break,
                        _ = tokio::time::sleep(Duration::from_millis(500)) => {}
                    }
                    continue;
                };
                tokio::select! {
                    _ = this.shutdown.cancelled() => break,
                    _ = tokio::time::sleep(Duration::from_millis(500)) => {},
                    accepted = socket.accept() => {
                        if let Ok((socket, _)) = accepted {
                            if let Ok(permit) = limit.clone().try_acquire_owned() {
                                let runtime = this.clone();
                                tokio::spawn(async move {
                                    let _permit = permit;
                                    let result = runtime.accept_standby(socket).await;
                                    if let Err(error) = result {
                                        runtime.trace("standby.rejected", json!({"error":error.to_string()})).await;
                                    }
                                });
                            }
                        }
                    }
                }
            }
        });
    }

    async fn accept_standby(self: &Arc<Self>, socket: TcpStream) -> Result<()> {
        let address = socket.peer_addr()?.ip();
        let key = protocol::pairing_key(&self.inner.lock().await.store.pairing)?;
        let mut wire =
            tokio::time::timeout(Duration::from_secs(5), Wire::handshake(socket, key, true))
                .await
                .context("待命认证超时")??;
        let text = tokio::time::timeout(Duration::from_secs(5), wire.socket.next())
            .await
            .context("启停请求超时")?
            .context("启停请求已断开")??;
        let command: Command = serde_json::from_value(wire.codec.open(&text)?)?;
        // Authenticate the saved device before taking any action or updating hints.
        {
            let i = self.inner.lock().await;
            let pin = i
                .store
                .value
                .trusted_peer
                .as_ref()
                .context("尚未保存配对")?;
            ensure!(
                pin.id == command.id && pin.fingerprint == command.fingerprint,
                "配对身份不匹配"
            );
            ensure!(!self.shutdown.is_cancelled(), "应用正在退出");
        }
        let result: Result<()> = async {
            {
                let i = self.inner.lock().await;
                authorize(
                    &i.state.config,
                    i.store.value.trusted_peer.as_ref(),
                    &command,
                )?;
            }
            match command.action.as_str() {
                "start" => {
                    let _action = self
                        .actions
                        .try_lock()
                        .context("本机正在处理其他操作，请稍后重试")?;
                    // Recheck under the action lock in case settings changed while queued.
                    {
                        let i = self.inner.lock().await;
                        authorize(
                            &i.state.config,
                            i.store.value.trusted_peer.as_ref(),
                            &command,
                        )?;
                    }
                    if self.snapshot().await.running {
                        Ok(())
                    } else {
                        self.start(None).await
                    }
                }
                "stop" => {
                    // Acknowledge receipt before acquiring the action lock. Simultaneous
                    // stops on both computers must never wait for each other's lock.
                    let runtime = self.clone();
                    tokio::spawn(async move {
                        if let Err(error) = runtime.emergency_stop_local().await {
                            runtime
                                .log("error", &format!("远程停止共享失败：{error:#}"))
                                .await;
                        }
                    });
                    Ok(())
                }
                _ => bail!("未知启停请求"),
            }
        }
        .await;
        let response = json!({
            "id":self.inner.lock().await.store.value.id,
            "fingerprint":self.fingerprint,
            "error":result.err().map(|e|format!("{e:#}")),
        });
        let sealed = wire.codec.seal(&response)?;
        wire.socket.send(sealed).await?;
        // A valid lifecycle request also refreshes the return address.
        let mut i = self.inner.lock().await;
        i.store.value.last_verified_address = Some(address);
        i.store.save()?;
        Ok(())
    }

    async fn peer_lifecycle(&self, action: &str) -> Result<()> {
        let (config, pin, cached, key, command) = {
            let i = self.inner.lock().await;
            (
                i.state.config.clone(),
                i.store
                    .value
                    .trusted_peer
                    .clone()
                    .context("请先完成双端配对")?,
                i.store.value.last_verified_address,
                protocol::pairing_key(&i.store.pairing)?,
                Command {
                    id: i.store.value.id,
                    fingerprint: self.fingerprint.clone(),
                    action: action.into(),
                },
            )
        };
        let mut devices = self.snapshot().await.discovered_devices;
        if let Ok(discovery) = Discovery::new_standby() {
            if let Ok(found) = discovery.browse(Duration::from_millis(700)).await {
                devices.extend(found);
            }
        }
        let candidates = discovery::candidates(&config, Some(&pin), cached, &devices).await?;
        let mut last_error = anyhow::anyhow!("对端未在线");
        for mut candidate in candidates {
            candidate.address.set_port(port(candidate.address.port()));
            let result: Result<Option<String>> = async {
                let socket = discovery::connect(&candidate).await?;
                let mut wire = tokio::time::timeout(
                    Duration::from_secs(5),
                    Wire::handshake(socket, key, false),
                )
                .await
                .context("待命认证超时")??;
                let sealed = wire.codec.seal(&serde_json::to_value(&command)?)?;
                wire.socket.send(sealed).await?;
                let response = tokio::time::timeout(Duration::from_secs(25), wire.socket.next())
                    .await
                    .context("远程启停响应超时")?
                    .context("待命通道已关闭")??;
                let response = wire.codec.open(&response)?;
                ensure!(
                    response["id"] == pin.id.to_string()
                        && response["fingerprint"] == pin.fingerprint,
                    "待命设备身份与配对不一致"
                );
                Ok(response["error"].as_str().map(str::to_owned))
            }
            .await;
            match result {
                Ok(None) => return Ok(()),
                Ok(Some(error)) => bail!(
                    "对端未能{}共享：{error}",
                    if action == "start" {
                        "开启"
                    } else {
                        "停止"
                    }
                ),
                Err(error) => last_error = error,
            }
        }
        Err(last_error.context("无法控制另一台电脑；请确认两端 App 正在运行、已更新且待命端口可达"))
    }

    pub async fn start_shared(self: &Arc<Self>, pairing: Option<String>) -> Result<()> {
        let state = self.snapshot().await;
        // First pairing retains the explicit two-sided setup flow.
        let paired =
            state.has_saved_pairing && pairing.as_ref().is_none_or(|p| p.trim().is_empty());
        self.start(pairing).await?;
        if state.config.mode == OperationMode::Sharing && paired {
            if let Err(error) = self.peer_lifecycle("start").await {
                if let Err(restore) = self.stop_local().await {
                    self.log("error", &format!("开启失败后恢复本机输出失败：{restore:#}"))
                        .await;
                }
                self.notify_peer_stop().await;
                return Err(error);
            }
        }
        Ok(())
    }

    pub(super) async fn notify_peer_stop(&self) {
        let state = self.snapshot().await;
        if state.config.mode == OperationMode::Sharing && state.has_saved_pairing {
            if let Err(error) =
                tokio::time::timeout(Duration::from_secs(8), self.peer_lifecycle("stop"))
                    .await
                    .context("通知对端停止超时")
                    .and_then(|result| result)
            {
                self.log(
                    "error",
                    &format!("本机已执行停止；通知对端停止失败：{error:#}"),
                )
                .await;
            }
        }
    }
}

fn authorize(config: &Config, pin: Option<&Pin>, command: &Command) -> Result<()> {
    ensure!(config.mode == OperationMode::Sharing, "本机未配置联网共享");
    let pin = pin.context("尚未保存配对，不能远程启停")?;
    ensure!(
        pin.id == command.id && pin.fingerprint == command.fingerprint,
        "远程启停设备与已配对身份不一致"
    );
    ensure!(
        matches!(command.action.as_str(), "start" | "stop"),
        "未知启停请求"
    );
    ensure!(
        command.action != "start" || config.allow_remote_start,
        "本机未允许已配对电脑开启共享"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn request(runtime: &Arc<Runtime>, command: &Command, key: [u8; 32]) -> Result<Value> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        let server = runtime.clone();
        let task = tokio::spawn(async move {
            server
                .accept_standby(listener.accept().await.unwrap().0)
                .await
        });
        let mut wire = Wire::handshake(TcpStream::connect(address).await?, key, false).await?;
        let sealed = wire.codec.seal(&serde_json::to_value(command)?)?;
        wire.socket.send(sealed).await?;
        let text = wire
            .socket
            .next()
            .await
            .context("server rejected request")??;
        let result = wire.codec.open(&text)?;
        task.await??;
        Ok(result)
    }

    #[tokio::test]
    async fn encrypted_lifecycle_survives_stop_and_rechecks_start_policy_and_dependencies() {
        let directory = tempfile::tempdir().unwrap();
        let runtime = super::super::pointer_bridge::tests::fixture(directory.path(), "host");
        let pin = Pin {
            id: uuid::Uuid::new_v4(),
            fingerprint: "b".repeat(64),
        };
        let key = {
            let mut i = runtime.inner.lock().await;
            i.store.value.trusted_peer = Some(pin.clone());
            i.state.has_saved_pairing = true;
            protocol::pairing_key(&i.store.pairing).unwrap()
        };
        let mut command = Command {
            id: pin.id,
            fingerprint: pin.fingerprint.clone(),
            action: "start".into(),
        };
        assert!(request(&runtime, &command, key).await.unwrap()["error"].is_null());
        runtime.inner.lock().await.state.config.allow_remote_start = false;
        assert!(request(&runtime, &command, key).await.unwrap()["error"]
            .as_str()
            .unwrap()
            .contains("未允许"));
        command.action = "stop".into();
        let action_lock = runtime.actions.lock().await;
        assert!(request(&runtime, &command, key).await.unwrap()["error"].is_null());
        drop(action_lock);
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let state = runtime.snapshot().await;
                if !state.running && !state.emergency_stopping {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        command.action = "start".into();
        assert!(request(&runtime, &command, key).await.unwrap()["error"]
            .as_str()
            .unwrap()
            .contains("未允许"));
        runtime.inner.lock().await.state.config.allow_remote_start = true;
        // No valid display/native engine exists in this fixture: the ordinary
        // start path must reject it instead of reporting a running share.
        assert!(request(&runtime, &command, key).await.unwrap()["error"].is_string());
        assert!(!runtime.snapshot().await.running);
        command.id = uuid::Uuid::new_v4();
        assert!(request(&runtime, &command, key).await.is_err());
        assert!(!runtime.snapshot().await.running);
    }
    #[tokio::test]
    async fn standby_listener_remains_reachable_after_the_input_session_stops() {
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        let local = super::super::pointer_bridge::tests::fixture(a.path(), "client");
        let remote = super::super::pointer_bridge::tests::fixture(b.path(), "host");
        let socket = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let listening_port = socket.local_addr().unwrap().port();
        let control_port = listening_port - 1;
        drop(socket);
        let code = protocol::new_pairing();
        for (runtime, peer) in [(&local, &remote), (&remote, &local)] {
            let id = peer.inner.lock().await.store.value.id;
            let mut i = runtime.inner.lock().await;
            i.store.pairing = code.clone();
            i.store.value.trusted_peer = Some(Pin {
                id,
                fingerprint: peer.fingerprint.clone(),
            });
            i.store.value.last_verified_address = Some("127.0.0.1".parse().unwrap());
            i.state.has_saved_pairing = true;
            i.state.config.control_port = control_port;
        }
        remote.start_standby();
        local.peer_lifecycle("start").await.unwrap();
        local.peer_lifecycle("stop").await.unwrap();
        tokio::time::timeout(Duration::from_secs(5), async {
            while remote.snapshot().await.running || remote.snapshot().await.emergency_stopping {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        remote.inner.lock().await.state.config.allow_remote_start = false;
        let error = local.peer_lifecycle("start").await.unwrap_err().to_string();
        assert!(error.contains("未允许"), "{error}");
        remote.shutdown.cancel();
    }

    #[test]
    fn remote_start_requires_saved_identity_and_permission_but_stop_remains_available() {
        let mut config = Config::default();
        let pin = Pin {
            id: uuid::Uuid::new_v4(),
            fingerprint: "a".repeat(64),
        };
        let mut command = Command {
            id: pin.id,
            fingerprint: pin.fingerprint.clone(),
            action: "start".into(),
        };
        assert!(authorize(&config, None, &command).is_err());
        assert!(authorize(&config, Some(&pin), &command).is_ok());
        command.fingerprint = "b".repeat(64);
        assert!(authorize(&config, Some(&pin), &command).is_err());
        command.fingerprint = pin.fingerprint.clone();
        config.allow_remote_start = false;
        assert!(authorize(&config, Some(&pin), &command).is_err());
        command.action = "stop".into();
        assert!(authorize(&config, Some(&pin), &command).is_ok());
        config.mode = OperationMode::DisplayOnly;
        assert!(authorize(&config, Some(&pin), &command).is_err());
    }
}
