use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter,
};

use crate::entity::{registration, retired_name};

use super::error::NamingError;
use super::word_list::WordList;

// Assigns the name an operator is given.
//
// Random and stored rather than derived from the node id. A derived name would let a
// banned operator recompute the same value from the same key, and would make
// reassignment impossible.
pub struct NameGenerator;

impl NameGenerator {
    // Random attempts before falling back to an exhaustive scan. Thirty draws against
    // a space that is orders of magnitude larger than the population effectively never
    // collides; the scan exists so exhaustion reports itself rather than looping.
    const RANDOM_ATTEMPTS: usize = 30;

    pub fn new() -> Self {
        Self
    }

    pub async fn assign<C: ConnectionTrait>(&self, conn: &C) -> Result<String, NamingError> {
        for _ in 0..Self::RANDOM_ATTEMPTS {
            let candidate = Self::draw();
            if Self::is_available(conn, &candidate).await? {
                return Ok(candidate);
            }
        }

        for adjective in WordList::ADJECTIVES {
            for noun in WordList::NOUNS {
                for place in WordList::PLACES {
                    let candidate = format!("{adjective}-{noun}-{place}");
                    if Self::is_available(conn, &candidate).await? {
                        return Ok(candidate);
                    }
                }
            }
        }

        Err(NamingError::Exhausted)
    }

    // Retirement is one-way. Nothing removes a row from this table.
    pub async fn retire<C: ConnectionTrait>(conn: &C, name: &str) -> Result<(), NamingError> {
        if retired_name::Entity::find_by_id(name)
            .one(conn)
            .await?
            .is_some()
        {
            return Ok(());
        }

        retired_name::ActiveModel {
            name: ActiveValue::Set(name.to_string()),
            retired_at: ActiveValue::Set(Self::now()),
        }
        .insert(conn)
        .await?;

        Ok(())
    }

    // Whether a name may still be handed out.
    //
    // False for a name a live registration holds and false for a retired one,
    // permanently. Public because "is this name available" is the question the
    // registry exists to answer, and the alternative is a probabilistic test that
    // draws repeatedly and hopes to catch a missing check.
    pub async fn is_available<C: ConnectionTrait>(
        conn: &C,
        name: &str,
    ) -> Result<bool, NamingError> {
        if retired_name::Entity::find_by_id(name)
            .one(conn)
            .await?
            .is_some()
        {
            return Ok(false);
        }

        let held = registration::Entity::find()
            .filter(registration::Column::Name.eq(name))
            .one(conn)
            .await?;

        Ok(held.is_none())
    }

    fn draw() -> String {
        let adjective = WordList::ADJECTIVES[Self::index(WordList::ADJECTIVES.len())];
        let noun = WordList::NOUNS[Self::index(WordList::NOUNS.len())];
        let place = WordList::PLACES[Self::index(WordList::PLACES.len())];
        format!("{adjective}-{noun}-{place}")
    }

    fn index(len: usize) -> usize {
        let mut bytes = [0u8; 8];
        getrandom::fill(&mut bytes).expect("the system random source is available");
        (u64::from_le_bytes(bytes) % len as u64) as usize
    }

    fn now() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or_default()
    }
}

impl Default for NameGenerator {
    fn default() -> Self {
        Self::new()
    }
}
