use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    payments::ledger::{self, TxType},
    AppState,
};

const VALID_AWARD_TYPES: &[&str] = &["spark", "fire", "star", "heart", "trophy", "crown"];

#[derive(Debug, serde::Serialize)]
pub struct AwardOutcome {
    pub award_id: Uuid,
    pub recipient_id: Uuid,
    pub award_type: String,
    pub spark_amount: i64,
    pub recipient_cut: i64,
    pub sender_balance: i64,
    pub recipient_balance: i64,
}

pub async fn send_award_internal(
    state: &AppState,
    sender_id: Uuid,
    chat_id: Uuid,
    award_type: String,
    spark_amount: i64,
) -> AppResult<AwardOutcome> {
    if !VALID_AWARD_TYPES.contains(&award_type.as_str()) {
        return Err(AppError::BadRequest(
            "invalid award type".to_owned(),
        ));
    }

    if spark_amount <= 0 {
        return Err(AppError::BadRequest(
            "spark_amount must be greater than 0".to_owned(),
        ));
    }

    let chat = sqlx::query_as::<_, (Uuid, Uuid)>(
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

    if chat.0 != sender_id && chat.1 != sender_id {
        return Err(AppError::Forbidden(
            "you are not a participant in this chat".to_owned(),
        ));
    }

    let recipient_id = if chat.0 == sender_id { chat.1 } else { chat.0 };

    if recipient_id == sender_id {
        return Err(AppError::Forbidden(
            "cannot send awards to yourself".to_owned(),
        ));
    }

    let recipient_cut = (spark_amount * 70) / 100;
    let award_id = Uuid::new_v4();

    let mut tx = ledger::begin_tx(state).await?;

    let sender_balance = ledger::apply_spark_transaction(
        &mut tx,
        sender_id,
        -spark_amount,
        TxType::AwardSent,
        Some(award_id),
    )
    .await?;

    let recipient_balance = ledger::apply_spark_transaction(
        &mut tx,
        recipient_id,
        recipient_cut,
        TxType::AwardReceived,
        Some(award_id),
    )
    .await?;

    sqlx::query(
        r#"
        INSERT INTO awards (id, sender_id, recipient_id, chat_id, award_type, spark_amount, recipient_cut)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
    )
    .bind(award_id)
    .bind(sender_id)
    .bind(recipient_id)
    .bind(chat_id)
    .bind(award_type.as_str())
    .bind(spark_amount)
    .bind(recipient_cut)
    .execute(&mut *tx)
    .await?;

    ledger::commit(tx).await?;

    Ok(AwardOutcome {
        award_id,
        recipient_id,
        award_type,
        spark_amount,
        recipient_cut,
        sender_balance,
        recipient_balance,
    })
}
