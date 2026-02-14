use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    AppState,
};

#[derive(Debug, Clone, Copy)]
pub enum TxType {
    Purchase,
    AwardSent,
    AwardReceived,
    FlarePurchase,
    Cashout,
}

impl TxType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Purchase => "purchase",
            Self::AwardSent => "award_sent",
            Self::AwardReceived => "award_received",
            Self::FlarePurchase => "flare_purchase",
            Self::Cashout => "cashout",
        }
    }
}

pub async fn begin_tx(state: &AppState) -> AppResult<Transaction<'_, Postgres>> {
    state.db.begin().await.map_err(Into::into)
}

pub async fn current_balance(tx: &mut Transaction<'_, Postgres>, user_id: Uuid) -> AppResult<i64> {
    let balance = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT sparks_balance
        FROM users
        WHERE id = $1
        FOR UPDATE
        "#,
    )
    .bind(user_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| AppError::NotFound("user not found".to_owned()))?;

    Ok(balance)
}

pub async fn apply_spark_transaction(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    amount: i64,
    tx_type: TxType,
    reference_id: Option<Uuid>,
) -> AppResult<i64> {
    let balance = current_balance(tx, user_id).await?;
    let next_balance = balance + amount;

    if next_balance < 0 {
        return Err(AppError::Conflict("insufficient sparks balance".to_owned()));
    }

    sqlx::query(
        r#"
        UPDATE users
        SET sparks_balance = $2, updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .bind(next_balance)
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO spark_transactions (user_id, amount, transaction_type, reference_id)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(user_id)
    .bind(amount)
    .bind(tx_type.as_str())
    .bind(reference_id)
    .execute(&mut **tx)
    .await?;

    Ok(next_balance)
}

pub async fn commit(tx: Transaction<'_, Postgres>) -> AppResult<()> {
    tx.commit().await.map_err(Into::into)
}
