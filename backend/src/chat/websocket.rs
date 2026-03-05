use std::{collections::HashMap, sync::Arc, time::Duration};

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::Response,
};
use chrono::{DateTime, Utc};
use futures::{SinkExt, StreamExt};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use tokio::{
    sync::{broadcast, mpsc, RwLock},
    task::JoinHandle,
    time::{interval, timeout},
};
use tracing::{debug, error, warn};
use uuid::Uuid;

use crate::{
    auth::{jwt::verify_token, middleware::ensure_account_active},
    awards::service,
    chat::{encryption, safety},
    error::{AppError, AppResult},
    matchmaking::queue::{self, QueueStatus},
    AppState,
};

#[derive(Clone)]
pub struct ChatHub {
    channels: Arc<RwLock<HashMap<Uuid, broadcast::Sender<ServerEvent>>>>,
    /// Redis subscriber tasks keyed by chat_id — one per active chat channel.
    redis_subscribers: Arc<RwLock<HashMap<Uuid, JoinHandle<()>>>>,
    /// Connection pool used for PUBLISH commands. `None` in tests.
    redis_pool: Option<crate::redis_client::RedisPool>,
    /// Dedicated Redis client for SUBSCRIBE connections (pub/sub requires a
    /// non-shared connection). `None` in tests.
    redis_client: Option<redis::Client>,
    /// Unique identifier used to ignore events this process published.
    redis_instance_id: Uuid,
}

impl Default for ChatHub {
    fn default() -> Self {
        Self {
            channels: Arc::default(),
            redis_subscribers: Arc::default(),
            redis_pool: None,
            redis_client: None,
            redis_instance_id: Uuid::new_v4(),
        }
    }
}

impl ChatHub {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a ChatHub wired to Redis for cross-instance message relay.
    ///
    /// `pool` is used for regular PUBLISH commands. `redis_url` is used to
    /// create dedicated connections for SUBSCRIBE (pub/sub requires a
    /// non-multiplexed connection).
    pub fn with_redis(pool: crate::redis_client::RedisPool, redis_url: &str) -> Self {
        let client = redis::Client::open(redis_url)
            .expect("invalid REDIS_URL for ChatHub subscriber client");
        Self {
            channels: Arc::default(),
            redis_subscribers: Arc::default(),
            redis_pool: Some(pool),
            redis_client: Some(client),
            redis_instance_id: Uuid::new_v4(),
        }
    }

    /// Returns the Redis pub/sub channel name for a given chat.
    fn redis_channel(chat_id: Uuid) -> String {
        format!("chat:{chat_id}")
    }

    async fn sender_for(&self, chat_id: Uuid) -> broadcast::Sender<ServerEvent> {
        let mut channels = self.channels.write().await;
        if let Some(sender) = channels.get(&chat_id) {
            return sender.clone();
        }
        let (sender, _) = broadcast::channel(256);
        channels.insert(chat_id, sender.clone());

        // Spawn a Redis subscriber for this chat if Redis is configured and
        // no subscriber task is already running.
        if let Some(ref client) = self.redis_client {
            let mut subs = self.redis_subscribers.write().await;
            if !subs.contains_key(&chat_id) {
                let task = spawn_redis_subscriber(
                    client.clone(),
                    chat_id,
                    sender.clone(),
                    self.redis_instance_id,
                );
                subs.insert(chat_id, task);
            }
        }

        sender
    }

    async fn receiver_for(&self, chat_id: Uuid) -> broadcast::Receiver<ServerEvent> {
        self.sender_for(chat_id).await.subscribe()
    }

    /// Broadcast an event to the local channel **and** publish it to Redis so
    /// other backend instances can relay it to their local subscribers.
    pub async fn send(&self, chat_id: Uuid, event: ServerEvent) {
        let sender = self.sender_for(chat_id).await;
        let _ = sender.send(event.clone());

        // Publish to Redis for cross-instance delivery.
        if let Some(ref pool) = self.redis_pool {
            let redis_event = RedisRelayEvent {
                event,
                origin_instance_id: Some(self.redis_instance_id),
            };

            if let Ok(payload) = serde_json::to_string(&redis_event) {
                let channel = Self::redis_channel(chat_id);
                let result: Result<(), Box<dyn std::error::Error + Send + Sync>> = async {
                    let mut conn = pool
                        .get()
                        .await
                        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;
                    let _: () = conn.publish(channel, payload).await?;
                    Ok(())
                }
                .await;
                if let Err(err) = result {
                    warn!(?err, %chat_id, "failed to publish chat event to Redis");
                }
            }
        }
    }

    async fn prune_if_no_receivers(&self, chat_id: Uuid) -> bool {
        let mut channels = self.channels.write().await;
        let should_remove = channels
            .get(&chat_id)
            .map(|sender| sender.receiver_count() == 0)
            .unwrap_or(false);

        if should_remove {
            channels.remove(&chat_id);

            // Also cancel the Redis subscriber task for this chat.
            let mut subs = self.redis_subscribers.write().await;
            if let Some(task) = subs.remove(&chat_id) {
                task.abort();
            }

            return true;
        }

        false
    }

    #[cfg(test)]
    async fn channel_exists(&self, chat_id: Uuid) -> bool {
        let channels = self.channels.read().await;
        channels.contains_key(&chat_id)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct RedisRelayEvent {
    #[serde(flatten)]
    event: ServerEvent,
    #[serde(default)]
    origin_instance_id: Option<Uuid>,
}

/// Spawn a background task that subscribes to the Redis pub/sub channel for
/// `chat_id` and relays incoming messages into the local broadcast `sender`.
///
/// The task reconnects automatically on errors with a short back-off, and
/// terminates when the `sender` has no more receivers (i.e. the chat was
/// pruned) or when the task is aborted.
fn spawn_redis_subscriber(
    client: redis::Client,
    chat_id: Uuid,
    sender: broadcast::Sender<ServerEvent>,
    redis_instance_id: Uuid,
) -> JoinHandle<()> {
    let channel_name = ChatHub::redis_channel(chat_id);
    tokio::spawn(async move {
        loop {
            match subscribe_loop(&client, &channel_name, &sender, redis_instance_id).await {
                Ok(()) => {
                    // Clean exit — no more receivers.
                    debug!(%chat_id, "Redis subscriber exiting: no receivers");
                    break;
                }
                Err(err) => {
                    warn!(?err, %chat_id, "Redis subscriber error, reconnecting in 2s");
                    tokio::time::sleep(Duration::from_secs(2)).await;

                    // If all receivers are gone while we were sleeping, stop.
                    if sender.receiver_count() == 0 {
                        debug!(%chat_id, "Redis subscriber exiting after reconnect: no receivers");
                        break;
                    }
                }
            }
        }
    })
}

/// Inner subscribe loop. Returns `Ok(())` when no receivers remain, or
/// `Err` on any Redis error so the caller can decide to reconnect.
async fn subscribe_loop(
    client: &redis::Client,
    channel_name: &str,
    sender: &broadcast::Sender<ServerEvent>,
    redis_instance_id: Uuid,
) -> Result<(), redis::RedisError> {
    let mut pubsub = client.get_async_pubsub().await?;
    pubsub.subscribe(channel_name).await?;

    let mut stream = pubsub.on_message();
    while let Some(msg) = stream.next().await {
        if sender.receiver_count() == 0 {
            return Ok(());
        }
        let payload: String = msg.get_payload()?;
        match parse_redis_relay_payload(&payload, redis_instance_id) {
            Ok(Some(event)) => {
                // Send to local broadcast; ignore error (no receivers).
                let _ = sender.send(event);
            }
            Ok(None) => continue,
            Err(err) => warn!(?err, "failed to deserialize Redis chat event"),
        }
    }

    if sender.receiver_count() == 0 {
        return Ok(());
    }

    Err(redis::RedisError::from((
        redis::ErrorKind::IoError,
        "redis pub/sub stream ended unexpectedly",
    )))
}

fn parse_redis_relay_payload(
    payload: &str,
    redis_instance_id: Uuid,
) -> Result<Option<ServerEvent>, serde_json::Error> {
    let event = serde_json::from_str::<RedisRelayEvent>(payload)?;

    if event.origin_instance_id == Some(redis_instance_id) {
        return Ok(None);
    }

    Ok(Some(event.event))
}

#[derive(Debug, Deserialize)]
pub struct WsQuery {
    pub token: Option<String>,
    pub chat_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientEvent {
    Auth {
        token: String,
        chat_id: Option<Uuid>,
    },
    Message {
        content: String,
    },
    Typing {
        is_typing: bool,
    },
    ReadReceipt {
        message_id: Uuid,
    },
    KeepVote {
        keep: bool,
    },
    Award {
        award_type: String,
        spark_amount: i64,
    },
    Leave,
}

#[derive(Debug, Deserialize)]
struct BootstrapEvent {
    token: String,
    chat_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum AuthResponse {
    Ok,
    Failed { code: String, message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerEvent {
    Queued {
        position: usize,
        queue: String,
    },
    Matched {
        chat_id: Uuid,
        partner: serde_json::Value,
    },
    Message {
        id: Uuid,
        chat_id: Uuid,
        sender_id: Uuid,
        content: String,
        timestamp: DateTime<Utc>,
        flagged: bool,
    },
    Typing {
        user_id: Uuid,
        is_typing: bool,
    },
    ReadReceipt {
        message_id: Uuid,
        reader_id: Uuid,
    },
    VoteAcknowledged,
    PartnerKeepVote,
    KeepResult {
        both_kept: bool,
    },
    AwardReceived {
        recipient_id: Uuid,
        award_type: String,
        spark_amount: i64,
        your_cut: i64,
    },
    PartnerLeft {
        user_id: Uuid,
    },
    Cooldown {
        seconds: u64,
    },
    Error {
        code: String,
        message: String,
    },
}

pub async fn ws_handler(
    State(state): State<AppState>,
    Query(params): Query<WsQuery>,
    ws: WebSocketUpgrade,
) -> AppResult<Response> {
    let pre_user_id = if let Some(token) = params.token.as_deref() {
        let claims = verify_token(token, &state.jwt)?;
        let user_id = Uuid::parse_str(claims.sub.as_str())
            .map_err(|_| AppError::Unauthorized("invalid token subject".to_owned()))?;
        ensure_account_active(&state.db, user_id).await?;

        if let Some(chat_id) = params.chat_id {
            ensure_chat_member(&state, chat_id, user_id).await?;
        }

        Some(user_id)
    } else {
        None
    };

    Ok(ws.on_upgrade(move |socket| handle_socket(socket, state, pre_user_id, params.chat_id)))
}

async fn handle_socket(
    mut socket: WebSocket,
    state: AppState,
    pre_user_id: Option<Uuid>,
    initial_chat_id: Option<Uuid>,
) {
    let (user_id, mut current_chat_id) =
        match authenticate_socket(&mut socket, &state, pre_user_id, initial_chat_id).await {
            Ok(data) => data,
            Err(err) => {
                let _ = socket
                    .send(Message::Text(
                        serde_json::to_string(&AuthResponse::Failed {
                            code: "UNAUTHORIZED".to_owned(),
                            message: err.to_string(),
                        })
                        .unwrap()
                        .into(),
                    ))
                    .await;
                let _ = socket.close().await;
                return;
            }
        };

    let _ = socket
        .send(Message::Text(
            serde_json::to_string(&AuthResponse::Ok).unwrap().into(),
        ))
        .await;

    crate::metrics::WEBSOCKET_CONNECTIONS_ACTIVE.inc();

    if let Err(err) = set_session_presence(&state, user_id).await {
        error!(?err, "failed to register websocket session");
    }

    let (mut ws_sender, mut ws_receiver) = socket.split();
    let (outbound_tx, mut outbound_rx) = mpsc::unbounded_channel::<ServerEvent>();

    let writer_task = tokio::spawn(async move {
        while let Some(event) = outbound_rx.recv().await {
            let Ok(payload) = serde_json::to_string(&event) else {
                continue;
            };
            if ws_sender.send(Message::Text(payload.into())).await.is_err() {
                break;
            }
        }
    });

    let mut chat_forward_task = if let Some(chat_id) = current_chat_id {
        Some(spawn_chat_forwarder(&state, chat_id, outbound_tx.clone()).await)
    } else {
        None
    };

    let mut last_queue_snapshot: Option<(String, usize)> = None;
    let mut status_ticker = interval(Duration::from_millis(5000));
    let mut left_explicitly = false;

    loop {
        tokio::select! {
            _ = status_ticker.tick() => {
                if let Err(err) = set_session_presence(&state, user_id).await {
                    error!(?err, "failed to refresh websocket session presence");
                }

                if current_chat_id.is_some() {
                    if let Err(err) = queue::refresh_active_chat_ttl(&state, user_id).await {
                        error!(?err, "failed to refresh active chat TTL");
                    }
                }

                if current_chat_id.is_none() {
                    if let Ok(Some(match_notice)) = queue::take_match_notification(&state, user_id).await {
                        current_chat_id = Some(match_notice.chat_id);
                    }
                }

                match queue::get_status(&state, user_id).await {
                    Ok(QueueStatus::Queued { queue, position, .. }) => {
                        if last_queue_snapshot.as_ref() != Some(&(queue.clone(), position)) {
                            let _ = outbound_tx.send(ServerEvent::Queued { position, queue: queue.clone() });
                            last_queue_snapshot = Some((queue, position));
                        }
                    }
                    Ok(QueueStatus::Matched { chat_id }) => {
                        if current_chat_id != Some(chat_id) {
                            current_chat_id = Some(chat_id);
                            if let Some(task) = chat_forward_task.take() {
                                abort_and_wait(task).await;
                            }
                            chat_forward_task = Some(spawn_chat_forwarder(&state, chat_id, outbound_tx.clone()).await);

                            match resolve_partner_payload(&state, chat_id, user_id).await {
                                Ok(partner) => {
                                    let _ = outbound_tx.send(ServerEvent::Matched { chat_id, partner });
                                }
                                Err(err) => error!(?err, "failed to resolve partner payload"),
                            }
                            last_queue_snapshot = None;
                        }
                    }
                    Ok(QueueStatus::Idle) => {
                        last_queue_snapshot = None;
                    }
                    Err(err) => {
                        error!(?err, "failed to read queue status");
                    }
                }
            }
            frame = ws_receiver.next() => {
                let Some(frame) = frame else { break; };

                match frame {
                    Ok(Message::Text(text)) => {
                        let event = match serde_json::from_str::<ClientEvent>(text.as_str()) {
                            Ok(event) => event,
                            Err(_) => {
                                let _ = outbound_tx.send(ServerEvent::Error {
                                    code: "BAD_EVENT".to_owned(),
                                    message: "invalid websocket message: expected typed JSON payload".to_owned(),
                                });
                                continue;
                            }
                        };

                        if matches!(event, ClientEvent::Auth { .. }) {
                            let _ = outbound_tx.send(ServerEvent::Error {
                                code: "BAD_EVENT".to_owned(),
                                message: "already authenticated".to_owned(),
                            });
                            continue;
                        }

                        let Some(chat_id) = current_chat_id else {
                            let _ = outbound_tx.send(ServerEvent::Error {
                                code: "NOT_IN_CHAT".to_owned(),
                                message: "no active chat yet".to_owned(),
                            });
                            continue;
                        };

                        match process_chat_event(&state, chat_id, user_id, event, &outbound_tx).await {
                            Ok(should_leave) => {
                                if should_leave {
                                    left_explicitly = true;
                                    break;
                                }
                            }
                            Err(err) => {
                                error!(?err, "failed to process websocket event");
                                let _ = outbound_tx.send(ServerEvent::Error {
                                    code: "BAD_EVENT".to_owned(),
                                    message: err.to_string(),
                                });
                            }
                        }
                    }
                    Ok(Message::Close(_)) => break,
                    Ok(_) => {}
                    Err(err) => {
                        error!(?err, "websocket read error");
                        break;
                    }
                }
            }
        }
    }

    crate::metrics::WEBSOCKET_CONNECTIONS_ACTIVE.dec();

    writer_task.abort();
    if let Some(task) = chat_forward_task {
        abort_and_wait(task).await;
    }

    if let Some(chat_id) = current_chat_id {
        if let Err(err) = finalize_chat(&state, chat_id, user_id).await {
            error!(?err, "failed to finalize chat");
        }
    }

    if let Err(err) = clear_session_presence(&state, user_id).await {
        error!(?err, "failed to clear websocket session presence");
    }

    debug!(user_id = %user_id, left_explicitly, "websocket closed");
}

async fn authenticate_socket(
    socket: &mut WebSocket,
    state: &AppState,
    pre_user_id: Option<Uuid>,
    initial_chat_id: Option<Uuid>,
) -> AppResult<(Uuid, Option<Uuid>)> {
    if let Some(user_id) = pre_user_id {
        return Ok((user_id, initial_chat_id));
    }

    let message = timeout(Duration::from_secs(15), socket.recv())
        .await
        .map_err(|_| AppError::Unauthorized("websocket auth timeout".to_owned()))?
        .ok_or_else(|| AppError::Unauthorized("missing auth payload".to_owned()))?
        .map_err(|_| AppError::Unauthorized("invalid auth frame".to_owned()))?;

    let Message::Text(payload) = message else {
        return Err(AppError::Unauthorized(
            "first websocket payload must be auth text".to_owned(),
        ));
    };

    if let Ok(ClientEvent::Auth { token, chat_id }) =
        serde_json::from_str::<ClientEvent>(payload.as_str())
    {
        let claims = verify_token(token.as_str(), &state.jwt)?;
        let user_id = Uuid::parse_str(claims.sub.as_str())
            .map_err(|_| AppError::Unauthorized("invalid token subject".to_owned()))?;
        ensure_account_active(&state.db, user_id).await?;
        let chat_id = chat_id.or(initial_chat_id);
        if let Some(chat_id) = chat_id {
            ensure_chat_member(state, chat_id, user_id).await?;
        }
        return Ok((user_id, chat_id));
    }

    let bootstrap = serde_json::from_str::<BootstrapEvent>(payload.as_str())
        .map_err(|_| AppError::Unauthorized("invalid websocket auth payload".to_owned()))?;
    let claims = verify_token(bootstrap.token.as_str(), &state.jwt)?;
    let user_id = Uuid::parse_str(claims.sub.as_str())
        .map_err(|_| AppError::Unauthorized("invalid token subject".to_owned()))?;
    ensure_account_active(&state.db, user_id).await?;
    let chat_id = bootstrap.chat_id.or(initial_chat_id);
    if let Some(chat_id) = chat_id {
        ensure_chat_member(state, chat_id, user_id).await?;
    }
    Ok((user_id, chat_id))
}

async fn spawn_chat_forwarder(
    state: &AppState,
    chat_id: Uuid,
    outbound_tx: mpsc::UnboundedSender<ServerEvent>,
) -> JoinHandle<()> {
    let mut broadcast_rx = state.chat_hub.receiver_for(chat_id).await;
    tokio::spawn(async move {
        while let Ok(event) = broadcast_rx.recv().await {
            let _ = outbound_tx.send(event);
        }
    })
}

async fn abort_and_wait(task: JoinHandle<()>) {
    task.abort();
    let _ = task.await;
}

async fn process_chat_event(
    state: &AppState,
    chat_id: Uuid,
    user_id: Uuid,
    event: ClientEvent,
    private_tx: &mpsc::UnboundedSender<ServerEvent>,
) -> AppResult<bool> {
    match event {
        ClientEvent::Message { content } => {
            let trimmed = content.trim();
            if trimmed.is_empty() {
                return Ok(false);
            }

            // Safety scan is fail-closed: if scan_message returns Err, the `?`
            // propagates it and the message is never stored or broadcast.
            let safety = safety::scan_message(state, chat_id, user_id, trimmed).await?;

            // If the safety scan determined the message should be rejected
            // (e.g. blocklist keyword match), refuse to store or broadcast it.
            if safety.rejected {
                return Err(AppError::Forbidden(
                    "message rejected by safety filter".to_owned(),
                ));
            }

            // Use the normalized content from the safety scan for storage and
            // broadcast. This ensures the exact content that was scanned is
            // what gets stored -- no divergence possible.
            let safe_content = &safety.normalized_content;

            // Skip empty messages that became empty after normalization
            // (e.g. messages consisting entirely of zero-width characters).
            if safe_content.is_empty() {
                return Ok(false);
            }

            // Verify the content we're about to store/broadcast is exactly
            // what was scanned -- guards against any future code path that
            // might mutate the content between the scan and here.
            assert!(
                safety.verify_content(safe_content),
                "content integrity check failed: scanned content differs from stored content"
            );

            let (content_encrypted, nonce) = encryption::encrypt_for_chat(
                &state.db,
                chat_id,
                safe_content,
                &state.config.chat_key_encryption_key,
                &state.config.legacy_chat_key_encryption_keys,
            )
            .await?;

            let (message_id, created_at): (Uuid, DateTime<Utc>) = sqlx::query_as(
                r#"
                INSERT INTO messages (chat_id, sender_id, content_encrypted, nonce, content_text)
                VALUES ($1, $2, $3, $4, NULL)
                RETURNING id, created_at
                "#,
            )
            .bind(chat_id)
            .bind(user_id)
            .bind(content_encrypted)
            .bind(nonce)
            .fetch_one(&state.db)
            .await?;

            if safety.flagged {
                sqlx::query(
                    r#"
                    INSERT INTO message_flags (message_id, user_id, reasons)
                    VALUES ($1, $2, $3)
                    "#,
                )
                .bind(message_id)
                .bind(user_id)
                .bind(serde_json::json!(safety.reasons))
                .execute(&state.db)
                .await?;

                return Err(AppError::BadRequest(
                    "message rejected by safety filter".to_owned(),
                ));
            }

            state
                .chat_hub
                .send(
                    chat_id,
                    ServerEvent::Message {
                        id: message_id,
                        chat_id,
                        sender_id: user_id,
                        content: safe_content.to_owned(),
                        timestamp: created_at,
                        flagged: false,
                    },
                )
                .await;
        }
        ClientEvent::Typing { is_typing } => {
            state
                .chat_hub
                .send(chat_id, ServerEvent::Typing { user_id, is_typing })
                .await;
        }
        ClientEvent::ReadReceipt { message_id } => {
            let result = sqlx::query(
                r#"
                UPDATE messages
                SET is_read = TRUE
                WHERE id = $1
                  AND chat_id = $2
                  AND sender_id != $3
                "#,
            )
            .bind(message_id)
            .bind(chat_id)
            .bind(user_id)
            .execute(&state.db)
            .await?;

            if result.rows_affected() > 0 {
                state
                    .chat_hub
                    .send(
                        chat_id,
                        ServerEvent::ReadReceipt {
                            message_id,
                            reader_id: user_id,
                        },
                    )
                    .await;
            }
        }
        ClientEvent::KeepVote { keep } => {
            match register_keep_vote(state, chat_id, user_id, keep).await? {
                KeepVoteRegistration::Pending => {
                    // Send acknowledgment only to the voter -- never broadcast
                    // to the partner, as that would reveal vote timing/identity.
                    let _ = private_tx.send(ServerEvent::VoteAcknowledged);
                }
                KeepVoteRegistration::Resolved { both_kept } => {
                    // Both users have voted; broadcast the result to the chat.
                    state
                        .chat_hub
                        .send(chat_id, ServerEvent::KeepResult { both_kept })
                        .await;
                }
                KeepVoteRegistration::NoopAlreadyResolved => {}
            }
        }
        ClientEvent::Award {
            award_type,
            spark_amount,
        } => {
            let outcome = service::send_award_internal(
                state,
                user_id,
                chat_id,
                award_type.clone(),
                spark_amount,
            )
            .await?;

            state
                .chat_hub
                .send(
                    chat_id,
                    ServerEvent::AwardReceived {
                        recipient_id: outcome.recipient_id,
                        award_type,
                        spark_amount,
                        your_cut: outcome.recipient_cut,
                    },
                )
                .await;
        }
        ClientEvent::Leave => return Ok(true),
        ClientEvent::Auth { .. } => {
            return Err(AppError::BadRequest("already authenticated".to_owned()));
        }
    }

    Ok(false)
}

async fn ensure_chat_member(state: &AppState, chat_id: Uuid, user_id: Uuid) -> AppResult<()> {
    let row = sqlx::query_as::<_, (Uuid, Uuid)>(
        r#"
        SELECT user_a_id, user_b_id
        FROM chats
        WHERE id = $1
        "#,
    )
    .bind(chat_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("chat not found".to_owned()))?;

    if row.0 != user_id && row.1 != user_id {
        return Err(AppError::Forbidden(
            "user is not a participant in this chat".to_owned(),
        ));
    }

    Ok(())
}

enum KeepVoteRegistration {
    Pending,
    Resolved { both_kept: bool },
    NoopAlreadyResolved,
}

async fn register_keep_vote(
    state: &AppState,
    chat_id: Uuid,
    voter_id: Uuid,
    keep: bool,
) -> AppResult<KeepVoteRegistration> {
    let (user_a_id, user_b_id, was_kept, keep_votes_resolved) =
        sqlx::query_as::<_, (Uuid, Uuid, bool, bool)>(
            r#"
        SELECT user_a_id, user_b_id, is_kept, keep_votes_resolved
        FROM chats
        WHERE id = $1
        "#,
        )
        .bind(chat_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("chat not found".to_owned()))?;

    if voter_id != user_a_id && voter_id != user_b_id {
        return Err(AppError::Forbidden("not a chat participant".to_owned()));
    }

    if keep_votes_resolved {
        return Ok(KeepVoteRegistration::NoopAlreadyResolved);
    }

    if keep {
        let row = sqlx::query_as::<_, (bool, i32)>(
            r#"
            SELECT is_premium, keep_count
            FROM users
            WHERE id = $1
            "#,
        )
        .bind(voter_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("user not found".to_owned()))?;

        if !row.0 && row.1 >= 3 {
            return Err(AppError::Conflict(
                "free users can only keep 3 chats; upgrade to premium for unlimited keeps"
                    .to_owned(),
            ));
        }
    }

    let mut conn = state
        .redis
        .get()
        .await
        .map_err(|e| AppError::Internal(format!("redis pool error: {e}")))?;
    let vote_key = keep_vote_key(chat_id, voter_id);
    let _: () = conn
        .set_ex(vote_key, if keep { "1" } else { "0" }, 2 * 60 * 60)
        .await?;

    let a_vote: Option<String> = conn.get(keep_vote_key(chat_id, user_a_id)).await?;
    let b_vote: Option<String> = conn.get(keep_vote_key(chat_id, user_b_id)).await?;

    let both_voted = a_vote.is_some() && b_vote.is_some();
    if !both_voted {
        return Ok(KeepVoteRegistration::Pending);
    }

    let keep_a = a_vote.as_deref() == Some("1");
    let keep_b = b_vote.as_deref() == Some("1");
    let both_kept = keep_a && keep_b;

    let mut tx = state.db.begin().await?;
    let finalize_result = sqlx::query(
        r#"
        UPDATE chats
        SET is_kept_by_a = $2,
            is_kept_by_b = $3,
            keep_votes_resolved = TRUE
        WHERE id = $1
          AND keep_votes_resolved = FALSE
        "#,
    )
    .bind(chat_id)
    .bind(keep_a)
    .bind(keep_b)
    .execute(&mut *tx)
    .await?;

    if finalize_result.rows_affected() == 0 {
        tx.rollback().await?;
        let _: usize = conn.del(keep_vote_key(chat_id, user_a_id)).await?;
        let _: usize = conn.del(keep_vote_key(chat_id, user_b_id)).await?;
        return Ok(KeepVoteRegistration::NoopAlreadyResolved);
    }

    if both_kept && !was_kept {
        sqlx::query(
            r#"
            UPDATE users
            SET keep_count = keep_count + 1, updated_at = NOW()
            WHERE id = ANY($1)
            "#,
        )
        .bind(vec![user_a_id, user_b_id])
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    let _: usize = conn.del(keep_vote_key(chat_id, user_a_id)).await?;
    let _: usize = conn.del(keep_vote_key(chat_id, user_b_id)).await?;

    Ok(KeepVoteRegistration::Resolved { both_kept })
}

async fn finalize_chat(state: &AppState, chat_id: Uuid, leaver_id: Uuid) -> AppResult<()> {
    let participants = sqlx::query_as::<_, (Uuid, Uuid)>(
        r#"
        SELECT user_a_id, user_b_id
        FROM chats
        WHERE id = $1
        "#,
    )
    .bind(chat_id)
    .fetch_optional(&state.db)
    .await?;

    let Some((user_a_id, user_b_id)) = participants else {
        let _ = state.chat_hub.prune_if_no_receivers(chat_id).await;
        return Ok(());
    };

    let update_result = sqlx::query(
        r#"
        UPDATE chats
        SET ended_at = NOW()
        WHERE id = $1 AND ended_at IS NULL
        "#,
    )
    .bind(chat_id)
    .execute(&state.db)
    .await?;

    if update_result.rows_affected() > 0 {
        queue::end_chat_session(state, user_a_id, user_b_id).await?;

        state
            .chat_hub
            .send(chat_id, ServerEvent::PartnerLeft { user_id: leaver_id })
            .await;
        state
            .chat_hub
            .send(
                chat_id,
                ServerEvent::Cooldown {
                    seconds: state.config.cooldown_seconds,
                },
            )
            .await;
    }

    let _ = state.chat_hub.prune_if_no_receivers(chat_id).await;

    Ok(())
}

async fn resolve_partner_payload(
    state: &AppState,
    chat_id: Uuid,
    requester_id: Uuid,
) -> AppResult<serde_json::Value> {
    // Single query: resolve the partner from the chats table and fetch their
    // profile in one round-trip instead of two separate queries.
    let row = sqlx::query_as::<
        _,
        (
            Uuid,
            String,
            String,
            bool,
            Option<Vec<String>>,
            serde_json::Value,
        ),
    >(
        r#"
        SELECT
            u.id,
            u.username,
            u.bio,
            u.is_premium,
            ARRAY(
                SELECT cat.slug
                FROM user_interests ui
                JOIN categories cat ON cat.id = ui.category_id
                WHERE ui.user_id = u.id
                ORDER BY cat.slug
            ) AS interest_slugs,
            COALESCE(
                (SELECT jsonb_object_agg(fi.item_type, fi.asset_data)
                 FROM user_flare uf
                 JOIN flare_items fi ON fi.id = uf.flare_item_id
                 WHERE uf.user_id = u.id AND uf.is_equipped = TRUE),
                '{}'::jsonb
            ) AS flare
        FROM chats c
        JOIN users u ON u.id = CASE
            WHEN c.user_a_id = $2 THEN c.user_b_id
            ELSE c.user_a_id
        END
        WHERE c.id = $1
        "#,
    )
    .bind(chat_id)
    .bind(requester_id)
    .fetch_optional(&state.db)
    .await?;

    if let Some((id, username, bio, is_premium, interest_slugs, flare)) = row {
        let badges = if is_premium { vec!["premium"] } else { vec![] };
        let interests = interest_slugs.unwrap_or_default();

        return Ok(serde_json::json!({
            "id": id,
            "username": username,
            "bio": bio,
            "badges": badges,
            "interests": interests,
            "flare": flare
        }));
    }

    Err(AppError::NotFound("chat not found".to_owned()))
}

async fn set_session_presence(state: &AppState, user_id: Uuid) -> AppResult<()> {
    let mut conn = state
        .redis
        .get()
        .await
        .map_err(|e| AppError::Internal(format!("redis pool error: {e}")))?;
    let ttl = state.config.queue_session_ttl_seconds;
    let _: () = conn.set_ex(format!("session:{user_id}"), "1", ttl).await?;
    Ok(())
}

async fn clear_session_presence(state: &AppState, user_id: Uuid) -> AppResult<()> {
    let mut conn = state
        .redis
        .get()
        .await
        .map_err(|e| AppError::Internal(format!("redis pool error: {e}")))?;
    let _: usize = conn.del(format!("session:{user_id}")).await?;
    Ok(())
}

fn keep_vote_key(chat_id: Uuid, user_id: Uuid) -> String {
    format!("keep_vote:{chat_id}:{user_id}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[tokio::test]
    async fn prune_removes_channel_when_no_receivers() {
        let hub = ChatHub::new();
        let chat_id = Uuid::new_v4();

        let _ = hub.sender_for(chat_id).await;
        assert!(hub.channel_exists(chat_id).await);

        assert!(hub.prune_if_no_receivers(chat_id).await);
        assert!(!hub.channel_exists(chat_id).await);
    }

    #[tokio::test]
    async fn prune_keeps_channel_with_active_receivers() {
        let hub = ChatHub::new();
        let chat_id = Uuid::new_v4();

        let receiver = hub.receiver_for(chat_id).await;
        assert!(hub.channel_exists(chat_id).await);

        assert!(!hub.prune_if_no_receivers(chat_id).await);
        assert!(hub.channel_exists(chat_id).await);

        drop(receiver);

        assert!(hub.prune_if_no_receivers(chat_id).await);
        assert!(!hub.channel_exists(chat_id).await);
    }

    #[tokio::test]
    async fn prune_succeeds_after_aborted_receiver_task_finishes() {
        let hub = ChatHub::new();
        let chat_id = Uuid::new_v4();

        let mut receiver = hub.receiver_for(chat_id).await;
        let task = tokio::spawn(async move {
            let _ = receiver.recv().await;
        });

        assert!(!hub.prune_if_no_receivers(chat_id).await);

        abort_and_wait(task).await;

        assert!(hub.prune_if_no_receivers(chat_id).await);
        assert!(!hub.channel_exists(chat_id).await);
    }

    #[test]
    fn parse_redis_relay_payload_skips_self_originated_events() {
        let instance_id = Uuid::new_v4();
        let payload = serde_json::to_string(&RedisRelayEvent {
            event: ServerEvent::Typing {
                user_id: Uuid::new_v4(),
                is_typing: true,
            },
            origin_instance_id: Some(instance_id),
        })
        .expect("serialize relay event");

        let parsed = parse_redis_relay_payload(&payload, instance_id).expect("parse relay payload");
        assert!(parsed.is_none());
    }

    #[test]
    fn parse_redis_relay_payload_accepts_legacy_payloads() {
        let event = ServerEvent::Message {
            id: Uuid::new_v4(),
            chat_id: Uuid::new_v4(),
            sender_id: Uuid::new_v4(),
            content: "hello".to_owned(),
            timestamp: Utc::now(),
            flagged: false,
        };
        let payload = serde_json::to_string(&event).expect("serialize legacy event");

        let parsed =
            parse_redis_relay_payload(&payload, Uuid::new_v4()).expect("parse legacy payload");
        assert!(matches!(parsed, Some(ServerEvent::Message { .. })));
    }
}
